use std::collections::BTreeSet;

use rag_debugger_core::{
    EmbeddingModelInfo, DEFAULT_EMBEDDING_DIMENSION, DEFAULT_EMBEDDING_MODEL_NAME,
    DEFAULT_EMBEDDING_PROVIDER,
};
use sha2::{Digest, Sha256};

use crate::RagError;

pub trait EmbeddingProvider: Send + Sync {
    fn model(&self) -> EmbeddingModelInfo;
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, RagError>;

    fn embed_one(&self, text: &str) -> Result<Vec<f32>, RagError> {
        self.embed(&[text])?
            .into_iter()
            .next()
            .ok_or(RagError::InvalidConfig(
                "embedding provider returned no vectors",
            ))
    }
}

#[derive(Debug, Clone)]
pub struct LocalHashEmbeddingProvider {
    model: EmbeddingModelInfo,
}

impl Default for LocalHashEmbeddingProvider {
    fn default() -> Self {
        Self {
            model: EmbeddingModelInfo {
                provider: DEFAULT_EMBEDDING_PROVIDER.to_owned(),
                model_name: DEFAULT_EMBEDDING_MODEL_NAME.to_owned(),
                dimension: DEFAULT_EMBEDDING_DIMENSION,
            },
        }
    }
}

impl LocalHashEmbeddingProvider {
    pub fn new(model: EmbeddingModelInfo) -> Self {
        Self { model }
    }
}

impl EmbeddingProvider for LocalHashEmbeddingProvider {
    fn model(&self) -> EmbeddingModelInfo {
        self.model.clone()
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, RagError> {
        let dimension = self.model.dimension as usize;
        if dimension == 0 {
            return Err(RagError::InvalidConfig(
                "embedding dimension must be greater than zero",
            ));
        }

        Ok(texts
            .iter()
            .map(|text| embed_text(text, dimension))
            .collect::<Vec<_>>())
    }
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut left_norm = 0.0f32;
    let mut right_norm = 0.0f32;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot += left_value * right_value;
        left_norm += left_value * left_value;
        right_norm += right_value * right_value;
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn embed_text(text: &str, dimension: usize) -> Vec<f32> {
    let mut vector = vec![0.0f32; dimension];
    let tokens = normalized_tokens(text);

    for token in &tokens {
        add_feature(&mut vector, &format!("token:{token}"), 1.0);

        for ngram in char_ngrams(token, 3) {
            add_feature(&mut vector, &format!("ngram:{ngram}"), 0.15);
        }

        for concept in semantic_concepts(token) {
            add_feature(&mut vector, &format!("concept:{concept}"), 2.2);
        }
    }

    for pair in tokens.windows(2) {
        add_feature(
            &mut vector,
            &format!("bigram:{}:{}", pair[0], pair[1]),
            0.45,
        );
    }

    normalize(&mut vector);
    vector
}

fn add_feature(vector: &mut [f32], feature: &str, weight: f32) {
    let index = feature_index(feature, vector.len());
    let sign = if feature_sign(feature) { 1.0 } else { -1.0 };
    vector[index] += weight * sign;
}

fn feature_index(feature: &str, dimension: usize) -> usize {
    let digest = Sha256::digest(feature.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(bytes) as usize) % dimension
}

fn feature_sign(feature: &str) -> bool {
    let digest = Sha256::digest(feature.as_bytes());
    digest[8] & 1 == 1
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }

    for value in vector {
        *value /= norm;
    }
}

fn normalized_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter_map(|token| {
            let token = token.trim().to_ascii_lowercase();
            if token.is_empty() || is_stop_word(&token) {
                None
            } else {
                Some(token)
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn char_ngrams(token: &str, size: usize) -> Vec<String> {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() < size {
        return Vec::new();
    }

    chars
        .windows(size)
        .map(|window| window.iter().collect::<String>())
        .collect()
}

fn semantic_concepts(token: &str) -> Vec<&'static str> {
    match token {
        "accelerator" | "accelerators" | "cuda" | "gpu" | "gpus" | "hpc" | "kernel" | "kernels"
        | "metal" | "onnx" | "parallel" | "rocm" | "tensor" | "tensors" | "vectorization" => {
            vec!["accelerated-compute"]
        }
        "chunk" | "chunking" | "citation" | "citations" | "embedding" | "embeddings"
        | "generation" | "index" | "indexing" | "llm" | "rag" | "retrieval" | "search"
        | "semantic" | "vector" | "vectors" => vec!["rag-retrieval"],
        "api" | "backend" | "database" | "postgres" | "rust" | "service" | "sql" | "sqlx" => {
            vec!["backend-systems"]
        }
        "frontend" | "react" | "typescript" | "ui" | "vite" | "web" => vec!["frontend-ui"],
        "resume" | "experience" | "project" | "projects" | "skill" | "skills" => {
            vec!["profile-evidence"]
        }
        _ => Vec::new(),
    }
}

fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "by"
            | "for"
            | "from"
            | "has"
            | "have"
            | "i"
            | "in"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "that"
            | "the"
            | "to"
            | "with"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_text_with_stable_dimension() {
        let provider = LocalHashEmbeddingProvider::default();
        let vector = provider
            .embed_one("Built GPU indexing experiments")
            .unwrap();

        assert_eq!(vector.len(), DEFAULT_EMBEDDING_DIMENSION as usize);
        assert!(vector.iter().any(|value| *value != 0.0));
    }

    #[test]
    fn cosine_similarity_is_high_for_related_text() {
        let provider = LocalHashEmbeddingProvider::default();
        let query = provider.embed_one("gpu acceleration").unwrap();
        let related = provider.embed_one("CUDA parallel kernels").unwrap();
        let unrelated = provider.embed_one("React dashboard layout").unwrap();

        assert!(cosine_similarity(&query, &related) > cosine_similarity(&query, &unrelated));
    }

    #[test]
    fn cosine_similarity_rejects_dimension_mismatch() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0]), 0.0);
    }
}
