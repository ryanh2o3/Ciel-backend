package com.ciel.config;

import lombok.Builder;
import lombok.Value;

/**
 * Rate limits for different user actions based on trust level. Mirrors Rust's
 * {@code config::rate_limits::RateLimits} — numeric values must stay in sync
 * with {@code src/config/rate_limits.rs} since both implementations share the
 * same Redis keys and are expected to enforce identical limits.
 */
@Value
@Builder
public class RateLimits {
    // Posts
    long postsPerHour;
    long postsPerDay;

    // Social actions
    long followsPerHour;
    long followsPerDay;
    long unfollowsPerDay;

    // Engagement
    long likesPerHour;
    long commentsPerHour;

    // Authentication
    long loginAttemptsPerHour;

    // Read-heavy endpoints
    long feedRequestsPerHour;
    long notificationsPerHour;
    long searchRequestsPerHour;

    // Media (separated into read vs write operations)
    long mediaReadPerHour;
    long mediaUploadPerHour;

    // Moderation
    long moderationActionsPerHour;

    /** Get rate limits for a specific trust level. */
    public static RateLimits forTrustLevel(TrustLevel level) {
        return switch (level) {
            // New users need room to explore (posts, stories, follows, uploads)
            // without hitting 429s on first session; still well below Basic.
            case NEW -> RateLimits.builder()
                    .postsPerHour(10)
                    .postsPerDay(30)
                    .followsPerHour(20)
                    .followsPerDay(50)
                    .unfollowsPerDay(30)
                    .likesPerHour(100)
                    .commentsPerHour(30)
                    .loginAttemptsPerHour(20)
                    .feedRequestsPerHour(300)
                    .notificationsPerHour(300)
                    .searchRequestsPerHour(120)
                    .mediaReadPerHour(1000)
                    .mediaUploadPerHour(30)
                    .moderationActionsPerHour(30)
                    .build();
            case BASIC -> RateLimits.builder()
                    .postsPerHour(15)
                    .postsPerDay(50)
                    .followsPerHour(40)
                    .followsPerDay(150)
                    .unfollowsPerDay(75)
                    .likesPerHour(200)
                    .commentsPerHour(60)
                    .loginAttemptsPerHour(30)
                    .feedRequestsPerHour(600)
                    .notificationsPerHour(600)
                    .searchRequestsPerHour(300)
                    .mediaReadPerHour(2000)
                    .mediaUploadPerHour(60)
                    .moderationActionsPerHour(50)
                    .build();
            case TRUSTED -> RateLimits.builder()
                    .postsPerHour(20)
                    .postsPerDay(100)
                    .followsPerHour(100)
                    .followsPerDay(500)
                    .unfollowsPerDay(200)
                    .likesPerHour(500)
                    .commentsPerHour(100)
                    .loginAttemptsPerHour(20)
                    .feedRequestsPerHour(1000)
                    .notificationsPerHour(1000)
                    .searchRequestsPerHour(600)
                    .mediaReadPerHour(3000)
                    .mediaUploadPerHour(100)
                    .moderationActionsPerHour(100)
                    .build();
            case VERIFIED -> RateLimits.builder()
                    .postsPerHour(500)
                    .postsPerDay(2000)
                    .followsPerHour(2000)
                    .followsPerDay(10000)
                    .unfollowsPerDay(5000)
                    .likesPerHour(10000)
                    .commentsPerHour(2000)
                    .loginAttemptsPerHour(300)
                    .feedRequestsPerHour(20000)
                    .notificationsPerHour(20000)
                    .searchRequestsPerHour(12000)
                    .mediaReadPerHour(10000)
                    .mediaUploadPerHour(500)
                    .moderationActionsPerHour(2000)
                    .build();
        };
    }
}
