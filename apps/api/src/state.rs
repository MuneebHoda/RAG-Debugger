use std::sync::Arc;

use crate::config::{ApiConfig, RuntimeEnvironment};

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Debug)]
struct AppStateInner {
    config: ApiConfig,
}

impl AppState {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            inner: Arc::new(AppStateInner { config }),
        }
    }

    pub fn config(&self) -> &ApiConfig {
        &self.inner.config
    }

    pub fn is_ready(&self) -> bool {
        // The scaffold has no required external dependencies yet.
        // Production readiness will include database and worker checks.
        !matches!(self.config().environment, RuntimeEnvironment::Test)
    }
}
