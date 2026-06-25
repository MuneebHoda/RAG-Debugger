use std::sync::Arc;

use rag_debugger_storage::repository::IngestionRepository;

use crate::config::{ApiConfig, RuntimeEnvironment};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: ApiConfig,
    repository: Option<Arc<dyn IngestionRepository>>,
}

impl AppState {
    pub fn new(config: ApiConfig, repository: Arc<dyn IngestionRepository>) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                repository: Some(repository),
            }),
        }
    }

    pub fn without_repository(config: ApiConfig) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config,
                repository: None,
            }),
        }
    }

    pub fn config(&self) -> &ApiConfig {
        &self.inner.config
    }

    pub fn repository(&self) -> Option<Arc<dyn IngestionRepository>> {
        self.inner.repository.clone()
    }

    pub async fn is_ready(&self) -> bool {
        if matches!(self.config().environment, RuntimeEnvironment::Test) {
            return false;
        }

        match self.repository() {
            Some(repository) => repository.ping().await.is_ok(),
            None => false,
        }
    }
}
