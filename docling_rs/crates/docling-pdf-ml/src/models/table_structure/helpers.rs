use tch::{nn, Kind, Tensor};

/// Multi-Layer Perceptron (MLP) with configurable layers
///
/// Used in BBoxDecoder for bbox prediction head:
/// - Input: 512 dims
/// - Hidden: 256 dims (2 layers)
/// - Output: 4 dims (bbox: cx, cy, w, h)
///
/// Architecture: Linear → ReLU → Linear → ReLU → Linear
pub struct MLP {
    layers: Vec<nn::Linear>,
}

impl std::fmt::Debug for MLP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MLP")
            .field("num_layers", &self.layers.len())
            .finish()
    }
}

impl MLP {
    /// Create MLP with specified layer dimensions
    ///
    /// # Arguments
    /// * `vs` - VarStore path for weight initialization
    /// * `dims` - Layer dimensions, e.g., [512, 256, 256, 4]
    ///
    /// # Example
    /// ```no_run
    /// use tch::nn;
    /// use docling_pdf_ml::models::table_structure::helpers::MLP;
    ///
    /// let vs = nn::VarStore::new(tch::Device::Cpu);
    /// let mlp = MLP::new(&vs.root(), &[512, 256, 256, 4]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(vs: &nn::Path, dims: &[i64]) -> Self {
        let mut layers = Vec::new();

        for i in 0..dims.len() - 1 {
            let linear = nn::linear(
                vs / "layers" / (i as i64), // PyTorch uses layers.0, layers.1, etc.
                dims[i],
                dims[i + 1],
                Default::default(),
            );
            layers.push(linear);
        }

        MLP { layers }
    }

    /// Forward pass through MLP
    ///
    /// Applies: Linear → ReLU → Linear → ReLU → ... → Linear (no final activation)
    pub fn forward(&self, x: &Tensor) -> Tensor {
        let mut out = x.shallow_clone();

        for (i, layer) in self.layers.iter().enumerate() {
            out = out.apply(layer);

            // Apply ReLU to all layers except the last
            if i < self.layers.len() - 1 {
                out = out.relu();
            }
        }

        out
    }
}

/// Cell Attention module for BBoxDecoder
///
/// Combines three sources of information:
/// 1. Encoder features (image spatial features)
/// 2. Tag decoder output (tag sequence context)
/// 3. Language model hidden state (cell-specific features)
///
/// Architecture:
/// - encoder_att: Attention over encoder features
/// - tag_decoder_att: Attention over tag decoder outputs
/// - language_att: Attention over language model hidden states
/// - full_att: Final attention combining all sources
pub struct CellAttention {
    encoder_att: nn::Linear,
    tag_decoder_att: nn::Linear,
    language_att: nn::Linear,
    full_att: nn::Linear,
}

impl std::fmt::Debug for CellAttention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellAttention")
            .field("encoder_att", &"<nn::Linear>")
            .field("tag_decoder_att", &"<nn::Linear>")
            .field("language_att", &"<nn::Linear>")
            .field("full_att", &"<nn::Linear>")
            .finish()
    }
}

impl CellAttention {
    /// Create CellAttention module
    ///
    /// # Arguments
    /// * `vs` - VarStore path for weight initialization
    /// * `hidden_dim` - Hidden dimension (512 for TableFormer)
    ///
    /// # Example
    /// ```no_run
    /// use tch::nn;
    /// use docling_pdf_ml::models::table_structure::helpers::CellAttention;
    ///
    /// let vs = nn::VarStore::new(tch::Device::Cpu);
    /// let attention = CellAttention::new(&vs.root(), 512);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(vs: &nn::Path, hidden_dim: i64) -> Self {
        CellAttention {
            encoder_att: nn::linear(
                vs / "_encoder_att",
                hidden_dim,
                hidden_dim,
                Default::default(),
            ),
            tag_decoder_att: nn::linear(
                vs / "_tag_decoder_att",
                hidden_dim,
                hidden_dim,
                Default::default(),
            ),
            language_att: nn::linear(
                vs / "_language_att",
                hidden_dim,
                hidden_dim,
                Default::default(),
            ),
            full_att: nn::linear(vs / "_full_att", hidden_dim, 1, Default::default()),
        }
    }

    /// Forward pass through CellAttention
    ///
    /// # Arguments
    /// * `encoder_out` - Encoder output (batch, enc_size, enc_size, hidden_dim)
    /// * `tag_decoder_out` - Tag decoder output (batch, seq_len, hidden_dim)
    /// * `language_hidden` - Language model hidden state (batch, hidden_dim)
    ///
    /// # Returns
    /// Attention-weighted features (batch, hidden_dim)
    pub fn forward(
        &self,
        encoder_out: &Tensor,
        tag_decoder_out: &Tensor,
        language_hidden: &Tensor,
    ) -> (Tensor, Tensor) {
        // Python reference:
        // att1 = self._encoder_att(encoder_out)  # (1, num_pixels, attention_dim)
        // att2 = self._tag_decoder_att(decoder_hidden)  # (num_cells, tag_decoder_dim)
        // att3 = self._language_att(language_out)  # (num_cells, attention_dim)
        // att = self._full_att(self._relu(att1 + att2.unsqueeze(1) + att3.unsqueeze(1))).squeeze(2)
        // alpha = self._softmax(att)  # (num_cells, num_pixels)
        // attention_weighted_encoding = (encoder_out * alpha.unsqueeze(2)).sum(dim=1)

        // encoder_out: (1, num_pixels=784, 512)
        // tag_decoder_out: (1, 512) for single cell
        // language_hidden: (1, 512)

        let att1 = encoder_out.apply(&self.encoder_att); // (1, 784, 512)
        let att2 = tag_decoder_out.apply(&self.tag_decoder_att); // (1, 512)
        let att3 = language_hidden.apply(&self.language_att); // (1, 512)

        // att1 + att2.unsqueeze(1) + att3.unsqueeze(1)
        // att2.unsqueeze(1) → (1, 1, 512) - adds spatial dimension
        // att3.unsqueeze(1) → (1, 1, 512) - adds spatial dimension
        let att2_expanded = att2.unsqueeze(1); // (1, 1, 512)
        let att3_expanded = att3.unsqueeze(1); // (1, 1, 512)
        let combined = (&att1 + &att2_expanded + &att3_expanded).relu(); // (1, 784, 512)

        // full_att: 512 → 1
        let att_scores = combined.apply(&self.full_att).squeeze_dim(2); // (1, 784)

        // Softmax over spatial dimension
        let alpha = att_scores.softmax(1, Kind::Float); // (1, 784)

        // attention_weighted_encoding = (encoder_out * alpha.unsqueeze(2)).sum(dim=1)
        let alpha_expanded = alpha.unsqueeze(2); // (1, 784, 1)
        let weighted = encoder_out * &alpha_expanded; // (1, 784, 512)
        let awe = weighted.sum_dim_intlist(&[1i64][..], false, Kind::Float); // (1, 512)

        (awe, alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::{nn, Device};

    #[test]
    fn test_mlp_creation() {
        let vs = nn::VarStore::new(Device::Cpu);
        let mlp = MLP::new(&vs.root(), &[512, 256, 256, 4]);
        // MLP with dims [512, 256, 256, 4] has 3 linear layers:
        // 512→256, 256→256, 256→4
        assert_eq!(mlp.layers.len(), 3);
    }

    #[test]
    fn test_mlp_forward() {
        let vs = nn::VarStore::new(Device::Cpu);
        let mlp = MLP::new(&vs.root(), &[512, 256, 256, 4]);
        let input = Tensor::randn([2, 512], (tch::Kind::Float, Device::Cpu));
        let output = mlp.forward(&input);
        assert_eq!(output.size(), vec![2, 4]);
    }

    #[test]
    fn test_cell_attention_creation() {
        let vs = nn::VarStore::new(Device::Cpu);
        let _attention = CellAttention::new(&vs.root(), 512);
        // Just verify it compiles and creates
    }

    #[test]
    fn test_cell_attention_forward() {
        let vs = nn::VarStore::new(Device::Cpu);
        let attention = CellAttention::new(&vs.root(), 512);

        // encoder_out should be (batch, num_pixels, hidden_dim)
        // Flatten the spatial dimensions: [2, 28, 28, 512] → [2, 784, 512]
        let encoder_out = Tensor::randn([2, 784, 512], (tch::Kind::Float, Device::Cpu));
        // tag_decoder_out and language_hidden should be (batch, hidden_dim)
        let tag_decoder_out = Tensor::randn([2, 512], (tch::Kind::Float, Device::Cpu));
        let language_hidden = Tensor::randn([2, 512], (tch::Kind::Float, Device::Cpu));

        let (awe, alpha) = attention.forward(&encoder_out, &tag_decoder_out, &language_hidden);
        assert_eq!(awe.size(), vec![2, 512]);
        assert_eq!(alpha.size(), vec![2, 784]); // 28*28 = 784
    }
}
