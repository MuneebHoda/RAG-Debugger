use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::privacy::PrivacyMode;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub privacy_mode: PrivacyMode,
    #[serde(with = "crate::wire_time")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::wire_time")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ProjectId(pub Uuid);
