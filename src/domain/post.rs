use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub owner_handle: Option<String>,
    pub owner_display_name: Option<String>,
    pub media_id: Uuid,
    pub caption: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    pub visibility: PostVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostVisibility {
    Public,
    FollowersOnly,
}

impl PostVisibility {
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "public" => Some(Self::Public),
            "followers_only" => Some(Self::FollowersOnly),
            _ => None,
        }
    }

    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::FollowersOnly => "followers_only",
        }
    }
}

