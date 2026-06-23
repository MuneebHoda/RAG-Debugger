use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct HealthResponse {
    pub status: String,
}

impl HealthResponse {
    pub fn alive() -> Self {
        Self {
            status: "ok".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ReadinessResponse {
    pub status: String,
}

impl ReadinessResponse {
    pub fn ready() -> Self {
        Self {
            status: "ready".to_owned(),
        }
    }
}
