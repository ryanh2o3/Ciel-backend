use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationAction {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFlag {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub target_id: Uuid,
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}
