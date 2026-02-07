use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub handle: String,
    pub email: String,
    pub display_name: String,
    pub bio: Option<String>,
    #[serde(skip_serializing)]
    pub avatar_key: Option<String>,
    /// Presigned URL for avatar (populated at response time)
    #[serde(skip_deserializing)]
    pub avatar_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    pub bio: Option<String>,
    /// Presigned URL for avatar (not the raw S3 key)
    pub avatar_url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl PublicUser {
    /// Create PublicUser with a resolved avatar URL
    pub fn from_user_with_url(user: User, avatar_url: Option<String>) -> Self {
        Self {
            id: user.id,
            handle: user.handle,
            display_name: user.display_name,
            bio: user.bio,
            avatar_url,
            created_at: user.created_at,
        }
    }
}

impl From<User> for PublicUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            handle: user.handle,
            display_name: user.display_name,
            bio: user.bio,
            avatar_url: user.avatar_url,
            created_at: user.created_at,
        }
    }
}

impl From<&User> for PublicUser {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            handle: user.handle.clone(),
            display_name: user.display_name.clone(),
            bio: user.bio.clone(),
            avatar_url: user.avatar_url.clone(),
            created_at: user.created_at,
        }
    }
}
