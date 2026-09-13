package com.ciel.app;

import com.ciel.config.RateLimits;
import com.ciel.config.RateWindow;
import com.ciel.config.TrustLevel;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.data.redis.core.script.DefaultRedisScript;
import org.springframework.stereotype.Component;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

/**
 * Redis-backed rate limiter. Mirrors Rust's {@code app::rate_limiter::RateLimiter}
 * exactly: same Redis key format, same Lua INCR+EXPIRE script, and the same
 * fixed-window semantics, so the Rust and Spring implementations share quota
 * atomically against the same Redis instance.
 */
@Component
public class RateLimiter {

    private static final Logger log = LoggerFactory.getLogger(RateLimiter.class);

    /**
     * Atomically increments the counter and sets the TTL on first increment.
     * Doing this in one Lua script closes the check-then-increment race that
     * would otherwise let bursts of concurrent requests exceed the limit.
     */
    private static final DefaultRedisScript<Long> INCR_WITH_TTL_SCRIPT = new DefaultRedisScript<>(
            """
            local count = redis.call('INCR', KEYS[1])
            if count == 1 then
              redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            return count
            """,
            Long.class);

    private final StringRedisTemplate redis;

    public RateLimiter(StringRedisTemplate redis) {
        this.redis = redis;
    }

    /** Matches Rust's {@code config::rate_limits::current_window}. */
    public static long currentWindow(long windowSeconds) {
        return Instant.now().getEpochSecond() / windowSeconds;
    }

    public record RateLimitInfo(boolean limited, long limit, long remaining) {}

    private record Check(long limit, RateWindow window) {}

    private long incrWithTtl(String key, long ttlSeconds) {
        Long count = redis.execute(INCR_WITH_TTL_SCRIPT, List.of(key), Long.toString(ttlSeconds));
        return count == null ? 0 : count;
    }

    /**
     * Atomically check-and-consume one unit of quota for an action. Redis
     * errors propagate to the caller (fail closed) rather than silently
     * allowing unlimited traffic when Redis is unavailable.
     */
    public RateLimitInfo checkAndIncrement(UUID userId, String action, TrustLevel trustLevel) {
        RateLimits limits = RateLimits.forTrustLevel(trustLevel);

        List<Check> checks = switch (action) {
            case "post" -> List.of(
                    new Check(limits.getPostsPerHour(), RateWindow.HOUR),
                    new Check(limits.getPostsPerDay(), RateWindow.DAY));
            case "follow" -> List.of(
                    new Check(limits.getFollowsPerHour(), RateWindow.HOUR),
                    new Check(limits.getFollowsPerDay(), RateWindow.DAY));
            case "unfollow" -> List.of(new Check(limits.getUnfollowsPerDay(), RateWindow.DAY));
            case "like" -> List.of(new Check(limits.getLikesPerHour(), RateWindow.HOUR));
            case "comment" -> List.of(new Check(limits.getCommentsPerHour(), RateWindow.HOUR));
            case "login" -> List.of(new Check(limits.getLoginAttemptsPerHour(), RateWindow.HOUR));
            case "feed" -> List.of(new Check(limits.getFeedRequestsPerHour(), RateWindow.HOUR));
            case "notifications" -> List.of(new Check(limits.getNotificationsPerHour(), RateWindow.HOUR));
            case "search" -> List.of(new Check(limits.getSearchRequestsPerHour(), RateWindow.HOUR));
            case "media_read" -> List.of(new Check(limits.getMediaReadPerHour(), RateWindow.HOUR));
            case "media_upload" -> List.of(new Check(limits.getMediaUploadPerHour(), RateWindow.HOUR));
            case "moderation" -> List.of(new Check(limits.getModerationActionsPerHour(), RateWindow.HOUR));
            default -> List.of();
        };

        if (checks.isEmpty()) {
            return new RateLimitInfo(false, 0, 0);
        }

        // Track the tightest (most constrained) window for response headers.
        long minRemaining = Long.MAX_VALUE;
        long effectiveLimit = 0;
        boolean limited = false;

        for (Check check : checks) {
            long windowSeconds = check.window().seconds();
            String key = "ratelimit:%s:%s:%d".formatted(userId, action, currentWindow(windowSeconds));

            long count = incrWithTtl(key, windowSeconds);
            long remaining = Math.max(0, check.limit() - count);

            if (remaining < minRemaining) {
                minRemaining = remaining;
                effectiveLimit = check.limit();
            }

            if (count > check.limit()) {
                log.debug("rate limit exceeded: user={} action={} window={} count={} limit={}",
                        userId, action, check.window(), count, check.limit());
                limited = true;
            }
        }

        return new RateLimitInfo(limited, effectiveLimit, minRemaining);
    }

    /**
     * Atomically check-and-consume IP-based quota (for unauthenticated
     * requests). Returns true when the request should be rejected.
     */
    public boolean checkAndIncrementIp(String ip, String action, long limit, RateWindow window) {
        long windowSeconds = window.seconds();
        String key = "ratelimit:ip:%s:%s:%d".formatted(ip, action, currentWindow(windowSeconds));

        long count = incrWithTtl(key, windowSeconds);

        if (count > limit) {
            log.debug("IP rate limit exceeded: ip={} action={} count={} limit={}", ip, action, count, limit);
            return true;
        }
        return false;
    }
}
