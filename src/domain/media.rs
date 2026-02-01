use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub original_key: String,
    pub thumb_key: String,
    pub medium_key: String,
    pub width: i32,
    pub height: i32,
    pub bytes: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub thumb_url: Option<String>,
    pub medium_url: Option<String>,
    pub original_url: Option<String>,
}

