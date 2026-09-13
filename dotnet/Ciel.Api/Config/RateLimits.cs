namespace Ciel.Api.Config;

/// <summary>
/// Trust levels for users, determining their rate limits and privileges.
/// Mirrors Rust's <c>TrustLevel</c> (src/config/rate_limits.rs).
/// </summary>
public enum TrustLevel
{
    New = 0,      // 0-7 days, < 5 posts
    Basic = 1,    // 7-30 days, 5+ posts, no violations
    Trusted = 2,  // 30+ days, 50+ posts, active engagement
    Verified = 3, // Manual verification or high trust score
}

public static class TrustLevelExtensions
{
    public static TrustLevel FromInt32(int value) => value switch
    {
        0 => TrustLevel.New,
        1 => TrustLevel.Basic,
        2 => TrustLevel.Trusted,
        3 => TrustLevel.Verified,
        _ => TrustLevel.New,
    };

    public static int AsInt32(this TrustLevel level) => (int)level;
}

/// <summary>Time window for rate limiting. Mirrors Rust's <c>RateWindow</c>.</summary>
public enum RateWindow
{
    Minute,
    Hour,
    Day,
}

public static class RateWindowExtensions
{
    public static long Seconds(this RateWindow window) => window switch
    {
        RateWindow.Minute => 60,
        RateWindow.Hour => 3600,
        RateWindow.Day => 86400,
        _ => throw new ArgumentOutOfRangeException(nameof(window)),
    };
}

/// <summary>
/// Rate limits for different user actions based on trust level.
/// Mirrors Rust's <c>RateLimits</c> (src/config/rate_limits.rs) field-for-field
/// and value-for-value so both stacks enforce identical thresholds against the
/// same Redis keys.
/// </summary>
public sealed record RateLimits(
    // Posts
    uint PostsPerHour,
    uint PostsPerDay,

    // Social actions
    uint FollowsPerHour,
    uint FollowsPerDay,
    uint UnfollowsPerDay,

    // Engagement
    uint LikesPerHour,
    uint CommentsPerHour,

    // Authentication
    uint LoginAttemptsPerHour,

    // Read-heavy endpoints
    uint FeedRequestsPerHour,
    uint NotificationsPerHour,
    uint SearchRequestsPerHour,

    // Media (separated into read vs write operations)
    uint MediaReadPerHour,
    uint MediaUploadPerHour,

    // Moderation
    uint ModerationActionsPerHour)
{
    public static RateLimits ForTrustLevel(TrustLevel level) => level switch
    {
        // New users need room to explore (posts, stories, follows, uploads)
        // without hitting 429s on first session; still well below Basic.
        TrustLevel.New => new RateLimits(
            PostsPerHour: 10,
            PostsPerDay: 30,
            FollowsPerHour: 20,
            FollowsPerDay: 50,
            UnfollowsPerDay: 30,
            LikesPerHour: 100,
            CommentsPerHour: 30,
            LoginAttemptsPerHour: 20,
            FeedRequestsPerHour: 300,
            NotificationsPerHour: 300,
            SearchRequestsPerHour: 120,
            MediaReadPerHour: 1000,
            MediaUploadPerHour: 30,
            ModerationActionsPerHour: 30),

        TrustLevel.Basic => new RateLimits(
            PostsPerHour: 15,
            PostsPerDay: 50,
            FollowsPerHour: 40,
            FollowsPerDay: 150,
            UnfollowsPerDay: 75,
            LikesPerHour: 200,
            CommentsPerHour: 60,
            LoginAttemptsPerHour: 30,
            FeedRequestsPerHour: 600,
            NotificationsPerHour: 600,
            SearchRequestsPerHour: 300,
            MediaReadPerHour: 2000,
            MediaUploadPerHour: 60,
            ModerationActionsPerHour: 50),

        TrustLevel.Trusted => new RateLimits(
            PostsPerHour: 20,
            PostsPerDay: 100,
            FollowsPerHour: 100,
            FollowsPerDay: 500,
            UnfollowsPerDay: 200,
            LikesPerHour: 500,
            CommentsPerHour: 100,
            LoginAttemptsPerHour: 20,
            FeedRequestsPerHour: 1000,
            NotificationsPerHour: 1000,
            SearchRequestsPerHour: 600,
            MediaReadPerHour: 3000,
            MediaUploadPerHour: 100,
            ModerationActionsPerHour: 100),

        TrustLevel.Verified => new RateLimits(
            PostsPerHour: 500,
            PostsPerDay: 2000,
            FollowsPerHour: 2000,
            FollowsPerDay: 10000,
            UnfollowsPerDay: 5000,
            LikesPerHour: 10000,
            CommentsPerHour: 2000,
            LoginAttemptsPerHour: 300,
            FeedRequestsPerHour: 20000,
            NotificationsPerHour: 20000,
            SearchRequestsPerHour: 12000,
            MediaReadPerHour: 10000,
            MediaUploadPerHour: 500,
            ModerationActionsPerHour: 2000),

        _ => throw new ArgumentOutOfRangeException(nameof(level)),
    };
}

/// <summary>Calculate current window bucket for rate limiting. Mirrors Rust's <c>current_window</c>.</summary>
public static class RateWindowClock
{
    public static long CurrentWindow(long windowSeconds) =>
        DateTimeOffset.UtcNow.ToUnixTimeSeconds() / windowSeconds;
}
