use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ModelConfig {
    pub id: ModelConfigId,
    pub provider: ModelProvider,
    pub model: String,
    pub purpose: ModelPurpose,
    pub execution: ModelExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ModelProvider {
    OpenAi,
    Anthropic,
    Ollama,
    LlamaCpp,
    Mlx,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ModelPurpose {
    Embedding,
    Reranking,
    Generation,
    Judge,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ModelExecutionMode {
    Local,
    HostedApi,
    RemoteWorker,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ModelConfigId(pub Uuid);
