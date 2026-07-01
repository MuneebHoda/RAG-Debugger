use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use rag_debugger_core::{
    AnswerabilityConfig, ChunkingConfig, ChunkingStrategy, DebuggerConfig, DeploymentMode,
    EmbeddingConfig, EmbeddingModelInfo, EmbeddingProviderKind, IngestionConfig, ProductConfig,
    ProductInfo, RetrievalConfig, RetrievalMode, RetrievalWeights, UiConfig,
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct ApiConfig {
    pub environment: RuntimeEnvironment,
    pub bind_addr: SocketAddr,
    pub storage_backend: StorageBackend,
    pub database_url: String,
    pub web_origin: String,
    pub auth: AuthConfig,
    pub product: ProductConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthConfig {
    pub provider: AuthProviderKind,
    pub session_cookie_name: String,
    pub session_ttl_hours: i64,
    pub cookie_secure: bool,
    pub bootstrap_email: String,
    pub bootstrap_password: String,
    pub bootstrap_user_name: String,
    pub bootstrap_organization_name: String,
    pub bootstrap_workspace_name: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthProviderKind {
    Local,
    External,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            provider: AuthProviderKind::Local,
            session_cookie_name: "corpuslab_session".to_owned(),
            session_ttl_hours: 168,
            cookie_secure: false,
            bootstrap_email: "demo@corpuslab.ai".to_owned(),
            bootstrap_password: "CorpusLab#2026".to_owned(),
            bootstrap_user_name: "Demo User".to_owned(),
            bootstrap_organization_name: "CorpusLab Demo Organization".to_owned(),
            bootstrap_workspace_name: "Corpus Demo Workspace".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StorageBackend {
    Postgres,
    Memory,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RuntimeEnvironment {
    Local,
    Test,
    Production,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid RAG_DEBUGGER_API_HOST value: {0}")]
    InvalidHost(String),
    #[error("invalid RAG_DEBUGGER_API_PORT value: {0}")]
    InvalidPort(String),
    #[error("invalid RAG_DEBUGGER_STORAGE_BACKEND value: {0}")]
    InvalidStorageBackend(String),
    #[error("invalid {name} value: {value}")]
    InvalidNumber { name: &'static str, value: String },
    #[error("{name} must be a finite number between 0 and 1, got: {value}")]
    InvalidRatio { name: &'static str, value: String },
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = match std::env::var("RAG_DEBUGGER_ENV")
            .unwrap_or_else(|_| "local".to_owned())
            .as_str()
        {
            "production" | "prod" => RuntimeEnvironment::Production,
            "test" => RuntimeEnvironment::Test,
            _ => RuntimeEnvironment::Local,
        };

        let host = std::env::var("RAG_DEBUGGER_API_HOST")
            .unwrap_or_else(|_| Ipv4Addr::LOCALHOST.to_string())
            .parse::<IpAddr>()
            .map_err(|error| ConfigError::InvalidHost(error.to_string()))?;

        let port = std::env::var("RAG_DEBUGGER_API_PORT")
            .unwrap_or_else(|_| "8080".to_owned())
            .parse::<u16>()
            .map_err(|error| ConfigError::InvalidPort(error.to_string()))?;

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/rag_debugger".to_owned()
        });
        let storage_backend = env_storage_backend("RAG_DEBUGGER_STORAGE_BACKEND")?;
        let web_origin = env_string("RAG_DEBUGGER_WEB_ORIGIN", "http://127.0.0.1:5173");
        let api_base_url = env_string("RAG_DEBUGGER_PUBLIC_API_BASE_URL", "http://127.0.0.1:8080");
        let product = ProductConfig {
            product: ProductInfo {
                name: env_string("RAG_DEBUGGER_PRODUCT_NAME", "CorpusLab"),
                workspace_name: env_string("RAG_DEBUGGER_WORKSPACE_NAME", "Corpus Workspace"),
                deployment_mode: match std::env::var("RAG_DEBUGGER_DEPLOYMENT_MODE")
                    .unwrap_or_else(|_| "hybrid".to_owned())
                    .as_str()
                {
                    "hosted" => DeploymentMode::Hosted,
                    "local" => DeploymentMode::Local,
                    _ => DeploymentMode::Hybrid,
                },
            },
            ingestion: IngestionConfig {
                max_files_per_request: env_u32("RAG_DEBUGGER_MAX_FILES_PER_REQUEST", 10)?,
                max_file_bytes: env_u64("RAG_DEBUGGER_MAX_FILE_BYTES", 20 * 1024 * 1024)?,
                max_request_bytes: env_u64("RAG_DEBUGGER_MAX_REQUEST_BYTES", 50 * 1024 * 1024)?,
                preview_chunk_limit: env_u32("RAG_DEBUGGER_PREVIEW_CHUNK_LIMIT", 8)?,
                supported_extensions: env_list(
                    "RAG_DEBUGGER_SUPPORTED_EXTENSIONS",
                    &["txt", "md", "markdown", "html", "htm", "pdf"],
                ),
            },
            chunking: ChunkingConfig {
                target_tokens: env_u32("RAG_DEBUGGER_DEFAULT_TARGET_TOKENS", 512)?,
                overlap_tokens: env_u32("RAG_DEBUGGER_DEFAULT_OVERLAP_TOKENS", 64)?,
                strategy: env_chunking_strategy("RAG_DEBUGGER_DEFAULT_CHUNKING_STRATEGY"),
            },
            retrieval: RetrievalConfig {
                default_top_k: env_u32("RAG_DEBUGGER_DEFAULT_TOP_K", 5)?,
                max_top_k: env_u32("RAG_DEBUGGER_MAX_TOP_K", 25)?,
                default_mode: env_retrieval_mode("RAG_DEBUGGER_DEFAULT_RETRIEVAL_MODE"),
                min_evidence_score: env_f32("RAG_DEBUGGER_MIN_EVIDENCE_SCORE", 0.35)?,
                min_semantic_similarity: env_f32("RAG_DEBUGGER_MIN_SEMANTIC_SIMILARITY", 0.25)?,
                answer_citation_limit: env_u32("RAG_DEBUGGER_ANSWER_CITATION_LIMIT", 3)?,
                answerability: AnswerabilityConfig {
                    min_body_term_coverage: env_ratio(
                        "RAG_DEBUGGER_MIN_ANSWER_BODY_TERM_COVERAGE",
                        0.50,
                    )?,
                    min_body_term_matches: env_positive_u32(
                        "RAG_DEBUGGER_MIN_ANSWER_BODY_TERM_MATCHES",
                        2,
                    )?,
                },
                weights: RetrievalWeights {
                    semantic_hybrid: env_f32("RAG_DEBUGGER_WEIGHT_SEMANTIC_HYBRID", 2.0)?,
                    semantic_vector: env_f32("RAG_DEBUGGER_WEIGHT_SEMANTIC_VECTOR", 3.0)?,
                    lexical: env_f32("RAG_DEBUGGER_WEIGHT_LEXICAL", 2.4)?,
                    frequency: env_f32("RAG_DEBUGGER_WEIGHT_FREQUENCY", 0.6)?,
                    phrase: env_f32("RAG_DEBUGGER_WEIGHT_PHRASE", 1.2)?,
                    section: env_f32("RAG_DEBUGGER_WEIGHT_SECTION", 0.75)?,
                    path: env_f32("RAG_DEBUGGER_WEIGHT_PATH", 0.5)?,
                    metadata: env_f32("RAG_DEBUGGER_WEIGHT_METADATA", 1.0)?,
                },
            },
            debugger: DebuggerConfig {
                low_score_margin_ratio: env_ratio("RAG_DEBUGGER_LOW_SCORE_MARGIN_RATIO", 0.10)?,
            },
            embedding: EmbeddingConfig {
                model: EmbeddingModelInfo {
                    provider: env_string("RAG_DEBUGGER_EMBEDDING_PROVIDER", "local"),
                    model_name: env_string("RAG_DEBUGGER_EMBEDDING_MODEL", "local-hash-v1"),
                    dimension: env_u32("RAG_DEBUGGER_EMBEDDING_DIMENSION", 384)?,
                },
                provider_kind: EmbeddingProviderKind::LocalHash,
            },
            ui: UiConfig {
                api_base_url,
                show_local_badges: env_bool("RAG_DEBUGGER_SHOW_LOCAL_BADGES", true),
            },
        };
        Ok(Self {
            environment,
            bind_addr: SocketAddr::new(host, port),
            storage_backend,
            database_url,
            web_origin,
            auth: AuthConfig {
                provider: match std::env::var("RAG_DEBUGGER_AUTH_PROVIDER")
                    .unwrap_or_else(|_| "local".to_owned())
                    .as_str()
                {
                    "external" => AuthProviderKind::External,
                    _ => AuthProviderKind::Local,
                },
                session_cookie_name: env_string(
                    "RAG_DEBUGGER_SESSION_COOKIE_NAME",
                    "corpuslab_session",
                ),
                session_ttl_hours: env_i64("RAG_DEBUGGER_SESSION_TTL_HOURS", 168)?,
                cookie_secure: env_bool("RAG_DEBUGGER_SESSION_COOKIE_SECURE", false),
                bootstrap_email: env_string("RAG_DEBUGGER_BOOTSTRAP_EMAIL", "demo@corpuslab.ai"),
                bootstrap_password: env_string("RAG_DEBUGGER_BOOTSTRAP_PASSWORD", "CorpusLab#2026"),
                bootstrap_user_name: env_string("RAG_DEBUGGER_BOOTSTRAP_USER_NAME", "Demo User"),
                bootstrap_organization_name: env_string(
                    "RAG_DEBUGGER_BOOTSTRAP_ORGANIZATION",
                    "CorpusLab Demo Organization",
                ),
                bootstrap_workspace_name: env_string(
                    "RAG_DEBUGGER_BOOTSTRAP_WORKSPACE",
                    "Corpus Demo Workspace",
                ),
            },
            product,
        })
    }
}

fn env_storage_backend(name: &str) -> Result<StorageBackend, ConfigError> {
    match std::env::var(name)
        .unwrap_or_else(|_| "postgres".to_owned())
        .to_ascii_lowercase()
        .as_str()
    {
        "postgres" | "postgresql" => Ok(StorageBackend::Postgres),
        "memory" | "in-memory" | "in_memory" => Ok(StorageBackend::Memory),
        other => Err(ConfigError::InvalidStorageBackend(other.to_owned())),
    }
}

fn env_string(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn env_list(name: &str, default: &[&str]) -> Vec<String> {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| {
                    let item = item.trim();
                    if item.is_empty() {
                        None
                    } else {
                        Some(item.trim_start_matches('.').to_ascii_lowercase())
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| default.iter().map(|item| item.to_string()).collect())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

fn env_u32(name: &'static str, default: u32) -> Result<u32, ConfigError> {
    std::env::var(name)
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| ConfigError::InvalidNumber {
                    name,
                    value: value.clone(),
                })
        })
        .unwrap_or(Ok(default))
}

fn env_positive_u32(name: &'static str, default: u32) -> Result<u32, ConfigError> {
    let value = env_u32(name, default)?;
    if value == 0 {
        return Err(ConfigError::InvalidNumber {
            name,
            value: value.to_string(),
        });
    }
    Ok(value)
}

fn env_u64(name: &'static str, default: u64) -> Result<u64, ConfigError> {
    std::env::var(name)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| ConfigError::InvalidNumber {
                    name,
                    value: value.clone(),
                })
        })
        .unwrap_or(Ok(default))
}

fn env_i64(name: &'static str, default: i64) -> Result<i64, ConfigError> {
    std::env::var(name)
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|_| ConfigError::InvalidNumber {
                    name,
                    value: value.clone(),
                })
        })
        .unwrap_or(Ok(default))
}

fn env_f32(name: &'static str, default: f32) -> Result<f32, ConfigError> {
    std::env::var(name)
        .map(|value| {
            value
                .parse::<f32>()
                .map_err(|_| ConfigError::InvalidNumber {
                    name,
                    value: value.clone(),
                })
        })
        .unwrap_or(Ok(default))
}

fn env_ratio(name: &'static str, default: f32) -> Result<f32, ConfigError> {
    match std::env::var(name) {
        Ok(value) => parse_ratio(name, &value),
        Err(_) => Ok(default),
    }
}

fn parse_ratio(name: &'static str, value: &str) -> Result<f32, ConfigError> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| ConfigError::InvalidNumber {
            name,
            value: value.to_owned(),
        })?;
    if parsed.is_finite() && (0.0..=1.0).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(ConfigError::InvalidRatio {
            name,
            value: value.to_owned(),
        })
    }
}

fn env_chunking_strategy(name: &str) -> ChunkingStrategy {
    match std::env::var(name)
        .unwrap_or_else(|_| "structured".to_owned())
        .as_str()
    {
        "smart_sections" | "structured" => ChunkingStrategy::Structured,
        "whitespace" => ChunkingStrategy::Whitespace,
        _ => ChunkingStrategy::Structured,
    }
}

fn env_retrieval_mode(name: &str) -> RetrievalMode {
    match std::env::var(name)
        .unwrap_or_else(|_| "hybrid".to_owned())
        .as_str()
    {
        "lexical" => RetrievalMode::Lexical,
        "vector" => RetrievalMode::Vector,
        _ => RetrievalMode::Hybrid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_localhost() {
        let config = ApiConfig {
            environment: RuntimeEnvironment::Local,
            bind_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8080),
            storage_backend: StorageBackend::Postgres,
            database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger".to_owned(),
            web_origin: "http://127.0.0.1:5173".to_owned(),
            auth: AuthConfig {
                provider: AuthProviderKind::Local,
                session_cookie_name: "corpuslab_session".to_owned(),
                session_ttl_hours: 168,
                cookie_secure: false,
                bootstrap_email: "demo@corpuslab.ai".to_owned(),
                bootstrap_password: "CorpusLab#2026".to_owned(),
                bootstrap_user_name: "Demo User".to_owned(),
                bootstrap_organization_name: "CorpusLab Demo Organization".to_owned(),
                bootstrap_workspace_name: "Corpus Demo Workspace".to_owned(),
            },
            product: ProductConfig::default(),
        };

        assert_eq!(config.environment, RuntimeEnvironment::Local);
        assert_eq!(config.bind_addr.port(), 8080);
    }

    #[test]
    fn debugger_margin_ratio_is_validated() {
        assert_eq!(parse_ratio("TEST_RATIO", "0.1").ok(), Some(0.1));
        assert!(matches!(
            parse_ratio("TEST_RATIO", "1.1"),
            Err(ConfigError::InvalidRatio { .. })
        ));
        assert!(matches!(
            parse_ratio("TEST_RATIO", "NaN"),
            Err(ConfigError::InvalidRatio { .. })
        ));
    }

    #[test]
    fn answerability_minimum_match_count_must_be_positive() {
        assert!(matches!(
            env_positive_u32("CORPUSLAB_TEST_ZERO", 0),
            Err(ConfigError::InvalidNumber { .. })
        ));
        assert_eq!(env_positive_u32("CORPUSLAB_TEST_TWO", 2).ok(), Some(2));
    }
}
