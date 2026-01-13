//! SQLite storage for documents, embeddings, and index state
//!
//! Schema:
//! - documents: id, path, hash, content, line_count
//! - embeddings: doc_id -> blob of f32 vectors (num_tokens x 128)
//! - centroids: cluster centers for k-means index
//! - index_state: metadata about index health

// Allow complex tuple types in batch operations - these pass chunk data through
// the indexing pipeline and refactoring to named types would add complexity
// without improving clarity.
#![allow(clippy::type_complexity)]

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

const SCHEMA: &str = r"
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    hash TEXT NOT NULL,
    content TEXT NOT NULL,
    line_count INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS embeddings (
    doc_id INTEGER PRIMARY KEY,
    data BLOB NOT NULL,
    num_tokens INTEGER NOT NULL,
    FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chunks (
    id INTEGER PRIMARY KEY,
    doc_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    header_context TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',
    language TEXT,
    links TEXT,
    FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE,
    UNIQUE(doc_id, chunk_index)
);

CREATE TABLE IF NOT EXISTS chunk_embeddings (
    chunk_id INTEGER PRIMARY KEY,
    data BLOB NOT NULL,
    num_tokens INTEGER NOT NULL,
    FOREIGN KEY (chunk_id) REFERENCES chunks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS centroids (
    id INTEGER PRIMARY KEY,
    data BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS index_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    path,
    content,
    content='documents',
    content_rowid='id'
);

CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    hash TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    indexed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE TABLE IF NOT EXISTS image_embeddings (
    image_id INTEGER PRIMARY KEY,
    data BLOB NOT NULL,
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS file_summaries (
    id INTEGER PRIMARY KEY,
    doc_id INTEGER NOT NULL,
    summary TEXT NOT NULL,
    source TEXT NOT NULL,
    model TEXT,
    hash TEXT NOT NULL,
    storage_tier TEXT NOT NULL DEFAULT 'sqlite',
    generated_at INTEGER NOT NULL,
    FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE,
    UNIQUE(doc_id)
);

CREATE INDEX IF NOT EXISTS idx_documents_path ON documents(path);
CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(hash);
CREATE INDEX IF NOT EXISTS idx_chunks_doc_id ON chunks(doc_id);
CREATE INDEX IF NOT EXISTS idx_images_path ON images(path);
CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_summaries_doc_id ON file_summaries(doc_id);
";

/// Database connection wrapper
pub struct DB {
    conn: Connection,
}

impl DB {
    /// Open or create database at path
    pub fn new(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;

        // Enable foreign keys and WAL mode for performance
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )?;

        // Create schema
        conn.execute_batch(SCHEMA)?;

        // Run migrations for existing databases
        run_migrations(&conn)?;

        ensure_fts_index(&conn)?;

        Ok(Self { conn })
    }

    /// Open in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        ensure_fts_index(&conn)?;
        Ok(Self { conn })
    }

    /// Add a document to the database
    ///
    /// Returns the document ID. If the document already exists with the same
    /// hash, returns the existing ID without re-adding.
    pub fn add_document(&self, path: &Path, content: &str) -> Result<u32> {
        let path_str = path.to_string_lossy();
        let hash = compute_hash(content);
        let line_count = content.lines().count() as i64;

        // Check if document exists with same hash
        if let Some(existing) = self.get_document_by_path(path)? {
            if existing.hash == hash {
                return Ok(existing.id);
            }
            // Hash changed, update the document
            self.conn.execute(
                "UPDATE documents SET hash = ?, content = ?, line_count = ?, indexed_at = strftime('%s', 'now') WHERE id = ?",
                params![hash, content, line_count, existing.id],
            )?;
            upsert_document_fts(&self.conn, existing.id, path_str.as_ref(), content)?;
            // Delete old embeddings
            self.conn.execute(
                "DELETE FROM embeddings WHERE doc_id = ?",
                params![existing.id],
            )?;
            return Ok(existing.id);
        }

        // Insert new document
        self.conn.execute(
            "INSERT INTO documents (path, hash, content, line_count) VALUES (?, ?, ?, ?)",
            params![path_str.as_ref(), hash, content, line_count],
        )?;

        let doc_id = self.conn.last_insert_rowid() as u32;
        upsert_document_fts(&self.conn, doc_id, path_str.as_ref(), content)?;
        Ok(doc_id)
    }

    /// Get a document by ID
    pub fn get_document(&self, id: u32) -> Result<Option<Document>> {
        self.conn
            .query_row(
                "SELECT id, path, hash, content, line_count FROM documents WHERE id = ?",
                params![id],
                |row| {
                    Ok(Document {
                        id: row.get::<_, i64>(0)? as u32,
                        path: row.get(1)?,
                        hash: row.get(2)?,
                        content: row.get(3)?,
                        line_count: row.get::<_, i64>(4)? as usize,
                    })
                },
            )
            .optional()
            .context("Failed to get document")
    }

    /// Get a document by path
    pub fn get_document_by_path(&self, path: &Path) -> Result<Option<Document>> {
        let path_str = path.to_string_lossy();
        self.conn
            .query_row(
                "SELECT id, path, hash, content, line_count FROM documents WHERE path = ?",
                params![path_str.as_ref()],
                |row| {
                    Ok(Document {
                        id: row.get::<_, i64>(0)? as u32,
                        path: row.get(1)?,
                        hash: row.get(2)?,
                        content: row.get(3)?,
                        line_count: row.get::<_, i64>(4)? as usize,
                    })
                },
            )
            .optional()
            .context("Failed to get document by path")
    }

    /// Remove a document by path
    pub fn remove_document(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        self.conn.execute(
            "DELETE FROM documents_fts WHERE rowid = (SELECT id FROM documents WHERE path = ?)",
            params![path_str.as_ref()],
        )?;
        let rows = self.conn.execute(
            "DELETE FROM documents WHERE path = ?",
            params![path_str.as_ref()],
        )?;
        Ok(rows > 0)
    }

    /// Remove all documents whose path starts with the given prefix
    ///
    /// Used for evicting entire projects from the index.
    /// Returns the number of documents removed.
    pub fn remove_documents_by_prefix(&self, prefix: &Path) -> Result<usize> {
        let prefix_str = path_prefix(prefix);
        let like_pattern = format!("{prefix_str}%");
        self.conn.execute(
            "DELETE FROM documents_fts WHERE rowid IN (SELECT id FROM documents WHERE path LIKE ?)",
            params![like_pattern],
        )?;
        let rows = self.conn.execute(
            "DELETE FROM documents WHERE path LIKE ?",
            params![like_pattern],
        )?;
        Ok(rows)
    }

    /// Search documents using the FTS index, returning ranked matches.
    pub fn search_documents_fts(&self, query: &str, limit: usize) -> Result<Vec<FtsDocumentHit>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.path, d.content, bm25(documents_fts) AS score
             FROM documents_fts
             JOIN documents d ON documents_fts.rowid = d.id
             WHERE documents_fts MATCH ?
             ORDER BY score
             LIMIT ?",
        )?;

        let mut rows = stmt.query(params![query, limit as i64])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(FtsDocumentHit {
                id: row.get::<_, i64>(0)? as u32,
                path: row.get(1)?,
                content: row.get(2)?,
                score: row.get::<_, f32>(3)?,
            });
        }
        Ok(results)
    }

    /// Add embeddings for a document
    ///
    /// Embeddings are stored as a blob of f32 values in row-major order.
    /// Shape: [num_tokens, 128]
    pub fn add_embeddings(&self, doc_id: u32, embeddings: &[f32], num_tokens: usize) -> Result<()> {
        // Convert f32 slice to bytes
        let bytes: Vec<u8> = embeddings.iter().flat_map(|f| f.to_le_bytes()).collect();

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings (doc_id, data, num_tokens) VALUES (?, ?, ?)",
            params![doc_id, bytes, num_tokens as i64],
        )?;

        Ok(())
    }

    /// Get embeddings for a document
    ///
    /// Returns (embeddings, num_tokens) where embeddings is [num_tokens * 128] f32 values
    pub fn get_embeddings(&self, doc_id: u32) -> Result<Option<(Vec<f32>, usize)>> {
        let result: Option<(Vec<u8>, i64)> = self
            .conn
            .query_row(
                "SELECT data, num_tokens FROM embeddings WHERE doc_id = ?",
                params![doc_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match result {
            Some((bytes, num_tokens)) => {
                // Convert bytes back to f32
                let embeddings: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some((embeddings, num_tokens as usize)))
            }
            None => Ok(None),
        }
    }

    /// Check if a document needs re-indexing (hash changed)
    pub fn needs_reindex(&self, path: &Path, content: &str) -> Result<bool> {
        let hash = compute_hash(content);
        match self.get_document_by_path(path)? {
            Some(doc) => Ok(doc.hash != hash),
            None => Ok(true), // New document
        }
    }

    /// Get all document IDs
    pub fn get_all_doc_ids(&self) -> Result<Vec<u32>> {
        let mut stmt = self.conn.prepare_cached("SELECT id FROM documents")?;
        let ids = stmt
            .query_map([], |row| row.get::<_, i64>(0))?
            .filter_map(Result::ok)
            .map(|id| id as u32)
            .collect();
        Ok(ids)
    }

    /// List all documents with chunk counts
    ///
    /// Optional path prefix to filter by directory.
    pub fn list_documents(&self, path_prefix: Option<&Path>) -> Result<Vec<DocumentSummary>> {
        let row_mapper = |row: &rusqlite::Row| -> rusqlite::Result<DocumentSummary> {
            Ok(DocumentSummary {
                id: row.get::<_, i64>(0)? as u32,
                path: row.get(1)?,
                line_count: row.get::<_, i64>(2)? as usize,
                indexed_at: row.get(3)?,
                chunk_count: row.get::<_, i64>(4)? as usize,
            })
        };

        let documents = if let Some(prefix) = path_prefix {
            let sql = "SELECT d.id, d.path, d.line_count, d.indexed_at, COUNT(c.id) as chunk_count
                 FROM documents d
                 LEFT JOIN chunks c ON c.doc_id = d.id
                 WHERE d.path LIKE ?
                 GROUP BY d.id
                 ORDER BY d.path";
            let mut stmt = self.conn.prepare_cached(sql)?;
            let prefix_str = format!("{}%", prefix.display());
            let rows = stmt.query_map(params![prefix_str], row_mapper)?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let sql = "SELECT d.id, d.path, d.line_count, d.indexed_at, COUNT(c.id) as chunk_count
                 FROM documents d
                 LEFT JOIN chunks c ON c.doc_id = d.id
                 GROUP BY d.id
                 ORDER BY d.path";
            let mut stmt = self.conn.prepare_cached(sql)?;
            let rows = stmt.query_map([], row_mapper)?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(documents)
    }

    /// Get document count
    pub fn document_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get total line count
    pub fn total_lines(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(line_count), 0) FROM documents",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Save index state
    pub fn set_index_state(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO index_state (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get index state
    pub fn get_index_state(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM index_state WHERE key = ?",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to get index state")
    }

    /// Save centroids
    pub fn save_centroids(&self, centroids: &[f32], num_centroids: usize) -> Result<()> {
        // Clear existing centroids
        self.conn.execute("DELETE FROM centroids", [])?;

        // Convert to bytes and save
        let bytes: Vec<u8> = centroids.iter().flat_map(|f| f.to_le_bytes()).collect();

        self.conn.execute(
            "INSERT INTO centroids (id, data) VALUES (0, ?)",
            params![bytes],
        )?;

        self.set_index_state("num_centroids", &num_centroids.to_string())?;
        Ok(())
    }

    /// Load centroids
    pub fn load_centroids(&self) -> Result<Option<(Vec<f32>, usize)>> {
        let num_centroids: Option<String> = self.get_index_state("num_centroids")?;
        let num_centroids = match num_centroids {
            Some(s) => s.parse::<usize>().unwrap_or(0),
            None => return Ok(None),
        };

        if num_centroids == 0 {
            return Ok(None);
        }

        let bytes: Option<Vec<u8>> = self
            .conn
            .query_row("SELECT data FROM centroids WHERE id = 0", [], |row| {
                row.get(0)
            })
            .optional()?;

        match bytes {
            Some(bytes) => {
                let centroids: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some((centroids, num_centroids)))
            }
            None => Ok(None),
        }
    }

    // ==================== Chunk Methods ====================

    /// Add a chunk record for a document
    pub fn add_chunk(
        &self,
        doc_id: u32,
        chunk_index: usize,
        start_line: usize,
        end_line: usize,
        header_context: &str,
    ) -> Result<u32> {
        self.add_chunk_full(
            doc_id,
            chunk_index,
            start_line,
            end_line,
            header_context,
            "",
            None,
        )
    }

    /// Add a chunk record with content hash for differential updates
    pub fn add_chunk_with_hash(
        &self,
        doc_id: u32,
        chunk_index: usize,
        start_line: usize,
        end_line: usize,
        header_context: &str,
        content_hash: &str,
    ) -> Result<u32> {
        self.add_chunk_full(
            doc_id,
            chunk_index,
            start_line,
            end_line,
            header_context,
            content_hash,
            None,
        )
    }

    /// Add a chunk record with all fields including language
    pub fn add_chunk_full(
        &self,
        doc_id: u32,
        chunk_index: usize,
        start_line: usize,
        end_line: usize,
        header_context: &str,
        content_hash: &str,
        language: Option<&str>,
    ) -> Result<u32> {
        self.add_chunk_with_links(
            doc_id,
            chunk_index,
            start_line,
            end_line,
            header_context,
            content_hash,
            language,
            &[],
        )
    }

    /// Add a chunk record with all fields including language and links
    pub fn add_chunk_with_links(
        &self,
        doc_id: u32,
        chunk_index: usize,
        start_line: usize,
        end_line: usize,
        header_context: &str,
        content_hash: &str,
        language: Option<&str>,
        links: &[StoredLink],
    ) -> Result<u32> {
        let links_json = encode_links(links)?;

        self.conn.execute(
            "INSERT INTO chunks (doc_id, chunk_index, start_line, end_line, header_context, content_hash, language, links) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![doc_id, chunk_index as i64, start_line as i64, end_line as i64, header_context, content_hash, language, links_json],
        )?;
        Ok(self.conn.last_insert_rowid() as u32)
    }

    /// Get a chunk by ID
    pub fn get_chunk(&self, chunk_id: u32) -> Result<Option<ChunkRecord>> {
        self.conn
            .query_row(
                "SELECT id, doc_id, chunk_index, start_line, end_line, header_context, content_hash, language, links FROM chunks WHERE id = ?",
                params![chunk_id],
                |row| {
                    let row_chunk_id = row.get::<_, i64>(0)? as u32;
                    let links_json: Option<String> = row.get(8)?;
                    let links = decode_links_with_context(links_json, row_chunk_id)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

                    Ok(ChunkRecord {
                        id: row_chunk_id,
                        doc_id: row.get::<_, i64>(1)? as u32,
                        chunk_index: row.get::<_, i64>(2)? as usize,
                        start_line: row.get::<_, i64>(3)? as usize,
                        end_line: row.get::<_, i64>(4)? as usize,
                        header_context: row.get::<_, String>(5)?,
                        content_hash: row.get::<_, String>(6).unwrap_or_default(),
                        language: row.get::<_, Option<String>>(7)?,
                        links,
                    })
                },
            )
            .optional()
            .context("Failed to get chunk")
    }

    /// Get all chunk IDs in the database
    pub fn get_all_chunk_ids(&self) -> Result<Vec<u32>> {
        let mut stmt = self.conn.prepare_cached("SELECT id FROM chunks")?;
        let ids = stmt
            .query_map([], |row| row.get::<_, i64>(0))?
            .filter_map(Result::ok)
            .map(|id| id as u32)
            .collect();
        Ok(ids)
    }

    /// Add embeddings for a chunk
    pub fn add_chunk_embeddings(
        &self,
        chunk_id: u32,
        embeddings: &[f32],
        num_tokens: usize,
    ) -> Result<()> {
        let bytes: Vec<u8> = embeddings.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.conn.execute(
            "INSERT OR REPLACE INTO chunk_embeddings (chunk_id, data, num_tokens) VALUES (?, ?, ?)",
            params![chunk_id, bytes, num_tokens as i64],
        )?;
        Ok(())
    }

    /// Get embeddings for a chunk
    pub fn get_chunk_embeddings(&self, chunk_id: u32) -> Result<Option<(Vec<f32>, usize)>> {
        let result: Option<(Vec<u8>, i64)> = self
            .conn
            .query_row(
                "SELECT data, num_tokens FROM chunk_embeddings WHERE chunk_id = ?",
                params![chunk_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match result {
            Some((bytes, num_tokens)) => {
                let embeddings: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some((embeddings, num_tokens as usize)))
            }
            None => Ok(None),
        }
    }

    /// Get all chunk embeddings with metadata
    pub fn get_all_chunk_embeddings(&self) -> Result<Vec<ChunkEmbeddingRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT chunks.id, chunks.doc_id, chunks.chunk_index, chunks.start_line, chunks.end_line, chunks.header_context, chunk_embeddings.data, chunk_embeddings.num_tokens, chunks.language, chunks.links \
             FROM chunks \
             JOIN chunk_embeddings ON chunk_embeddings.chunk_id = chunks.id \
             ORDER BY chunks.doc_id, chunks.chunk_index",
        )?;

        let rows = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(6)?;
            let embeddings: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            let chunk_id = row.get::<_, i64>(0)? as u32;
            let links_json: Option<String> = row.get(9)?;
            let links = decode_links_with_context(links_json, chunk_id)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
            Ok(ChunkEmbeddingRecord {
                chunk_id,
                doc_id: row.get::<_, i64>(1)? as u32,
                chunk_index: row.get::<_, i64>(2)? as usize,
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                header_context: row.get::<_, String>(5)?,
                embeddings,
                num_tokens: row.get::<_, i64>(7)? as usize,
                language: row.get::<_, Option<String>>(8)?,
                links,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        Ok(records)
    }

    /// Get all chunks for a document
    pub fn get_chunks_for_doc(&self, doc_id: u32) -> Result<Vec<ChunkRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, doc_id, chunk_index, start_line, end_line, header_context, content_hash, language, links FROM chunks WHERE doc_id = ? ORDER BY chunk_index",
        )?;
        let rows = stmt.query_map(params![doc_id], |row| {
            let chunk_id = row.get::<_, i64>(0)? as u32;
            let links_json: Option<String> = row.get(8)?;
            let links = decode_links_with_context(links_json, chunk_id)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
            Ok(ChunkRecord {
                id: chunk_id,
                doc_id: row.get::<_, i64>(1)? as u32,
                chunk_index: row.get::<_, i64>(2)? as usize,
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                header_context: row.get::<_, String>(5)?,
                content_hash: row.get::<_, String>(6).unwrap_or_default(),
                language: row.get::<_, Option<String>>(7)?,
                links,
            })
        })?;
        let mut chunks = Vec::new();
        for row in rows {
            chunks.push(row?);
        }
        Ok(chunks)
    }

    /// Get chunk content hashes for a document (for differential updates)
    ///
    /// Returns a map of content_hash -> chunk_id for existing chunks.
    /// This allows fast lookup of unchanged chunks during re-indexing.
    pub fn get_chunk_hashes_for_doc(
        &self,
        doc_id: u32,
    ) -> Result<std::collections::HashMap<String, u32>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, content_hash FROM chunks WHERE doc_id = ?")?;
        let mut hash_map = std::collections::HashMap::new();
        let rows = stmt.query_map(params![doc_id], |row| {
            Ok((row.get::<_, i64>(0)? as u32, row.get::<_, String>(1)?))
        })?;
        for (id, hash) in rows.flatten().filter(|(_, h)| !h.is_empty()) {
            hash_map.insert(hash, id);
        }
        Ok(hash_map)
    }

    /// Delete a specific chunk by ID
    pub fn delete_chunk(&self, chunk_id: u32) -> Result<()> {
        self.conn
            .execute("DELETE FROM chunks WHERE id = ?", params![chunk_id])?;
        Ok(())
    }

    /// Delete all chunks for a document (for re-indexing)
    pub fn delete_chunks_for_doc(&self, doc_id: u32) -> Result<()> {
        // chunk_embeddings are cascade-deleted automatically
        self.conn
            .execute("DELETE FROM chunks WHERE doc_id = ?", params![doc_id])?;
        Ok(())
    }

    /// Batch add chunks with embeddings in a single transaction
    ///
    /// This is significantly faster than individual inserts (5-10x) due to
    /// reduced transaction overhead and fsync calls.
    ///
    /// Format: (index, start, end, header, embeddings, num_tokens)
    /// For differential updates, use batch_add_chunks_with_hashes instead.
    pub fn batch_add_chunks_with_embeddings(
        &self,
        doc_id: u32,
        chunks: &[(usize, usize, usize, &str, &[f32], usize)], // index, start, end, header, embeddings, num_tokens
    ) -> Result<()> {
        // Convert to format with empty hashes and no language
        let with_hashes: Vec<(usize, usize, usize, &str, &str, Option<&str>, &[f32], usize)> =
            chunks
                .iter()
                .map(|(idx, start, end, header, emb, n_tok)| {
                    (*idx, *start, *end, *header, "", None, *emb, *n_tok)
                })
                .collect();
        self.batch_add_chunks_full(doc_id, &with_hashes)
    }

    /// Batch add chunks with embeddings and content hashes in a single transaction
    ///
    /// This is the full version that stores content hashes for differential updates.
    /// Format: (index, start, end, header, content_hash, embeddings, num_tokens)
    pub fn batch_add_chunks_with_hashes(
        &self,
        doc_id: u32,
        chunks: &[(usize, usize, usize, &str, &str, &[f32], usize)], // index, start, end, header, hash, embeddings, num_tokens
    ) -> Result<()> {
        // Convert to format with no language
        let with_lang: Vec<(usize, usize, usize, &str, &str, Option<&str>, &[f32], usize)> = chunks
            .iter()
            .map(|(idx, start, end, header, hash, emb, n_tok)| {
                (*idx, *start, *end, *header, *hash, None, *emb, *n_tok)
            })
            .collect();
        self.batch_add_chunks_full(doc_id, &with_lang)
    }

    /// Batch add chunks with all fields including language
    ///
    /// Format: (index, start, end, header, content_hash, language, embeddings, num_tokens)
    pub fn batch_add_chunks_full(
        &self,
        doc_id: u32,
        chunks: &[(usize, usize, usize, &str, &str, Option<&str>, &[f32], usize)],
    ) -> Result<()> {
        // Convert to format with empty links
        let with_links: Vec<_> = chunks
            .iter()
            .map(|(idx, start, end, header, hash, lang, emb, n_tok)| {
                (
                    *idx,
                    *start,
                    *end,
                    *header,
                    *hash,
                    *lang,
                    Vec::new(),
                    *emb,
                    *n_tok,
                )
            })
            .collect();
        self.batch_add_chunks_with_links(doc_id, &with_links)
    }

    /// Batch add chunks with all fields including language and links
    ///
    /// Format: (index, start, end, header, content_hash, language, links, embeddings, num_tokens)
    pub fn batch_add_chunks_with_links(
        &self,
        doc_id: u32,
        chunks: &[(
            usize,
            usize,
            usize,
            &str,
            &str,
            Option<&str>,
            Vec<StoredLink>,
            &[f32],
            usize,
        )],
    ) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        // Use a savepoint for transaction-like behavior
        self.conn.execute("SAVEPOINT batch_chunks", [])?;

        let result = (|| -> Result<()> {
            for (
                chunk_index,
                start_line,
                end_line,
                header_context,
                content_hash,
                language,
                links,
                embeddings,
                num_tokens,
            ) in chunks
            {
                // Serialize links to JSON
                let links_json = encode_links(links)?;

                // Insert chunk with all fields
                self.conn.execute(
                    "INSERT INTO chunks (doc_id, chunk_index, start_line, end_line, header_context, content_hash, language, links) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    params![doc_id, *chunk_index as i64, *start_line as i64, *end_line as i64, *header_context, *content_hash, *language, links_json],
                )?;
                let chunk_id = self.conn.last_insert_rowid();

                // Insert embeddings
                let bytes: Vec<u8> = embeddings.iter().flat_map(|f| f.to_le_bytes()).collect();
                self.conn.execute(
                    "INSERT INTO chunk_embeddings (chunk_id, data, num_tokens) VALUES (?, ?, ?)",
                    params![chunk_id, bytes, *num_tokens as i64],
                )?;
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute("RELEASE SAVEPOINT batch_chunks", [])?;
                Ok(())
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK TO SAVEPOINT batch_chunks", []);
                Err(e)
            }
        }
    }

    /// Get total chunk count
    pub fn chunk_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Compact the database by removing orphaned data and reclaiming space
    ///
    /// This method:
    /// 1. Removes orphaned embeddings (embeddings without a document)
    /// 2. Removes orphaned chunks (chunks without a document)
    /// 3. Removes orphaned chunk_embeddings (chunk_embeddings without a chunk)
    /// 4. Runs VACUUM to reclaim disk space
    /// 5. Runs ANALYZE to update query planner statistics
    ///
    /// Returns compaction statistics.
    pub fn compact(&mut self) -> Result<CompactionStats> {
        // Get initial sizes upfront
        let chunks_before = self.chunk_count()?;
        let documents_before = self.document_count()?;
        let page_count_before: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let size_before_bytes = (page_count_before * page_size) as u64;

        let mut stats = CompactionStats {
            chunks_before,
            documents_before,
            size_before_bytes,
            ..Default::default()
        };

        // Remove orphaned embeddings (doc-level, if any - legacy)
        let orphaned_embeddings = self.conn.execute(
            "DELETE FROM embeddings WHERE doc_id NOT IN (SELECT id FROM documents)",
            [],
        )?;
        stats.orphaned_embeddings_removed = orphaned_embeddings;

        // Remove orphaned chunks (chunks whose document was deleted but CASCADE didn't fire)
        let orphaned_chunks = self.conn.execute(
            "DELETE FROM chunks WHERE doc_id NOT IN (SELECT id FROM documents)",
            [],
        )?;
        stats.orphaned_chunks_removed = orphaned_chunks;

        // Remove orphaned chunk_embeddings (chunk_embeddings whose chunk was deleted)
        let orphaned_chunk_embeddings = self.conn.execute(
            "DELETE FROM chunk_embeddings WHERE chunk_id NOT IN (SELECT id FROM chunks)",
            [],
        )?;
        stats.orphaned_chunk_embeddings_removed = orphaned_chunk_embeddings;

        // Remove stale centroids if there are no embeddings
        let has_embeddings: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM chunk_embeddings", [], |row| {
                    row.get(0)
                })?;
        if has_embeddings == 0 {
            let stale_centroids = self.conn.execute("DELETE FROM centroids", [])?;
            stats.stale_centroids_removed = stale_centroids;
        }

        // Run VACUUM to reclaim space (requires exclusive access, no active transactions)
        self.conn.execute("VACUUM", [])?;

        // Run ANALYZE to update statistics for query planner
        self.conn.execute("ANALYZE", [])?;

        // Get sizes after compaction
        stats.chunks_after = self.chunk_count()?;
        stats.documents_after = self.document_count()?;

        let page_count_after: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        stats.size_after_bytes = (page_count_after * page_size) as u64;

        Ok(stats)
    }

    /// Get database file size in bytes
    pub fn database_size(&self) -> Result<u64> {
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        Ok((page_count * page_size) as u64)
    }
}

/// Statistics from a compaction operation
#[derive(Debug, Default, Clone)]
pub struct CompactionStats {
    /// Number of chunks before compaction
    pub chunks_before: usize,
    /// Number of chunks after compaction
    pub chunks_after: usize,
    /// Number of documents before compaction
    pub documents_before: usize,
    /// Number of documents after compaction
    pub documents_after: usize,
    /// Database size before compaction (bytes)
    pub size_before_bytes: u64,
    /// Database size after compaction (bytes)
    pub size_after_bytes: u64,
    /// Number of orphaned embeddings removed
    pub orphaned_embeddings_removed: usize,
    /// Number of orphaned chunks removed
    pub orphaned_chunks_removed: usize,
    /// Number of orphaned chunk_embeddings removed
    pub orphaned_chunk_embeddings_removed: usize,
    /// Number of stale centroids removed
    pub stale_centroids_removed: usize,
}

impl CompactionStats {
    /// Total number of orphaned records removed
    pub fn total_removed(&self) -> usize {
        self.orphaned_embeddings_removed
            + self.orphaned_chunks_removed
            + self.orphaned_chunk_embeddings_removed
            + self.stale_centroids_removed
    }

    /// Space reclaimed in bytes
    pub fn space_reclaimed(&self) -> u64 {
        self.size_before_bytes.saturating_sub(self.size_after_bytes)
    }

    /// Format size in human-readable form
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{bytes} bytes")
        }
    }
}

/// Document record
#[derive(Debug, Clone)]
pub struct Document {
    pub id: u32,
    pub path: String,
    pub hash: String,
    pub content: String,
    pub line_count: usize,
}

/// FTS search hit for a document
#[derive(Debug, Clone)]
pub struct FtsDocumentHit {
    pub id: u32,
    pub path: String,
    pub content: String,
    pub score: f32,
}

/// Document summary with chunk count (for listing indexed files)
#[derive(Debug, Clone)]
pub struct DocumentSummary {
    pub id: u32,
    pub path: String,
    pub line_count: usize,
    pub chunk_count: usize,
    pub indexed_at: i64,
}

/// Compute SHA-256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn upsert_document_fts(conn: &Connection, doc_id: u32, path: &str, content: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO documents_fts (rowid, path, content) VALUES (?, ?, ?)",
        params![doc_id, path, content],
    )?;
    Ok(())
}

/// Run database migrations for schema changes
///
/// This adds new columns to existing databases without losing data.
fn run_migrations(conn: &Connection) -> Result<()> {
    // Migration 1: Add language column to chunks table
    // Check if column exists by querying table info
    let has_language: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('chunks') WHERE name = 'language'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !has_language {
        conn.execute("ALTER TABLE chunks ADD COLUMN language TEXT", [])?;
    }

    // Migration 2: Add links column to chunks table (stores JSON array of links)
    let has_links: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('chunks') WHERE name = 'links'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !has_links {
        conn.execute("ALTER TABLE chunks ADD COLUMN links TEXT", [])?;
    }

    // Migration 3: Add file_summaries table
    let has_summaries: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_summaries'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false);

    if !has_summaries {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_summaries (
                id INTEGER PRIMARY KEY,
                doc_id INTEGER NOT NULL,
                summary TEXT NOT NULL,
                source TEXT NOT NULL,
                model TEXT,
                hash TEXT NOT NULL,
                storage_tier TEXT NOT NULL DEFAULT 'sqlite',
                generated_at INTEGER NOT NULL,
                FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE,
                UNIQUE(doc_id)
            );
            CREATE INDEX IF NOT EXISTS idx_summaries_doc_id ON file_summaries(doc_id);",
        )?;
    }

    Ok(())
}

fn ensure_fts_index(conn: &Connection) -> Result<()> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT value FROM index_state WHERE key = 'fts5_built'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    if existing.is_none() {
        conn.execute(
            "INSERT INTO documents_fts(documents_fts) VALUES ('rebuild')",
            [],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO index_state (key, value) VALUES ('fts5_built', '1')",
            [],
        )?;
    }

    Ok(())
}

/// A stored link from chunk content
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredLink {
    /// The display text of the link
    pub text: String,
    /// The link target (URL, path, or wiki-style reference)
    pub target: String,
    /// Whether this is an internal link (relative path) vs external (URL)
    pub is_internal: bool,
}

fn encode_links(links: &[StoredLink]) -> Result<Option<String>> {
    if links.is_empty() {
        return Ok(None);
    }

    serde_json::to_string(links)
        .map(Some)
        .context("Failed to serialize chunk links")
}

fn decode_links(links_json: Option<String>) -> Result<Vec<StoredLink>> {
    match links_json {
        None => Ok(Vec::new()),
        Some(raw) => serde_json::from_str(&raw).context("Failed to deserialize chunk links"),
    }
}

fn decode_links_with_context(links_json: Option<String>, chunk_id: u32) -> Result<Vec<StoredLink>> {
    decode_links(links_json)
        .with_context(|| format!("Failed to decode links for chunk {chunk_id}"))
}

/// Chunk record - a portion of a document for embedding
#[derive(Debug, Clone)]
pub struct ChunkRecord {
    pub id: u32,
    pub doc_id: u32,
    pub chunk_index: usize,
    pub start_line: usize,
    pub end_line: usize,
    /// Header context for search result display (e.g., "# Title > ## Section")
    pub header_context: String,
    /// Hash of chunk content for differential updates
    pub content_hash: String,
    /// Programming language for code blocks (e.g., "rust", "python")
    pub language: Option<String>,
    /// Links found in this chunk (serialized as JSON in database)
    pub links: Vec<StoredLink>,
}

/// Chunk embeddings with metadata for search and indexing
#[derive(Debug, Clone)]
pub struct ChunkEmbeddingRecord {
    pub chunk_id: u32,
    pub doc_id: u32,
    pub chunk_index: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub embeddings: Vec<f32>,
    pub num_tokens: usize,
    /// Header context for search result display
    pub header_context: String,
    /// Programming language for code blocks
    pub language: Option<String>,
    /// Links found in this chunk
    pub links: Vec<StoredLink>,
}

/// Image record - an indexed image file
#[derive(Debug, Clone)]
pub struct ImageRecord {
    pub id: u32,
    pub path: String,
    pub hash: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub indexed_at: u64,
}

/// Image embedding record with CLIP embedding data
#[derive(Debug, Clone)]
pub struct ImageEmbeddingRecord {
    pub image_id: u32,
    pub path: String,
    /// CLIP embedding (512-dim single vector)
    pub embedding: Vec<f32>,
}

/// Summary of an indexed image for listing
#[derive(Debug, Clone)]
pub struct ImageSummary {
    pub id: u32,
    pub path: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub indexed_at: u64,
}

/// File summary record from database.
#[derive(Debug, Clone)]
pub struct FileSummaryRecord {
    pub id: u32,
    pub doc_id: u32,
    pub summary: String,
    /// Source of the summary: "firstline", "mime", "llm"
    pub source: String,
    /// Model name if source is "llm"
    pub model: Option<String>,
    /// Content hash when summary was generated
    pub hash: String,
    /// Storage tier: "xattr" or "sqlite"
    pub storage_tier: String,
    /// Unix timestamp when generated
    pub generated_at: u64,
    /// True if file content changed since summary was generated
    pub is_stale: bool,
}

/// Project statistics derived from indexed documents.
#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub file_count: usize,
    pub last_indexed: Option<u64>,
}

impl DB {
    /// Get document counts and latest index timestamp for a project root.
    pub fn project_stats(&self, root: &Path) -> Result<ProjectStats> {
        let prefix = path_prefix(root);
        let like_pattern = format!("{prefix}%");
        let (count, last_indexed): (i64, Option<i64>) = self.conn.query_row(
            "SELECT COUNT(*), MAX(indexed_at) FROM documents WHERE path LIKE ?",
            params![like_pattern],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(ProjectStats {
            file_count: count.max(0) as usize,
            last_indexed: last_indexed.map(|ts| ts as u64),
        })
    }

    /// Estimate database size in bytes using SQLite page stats.
    pub fn storage_bytes(&self) -> Result<u64> {
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        Ok(page_count.saturating_mul(page_size).max(0) as u64)
    }

    // =========================================================================
    // Cross-file dedup support (Bloom filter)
    // =========================================================================

    /// Get all non-empty content hashes from indexed chunks.
    ///
    /// Used to initialize the Bloom filter from existing index data.
    pub fn get_all_content_hashes(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT DISTINCT content_hash FROM chunks WHERE content_hash != ''")?;
        let hashes: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(hashes)
    }

    /// Get content hash count (for sizing Bloom filter).
    pub fn content_hash_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT content_hash) FROM chunks WHERE content_hash != ''",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Find a chunk's embedding by its content hash (for cross-file reuse).
    ///
    /// Returns the first matching chunk's embedding data if found.
    /// This enables reusing embeddings across different files with identical content.
    pub fn get_embedding_by_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Option<(Vec<f32>, usize)>> {
        // Find any chunk with this hash and get its embedding
        let result: Option<(Vec<u8>, i64)> = self
            .conn
            .query_row(
                "SELECT ce.data, ce.num_tokens
                 FROM chunks c
                 JOIN chunk_embeddings ce ON c.id = ce.chunk_id
                 WHERE c.content_hash = ?
                 LIMIT 1",
                params![content_hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match result {
            Some((bytes, num_tokens)) => {
                let embeddings: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some((embeddings, num_tokens as usize)))
            }
            None => Ok(None),
        }
    }

    /// Save Bloom filter state to index_state for persistence.
    pub fn save_bloom_filter(&self, data: &[u8]) -> Result<()> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.set_index_state("bloom_filter", &encoded)
    }

    /// Load Bloom filter state from index_state.
    pub fn load_bloom_filter(&self) -> Result<Option<Vec<u8>>> {
        use base64::Engine;
        match self.get_index_state("bloom_filter")? {
            Some(encoded) => {
                let data = base64::engine::general_purpose::STANDARD
                    .decode(&encoded)
                    .context("Failed to decode bloom filter data")?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Clear saved Bloom filter state (e.g., after rebuild).
    pub fn clear_bloom_filter(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM index_state WHERE key = 'bloom_filter'", [])?;
        Ok(())
    }

    // =========================================================================
    // Image storage (CLIP embeddings for image search)
    // =========================================================================

    /// Add an image to the database.
    ///
    /// Returns the image ID. If the image already exists with the same hash,
    /// returns the existing ID without re-adding.
    pub fn add_image(
        &self,
        path: &Path,
        hash: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<u32> {
        let path_str = path.to_string_lossy();

        // Check if image exists with same hash
        if let Some(existing) = self.get_image_by_path(path)? {
            if existing.hash == hash {
                return Ok(existing.id);
            }
            // Hash changed, update the image
            self.conn.execute(
                "UPDATE images SET hash = ?, width = ?, height = ?, indexed_at = strftime('%s', 'now') WHERE id = ?",
                params![hash, width.map(|w| w as i64), height.map(|h| h as i64), existing.id],
            )?;
            // Delete old embedding
            self.conn.execute(
                "DELETE FROM image_embeddings WHERE image_id = ?",
                params![existing.id],
            )?;
            return Ok(existing.id);
        }

        // Insert new image
        self.conn.execute(
            "INSERT INTO images (path, hash, width, height) VALUES (?, ?, ?, ?)",
            params![
                path_str.as_ref(),
                hash,
                width.map(|w| w as i64),
                height.map(|h| h as i64)
            ],
        )?;

        Ok(self.conn.last_insert_rowid() as u32)
    }

    /// Get an image by ID.
    pub fn get_image(&self, id: u32) -> Result<Option<ImageRecord>> {
        self.conn
            .query_row(
                "SELECT id, path, hash, width, height, indexed_at FROM images WHERE id = ?",
                params![id],
                |row| {
                    Ok(ImageRecord {
                        id: row.get::<_, i64>(0)? as u32,
                        path: row.get(1)?,
                        hash: row.get(2)?,
                        width: row.get::<_, Option<i64>>(3)?.map(|w| w as u32),
                        height: row.get::<_, Option<i64>>(4)?.map(|h| h as u32),
                        indexed_at: row.get::<_, i64>(5)? as u64,
                    })
                },
            )
            .optional()
            .context("Failed to get image")
    }

    /// Get an image by path.
    pub fn get_image_by_path(&self, path: &Path) -> Result<Option<ImageRecord>> {
        let path_str = path.to_string_lossy();
        self.conn
            .query_row(
                "SELECT id, path, hash, width, height, indexed_at FROM images WHERE path = ?",
                params![path_str.as_ref()],
                |row| {
                    Ok(ImageRecord {
                        id: row.get::<_, i64>(0)? as u32,
                        path: row.get(1)?,
                        hash: row.get(2)?,
                        width: row.get::<_, Option<i64>>(3)?.map(|w| w as u32),
                        height: row.get::<_, Option<i64>>(4)?.map(|h| h as u32),
                        indexed_at: row.get::<_, i64>(5)? as u64,
                    })
                },
            )
            .optional()
            .context("Failed to get image by path")
    }

    /// Remove an image by path.
    pub fn remove_image(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        // image_embeddings are cascade-deleted automatically
        let rows = self.conn.execute(
            "DELETE FROM images WHERE path = ?",
            params![path_str.as_ref()],
        )?;
        Ok(rows > 0)
    }

    /// Remove all images whose path starts with the given prefix.
    pub fn remove_images_by_prefix(&self, prefix: &Path) -> Result<usize> {
        let prefix_str = path_prefix(prefix);
        let like_pattern = format!("{prefix_str}%");
        let rows = self.conn.execute(
            "DELETE FROM images WHERE path LIKE ?",
            params![like_pattern],
        )?;
        Ok(rows)
    }

    /// Check if an image needs re-indexing (hash changed).
    pub fn needs_image_reindex(&self, path: &Path, hash: &str) -> Result<bool> {
        match self.get_image_by_path(path)? {
            Some(img) => Ok(img.hash != hash),
            None => Ok(true), // New image
        }
    }

    /// Add CLIP embedding for an image.
    ///
    /// Embeddings are stored as a blob of 512 f32 values (CLIP_DIM).
    pub fn add_image_embedding(&self, image_id: u32, embedding: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.conn.execute(
            "INSERT OR REPLACE INTO image_embeddings (image_id, data) VALUES (?, ?)",
            params![image_id, bytes],
        )?;
        Ok(())
    }

    /// Get CLIP embedding for an image.
    pub fn get_image_embedding(&self, image_id: u32) -> Result<Option<Vec<f32>>> {
        let result: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM image_embeddings WHERE image_id = ?",
                params![image_id],
                |row| row.get(0),
            )
            .optional()?;

        match result {
            Some(bytes) => {
                let embedding: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some(embedding))
            }
            None => Ok(None),
        }
    }

    /// Get all image embeddings for search.
    ///
    /// Returns all images with their CLIP embeddings for similarity search.
    pub fn get_all_image_embeddings(&self) -> Result<Vec<ImageEmbeddingRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT i.id, i.path, ie.data
             FROM images i
             JOIN image_embeddings ie ON ie.image_id = i.id
             ORDER BY i.path",
        )?;

        let rows = stmt.query_map([], |row| {
            let bytes: Vec<u8> = row.get(2)?;
            let embedding: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            Ok(ImageEmbeddingRecord {
                image_id: row.get::<_, i64>(0)? as u32,
                path: row.get(1)?,
                embedding,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// List all indexed images.
    pub fn list_images(&self, path_prefix: Option<&Path>) -> Result<Vec<ImageSummary>> {
        let row_mapper = |row: &rusqlite::Row| -> rusqlite::Result<ImageSummary> {
            Ok(ImageSummary {
                id: row.get::<_, i64>(0)? as u32,
                path: row.get(1)?,
                width: row.get::<_, Option<i64>>(2)?.map(|w| w as u32),
                height: row.get::<_, Option<i64>>(3)?.map(|h| h as u32),
                indexed_at: row.get::<_, i64>(4)? as u64,
            })
        };

        let images = if let Some(prefix) = path_prefix {
            let sql = "SELECT id, path, width, height, indexed_at
                       FROM images
                       WHERE path LIKE ?
                       ORDER BY path";
            let mut stmt = self.conn.prepare_cached(sql)?;
            let prefix_str = format!("{}%", prefix.display());
            let rows = stmt.query_map(params![prefix_str], row_mapper)?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let sql = "SELECT id, path, width, height, indexed_at
                       FROM images
                       ORDER BY path";
            let mut stmt = self.conn.prepare_cached(sql)?;
            let rows = stmt.query_map([], row_mapper)?;
            rows.filter_map(|r| r.ok()).collect()
        };

        Ok(images)
    }

    /// Get total count of indexed images.
    pub fn image_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM images", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get all image IDs.
    pub fn get_all_image_ids(&self) -> Result<Vec<u32>> {
        let mut stmt = self.conn.prepare_cached("SELECT id FROM images")?;
        let ids = stmt
            .query_map([], |row| row.get::<_, i64>(0))?
            .filter_map(Result::ok)
            .map(|id| id as u32)
            .collect();
        Ok(ids)
    }

    // ===== File Summary Methods =====

    /// Get summary for a document by path.
    pub fn get_summary_by_path(&self, path: &Path) -> Result<Option<FileSummaryRecord>> {
        let path_str = path.to_string_lossy();
        self.conn
            .query_row(
                "SELECT fs.id, fs.doc_id, fs.summary, fs.source, fs.model, fs.hash,
                        fs.storage_tier, fs.generated_at, d.hash as doc_hash
                 FROM file_summaries fs
                 JOIN documents d ON fs.doc_id = d.id
                 WHERE d.path = ?",
                params![path_str.as_ref()],
                |row| {
                    let summary_hash: String = row.get(5)?;
                    let doc_hash: String = row.get(8)?;
                    Ok(FileSummaryRecord {
                        id: row.get::<_, i64>(0)? as u32,
                        doc_id: row.get::<_, i64>(1)? as u32,
                        summary: row.get(2)?,
                        source: row.get(3)?,
                        model: row.get(4)?,
                        hash: summary_hash.clone(),
                        storage_tier: row.get(6)?,
                        generated_at: row.get::<_, i64>(7)? as u64,
                        is_stale: summary_hash != doc_hash,
                    })
                },
            )
            .optional()
            .context("Failed to get summary by path")
    }

    /// Get summary for a document by doc_id.
    pub fn get_summary(&self, doc_id: u32) -> Result<Option<FileSummaryRecord>> {
        self.conn
            .query_row(
                "SELECT fs.id, fs.doc_id, fs.summary, fs.source, fs.model, fs.hash,
                        fs.storage_tier, fs.generated_at, d.hash as doc_hash
                 FROM file_summaries fs
                 JOIN documents d ON fs.doc_id = d.id
                 WHERE fs.doc_id = ?",
                params![doc_id],
                |row| {
                    let summary_hash: String = row.get(5)?;
                    let doc_hash: String = row.get(8)?;
                    Ok(FileSummaryRecord {
                        id: row.get::<_, i64>(0)? as u32,
                        doc_id: row.get::<_, i64>(1)? as u32,
                        summary: row.get(2)?,
                        source: row.get(3)?,
                        model: row.get(4)?,
                        hash: summary_hash.clone(),
                        storage_tier: row.get(6)?,
                        generated_at: row.get::<_, i64>(7)? as u64,
                        is_stale: summary_hash != doc_hash,
                    })
                },
            )
            .optional()
            .context("Failed to get summary")
    }

    /// Store or update a summary.
    pub fn upsert_summary(
        &self,
        doc_id: u32,
        summary: &str,
        source: &str,
        model: Option<&str>,
        hash: &str,
        storage_tier: &str,
    ) -> Result<u32> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO file_summaries (doc_id, summary, source, model, hash, storage_tier, generated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(doc_id) DO UPDATE SET
                 summary = excluded.summary,
                 source = excluded.source,
                 model = excluded.model,
                 hash = excluded.hash,
                 storage_tier = excluded.storage_tier,
                 generated_at = excluded.generated_at",
            params![doc_id, summary, source, model, hash, storage_tier, now],
        )?;

        let id = self.conn.last_insert_rowid() as u32;
        Ok(id)
    }

    /// Get all documents that need summaries generated.
    pub fn get_docs_needing_summaries(&self, limit: usize) -> Result<Vec<(u32, String, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT d.id, d.path, d.content
             FROM documents d
             LEFT JOIN file_summaries fs ON d.id = fs.doc_id
             WHERE fs.id IS NULL OR fs.hash != d.hash
             LIMIT ?",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to get docs needing summaries")
    }

    /// Delete summary for a document.
    pub fn delete_summary(&self, doc_id: u32) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM file_summaries WHERE doc_id = ?", params![doc_id])?;
        Ok(rows > 0)
    }

    /// Get total count of summaries.
    pub fn summary_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM file_summaries", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get count of stale summaries (hash mismatch).
    pub fn stale_summary_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM file_summaries fs
             JOIN documents d ON fs.doc_id = d.id
             WHERE fs.hash != d.hash",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}

fn path_prefix(root: &Path) -> String {
    let mut prefix = root.to_string_lossy().to_string();
    if !prefix.ends_with(std::path::MAIN_SEPARATOR) {
        prefix.push(std::path::MAIN_SEPARATOR);
    }
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_document() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let content = "fn main() {\n    println!(\"Hello\");\n}";

        let id = db.add_document(path, content).unwrap();
        assert!(id > 0);

        let doc = db.get_document(id).unwrap().unwrap();
        assert_eq!(doc.path, "/test/file.rs");
        assert_eq!(doc.content, content);
        assert_eq!(doc.line_count, 3);
    }

    #[test]
    fn test_document_deduplication() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let content = "fn main() {}";

        let id1 = db.add_document(path, content).unwrap();
        let id2 = db.add_document(path, content).unwrap();

        // Same content, same hash -> same ID
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_document_update_on_change() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");

        let id1 = db.add_document(path, "version 1").unwrap();
        let id2 = db.add_document(path, "version 2").unwrap();

        // Same path, different content -> same ID but updated content
        assert_eq!(id1, id2);

        let doc = db.get_document(id1).unwrap().unwrap();
        assert_eq!(doc.content, "version 2");
    }

    #[test]
    fn test_embeddings() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let id = db.add_document(path, "test").unwrap();

        // Store 2 tokens x 128 dims = 256 floats
        let embeddings: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
        db.add_embeddings(id, &embeddings, 2).unwrap();

        let (loaded, num_tokens) = db.get_embeddings(id).unwrap().unwrap();
        assert_eq!(num_tokens, 2);
        assert_eq!(loaded.len(), 256);
        assert!((loaded[0] - 0.0).abs() < 1e-6);
        assert!((loaded[255] - 255.0 / 256.0).abs() < 1e-6);
    }

    #[test]
    fn test_needs_reindex() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");

        // New file needs indexing
        assert!(db.needs_reindex(path, "content").unwrap());

        db.add_document(path, "content").unwrap();

        // Same content doesn't need reindex
        assert!(!db.needs_reindex(path, "content").unwrap());

        // Changed content needs reindex
        assert!(db.needs_reindex(path, "new content").unwrap());
    }

    #[test]
    fn test_save_and_load_centroids() {
        let db = DB::in_memory().unwrap();

        // Initially no centroids
        let result = db.load_centroids().unwrap();
        assert!(result.is_none());

        // Save 4 centroids with 128 dims each = 512 floats
        let num_centroids = 4;
        let dim = 128;
        let centroids: Vec<f32> = (0..(num_centroids * dim))
            .map(|i| (i as f32) / 1000.0)
            .collect();

        db.save_centroids(&centroids, num_centroids).unwrap();

        // Load and verify
        let (loaded, loaded_num) = db.load_centroids().unwrap().unwrap();
        assert_eq!(loaded_num, num_centroids);
        assert_eq!(loaded.len(), num_centroids * dim);

        // Check values match
        for i in 0..loaded.len() {
            assert!(
                (loaded[i] - centroids[i]).abs() < 1e-6,
                "Mismatch at index {}: {} vs {}",
                i,
                loaded[i],
                centroids[i]
            );
        }
    }

    #[test]
    fn test_centroids_overwrite() {
        let db = DB::in_memory().unwrap();

        // Save first set
        let centroids1: Vec<f32> = vec![1.0; 128 * 2];
        db.save_centroids(&centroids1, 2).unwrap();

        // Overwrite with second set
        let centroids2: Vec<f32> = vec![2.0; 128 * 4];
        db.save_centroids(&centroids2, 4).unwrap();

        // Should load the second set
        let (loaded, num) = db.load_centroids().unwrap().unwrap();
        assert_eq!(num, 4);
        assert_eq!(loaded.len(), 128 * 4);
        assert!((loaded[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_project_stats() {
        let db = DB::in_memory().unwrap();
        let root = Path::new("/project");

        // Empty project
        let stats = db.project_stats(root).unwrap();
        assert_eq!(stats.file_count, 0);
        assert!(stats.last_indexed.is_none());

        // Add some files
        db.add_document(Path::new("/project/src/main.rs"), "fn main() {}")
            .unwrap();
        db.add_document(Path::new("/project/src/lib.rs"), "pub mod foo;")
            .unwrap();
        db.add_document(Path::new("/other/file.rs"), "// not in project")
            .unwrap();

        // Should count only project files
        let stats = db.project_stats(root).unwrap();
        assert_eq!(stats.file_count, 2);
        assert!(stats.last_indexed.is_some());
    }

    #[test]
    fn test_storage_bytes() {
        let db = DB::in_memory().unwrap();

        // Should return some non-zero value even for empty db
        let bytes = db.storage_bytes().unwrap();
        // In-memory db may report 0 or small value
        assert!(bytes < 1024 * 1024, "Expected reasonable size");

        // Add some data
        for i in 0..10 {
            db.add_document(Path::new(&format!("/test/file{i}.rs")), &"x".repeat(1000))
                .unwrap();
        }

        // Should increase
        let bytes_after = db.storage_bytes().unwrap();
        assert!(
            bytes_after >= bytes,
            "Storage should not decrease after adding data"
        );
    }

    #[test]
    fn test_remove_document() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let content = "fn main() {}";

        // Add a document
        let id = db.add_document(path, content).unwrap();
        assert!(id > 0);
        assert_eq!(db.document_count().unwrap(), 1);

        // Add embeddings for the document
        let embeddings: Vec<f32> = (0..256).map(|i| i as f32 / 256.0).collect();
        db.add_embeddings(id, &embeddings, 2).unwrap();
        assert!(db.get_embeddings(id).unwrap().is_some());

        // Remove the document
        let removed = db.remove_document(path).unwrap();
        assert!(removed);
        assert_eq!(db.document_count().unwrap(), 0);

        // Document should no longer exist
        assert!(db.get_document(id).unwrap().is_none());
        assert!(db.get_document_by_path(path).unwrap().is_none());

        // Embeddings should be cascade-deleted
        assert!(db.get_embeddings(id).unwrap().is_none());

        // Removing non-existent document returns false
        let removed_again = db.remove_document(path).unwrap();
        assert!(!removed_again);
    }

    #[test]
    fn test_remove_documents_by_prefix() {
        let db = DB::in_memory().unwrap();

        // Add documents in different projects
        db.add_document(Path::new("/project_a/src/main.rs"), "fn main() {}")
            .unwrap();
        db.add_document(Path::new("/project_a/src/lib.rs"), "pub mod foo;")
            .unwrap();
        db.add_document(Path::new("/project_a/README.md"), "# Project A")
            .unwrap();
        db.add_document(Path::new("/project_b/src/main.rs"), "fn main() {}")
            .unwrap();
        db.add_document(Path::new("/project_b/Cargo.toml"), "[package]")
            .unwrap();
        db.add_document(Path::new("/other/file.rs"), "// other")
            .unwrap();

        assert_eq!(db.document_count().unwrap(), 6);

        // Remove all documents from project_a
        let removed = db
            .remove_documents_by_prefix(Path::new("/project_a"))
            .unwrap();
        assert_eq!(removed, 3);
        assert_eq!(db.document_count().unwrap(), 3);

        // Verify project_a documents are gone
        assert!(db
            .get_document_by_path(Path::new("/project_a/src/main.rs"))
            .unwrap()
            .is_none());
        assert!(db
            .get_document_by_path(Path::new("/project_a/src/lib.rs"))
            .unwrap()
            .is_none());
        assert!(db
            .get_document_by_path(Path::new("/project_a/README.md"))
            .unwrap()
            .is_none());

        // Verify project_b and other documents remain
        assert!(db
            .get_document_by_path(Path::new("/project_b/src/main.rs"))
            .unwrap()
            .is_some());
        assert!(db
            .get_document_by_path(Path::new("/project_b/Cargo.toml"))
            .unwrap()
            .is_some());
        assert!(db
            .get_document_by_path(Path::new("/other/file.rs"))
            .unwrap()
            .is_some());

        // Remove project_b
        let removed = db
            .remove_documents_by_prefix(Path::new("/project_b"))
            .unwrap();
        assert_eq!(removed, 2);
        assert_eq!(db.document_count().unwrap(), 1);

        // Only /other/file.rs should remain
        assert!(db
            .get_document_by_path(Path::new("/other/file.rs"))
            .unwrap()
            .is_some());

        // Removing non-existent prefix returns 0
        let removed = db
            .remove_documents_by_prefix(Path::new("/nonexistent"))
            .unwrap();
        assert_eq!(removed, 0);
    }

    #[test]
    fn test_get_all_doc_ids() {
        let db = DB::in_memory().unwrap();

        // Initially empty
        let ids = db.get_all_doc_ids().unwrap();
        assert!(ids.is_empty());

        // Add some documents
        let id1 = db
            .add_document(Path::new("/test/a.rs"), "content a")
            .unwrap();
        let id2 = db
            .add_document(Path::new("/test/b.rs"), "content b")
            .unwrap();
        let id3 = db
            .add_document(Path::new("/test/c.rs"), "content c")
            .unwrap();

        // Should return all IDs
        let ids = db.get_all_doc_ids().unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_total_lines() {
        let db = DB::in_memory().unwrap();

        // Empty db should have 0 lines
        assert_eq!(db.total_lines().unwrap(), 0);

        // Add documents with known line counts
        db.add_document(Path::new("/test/a.rs"), "line 1\nline 2\nline 3")
            .unwrap();
        assert_eq!(db.total_lines().unwrap(), 3);

        db.add_document(Path::new("/test/b.rs"), "single line")
            .unwrap();
        assert_eq!(db.total_lines().unwrap(), 4);

        // Multi-line document
        db.add_document(Path::new("/test/c.rs"), "a\nb\nc\nd\ne")
            .unwrap();
        assert_eq!(db.total_lines().unwrap(), 9);
    }

    #[test]
    fn test_document_count() {
        let db = DB::in_memory().unwrap();

        // Empty db
        assert_eq!(db.document_count().unwrap(), 0);

        // Add documents
        db.add_document(Path::new("/test/a.rs"), "content a")
            .unwrap();
        assert_eq!(db.document_count().unwrap(), 1);

        db.add_document(Path::new("/test/b.rs"), "content b")
            .unwrap();
        assert_eq!(db.document_count().unwrap(), 2);

        // Update existing document shouldn't increase count
        db.add_document(Path::new("/test/a.rs"), "updated content")
            .unwrap();
        assert_eq!(db.document_count().unwrap(), 2);

        // Remove document should decrease count
        db.remove_document(Path::new("/test/a.rs")).unwrap();
        assert_eq!(db.document_count().unwrap(), 1);
    }

    #[test]
    fn test_set_and_get_index_state() {
        let db = DB::in_memory().unwrap();

        // Initially no state
        let state = db.get_index_state("test_key").unwrap();
        assert!(state.is_none());

        // Set state
        db.set_index_state("test_key", "test_value").unwrap();
        let state = db.get_index_state("test_key").unwrap();
        assert_eq!(state, Some("test_value".to_string()));

        // Update state
        db.set_index_state("test_key", "updated_value").unwrap();
        let state = db.get_index_state("test_key").unwrap();
        assert_eq!(state, Some("updated_value".to_string()));

        // Multiple keys
        db.set_index_state("key2", "value2").unwrap();
        assert_eq!(
            db.get_index_state("test_key").unwrap(),
            Some("updated_value".to_string())
        );
        assert_eq!(
            db.get_index_state("key2").unwrap(),
            Some("value2".to_string())
        );
        assert!(db.get_index_state("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_index_state_with_special_characters() {
        let db = DB::in_memory().unwrap();

        // Test with JSON-like value
        let json_value = r#"{"centroids": 8, "health": 0.95}"#;
        db.set_index_state("cluster_config", json_value).unwrap();
        assert_eq!(
            db.get_index_state("cluster_config").unwrap(),
            Some(json_value.to_string())
        );

        // Test with newlines and special chars
        let special_value = "line1\nline2\ttab\r\nwindows";
        db.set_index_state("special", special_value).unwrap();
        assert_eq!(
            db.get_index_state("special").unwrap(),
            Some(special_value.to_string())
        );
    }

    #[test]
    fn test_document_debug_trait() {
        let doc = Document {
            id: 42,
            path: "/test/file.rs".to_string(),
            hash: "abc123".to_string(),
            content: "fn main() {}".to_string(),
            line_count: 1,
        };

        let debug_str = format!("{doc:?}");
        assert!(debug_str.contains("Document"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("/test/file.rs"));
        assert!(debug_str.contains("abc123"));
        assert!(debug_str.contains("fn main()"));
        assert!(debug_str.contains("line_count"));
    }

    #[test]
    fn test_document_clone_trait() {
        let doc = Document {
            id: 99,
            path: "/project/src/lib.rs".to_string(),
            hash: "deadbeef".to_string(),
            content: "pub mod tests;".to_string(),
            line_count: 1,
        };

        let cloned = doc.clone();

        // Verify all fields are equal
        assert_eq!(cloned.id, doc.id);
        assert_eq!(cloned.path, doc.path);
        assert_eq!(cloned.hash, doc.hash);
        assert_eq!(cloned.content, doc.content);
        assert_eq!(cloned.line_count, doc.line_count);

        // Verify they are independent (modifying clone doesn't affect original)
        // Since String is Clone, the cloned strings are independent
        assert_eq!(doc.path, "/project/src/lib.rs");
    }

    #[test]
    fn test_project_stats_debug_trait() {
        let stats = ProjectStats {
            file_count: 150,
            last_indexed: Some(1704067200),
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("ProjectStats"));
        assert!(debug_str.contains("150"));
        assert!(debug_str.contains("1704067200"));
    }

    #[test]
    fn test_project_stats_debug_with_none() {
        let stats = ProjectStats {
            file_count: 0,
            last_indexed: None,
        };

        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("ProjectStats"));
        assert!(debug_str.contains("None"));
    }

    #[test]
    fn test_project_stats_clone_trait() {
        let stats = ProjectStats {
            file_count: 42,
            last_indexed: Some(1700000000),
        };

        let cloned = stats.clone();

        assert_eq!(cloned.file_count, stats.file_count);
        assert_eq!(cloned.last_indexed, stats.last_indexed);
    }

    #[test]
    fn test_project_stats_clone_with_none() {
        let stats = ProjectStats {
            file_count: 0,
            last_indexed: None,
        };

        let cloned = stats.clone();

        assert_eq!(cloned.file_count, 0);
        assert!(cloned.last_indexed.is_none());
    }

    // ==================== Chunk Tests ====================

    #[test]
    fn test_add_and_get_chunk() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let doc_id = db.add_document(path, "content").unwrap();

        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "# Test Header").unwrap();
        assert!(chunk_id > 0);

        let chunk = db.get_chunk(chunk_id).unwrap().unwrap();
        assert_eq!(chunk.id, chunk_id);
        assert_eq!(chunk.doc_id, doc_id);
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.start_line, 0);
        assert_eq!(chunk.end_line, 10);
        assert_eq!(chunk.header_context, "# Test Header");
    }

    #[test]
    fn test_add_and_get_chunk_with_links() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let links = vec![
            StoredLink {
                text: "Rust".to_string(),
                target: "https://www.rust-lang.org/".to_string(),
                is_internal: false,
            },
            StoredLink {
                text: "Local".to_string(),
                target: "src/lib.rs".to_string(),
                is_internal: true,
            },
        ];

        let chunk_id = db
            .add_chunk_with_links(doc_id, 0, 0, 10, "# Header", "", None, &links)
            .unwrap();

        let chunk = db.get_chunk(chunk_id).unwrap().unwrap();
        assert_eq!(chunk.links.len(), 2);
        assert_eq!(chunk.links[0].text, "Rust");
        assert_eq!(chunk.links[0].target, "https://www.rust-lang.org/");
        assert!(!chunk.links[0].is_internal);
        assert_eq!(chunk.links[1].text, "Local");
        assert_eq!(chunk.links[1].target, "src/lib.rs");
        assert!(chunk.links[1].is_internal);
    }

    #[test]
    fn test_get_chunk_rejects_invalid_links_json() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();

        db.conn
            .execute(
                "UPDATE chunks SET links = ? WHERE id = ?",
                rusqlite::params!["{", chunk_id],
            )
            .unwrap();

        let err = db.get_chunk(chunk_id).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Failed to get chunk"));
        assert!(message.contains("Failed to deserialize chunk links"));
    }

    #[test]
    fn test_get_chunks_for_doc_rejects_invalid_links_json() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();

        db.conn
            .execute(
                "UPDATE chunks SET links = ? WHERE id = ?",
                rusqlite::params!["{", chunk_id],
            )
            .unwrap();

        let err = db.get_chunks_for_doc(doc_id).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Failed to deserialize chunk links"));
    }

    #[test]
    fn test_multiple_chunks_per_document() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let doc_id = db.add_document(path, "content").unwrap();

        // Add 3 chunks
        let chunk0 = db.add_chunk(doc_id, 0, 0, 100, "").unwrap();
        let chunk1 = db.add_chunk(doc_id, 1, 80, 200, "").unwrap();
        let chunk2 = db.add_chunk(doc_id, 2, 180, 300, "").unwrap();

        // Verify all chunks exist
        let chunks = db.get_chunks_for_doc(doc_id).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].id, chunk0);
        assert_eq!(chunks[1].id, chunk1);
        assert_eq!(chunks[2].id, chunk2);

        // Verify ordering by chunk_index
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[2].chunk_index, 2);
    }

    #[test]
    fn test_chunk_embeddings() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let doc_id = db.add_document(path, "content").unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();

        // Store embeddings (5 tokens x 128 dims = 640 floats)
        let embeddings: Vec<f32> = (0..640).map(|i| i as f32 / 640.0).collect();
        db.add_chunk_embeddings(chunk_id, &embeddings, 5).unwrap();

        // Retrieve and verify
        let (loaded, num_tokens) = db.get_chunk_embeddings(chunk_id).unwrap().unwrap();
        assert_eq!(num_tokens, 5);
        assert_eq!(loaded.len(), 640);
        assert!((loaded[0] - 0.0).abs() < 1e-6);
        assert!((loaded[639] - 639.0 / 640.0).abs() < 1e-6);
    }

    #[test]
    fn test_get_all_chunk_embeddings() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk1 = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();
        let chunk2 = db.add_chunk(doc_id, 1, 11, 20, "").unwrap();

        let embeddings1: Vec<f32> = vec![0.5; 128];
        db.add_chunk_embeddings(chunk1, &embeddings1, 1).unwrap();

        let records = db.get_all_chunk_embeddings().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].chunk_id, chunk1);
        assert_eq!(records[0].doc_id, doc_id);
        assert_eq!(records[0].chunk_index, 0);
        assert_eq!(records[0].start_line, 0);
        assert_eq!(records[0].end_line, 10);
        assert_eq!(records[0].num_tokens, 1);
        assert_eq!(records[0].embeddings.len(), 128);

        let embeddings2: Vec<f32> = (0..256).map(|i| i as f32).collect();
        db.add_chunk_embeddings(chunk2, &embeddings2, 2).unwrap();

        let records = db.get_all_chunk_embeddings().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|record| record.chunk_id == chunk1));
        assert!(records.iter().any(|record| record.chunk_id == chunk2));
    }

    #[test]
    fn test_get_all_chunk_embeddings_rejects_invalid_links_json() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();
        let embeddings: Vec<f32> = vec![0.5; 128];
        db.add_chunk_embeddings(chunk_id, &embeddings, 1).unwrap();

        db.conn
            .execute(
                "UPDATE chunks SET links = ? WHERE id = ?",
                rusqlite::params!["{", chunk_id],
            )
            .unwrap();

        let err = db.get_all_chunk_embeddings().unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Failed to deserialize chunk links"));
    }

    #[test]
    fn test_get_all_chunk_ids() {
        let db = DB::in_memory().unwrap();

        // Initially empty
        assert!(db.get_all_chunk_ids().unwrap().is_empty());

        // Add document and chunks
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk1 = db.add_chunk(doc_id, 0, 0, 50, "").unwrap();
        let chunk2 = db.add_chunk(doc_id, 1, 40, 100, "").unwrap();

        let ids = db.get_all_chunk_ids().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&chunk1));
        assert!(ids.contains(&chunk2));
    }

    #[test]
    fn test_delete_chunks_for_doc() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();

        // Add chunks with embeddings
        let chunk1 = db.add_chunk(doc_id, 0, 0, 50, "").unwrap();
        let chunk2 = db.add_chunk(doc_id, 1, 40, 100, "").unwrap();
        let embeddings: Vec<f32> = vec![1.0; 128];
        db.add_chunk_embeddings(chunk1, &embeddings, 1).unwrap();
        db.add_chunk_embeddings(chunk2, &embeddings, 1).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 2);

        // Delete chunks
        db.delete_chunks_for_doc(doc_id).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 0);
        assert!(db.get_chunk(chunk1).unwrap().is_none());
        assert!(db.get_chunk(chunk2).unwrap().is_none());
        // Embeddings should be cascade-deleted
        assert!(db.get_chunk_embeddings(chunk1).unwrap().is_none());
        assert!(db.get_chunk_embeddings(chunk2).unwrap().is_none());
    }

    #[test]
    fn test_chunk_cascade_delete_on_document_removal() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/file.rs");
        let doc_id = db.add_document(path, "content").unwrap();

        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();
        let embeddings: Vec<f32> = vec![1.0; 128];
        db.add_chunk_embeddings(chunk_id, &embeddings, 1).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 1);

        // Remove document - chunks should be cascade-deleted
        db.remove_document(path).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 0);
        assert!(db.get_chunk(chunk_id).unwrap().is_none());
        assert!(db.get_chunk_embeddings(chunk_id).unwrap().is_none());
    }

    #[test]
    fn test_chunk_count() {
        let db = DB::in_memory().unwrap();
        assert_eq!(db.chunk_count().unwrap(), 0);

        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        db.add_chunk(doc_id, 0, 0, 50, "").unwrap();
        assert_eq!(db.chunk_count().unwrap(), 1);

        db.add_chunk(doc_id, 1, 40, 100, "").unwrap();
        assert_eq!(db.chunk_count().unwrap(), 2);
    }

    #[test]
    fn test_add_chunk_with_hash() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();

        let chunk_id = db
            .add_chunk_with_hash(doc_id, 0, 0, 10, "# Header", "abc123")
            .unwrap();
        let chunk = db.get_chunk(chunk_id).unwrap().unwrap();

        assert_eq!(chunk.header_context, "# Header");
        assert_eq!(chunk.content_hash, "abc123");
    }

    #[test]
    fn test_get_chunk_hashes_for_doc() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();

        // Add chunks with hashes
        let _chunk1 = db
            .add_chunk_with_hash(doc_id, 0, 0, 10, "", "hash_one")
            .unwrap();
        let _chunk2 = db
            .add_chunk_with_hash(doc_id, 1, 11, 20, "", "hash_two")
            .unwrap();
        let _chunk3 = db.add_chunk(doc_id, 2, 21, 30, "").unwrap(); // Empty hash

        let hashes = db.get_chunk_hashes_for_doc(doc_id).unwrap();

        // Should have 2 entries (empty hash is excluded)
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains_key("hash_one"));
        assert!(hashes.contains_key("hash_two"));
    }

    #[test]
    fn test_delete_chunk() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();

        let chunk1 = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();
        let chunk2 = db.add_chunk(doc_id, 1, 11, 20, "").unwrap();

        // Add embeddings
        let embeddings: Vec<f32> = vec![1.0; 128];
        db.add_chunk_embeddings(chunk1, &embeddings, 1).unwrap();
        db.add_chunk_embeddings(chunk2, &embeddings, 1).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 2);

        // Delete just chunk1
        db.delete_chunk(chunk1).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 1);
        assert!(db.get_chunk(chunk1).unwrap().is_none());
        assert!(db.get_chunk(chunk2).unwrap().is_some());
        // Embeddings should be cascade-deleted for chunk1 only
        assert!(db.get_chunk_embeddings(chunk1).unwrap().is_none());
        assert!(db.get_chunk_embeddings(chunk2).unwrap().is_some());
    }

    #[test]
    fn test_batch_add_chunks_with_hashes() {
        let db = DB::in_memory().unwrap();
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();

        let embeddings1: Vec<f32> = vec![1.0; 128];
        let embeddings2: Vec<f32> = vec![2.0; 256];

        let chunks = vec![
            (
                0usize,
                0usize,
                10usize,
                "# Header 1",
                "hash_a",
                embeddings1.as_slice(),
                1usize,
            ),
            (
                1,
                11,
                20,
                "## Header 2",
                "hash_b",
                embeddings2.as_slice(),
                2,
            ),
        ];

        db.batch_add_chunks_with_hashes(doc_id, &chunks).unwrap();

        assert_eq!(db.chunk_count().unwrap(), 2);

        let stored_chunks = db.get_chunks_for_doc(doc_id).unwrap();
        assert_eq!(stored_chunks.len(), 2);

        assert_eq!(stored_chunks[0].content_hash, "hash_a");
        assert_eq!(stored_chunks[0].header_context, "# Header 1");
        assert_eq!(stored_chunks[1].content_hash, "hash_b");
        assert_eq!(stored_chunks[1].header_context, "## Header 2");

        // Check embeddings were stored
        let (emb1, n_tok1) = db
            .get_chunk_embeddings(stored_chunks[0].id)
            .unwrap()
            .unwrap();
        assert_eq!(n_tok1, 1);
        assert_eq!(emb1.len(), 128);

        let (emb2, n_tok2) = db
            .get_chunk_embeddings(stored_chunks[1].id)
            .unwrap()
            .unwrap();
        assert_eq!(n_tok2, 2);
        assert_eq!(emb2.len(), 256);
    }

    #[test]
    fn test_chunk_record_debug_clone() {
        let chunk = ChunkRecord {
            id: 1,
            doc_id: 42,
            chunk_index: 5,
            start_line: 100,
            end_line: 200,
            header_context: "# Test".to_string(),
            content_hash: "abc123".to_string(),
            language: Some("rust".to_string()),
            links: vec![],
        };

        // Test Debug
        let debug_str = format!("{chunk:?}");
        assert!(debug_str.contains("ChunkRecord"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("100"));

        // Test Clone
        let cloned = chunk.clone();
        assert_eq!(cloned.id, chunk.id);
        assert_eq!(cloned.doc_id, chunk.doc_id);
        assert_eq!(cloned.chunk_index, chunk.chunk_index);
        assert_eq!(cloned.content_hash, chunk.content_hash);
        assert_eq!(cloned.language, chunk.language);
    }

    #[test]
    fn test_compact_empty_db() {
        let mut db = DB::in_memory().unwrap();

        let stats = db.compact().unwrap();

        assert_eq!(stats.documents_before, 0);
        assert_eq!(stats.documents_after, 0);
        assert_eq!(stats.chunks_before, 0);
        assert_eq!(stats.chunks_after, 0);
        assert_eq!(stats.total_removed(), 0);
    }

    #[test]
    fn test_compact_with_documents() {
        let mut db = DB::in_memory().unwrap();

        // Add some documents with chunks
        let doc_id = db
            .add_document(Path::new("/test/file.rs"), "content")
            .unwrap();
        let chunk_id = db.add_chunk(doc_id, 0, 0, 10, "").unwrap();
        let embeddings: Vec<f32> = vec![1.0; 128];
        db.add_chunk_embeddings(chunk_id, &embeddings, 1).unwrap();

        let stats = db.compact().unwrap();

        // Should have no orphans
        assert_eq!(stats.documents_before, 1);
        assert_eq!(stats.documents_after, 1);
        assert_eq!(stats.chunks_before, 1);
        assert_eq!(stats.chunks_after, 1);
        assert_eq!(stats.orphaned_embeddings_removed, 0);
        assert_eq!(stats.orphaned_chunks_removed, 0);
        assert_eq!(stats.orphaned_chunk_embeddings_removed, 0);
    }

    #[test]
    fn test_compaction_stats_methods() {
        let stats = CompactionStats {
            chunks_before: 100,
            chunks_after: 90,
            documents_before: 10,
            documents_after: 10,
            size_before_bytes: 1024 * 1024, // 1 MB
            size_after_bytes: 512 * 1024,   // 512 KB
            orphaned_embeddings_removed: 2,
            orphaned_chunks_removed: 5,
            orphaned_chunk_embeddings_removed: 3,
            stale_centroids_removed: 1,
        };

        assert_eq!(stats.total_removed(), 11); // 2 + 5 + 3 + 1
        assert_eq!(stats.space_reclaimed(), 512 * 1024); // 1MB - 512KB

        // Test format_size
        assert_eq!(CompactionStats::format_size(500), "500 bytes");
        assert_eq!(CompactionStats::format_size(1024), "1.00 KB");
        assert_eq!(CompactionStats::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(CompactionStats::format_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_database_size() {
        let db = DB::in_memory().unwrap();
        let size = db.database_size().unwrap();
        // In-memory database should have some minimal size
        assert!(size > 0);
    }

    #[test]
    fn test_list_documents() {
        let db = DB::in_memory().unwrap();

        // Empty db should return empty list
        let docs = db.list_documents(None).unwrap();
        assert!(docs.is_empty());

        // Add documents with different paths
        let doc1 = db
            .add_document(Path::new("/project/src/main.rs"), "fn main() {}\ntest")
            .unwrap();
        let doc2 = db
            .add_document(Path::new("/project/src/lib.rs"), "pub mod foo;")
            .unwrap();
        let _doc3 = db
            .add_document(Path::new("/other/file.rs"), "// comment")
            .unwrap();

        // Add chunks to some documents
        db.add_chunk(doc1, 0, 0, 1, "# Main").unwrap();
        db.add_chunk(doc1, 1, 1, 2, "# Test").unwrap();
        db.add_chunk(doc2, 0, 0, 1, "").unwrap();

        // List all documents
        let docs = db.list_documents(None).unwrap();
        assert_eq!(docs.len(), 3);

        // Verify chunk counts
        let main_doc = docs.iter().find(|d| d.path.ends_with("main.rs")).unwrap();
        assert_eq!(main_doc.chunk_count, 2);

        let lib_doc = docs.iter().find(|d| d.path.ends_with("lib.rs")).unwrap();
        assert_eq!(lib_doc.chunk_count, 1);

        let other_doc = docs.iter().find(|d| d.path.ends_with("file.rs")).unwrap();
        assert_eq!(other_doc.chunk_count, 0);

        // Filter by path prefix
        let project_docs = db.list_documents(Some(Path::new("/project"))).unwrap();
        assert_eq!(project_docs.len(), 2);
        assert!(project_docs.iter().all(|d| d.path.starts_with("/project")));

        // Non-matching prefix returns empty
        let no_docs = db.list_documents(Some(Path::new("/nonexistent"))).unwrap();
        assert!(no_docs.is_empty());
    }

    // =========================================================================
    // Image storage tests
    // =========================================================================

    #[test]
    fn test_add_and_get_image() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");
        let hash = "abc123";

        let id = db.add_image(path, hash, Some(1920), Some(1080)).unwrap();
        assert!(id > 0);

        let img = db.get_image(id).unwrap().unwrap();
        assert_eq!(img.path, "/test/photo.jpg");
        assert_eq!(img.hash, hash);
        assert_eq!(img.width, Some(1920));
        assert_eq!(img.height, Some(1080));
    }

    #[test]
    fn test_image_deduplication() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");
        let hash = "same_hash";

        let id1 = db.add_image(path, hash, Some(100), Some(100)).unwrap();
        let id2 = db.add_image(path, hash, Some(100), Some(100)).unwrap();

        // Same hash -> same ID
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_image_update_on_change() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");

        let id1 = db.add_image(path, "hash_v1", Some(100), Some(100)).unwrap();
        let id2 = db.add_image(path, "hash_v2", Some(200), Some(200)).unwrap();

        // Same path, different hash -> same ID but updated
        assert_eq!(id1, id2);

        let img = db.get_image(id1).unwrap().unwrap();
        assert_eq!(img.hash, "hash_v2");
        assert_eq!(img.width, Some(200));
        assert_eq!(img.height, Some(200));
    }

    #[test]
    fn test_image_embedding() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");
        let id = db.add_image(path, "hash", Some(100), Some(100)).unwrap();

        // Store 512-dim CLIP embedding
        let embedding: Vec<f32> = (0..512).map(|i| i as f32 / 512.0).collect();
        db.add_image_embedding(id, &embedding).unwrap();

        let loaded = db.get_image_embedding(id).unwrap().unwrap();
        assert_eq!(loaded.len(), 512);
        assert!((loaded[0] - 0.0).abs() < 1e-6);
        assert!((loaded[511] - 511.0 / 512.0).abs() < 1e-6);
    }

    #[test]
    fn test_get_all_image_embeddings() {
        let db = DB::in_memory().unwrap();

        // Add two images with embeddings
        let id1 = db
            .add_image(Path::new("/test/a.jpg"), "h1", Some(100), Some(100))
            .unwrap();
        let id2 = db
            .add_image(Path::new("/test/b.jpg"), "h2", Some(200), Some(200))
            .unwrap();

        let emb1: Vec<f32> = (0..512).map(|_| 0.1).collect();
        let emb2: Vec<f32> = (0..512).map(|_| 0.2).collect();
        db.add_image_embedding(id1, &emb1).unwrap();
        db.add_image_embedding(id2, &emb2).unwrap();

        let all = db.get_all_image_embeddings().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].embedding.len(), 512);
        assert_eq!(all[1].embedding.len(), 512);
    }

    #[test]
    fn test_needs_image_reindex() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");

        // New image needs indexing
        assert!(db.needs_image_reindex(path, "hash").unwrap());

        db.add_image(path, "hash", None, None).unwrap();

        // Same hash doesn't need reindex
        assert!(!db.needs_image_reindex(path, "hash").unwrap());

        // Changed hash needs reindex
        assert!(db.needs_image_reindex(path, "new_hash").unwrap());
    }

    #[test]
    fn test_remove_image() {
        let db = DB::in_memory().unwrap();
        let path = Path::new("/test/photo.jpg");

        let id = db.add_image(path, "hash", None, None).unwrap();
        db.add_image_embedding(id, &[0.0; 512]).unwrap();

        assert!(db.get_image(id).unwrap().is_some());
        assert!(db.get_image_embedding(id).unwrap().is_some());

        let removed = db.remove_image(path).unwrap();
        assert!(removed);

        // Image and embedding should be gone (cascade delete)
        assert!(db.get_image(id).unwrap().is_none());
        assert!(db.get_image_embedding(id).unwrap().is_none());
    }

    #[test]
    fn test_list_images() {
        let db = DB::in_memory().unwrap();

        // Empty db
        let images = db.list_images(None).unwrap();
        assert!(images.is_empty());

        // Add images
        db.add_image(Path::new("/project/imgs/a.jpg"), "h1", Some(100), Some(100))
            .unwrap();
        db.add_image(Path::new("/project/imgs/b.png"), "h2", Some(200), Some(200))
            .unwrap();
        db.add_image(Path::new("/other/c.gif"), "h3", None, None)
            .unwrap();

        // List all
        let all = db.list_images(None).unwrap();
        assert_eq!(all.len(), 3);

        // Filter by prefix
        let project = db.list_images(Some(Path::new("/project"))).unwrap();
        assert_eq!(project.len(), 2);
    }

    #[test]
    fn test_image_count() {
        let db = DB::in_memory().unwrap();

        assert_eq!(db.image_count().unwrap(), 0);

        db.add_image(Path::new("/a.jpg"), "h1", None, None).unwrap();
        db.add_image(Path::new("/b.jpg"), "h2", None, None).unwrap();

        assert_eq!(db.image_count().unwrap(), 2);
    }

    #[test]
    fn test_remove_images_by_prefix() {
        let db = DB::in_memory().unwrap();

        db.add_image(Path::new("/project/a.jpg"), "h1", None, None)
            .unwrap();
        db.add_image(Path::new("/project/b.jpg"), "h2", None, None)
            .unwrap();
        db.add_image(Path::new("/other/c.jpg"), "h3", None, None)
            .unwrap();

        let removed = db.remove_images_by_prefix(Path::new("/project")).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(db.image_count().unwrap(), 1);
    }
}
