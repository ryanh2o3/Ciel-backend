using Ciel.Api.Config;
using StackExchange.Redis;

namespace Ciel.Api.Services;

public sealed record RateLimitInfo(bool Limited, long Limit, long Remaining);

/// <summary>
/// Redis-backed rate limiter matching Rust's <c>RateLimiter</c> (src/app/rate_limiter.rs)
/// key-for-key: same <c>ratelimit:{userId}:{action}:{window}</c> / <c>ratelimit:ip:{ip}:{action}:{window}</c>
/// key format, same atomic INCR+EXPIRE Lua script (closing the check-then-increment race), and
/// the same fail-closed behavior (Redis errors propagate instead of silently disabling limits).
/// </summary>
public sealed class RateLimiterService(IConnectionMultiplexer redis, ILogger<RateLimiterService> logger)
{
    // Atomically increments the counter and sets the TTL on first increment.
    private const string IncrWithTtlScript = """
        local count = redis.call('INCR', KEYS[1])
        if count == 1 then
          redis.call('EXPIRE', KEYS[1], ARGV[1])
        end
        return count
        """;

    private async Task<long> IncrWithTtlAsync(string key, long ttlSeconds)
    {
        var db = redis.GetDatabase();
        var result = await db.ScriptEvaluateAsync(IncrWithTtlScript, [(RedisKey)key], [(RedisValue)ttlSeconds]);
        return (long)result;
    }

    /// <summary>
    /// Atomically check-and-consume one unit of quota for an action. Errors
    /// propagate to the caller (fail closed) rather than silently allowing
    /// unlimited traffic when Redis is unavailable.
    /// </summary>
    public async Task<RateLimitInfo> CheckAndIncrementAsync(Guid userId, string action, TrustLevel trustLevel)
    {
        var limits = RateLimits.ForTrustLevel(trustLevel);

        var checks = action switch
        {
            "post" => new[] { (limits.PostsPerHour, RateWindow.Hour), (limits.PostsPerDay, RateWindow.Day) },
            "follow" => new[] { (limits.FollowsPerHour, RateWindow.Hour), (limits.FollowsPerDay, RateWindow.Day) },
            "unfollow" => new[] { (limits.UnfollowsPerDay, RateWindow.Day) },
            "like" => new[] { (limits.LikesPerHour, RateWindow.Hour) },
            "comment" => new[] { (limits.CommentsPerHour, RateWindow.Hour) },
            "login" => new[] { (limits.LoginAttemptsPerHour, RateWindow.Hour) },
            "feed" => new[] { (limits.FeedRequestsPerHour, RateWindow.Hour) },
            "notifications" => new[] { (limits.NotificationsPerHour, RateWindow.Hour) },
            "search" => new[] { (limits.SearchRequestsPerHour, RateWindow.Hour) },
            "media_read" => new[] { (limits.MediaReadPerHour, RateWindow.Hour) },
            "media_upload" => new[] { (limits.MediaUploadPerHour, RateWindow.Hour) },
            "moderation" => new[] { (limits.ModerationActionsPerHour, RateWindow.Hour) },
            _ => null,
        };

        if (checks is null)
        {
            return new RateLimitInfo(false, 0, 0);
        }

        // Track the tightest (most constrained) window for response headers.
        var minRemaining = long.MaxValue;
        long effectiveLimit = 0;
        var limited = false;

        foreach (var (limit, window) in checks)
        {
            var windowSeconds = window.Seconds();
            var key = $"ratelimit:{userId}:{action}:{RateWindowClock.CurrentWindow(windowSeconds)}";

            var count = await IncrWithTtlAsync(key, windowSeconds);
            var remaining = Math.Max(limit - count, 0);

            if (remaining < minRemaining)
            {
                minRemaining = remaining;
                effectiveLimit = limit;
            }

            if (count > limit)
            {
                logger.LogDebug(
                    "rate limit exceeded for user {UserId} action {Action} window {Window}: {Count}/{Limit}",
                    userId,
                    action,
                    window,
                    count,
                    limit);
                limited = true;
            }
        }

        return new RateLimitInfo(limited, effectiveLimit, minRemaining);
    }

    /// <summary>
    /// Atomically check-and-consume IP-based quota (for unauthenticated requests).
    /// Returns true when the request should be rejected.
    /// </summary>
    public async Task<bool> CheckAndIncrementIpAsync(string ip, string action, long limit, RateWindow window)
    {
        var windowSeconds = window.Seconds();
        var key = $"ratelimit:ip:{ip}:{action}:{RateWindowClock.CurrentWindow(windowSeconds)}";

        var count = await IncrWithTtlAsync(key, windowSeconds);
        if (count > limit)
        {
            logger.LogDebug("IP rate limit exceeded for {Ip} action {Action}: {Count}/{Limit}", ip, action, count, limit);
            return true;
        }

        return false;
    }
}

/// <summary>
/// Maps request path + method to a rate-limit action name. Mirrors Rust's
/// <c>rate_limit_action</c> / <c>ip_rate_limit_config</c> (src/http/middleware/rate_limit.rs)
/// so both stacks apply identical limits to identical routes.
/// </summary>
public static class RateLimitActions
{
    /// <summary>Strip <c>/v1</c> prefix so matchers work for nested API routes.</summary>
    private static string LogicalPath(string path) =>
        path.StartsWith("/v1", StringComparison.Ordinal) ? path["/v1".Length..] : path;

    public static string? ForAuthenticatedRequest(string path, string method)
    {
        var p = LogicalPath(path);

        if (p == "/posts" && method == "POST") return "post";
        if (method == "POST" && p.StartsWith("/posts/", StringComparison.Ordinal) && p.EndsWith("/like", StringComparison.Ordinal)) return "like";
        if (method == "POST" && p.StartsWith("/posts/", StringComparison.Ordinal) && p.EndsWith("/comment", StringComparison.Ordinal)) return "comment";
        if (method == "POST" && p.Contains("/follow", StringComparison.Ordinal) && !p.Contains("/unfollow", StringComparison.Ordinal)) return "follow";
        if (method == "POST" && p.Contains("/unfollow", StringComparison.Ordinal)) return "unfollow";
        if ((p == "/feed" || p == "/feed/stories") && method == "GET") return "feed";
        if (p == "/feed/refresh" && method == "POST") return "feed";
        if (p == "/stories" && method == "POST") return "post";
        if (method == "POST" && p.StartsWith("/stories/", StringComparison.Ordinal) && p.EndsWith("/seen", StringComparison.Ordinal)) return "like";
        if (method == "POST" && p.StartsWith("/stories/", StringComparison.Ordinal) && p.EndsWith("/reactions", StringComparison.Ordinal)) return "like";
        if (p.StartsWith("/notifications", StringComparison.Ordinal)) return "notifications";
        if (p.StartsWith("/search/", StringComparison.Ordinal)) return "search";
        if (method == "GET" && p.StartsWith("/media", StringComparison.Ordinal)) return "media_read";
        if (method == "POST" && p.StartsWith("/media", StringComparison.Ordinal)) return "media_upload";
        if (method == "DELETE" && p.StartsWith("/media", StringComparison.Ordinal)) return "media_upload";
        if (p.StartsWith("/moderation/", StringComparison.Ordinal)) return "moderation";

        return null;
    }

    public static (string Action, long Limit, RateWindow Window)? ForIpRequest(string path, string method, uint ipSignupLimit)
    {
        var p = LogicalPath(path);

        // Shared NAT / flaky first-run auth should not burn the whole hour.
        if (p == "/auth/login" && method == "POST") return ("login", 30, RateWindow.Hour);
        if (p == "/auth/refresh" && method == "POST") return ("auth_refresh", 120, RateWindow.Hour);
        if (p == "/auth/revoke" && method == "POST") return ("auth_revoke", 60, RateWindow.Hour);
        if (p == "/users" && method == "POST") return ("signup", ipSignupLimit, RateWindow.Day);
        if (p == "/health" && method == "GET") return ("health", 60, RateWindow.Minute);
        if (p == "/account/device/register" && method == "POST") return ("device_register", 60, RateWindow.Hour);
        if (method == "GET" && p.StartsWith("/invites/validate/", StringComparison.Ordinal)) return ("invite_validate", 120, RateWindow.Hour);

        return null;
    }
}
