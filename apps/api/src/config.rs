use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use thiserror::Error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiConfig {
    pub environment: RuntimeEnvironment,
    pub bind_addr: SocketAddr,
    pub database_url: String,
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

        Ok(Self {
            environment,
            bind_addr: SocketAddr::new(host, port),
            database_url,
        })
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
            database_url: "postgres://postgres:postgres@localhost:5432/rag_debugger".to_owned(),
        };

        assert_eq!(config.environment, RuntimeEnvironment::Local);
        assert_eq!(config.bind_addr.port(), 8080);
    }
}
