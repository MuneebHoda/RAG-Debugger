use std::env::VarError;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use axum::http::Uri;
use rag_debugger_core::{
    AnswerabilityConfig, ChunkingConfig, ChunkingStrategy, DebuggerConfig, DeploymentMode,
    EmbeddingConfig, EmbeddingModelInfo, EmbeddingProviderKind, IngestionConfig, ProductConfig,
    ProductInfo, RetrievalConfig, RetrievalMode, RetrievalWeights, UiConfig,
};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use thiserror::Error;
use tracing_subscriber::EnvFilter;

const HOSTED_MAX_FILES_PER_REQUEST: u32 = 10;
const HOSTED_MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const HOSTED_MAX_REQUEST_BYTES: u64 = 50 * 1024 * 1024;
const HOSTED_MAX_SESSION_TTL_HOURS: i64 = 168;
const HOSTED_MIN_BOOTSTRAP_PASSWORD_CHARS: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct ApiConfig {
    pub environment: RuntimeEnvironment,
    pub release_sha: String,
    pub log_filter: String,
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

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationConfig {
    pub database_url: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthProviderKind {
    Local,
    External,
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
    Staging,
    Production,
}

impl RuntimeEnvironment {
    pub fn is_hosted(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }
}

impl MigrationConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = parse_runtime_environment(
            &std::env::var("RAG_DEBUGGER_ENV").unwrap_or_else(|_| "local".to_owned()),
        )?;
        let database_url = required_env_string("DATABASE_URL")?;
        if environment.is_hosted() {
            validate_hosted_database_url(&database_url)?;
        }
        Ok(Self { database_url })
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid RAG_DEBUGGER_ENV value: {0}")]
    InvalidEnvironment(String),
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
    #[error("required environment variable {name} is not set")]
    MissingEnvironmentVariable { name: &'static str },
    #[error("required environment variable {name} must not be empty")]
    EmptyEnvironmentVariable { name: &'static str },
    #[error("environment variable {name} is not valid Unicode")]
    InvalidEnvironmentVariable { name: &'static str },
    #[error("unsafe hosted configuration for {name}: {requirement}")]
    UnsafeHostedConfiguration {
        name: &'static str,
        requirement: &'static str,
    },
}

impl ApiConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = parse_runtime_environment(
            &std::env::var("RAG_DEBUGGER_ENV").unwrap_or_else(|_| "local".to_owned()),
        )?;

        let host = if environment.is_hosted() {
            required_env_string("RAG_DEBUGGER_API_HOST")?
        } else {
            std::env::var("RAG_DEBUGGER_API_HOST")
                .unwrap_or_else(|_| Ipv4Addr::LOCALHOST.to_string())
        }
        .parse::<IpAddr>()
        .map_err(|error| ConfigError::InvalidHost(error.to_string()))?;

        let port = if environment.is_hosted() {
            required_env_string("RAG_DEBUGGER_API_PORT")?
        } else {
            std::env::var("RAG_DEBUGGER_API_PORT").unwrap_or_else(|_| 8080_u16.to_string())
        }
        .parse::<u16>()
        .map_err(|error| ConfigError::InvalidPort(error.to_string()))?;

        let storage_backend = env_storage_backend("RAG_DEBUGGER_STORAGE_BACKEND")?;
        let database_url = match storage_backend {
            StorageBackend::Postgres => required_env_string("DATABASE_URL")?,
            StorageBackend::Memory => std::env::var("DATABASE_URL").unwrap_or_default(),
        };
        let web_origin = if environment.is_hosted() {
            required_env_string("RAG_DEBUGGER_WEB_ORIGIN")?
        } else {
            std::env::var("RAG_DEBUGGER_WEB_ORIGIN")
                .unwrap_or_else(|_| format!("http://{}:5173", Ipv4Addr::LOCALHOST))
        };
        let api_base_url = if environment.is_hosted() {
            required_env_string("RAG_DEBUGGER_PUBLIC_API_BASE_URL")?
        } else {
            std::env::var("RAG_DEBUGGER_PUBLIC_API_BASE_URL")
                .unwrap_or_else(|_| format!("http://{}:8080", Ipv4Addr::LOCALHOST))
        };
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
        let config = Self {
            environment,
            release_sha: if environment.is_hosted() {
                required_env_string("RAG_DEBUGGER_RELEASE_SHA")?
            } else {
                env_string("RAG_DEBUGGER_RELEASE_SHA", "development")
            },
            log_filter: env_string("RAG_DEBUGGER_LOG", "info"),
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
                bootstrap_password: required_env_string("RAG_DEBUGGER_BOOTSTRAP_PASSWORD")?,
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
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.environment.is_hosted() {
            return Ok(());
        }

        ensure_hosted(
            !self.bind_addr.ip().is_loopback() && self.bind_addr.port() != 0,
            "RAG_DEBUGGER_API_HOST/RAG_DEBUGGER_API_PORT",
            "must explicitly bind a non-loopback address and non-zero port",
        )?;
        ensure_hosted(
            self.storage_backend == StorageBackend::Postgres,
            "RAG_DEBUGGER_STORAGE_BACKEND",
            "must be postgres",
        )?;
        validate_hosted_database_url(&self.database_url)?;
        validate_https_origin("RAG_DEBUGGER_WEB_ORIGIN", &self.web_origin)?;
        validate_https_origin(
            "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
            &self.product.ui.api_base_url,
        )?;
        ensure_hosted(
            self.product.product.deployment_mode == DeploymentMode::Hosted,
            "RAG_DEBUGGER_DEPLOYMENT_MODE",
            "must be hosted",
        )?;
        ensure_hosted(
            self.auth.provider == AuthProviderKind::Local,
            "RAG_DEBUGGER_AUTH_PROVIDER",
            "must remain local until external authentication is implemented",
        )?;
        ensure_hosted(
            self.auth.cookie_secure,
            "RAG_DEBUGGER_SESSION_COOKIE_SECURE",
            "must be true",
        )?;
        ensure_hosted(
            self.auth.session_cookie_name.starts_with("__Host-")
                && self.auth.session_cookie_name.len() > "__Host-".len(),
            "RAG_DEBUGGER_SESSION_COOKIE_NAME",
            "must use a non-empty __Host- cookie name",
        )?;
        ensure_hosted(
            (1..=HOSTED_MAX_SESSION_TTL_HOURS).contains(&self.auth.session_ttl_hours),
            "RAG_DEBUGGER_SESSION_TTL_HOURS",
            "must be between 1 and 168 hours",
        )?;
        ensure_hosted(
            self.auth.bootstrap_password.chars().count() >= HOSTED_MIN_BOOTSTRAP_PASSWORD_CHARS,
            "RAG_DEBUGGER_BOOTSTRAP_PASSWORD",
            "must contain at least 16 characters",
        )?;
        ensure_hosted(
            is_full_commit_sha(&self.release_sha),
            "RAG_DEBUGGER_RELEASE_SHA",
            "must be the full 40-character lowercase commit SHA",
        )?;
        ensure_hosted(
            hosted_log_filter_is_safe(&self.log_filter),
            "RAG_DEBUGGER_LOG",
            "must be a valid filter that enables only info, warn, or error levels",
        )?;
        ensure_hosted(
            self.product.embedding.model.provider == "local",
            "RAG_DEBUGGER_EMBEDDING_PROVIDER",
            "must be local until an external provider boundary is approved",
        )?;

        let ingestion = &self.product.ingestion;
        ensure_hosted(
            (1..=HOSTED_MAX_FILES_PER_REQUEST).contains(&ingestion.max_files_per_request),
            "RAG_DEBUGGER_MAX_FILES_PER_REQUEST",
            "must be between 1 and 10",
        )?;
        ensure_hosted(
            ingestion.max_file_bytes > 0 && ingestion.max_file_bytes <= HOSTED_MAX_FILE_BYTES,
            "RAG_DEBUGGER_MAX_FILE_BYTES",
            "must be between 1 byte and 20 MiB",
        )?;
        ensure_hosted(
            ingestion.max_request_bytes >= ingestion.max_file_bytes
                && ingestion.max_request_bytes <= HOSTED_MAX_REQUEST_BYTES,
            "RAG_DEBUGGER_MAX_REQUEST_BYTES",
            "must cover one file and not exceed 50 MiB",
        )?;

        Ok(())
    }
}

fn parse_runtime_environment(value: &str) -> Result<RuntimeEnvironment, ConfigError> {
    match value {
        "local" | "development" | "dev" => Ok(RuntimeEnvironment::Local),
        "test" => Ok(RuntimeEnvironment::Test),
        "staging" => Ok(RuntimeEnvironment::Staging),
        "production" | "prod" => Ok(RuntimeEnvironment::Production),
        other => Err(ConfigError::InvalidEnvironment(other.to_owned())),
    }
}

fn ensure_hosted(
    condition: bool,
    name: &'static str,
    requirement: &'static str,
) -> Result<(), ConfigError> {
    if condition {
        Ok(())
    } else {
        Err(ConfigError::UnsafeHostedConfiguration { name, requirement })
    }
}

fn validate_https_origin(name: &'static str, value: &str) -> Result<(), ConfigError> {
    let uri = value
        .parse::<Uri>()
        .map_err(|_| ConfigError::UnsafeHostedConfiguration {
            name,
            requirement:
                "must be a non-local absolute HTTPS origin without a path, query, or fragment",
        })?;
    let valid_authority = uri.authority().is_some_and(|authority| {
        value == format!("https://{authority}")
            && is_non_local_host(authority.host())
            && !authority.as_str().contains('@')
    });
    ensure_hosted(
        uri.scheme_str() == Some("https") && valid_authority,
        name,
        "must be a non-local absolute HTTPS origin without a path, query, or fragment",
    )
}

fn is_non_local_host(host: &str) -> bool {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.');
    let is_localhost = host.eq_ignore_ascii_case("localhost")
        || host
            .rsplit_once('.')
            .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case("localhost"));

    !host.is_empty()
        && !is_localhost
        && host
            .parse::<IpAddr>()
            .map_or(true, |address| !address.is_loopback())
}

fn validate_hosted_database_url(value: &str) -> Result<(), ConfigError> {
    let options =
        PgConnectOptions::from_str(value).map_err(|_| ConfigError::UnsafeHostedConfiguration {
            name: "DATABASE_URL",
            requirement: "must be a valid credentialed PostgreSQL URL",
        })?;
    let credentials = value
        .split_once("://")
        .and_then(|(_, remainder)| remainder.split('/').next())
        .and_then(|authority| authority.rsplit_once('@'))
        .map(|(credentials, _)| credentials)
        .and_then(|credentials| credentials.split_once(':'));
    let has_non_default_credentials = credentials.is_some_and(|(username, password)| {
        !(username.is_empty()
            || password.is_empty()
            || (username == "postgres" && password == "postgres"))
    });
    ensure_hosted(
        options.get_socket().is_none()
            && is_non_local_host(options.get_host())
            && !options.get_username().is_empty()
            && options
                .get_database()
                .is_some_and(|database| !database.is_empty())
            && has_non_default_credentials
            && matches!(
                options.get_ssl_mode(),
                PgSslMode::Require | PgSslMode::VerifyCa | PgSslMode::VerifyFull
            ),
        "DATABASE_URL",
        "must use non-default credentials, a non-local host, a database name, and required TLS",
    )
}

fn is_full_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hosted_log_filter_is_safe(value: &str) -> bool {
    EnvFilter::try_new(value).is_ok()
        && !value.trim().is_empty()
        && value
            .split(',')
            .filter_map(|directive| directive.rsplit('=').next())
            .map(str::trim)
            .all(|level| matches!(level, "info" | "warn" | "error"))
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

fn required_env_string(name: &'static str) -> Result<String, ConfigError> {
    validate_required_env_string(name, std::env::var(name))
}

fn validate_required_env_string(
    name: &'static str,
    value: Result<String, VarError>,
) -> Result<String, ConfigError> {
    match value {
        Ok(value) if value.trim().is_empty() => Err(ConfigError::EmptyEnvironmentVariable { name }),
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => Err(ConfigError::MissingEnvironmentVariable { name }),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::InvalidEnvironmentVariable { name }),
    }
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
    use std::process::Command;

    use super::*;

    const FROM_ENV_EXPECTATION: &str = "CORPUSLAB_CONFIG_TEST_EXPECTATION";
    const FROM_ENV_EXPECTED_NAME: &str = "CORPUSLAB_CONFIG_TEST_EXPECTED_NAME";

    #[test]
    fn default_config_is_localhost() {
        let config = ApiConfig {
            environment: RuntimeEnvironment::Local,
            release_sha: "development".to_owned(),
            log_filter: "info".to_owned(),
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
                bootstrap_password: "test-only-bootstrap-password".to_owned(),
                bootstrap_user_name: "Demo User".to_owned(),
                bootstrap_organization_name: "CorpusLab Demo Organization".to_owned(),
                bootstrap_workspace_name: "Corpus Demo Workspace".to_owned(),
            },
            product: ProductConfig::default(),
        };

        assert_eq!(config.environment, RuntimeEnvironment::Local);
        assert_eq!(config.bind_addr.port(), 8080);
    }

    fn hosted_config(environment: RuntimeEnvironment) -> ApiConfig {
        let mut product = ProductConfig::default();
        product.product.deployment_mode = DeploymentMode::Hosted;
        product.ui.api_base_url = "https://api.alpha.example.com".to_owned();

        ApiConfig {
            environment,
            release_sha: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            log_filter: "info".to_owned(),
            bind_addr: "0.0.0.0:10000".parse().expect("valid hosted bind address"),
            storage_backend: StorageBackend::Postgres,
            database_url:
                "postgres://corpuslab:managed-secret@db.internal/corpuslab?sslmode=require"
                    .to_owned(),
            web_origin: "https://app.alpha.example.com".to_owned(),
            auth: AuthConfig {
                provider: AuthProviderKind::Local,
                session_cookie_name: "__Host-corpuslab_alpha_session".to_owned(),
                session_ttl_hours: 24,
                cookie_secure: true,
                bootstrap_email: "owner@example.test".to_owned(),
                bootstrap_password: "hosted-bootstrap-password".to_owned(),
                bootstrap_user_name: "Alpha Owner".to_owned(),
                bootstrap_organization_name: "Alpha Organization".to_owned(),
                bootstrap_workspace_name: "Alpha Workspace".to_owned(),
            },
            product,
        }
    }

    fn generated_test_value(character: char, length: usize) -> String {
        std::iter::repeat_n(character, length).collect()
    }

    fn hosted_from_env_command(
        environment: &str,
        expectation: &str,
        expected_name: &str,
    ) -> Command {
        let mut command = Command::new(std::env::current_exe().expect("current test executable"));
        command
            .arg("--exact")
            .arg("config::tests::hosted_from_env_subprocess")
            .arg("--nocapture");

        for (name, _) in std::env::vars_os() {
            let name_text = name.to_string_lossy();
            if name_text == "DATABASE_URL"
                || name_text.starts_with("RAG_DEBUGGER_")
                || name_text.starts_with("CORPUSLAB_CONFIG_TEST_")
            {
                command.env_remove(name);
            }
        }

        command
            .env(FROM_ENV_EXPECTATION, expectation)
            .env(FROM_ENV_EXPECTED_NAME, expected_name)
            .env("RAG_DEBUGGER_ENV", environment)
            .env("RAG_DEBUGGER_API_HOST", "0.0.0.0")
            .env("RAG_DEBUGGER_API_PORT", "10000")
            .env("RAG_DEBUGGER_STORAGE_BACKEND", "postgres")
            .env(
                "DATABASE_URL",
                format!(
                    "postgres://corpuslab:{}@db.internal/corpuslab?sslmode=require",
                    generated_test_value('d', 24)
                ),
            )
            .env("RAG_DEBUGGER_WEB_ORIGIN", "https://app.alpha.example.com")
            .env(
                "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
                "https://api.alpha.example.com",
            )
            .env("RAG_DEBUGGER_DEPLOYMENT_MODE", "hosted")
            .env("RAG_DEBUGGER_RELEASE_SHA", generated_test_value('a', 40))
            .env("RAG_DEBUGGER_LOG", "info")
            .env("RAG_DEBUGGER_AUTH_PROVIDER", "local")
            .env(
                "RAG_DEBUGGER_SESSION_COOKIE_NAME",
                "__Host-corpuslab_alpha_session",
            )
            .env("RAG_DEBUGGER_SESSION_TTL_HOURS", "24")
            .env("RAG_DEBUGGER_SESSION_COOKIE_SECURE", "true")
            .env(
                "RAG_DEBUGGER_BOOTSTRAP_PASSWORD",
                generated_test_value('p', 24),
            )
            .env("RAG_DEBUGGER_EMBEDDING_PROVIDER", "local")
            .env("RAG_DEBUGGER_MAX_FILES_PER_REQUEST", "10")
            .env("RAG_DEBUGGER_MAX_FILE_BYTES", "20971520")
            .env("RAG_DEBUGGER_MAX_REQUEST_BYTES", "52428800");

        command
    }

    fn run_hosted_from_env(command: &mut Command) {
        let output = command.output().expect("run isolated config test");
        assert!(
            output.status.success(),
            "isolated config test failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn hosted_from_env_subprocess() {
        let Ok(expectation) = std::env::var(FROM_ENV_EXPECTATION) else {
            return;
        };
        let expected_name = std::env::var(FROM_ENV_EXPECTED_NAME).expect("expected config name");
        let result = ApiConfig::from_env();

        match expectation.as_str() {
            "valid" => {
                let config = result.expect("valid hosted environment");
                assert_eq!(
                    config.environment,
                    parse_runtime_environment(
                        &std::env::var("RAG_DEBUGGER_ENV").expect("hosted environment")
                    )
                    .expect("valid hosted environment name")
                );
                assert_eq!(
                    config.bind_addr,
                    "0.0.0.0:10000".parse().expect("valid bind address")
                );
                assert_eq!(config.web_origin, "https://app.alpha.example.com");
                assert_eq!(
                    config.product.ui.api_base_url,
                    "https://api.alpha.example.com"
                );
            }
            "missing" => assert!(matches!(
                result,
                Err(ConfigError::MissingEnvironmentVariable { name }) if name == expected_name
            )),
            "empty" => assert!(matches!(
                result,
                Err(ConfigError::EmptyEnvironmentVariable { name }) if name == expected_name
            )),
            "unsafe" => assert!(matches!(
                result,
                Err(ConfigError::UnsafeHostedConfiguration { name, .. }) if name == expected_name
            )),
            other => panic!("unknown config-test expectation: {other}"),
        }
    }

    #[test]
    fn hosted_from_env_loads_valid_staging_and_production() {
        for environment in ["staging", "production"] {
            run_hosted_from_env(&mut hosted_from_env_command(environment, "valid", ""));
        }
    }

    #[test]
    fn hosted_from_env_requires_nonempty_values() {
        for name in [
            "RAG_DEBUGGER_API_HOST",
            "RAG_DEBUGGER_API_PORT",
            "DATABASE_URL",
            "RAG_DEBUGGER_WEB_ORIGIN",
            "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
            "RAG_DEBUGGER_RELEASE_SHA",
            "RAG_DEBUGGER_BOOTSTRAP_PASSWORD",
        ] {
            let mut missing = hosted_from_env_command("staging", "missing", name);
            missing.env_remove(name);
            run_hosted_from_env(&mut missing);

            let mut empty = hosted_from_env_command("production", "empty", name);
            empty.env(name, "   ");
            run_hosted_from_env(&mut empty);
        }
    }

    #[test]
    fn hosted_from_env_rejects_local_network_configuration() {
        let database_credential = generated_test_value('d', 24);
        let cases = [
            (
                "RAG_DEBUGGER_API_HOST",
                "127.0.0.1".to_owned(),
                "RAG_DEBUGGER_API_HOST/RAG_DEBUGGER_API_PORT",
            ),
            (
                "DATABASE_URL",
                format!(
                    "postgres://corpuslab:{database_credential}@localhost/corpuslab?sslmode=require"
                ),
                "DATABASE_URL",
            ),
            (
                "DATABASE_URL",
                format!(
                    "postgres://corpuslab:{database_credential}@127.0.0.2/corpuslab?sslmode=require"
                ),
                "DATABASE_URL",
            ),
            (
                "DATABASE_URL",
                format!(
                    "postgres://corpuslab:{database_credential}@[::1]/corpuslab?sslmode=require"
                ),
                "DATABASE_URL",
            ),
            (
                "RAG_DEBUGGER_WEB_ORIGIN",
                "https://localhost".to_owned(),
                "RAG_DEBUGGER_WEB_ORIGIN",
            ),
            (
                "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
                "https://127.0.0.1".to_owned(),
                "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
            ),
            (
                "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
                "https://[::1]".to_owned(),
                "RAG_DEBUGGER_PUBLIC_API_BASE_URL",
            ),
        ];

        for (name, value, expected_name) in cases {
            let mut command = hosted_from_env_command("staging", "unsafe", expected_name);
            command.env(name, value);
            run_hosted_from_env(&mut command);
        }
    }

    #[test]
    fn staging_and_production_accept_the_safe_hosted_contract() {
        for environment in [RuntimeEnvironment::Staging, RuntimeEnvironment::Production] {
            hosted_config(environment)
                .validate()
                .expect("safe hosted config");
        }
    }

    #[test]
    fn hosted_contract_rejects_unsafe_values_before_startup() {
        type UnsafeCase = (&'static str, fn(&mut ApiConfig));

        let cases: [UnsafeCase; 13] = [
            ("loopback bind", |config| {
                config.bind_addr = "127.0.0.1:8080".parse().expect("valid socket")
            }),
            ("memory storage", |config| {
                config.storage_backend = StorageBackend::Memory
            }),
            ("insecure web origin", |config| {
                config.web_origin = "http://app.example.com".to_owned()
            }),
            ("API origin path", |config| {
                config.product.ui.api_base_url = "https://api.example.com/v1".to_owned()
            }),
            ("insecure cookie", |config| {
                config.auth.cookie_secure = false
            }),
            ("unscoped cookie name", |config| {
                config.auth.session_cookie_name = "corpuslab_session".to_owned()
            }),
            ("unbounded session", |config| {
                config.auth.session_ttl_hours = 169
            }),
            ("short bootstrap password", |config| {
                config.auth.bootstrap_password = "too-short".to_owned()
            }),
            ("non-hosted mode", |config| {
                config.product.product.deployment_mode = DeploymentMode::Hybrid
            }),
            ("unimplemented external auth", |config| {
                config.auth.provider = AuthProviderKind::External
            }),
            ("verbose logs", |config| {
                config.log_filter = "rag_debugger_api=debug,info".to_owned()
            }),
            ("mutable release identity", |config| {
                config.release_sha = "main".to_owned()
            }),
            ("oversized upload boundary", |config| {
                config.product.ingestion.max_request_bytes = HOSTED_MAX_REQUEST_BYTES + 1
            }),
        ];

        for (name, mutate) in cases {
            let mut config = hosted_config(RuntimeEnvironment::Production);
            mutate(&mut config);
            assert!(
                matches!(
                    config.validate(),
                    Err(ConfigError::UnsafeHostedConfiguration { .. })
                ),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn hosted_origin_and_database_validation_is_strict() {
        for origin in [
            "http://app.example.com",
            "https://user@app.example.com",
            "https://app.example.com/",
            "https://app.example.com/path",
            "https://app.example.com?query=true",
        ] {
            assert!(validate_https_origin("TEST_ORIGIN", origin).is_err());
        }
        assert!(validate_https_origin("TEST_ORIGIN", "https://app.example.com").is_ok());

        for database_url in [
            "postgres://postgres:postgres@db.internal/corpuslab?sslmode=require",
            "postgres://corpuslab:secret@localhost/corpuslab?sslmode=require",
            "postgres://corpuslab:secret@db.internal/corpuslab",
            "postgres://corpuslab@db.internal/corpuslab?sslmode=require",
        ] {
            assert!(validate_hosted_database_url(database_url).is_err());
        }
        assert!(validate_hosted_database_url(
            "postgres://corpuslab:secret@db.internal/corpuslab?sslmode=verify-full"
        )
        .is_ok());
    }

    #[test]
    fn runtime_environment_values_are_explicit() {
        assert_eq!(
            parse_runtime_environment("staging").ok(),
            Some(RuntimeEnvironment::Staging)
        );
        assert!(matches!(
            parse_runtime_environment("preview"),
            Err(ConfigError::InvalidEnvironment(_))
        ));
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

    #[test]
    fn missing_bootstrap_password_is_rejected() {
        assert!(matches!(
            validate_required_env_string(
                "RAG_DEBUGGER_BOOTSTRAP_PASSWORD",
                Err(VarError::NotPresent)
            ),
            Err(ConfigError::MissingEnvironmentVariable {
                name: "RAG_DEBUGGER_BOOTSTRAP_PASSWORD"
            })
        ));
    }

    #[test]
    fn empty_bootstrap_password_is_rejected() {
        assert!(matches!(
            validate_required_env_string("RAG_DEBUGGER_BOOTSTRAP_PASSWORD", Ok("  ".to_owned())),
            Err(ConfigError::EmptyEnvironmentVariable {
                name: "RAG_DEBUGGER_BOOTSTRAP_PASSWORD"
            })
        ));
    }

    #[test]
    fn explicitly_supplied_bootstrap_password_is_accepted() {
        let password = "test-only-bootstrap-password".to_owned();

        assert_eq!(
            validate_required_env_string("RAG_DEBUGGER_BOOTSTRAP_PASSWORD", Ok(password.clone()))
                .ok(),
            Some(password)
        );
    }
}
