use serde::{Deserialize, Serialize};

use crate::{
    chunk::{ChunkingConfig, ChunkingStrategy},
    embedding::EmbeddingModelInfo,
    retrieval::RetrievalMode,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProductConfig {
    pub product: ProductInfo,
    pub ingestion: IngestionConfig,
    pub chunking: ChunkingConfig,
    pub retrieval: RetrievalConfig,
    pub embedding: EmbeddingConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProductInfo {
    pub name: String,
    pub workspace_name: String,
    pub deployment_mode: DeploymentMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentMode {
    Local,
    Hosted,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct IngestionConfig {
    pub max_files_per_request: u32,
    pub max_file_bytes: u64,
    pub max_request_bytes: u64,
    pub preview_chunk_limit: u32,
    pub supported_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalConfig {
    pub default_top_k: u32,
    pub max_top_k: u32,
    pub default_mode: RetrievalMode,
    pub min_evidence_score: f32,
    pub min_semantic_similarity: f32,
    pub answer_citation_limit: u32,
    pub weights: RetrievalWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalWeights {
    pub semantic_hybrid: f32,
    pub semantic_vector: f32,
    pub lexical: f32,
    pub frequency: f32,
    pub phrase: f32,
    pub section: f32,
    pub path: f32,
    pub metadata: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EmbeddingConfig {
    pub model: EmbeddingModelInfo,
    pub provider_kind: EmbeddingProviderKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProviderKind {
    LocalHash,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct UiConfig {
    pub api_base_url: String,
    pub show_local_badges: bool,
}

impl Default for ProductConfig {
    fn default() -> Self {
        Self {
            product: ProductInfo {
                name: "CorpusLab".to_owned(),
                workspace_name: "Corpus Workspace".to_owned(),
                deployment_mode: DeploymentMode::Hybrid,
            },
            ingestion: IngestionConfig::default(),
            chunking: ChunkingConfig::default(),
            retrieval: RetrievalConfig::default(),
            embedding: EmbeddingConfig::default(),
            ui: UiConfig {
                api_base_url: "http://127.0.0.1:8080".to_owned(),
                show_local_badges: true,
            },
        }
    }
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_files_per_request: 10,
            max_file_bytes: 20 * 1024 * 1024,
            max_request_bytes: 50 * 1024 * 1024,
            preview_chunk_limit: 8,
            supported_extensions: vec![
                "txt".to_owned(),
                "md".to_owned(),
                "markdown".to_owned(),
                "html".to_owned(),
                "htm".to_owned(),
                "pdf".to_owned(),
            ],
        }
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            default_top_k: 5,
            max_top_k: 25,
            default_mode: RetrievalMode::Hybrid,
            min_evidence_score: 0.35,
            min_semantic_similarity: 0.25,
            answer_citation_limit: 3,
            weights: RetrievalWeights::default(),
        }
    }
}

impl Default for RetrievalWeights {
    fn default() -> Self {
        Self {
            semantic_hybrid: 2.0,
            semantic_vector: 3.0,
            lexical: 2.4,
            frequency: 0.6,
            phrase: 1.2,
            section: 0.75,
            path: 0.5,
            metadata: 1.0,
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: EmbeddingModelInfo::default(),
            provider_kind: EmbeddingProviderKind::LocalHash,
        }
    }
}

pub fn default_chunking_strategy() -> ChunkingStrategy {
    ChunkingStrategy::Structured
}
