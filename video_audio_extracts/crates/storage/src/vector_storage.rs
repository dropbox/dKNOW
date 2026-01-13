//! Vector storage implementation using Qdrant
//!
//! This module provides an interface for storing and searching high-dimensional
//! embeddings for semantic similarity search across video frames, audio segments,
//! and text transcriptions.

use crate::{EmbeddingVector, StorageError, StorageResult};
use qdrant_client::{
    qdrant::{
        vectors_config::Config, CreateCollectionBuilder, DeletePointsBuilder, Distance,
        GetPointsBuilder, PointStruct, SearchPointsBuilder, UpsertPointsBuilder, VectorParams,
        VectorsConfig,
    },
    Qdrant,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Qdrant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    /// Qdrant URL (e.g., "<http://localhost:6334>")
    pub url: String,

    /// API key (optional, for cloud deployment)
    pub api_key: Option<String>,

    /// Collection name
    pub collection: String,

    /// Vector dimension (e.g., 512 for CLIP, 384 for sentence-transformers)
    pub vector_dim: u64,

    /// Distance metric (Cosine, Euclidean, Dot)
    pub distance: VectorDistance,
}

/// Vector distance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorDistance {
    /// Cosine similarity (default for most embeddings)
    Cosine,
    /// Euclidean distance (L2)
    Euclidean,
    /// Dot product
    Dot,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6334".to_string()),
            api_key: std::env::var("QDRANT_API_KEY").ok(),
            collection: "media_embeddings".to_string(),
            vector_dim: 512,
            distance: VectorDistance::Cosine,
        }
    }
}

impl VectorDistance {
    fn to_qdrant_distance(&self) -> Distance {
        match self {
            VectorDistance::Cosine => Distance::Cosine,
            VectorDistance::Euclidean => Distance::Euclid,
            VectorDistance::Dot => Distance::Dot,
        }
    }
}

/// Vector storage trait
#[async_trait::async_trait]
pub trait VectorStorage: Send + Sync {
    /// Initialize collection (creates if not exists)
    async fn init_collection(&self) -> StorageResult<()>;

    /// Store a single embedding vector
    async fn store_embedding(&self, embedding: &EmbeddingVector) -> StorageResult<String>;

    /// Store multiple embedding vectors (batch)
    async fn store_embeddings(&self, embeddings: &[EmbeddingVector]) -> StorageResult<Vec<String>>;

    /// Search for similar vectors
    async fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<HashMap<String, String>>,
    ) -> StorageResult<Vec<SimilarityResult>>;

    /// Search for similar vectors by ID
    async fn search_similar_by_id(
        &self,
        vector_id: &str,
        limit: usize,
        filter: Option<HashMap<String, String>>,
    ) -> StorageResult<Vec<SimilarityResult>>;

    /// Retrieve embedding by ID
    async fn get_embedding(&self, vector_id: &str) -> StorageResult<EmbeddingVector>;

    /// Delete embedding by ID
    async fn delete_embedding(&self, vector_id: &str) -> StorageResult<()>;

    /// Delete all embeddings for a job
    async fn delete_job_embeddings(&self, job_id: &str) -> StorageResult<usize>;
}

/// Similarity search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// Vector ID
    pub vector_id: String,

    /// Similarity score (higher is more similar)
    pub score: f32,

    /// Embedding metadata
    pub metadata: HashMap<String, String>,

    /// Original embedding (optional)
    pub embedding: Option<EmbeddingVector>,
}

/// Qdrant vector storage implementation
pub struct QdrantVectorStorage {
    client: Qdrant,
    collection: String,
    vector_dim: u64,
    distance: VectorDistance,
}

impl QdrantVectorStorage {
    /// Create a new Qdrant vector storage client
    pub async fn new(config: QdrantConfig) -> StorageResult<Self> {
        let client = if let Some(api_key) = &config.api_key {
            Qdrant::from_url(&config.url)
                .api_key(api_key.clone())
                .build()
                .map_err(|e| StorageError::QdrantError(e.to_string()))?
        } else {
            Qdrant::from_url(&config.url)
                .build()
                .map_err(|e| StorageError::QdrantError(e.to_string()))?
        };

        Ok(Self {
            client,
            collection: config.collection,
            vector_dim: config.vector_dim,
            distance: config.distance,
        })
    }

    /// Generate point ID from `vector_id`
    fn point_id(&self, vector_id: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        vector_id.hash(&mut hasher);
        hasher.finish()
    }
}

#[async_trait::async_trait]
impl VectorStorage for QdrantVectorStorage {
    async fn init_collection(&self) -> StorageResult<()> {
        // Check if collection exists
        let collections = self
            .client
            .list_collections()
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        let exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection);

        if !exists {
            // Create collection
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection).vectors_config(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: self.vector_dim,
                            distance: self.distance.to_qdrant_distance().into(),
                            ..Default::default()
                        })),
                    }),
                )
                .await
                .map_err(|e| StorageError::QdrantError(e.to_string()))?;

            tracing::info!("Created Qdrant collection: {}", self.collection);
        }

        Ok(())
    }

    async fn store_embedding(&self, embedding: &EmbeddingVector) -> StorageResult<String> {
        let point_id = self.point_id(&embedding.vector_id);

        // Pre-allocate: 3 fixed fields + metadata size
        let mut payload = HashMap::with_capacity(3 + embedding.metadata.len());
        payload.insert("job_id".to_string(), embedding.job_id.clone().into());
        payload.insert("vector_id".to_string(), embedding.vector_id.clone().into());
        payload.insert(
            "embedding_type".to_string(),
            embedding.embedding_type.clone().into(),
        );

        for (k, v) in &embedding.metadata {
            payload.insert(k.clone(), v.clone().into());
        }

        let point = PointStruct::new(point_id, embedding.vector.clone(), payload);

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, vec![point]))
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        Ok(embedding.vector_id.clone())
    }

    async fn store_embeddings(&self, embeddings: &[EmbeddingVector]) -> StorageResult<Vec<String>> {
        // Pre-allocate points Vec with exact size
        let mut points = Vec::with_capacity(embeddings.len());
        points.extend(embeddings.iter().map(|embedding| {
            let point_id = self.point_id(&embedding.vector_id);

            // Pre-allocate: 3 fixed fields + metadata size
            let mut payload = HashMap::with_capacity(3 + embedding.metadata.len());
            payload.insert("job_id".to_string(), embedding.job_id.clone().into());
            payload.insert("vector_id".to_string(), embedding.vector_id.clone().into());
            payload.insert(
                "embedding_type".to_string(),
                embedding.embedding_type.clone().into(),
            );

            for (k, v) in &embedding.metadata {
                payload.insert(k.clone(), v.clone().into());
            }

            PointStruct::new(point_id, embedding.vector.clone(), payload)
        }));

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, points))
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        // Pre-allocate result Vec with exact size
        let mut result = Vec::with_capacity(embeddings.len());
        result.extend(embeddings.iter().map(|e| e.vector_id.clone()));
        Ok(result)
    }

    async fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
        filter: Option<HashMap<String, String>>,
    ) -> StorageResult<Vec<SimilarityResult>> {
        let mut search_builder =
            SearchPointsBuilder::new(&self.collection, query_vector, limit as u64)
                .with_payload(true);

        // Apply filter if provided
        if let Some(filter_map) = filter {
            use qdrant_client::qdrant::{Condition, Filter};

            // Pre-allocate conditions Vec with exact size
            let mut conditions = Vec::with_capacity(filter_map.len());
            conditions.extend(
                filter_map
                    .iter()
                    .map(|(key, value)| Condition::matches(key.clone(), value.clone())),
            );

            let filter_obj = Filter {
                must: conditions,
                ..Default::default()
            };

            search_builder = search_builder.filter(filter_obj);
        }

        let search_result = self
            .client
            .search_points(search_builder)
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        // Pre-allocate results Vec with exact size
        let mut results = Vec::with_capacity(search_result.result.len());
        results.extend(search_result.result.into_iter().map(|scored_point| {
            // Pre-allocate metadata with payload size
            let mut metadata = HashMap::with_capacity(scored_point.payload.len());

            for (key, value) in scored_point.payload {
                if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = value.kind {
                    metadata.insert(key, s);
                }
            }

            let vector_id = metadata.get("vector_id").cloned().unwrap_or_default();

            SimilarityResult {
                vector_id,
                score: scored_point.score,
                metadata,
                embedding: None,
            }
        }));

        Ok(results)
    }

    async fn search_similar_by_id(
        &self,
        vector_id: &str,
        limit: usize,
        filter: Option<HashMap<String, String>>,
    ) -> StorageResult<Vec<SimilarityResult>> {
        // First retrieve the embedding
        let embedding = self.get_embedding(vector_id).await?;

        // Then search with that vector
        self.search_similar(&embedding.vector, limit, filter).await
    }

    async fn get_embedding(&self, vector_id: &str) -> StorageResult<EmbeddingVector> {
        let point_id = self.point_id(vector_id);

        let points = self
            .client
            .get_points(
                GetPointsBuilder::new(&self.collection, vec![point_id.into()])
                    .with_payload(true)
                    .with_vectors(true),
            )
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        let point = points
            .result
            .first()
            .ok_or_else(|| StorageError::NotFound(vector_id.to_string()))?;

        // Pre-allocate metadata with payload size (most will be metadata)
        let mut metadata = HashMap::with_capacity(point.payload.len());
        let mut job_id = String::new();
        let mut embedding_type = String::new();

        for (key, value) in &point.payload {
            if let Some(qdrant_client::qdrant::value::Kind::StringValue(s)) = &value.kind {
                match key.as_str() {
                    "job_id" => job_id = s.clone(),
                    "vector_id" => {}
                    "embedding_type" => embedding_type = s.clone(),
                    _ => {
                        metadata.insert(key.clone(), s.clone());
                    }
                }
            }
        }

        let vector = point
            .vectors
            .as_ref()
            .and_then(|v| v.vectors_options.as_ref())
            .and_then(|opts| match opts {
                qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v) => {
                    Some(v.data.clone())
                }
                _ => None,
            })
            .ok_or_else(|| StorageError::QdrantError("No vector data found".to_string()))?;

        Ok(EmbeddingVector {
            job_id,
            vector_id: vector_id.to_string(),
            embedding_type,
            vector,
            metadata,
        })
    }

    async fn delete_embedding(&self, vector_id: &str) -> StorageResult<()> {
        use qdrant_client::qdrant::PointId;

        let point_id = self.point_id(vector_id);

        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection).points(vec![PointId::from(point_id)]),
            )
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        Ok(())
    }

    async fn delete_job_embeddings(&self, job_id: &str) -> StorageResult<usize> {
        use qdrant_client::qdrant::{Condition, Filter};

        let filter = Filter {
            must: vec![Condition::matches("job_id", job_id.to_string())],
            ..Default::default()
        };

        self.client
            .delete_points(DeletePointsBuilder::new(&self.collection).points(filter))
            .await
            .map_err(|e| StorageError::QdrantError(e.to_string()))?;

        // Qdrant doesn't return count in delete response
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qdrant_config_default() {
        let config = QdrantConfig::default();
        assert_eq!(config.collection, "media_embeddings");
        assert_eq!(config.vector_dim, 512);
    }

    #[test]
    fn test_vector_distance_conversion() {
        assert_eq!(
            VectorDistance::Cosine.to_qdrant_distance(),
            Distance::Cosine
        );
        assert_eq!(
            VectorDistance::Euclidean.to_qdrant_distance(),
            Distance::Euclid
        );
        assert_eq!(VectorDistance::Dot.to_qdrant_distance(), Distance::Dot);
    }

    #[test]
    fn test_point_id_generation() {
        let config = QdrantConfig::default();
        let client = Qdrant::from_url(&config.url).build().unwrap();
        let storage = QdrantVectorStorage {
            client,
            collection: config.collection,
            vector_dim: config.vector_dim,
            distance: config.distance,
        };

        let id1 = storage.point_id("test_vector_1");
        let id2 = storage.point_id("test_vector_1");
        let id3 = storage.point_id("test_vector_2");

        // Same input should produce same ID
        assert_eq!(id1, id2);

        // Different input should produce different ID
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_similarity_result_creation() {
        let result = SimilarityResult {
            vector_id: "vec_1".to_string(),
            score: 0.95,
            metadata: HashMap::new(),
            embedding: None,
        };

        assert_eq!(result.vector_id, "vec_1");
        assert_eq!(result.score, 0.95);
    }
}
