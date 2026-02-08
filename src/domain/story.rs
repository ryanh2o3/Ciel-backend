use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_handle: Option<String>,
    pub user_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_avatar_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_avatar_url: Option<String>,
    pub media_id: Uuid,
    pub caption: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub visibility: StoryVisibility,
    pub view_count: i32,
    pub reaction_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoryVisibility {
    Public,
    FriendsOnly,
    CloseFriendsOnly,
}

impl StoryVisibility {
    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "public" => Some(Self::Public),
            "friends_only" => Some(Self::FriendsOnly),
            "close_friends_only" => Some(Self::CloseFriendsOnly),
            _ => None,
        }
    }

    pub fn as_db(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::FriendsOnly => "friends_only",
            Self::CloseFriendsOnly => "close_friends_only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryReaction {
    pub id: Uuid,
    pub story_id: Uuid,
    pub user_id: Uuid,
    pub user_handle: Option<String>,
    pub emoji: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryView {
    pub viewer_id: Uuid,
    pub viewer_handle: Option<String>,
    pub viewer_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer_avatar_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer_avatar_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub viewed_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryHighlight {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub cover_story_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryMetrics {
    pub story_id: Uuid,
    pub view_count: i32,
    pub reaction_count: i32,
    pub reactions_by_emoji: Vec<EmojiCount>,
    pub viewer_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmojiCount {
    pub emoji: String,
    pub count: i64,
}
