use serde::{Deserialize, Serialize};

/// Trust levels for users, determining their rate limits and privileges
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum TrustLevel {
    New = 0,      // 0-7 days, < 5 posts
    Basic = 1,    // 7-30 days, 5+ posts, no violations
    Trusted = 2,  // 30+ days, 50+ posts, active engagement
    Verified = 3, // Manual verification or high trust score
}

impl TrustLevel {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => TrustLevel::New,
            1 => TrustLevel::Basic,
            2 => TrustLevel::Trusted,
            3 => TrustLevel::Verified,
            _ => TrustLevel::New,
        }
    }

    #[allow(dead_code)]
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Rate limits for different user actions based on trust level
#[derive(Debug, Clone, Copy)]
pub struct RateLimits {
    // Posts
    pub posts_per_hour: u32,
    pub posts_per_day: u32,

    // Social actions
    pub follows_per_hour: u32,
    pub follows_per_day: u32,
    pub unfollows_per_day: u32,

    // Engagement
    pub likes_per_hour: u32,
    pub comments_per_hour: u32,

    // Authentication
    pub login_attempts_per_hour: u32,

    // Read-heavy endpoints
    pub feed_requests_per_hour: u32,
    pub notifications_per_hour: u32,
    pub search_requests_per_hour: u32,

    // Media & moderation
    pub media_requests_per_hour: u32,
    pub moderation_actions_per_hour: u32,
}

impl RateLimits {
    /// Get rate limits for a specific trust level
    pub fn for_trust_level(level: TrustLevel) -> Self {
        match level {
            TrustLevel::New => RateLimits {
                posts_per_hour: 1,
                posts_per_day: 5,
                follows_per_hour: 5,
                follows_per_day: 20,
                unfollows_per_day: 10,
                likes_per_hour: 30,
                comments_per_hour: 10,
                login_attempts_per_hour: 5,
                feed_requests_per_hour: 120,
                notifications_per_hour: 120,
                search_requests_per_hour: 60,
                media_requests_per_hour: 30,
                moderation_actions_per_hour: 20,
            },
            TrustLevel::Basic => RateLimits {
                posts_per_hour: 5,
                posts_per_day: 20,
                follows_per_hour: 20,
                follows_per_day: 100,
                unfollows_per_day: 50,
                likes_per_hour: 100,
                comments_per_hour: 30,
                login_attempts_per_hour: 10,
                feed_requests_per_hour: 300,
                notifications_per_hour: 300,
                search_requests_per_hour: 200,
                media_requests_per_hour: 60,
                moderation_actions_per_hour: 50,
            },
            TrustLevel::Trusted => RateLimits {
                posts_per_hour: 20,
                posts_per_day: 100,
                follows_per_hour: 100,
                follows_per_day: 500,
                unfollows_per_day: 200,
                likes_per_hour: 500,
                comments_per_hour: 100,
                login_attempts_per_hour: 20,
                feed_requests_per_hour: 1000,
                notifications_per_hour: 1000,
                search_requests_per_hour: 600,
                media_requests_per_hour: 200,
                moderation_actions_per_hour: 100,
            },
            TrustLevel::Verified => RateLimits {
                posts_per_hour: 500,
                posts_per_day: 2000,
                follows_per_hour: 2000,
                follows_per_day: 10000,
                unfollows_per_day: 5000,
                likes_per_hour: 10000,
                comments_per_hour: 2000,
                login_attempts_per_hour: 300,
                feed_requests_per_hour: 20000,
                notifications_per_hour: 20000,
                search_requests_per_hour: 12000,
                media_requests_per_hour: 4000,
                moderation_actions_per_hour: 2000,
            },
        }
    }

    /// Get the limit for a specific action type
    #[allow(dead_code)]
    pub fn limit_for_action(&self, action: &str, window: RateWindow) -> Option<u32> {
        match (action, window) {
            ("post", RateWindow::Hour) => Some(self.posts_per_hour),
            ("post", RateWindow::Day) => Some(self.posts_per_day),
            ("follow", RateWindow::Hour) => Some(self.follows_per_hour),
            ("follow", RateWindow::Day) => Some(self.follows_per_day),
            ("unfollow", RateWindow::Day) => Some(self.unfollows_per_day),
            ("like", RateWindow::Hour) => Some(self.likes_per_hour),
            ("comment", RateWindow::Hour) => Some(self.comments_per_hour),
            ("login", RateWindow::Hour) => Some(self.login_attempts_per_hour),
            ("feed", RateWindow::Hour) => Some(self.feed_requests_per_hour),
            ("notifications", RateWindow::Hour) => Some(self.notifications_per_hour),
            ("search", RateWindow::Hour) => Some(self.search_requests_per_hour),
            ("media", RateWindow::Hour) => Some(self.media_requests_per_hour),
            ("moderation", RateWindow::Hour) => Some(self.moderation_actions_per_hour),
            _ => None,
        }
    }
}

/// Time window for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateWindow {
    Hour,
    Day,
}

impl RateWindow {
    pub fn seconds(&self) -> u64 {
        match self {
            RateWindow::Hour => 3600,
            RateWindow::Day => 86400,
        }
    }
}

/// Calculate current window timestamp for rate limiting
pub fn current_window(window_seconds: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now / window_seconds
}
