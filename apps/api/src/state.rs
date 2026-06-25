use std::sync::Arc;

use rag_debugger_storage::repository::AppRepository;

use crate::config::{ApiConfig, RuntimeEnvironment};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: ApiConfig,
    repository: Option<Arc<dyn AppRepository>>,
}

impl AppState {
    pub fn new(config: ApiConfig, repository: Arc<dyn AppRepository>) -> Self {
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

    pub fn repository(&self) -> Option<Arc<dyn AppRepository>> {
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
