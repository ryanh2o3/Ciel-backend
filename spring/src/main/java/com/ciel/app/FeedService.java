package com.ciel.app;

import com.ciel.domain.Post;
import com.ciel.domain.PostVisibility;
import com.ciel.web.error.ApiException;
import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.databind.ObjectMapper;
import lombok.Data;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@Service
public class FeedService {

    private static final Logger log = LoggerFactory.getLogger(FeedService.class);
    private static final long FEED_CACHE_TTL_SECONDS = 30;

    private final JdbcTemplate jdbc;
    private final StringRedisTemplate redis;
    private final ObjectMapper objectMapper;

    public FeedService(JdbcTemplate jdbc, StringRedisTemplate redis, ObjectMapper objectMapper) {
        this.jdbc = jdbc;
        this.redis = redis;
        this.objectMapper = objectMapper;
    }

    public record FeedPage(List<Post> posts, Optional<CursorUtil.Cursor> nextCursor) {}

    public FeedPage getHomeFeed(UUID userId, Optional<CursorUtil.Cursor> cursor, int limit) {
        boolean shouldCache = cursor.isEmpty();
        String cacheKey = "feed:home:" + userId;

        if (shouldCache) {
            try {
                String payload = redis.opsForValue().get(cacheKey);
                if (payload != null) {
                    CachedHomeFeed cached = objectMapper.readValue(payload, CachedHomeFeed.class);
                    FeedPage page = cached.toPage();
                    if (page != null) {
                        return page;
                    }
                }
            } catch (Exception e) {
                log.warn("failed to read feed cache: {}", e.toString());
            }
        }

        int limitPlus = limit + 1;
        List<Post> posts;
        if (cursor.isPresent()) {
            CursorUtil.Cursor c = cursor.get();
            posts = jdbc.query(
                    """
                    SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                           u.avatar_key AS owner_avatar_key,
                           COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                           p.caption, p.visibility::text AS visibility, p.created_at
                    FROM posts p
                    JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                    WHERE (p.owner_id = ?
                       OR (p.owner_id IN (
                           SELECT followee_id FROM follows WHERE follower_id = ?
                       ) AND NOT EXISTS (
                           SELECT 1 FROM blocks
                           WHERE (blocker_id = p.owner_id AND blocked_id = ?)
                              OR (blocker_id = ? AND blocked_id = p.owner_id)
                       )))
                      AND (p.created_at < ? OR (p.created_at = ? AND p.id < ?))
                    ORDER BY p.created_at DESC, p.id DESC
                    LIMIT ?
                    """,
                    (rs, rowNum) -> mapFeedPost(rs),
                    userId,
                    userId,
                    userId,
                    userId,
                    Timestamp.from(c.timestamp().toInstant()),
                    Timestamp.from(c.timestamp().toInstant()),
                    c.id(),
                    limitPlus);
        } else {
            posts = jdbc.query(
                    """
                    SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                           u.avatar_key AS owner_avatar_key,
                           COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                           p.caption, p.visibility::text AS visibility, p.created_at
                    FROM posts p
                    JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                    WHERE p.owner_id = ?
                       OR (p.owner_id IN (
                           SELECT followee_id FROM follows WHERE follower_id = ?
                       ) AND NOT EXISTS (
                           SELECT 1 FROM blocks
                           WHERE (blocker_id = p.owner_id AND blocked_id = ?)
                              OR (blocker_id = ? AND blocked_id = p.owner_id)
                       ))
                    ORDER BY p.created_at DESC, p.id DESC
                    LIMIT ?
                    """,
                    (rs, rowNum) -> mapFeedPost(rs),
                    userId,
                    userId,
                    userId,
                    userId,
                    limitPlus);
        }

        Optional<CursorUtil.Cursor> next = Optional.empty();
        if (posts.size() > limit) {
            Post extra = posts.remove(posts.size() - 1);
            next = Optional.of(new CursorUtil.Cursor(extra.getCreatedAt(), extra.getId()));
        }

        if (shouldCache) {
            try {
                String payload = objectMapper.writeValueAsString(CachedHomeFeed.fromPage(posts, next));
                redis.opsForValue().set(cacheKey, payload, Duration.ofSeconds(FEED_CACHE_TTL_SECONDS));
            } catch (Exception e) {
                log.warn("failed to write feed cache: {}", e.toString());
            }
        }

        return new FeedPage(posts, next);
    }

    public void refreshHomeFeed(UUID userId) {
        try {
            redis.delete("feed:home:" + userId);
        } catch (Exception e) {
            log.warn("failed to invalidate feed cache for {}: {}", userId, e.toString());
        }
    }

    private static Post mapFeedPost(ResultSet rs) throws SQLException {
        String vis = rs.getString("visibility");
        PostVisibility visibility = PostVisibility.fromDb(vis);
        if (visibility == null) {
            throw ApiException.internal("unknown post visibility: " + vis);
        }
        Timestamp created = rs.getTimestamp("created_at");
        return Post.builder()
                .id((UUID) rs.getObject("id"))
                .ownerId((UUID) rs.getObject("owner_id"))
                .ownerHandle(rs.getString("owner_handle"))
                .ownerDisplayName(rs.getString("owner_display_name"))
                .mediaIds(PostService.readUuidList(rs, "media_ids"))
                .caption(rs.getString("caption"))
                .visibility(visibility)
                .createdAt(created == null ? null : OffsetDateTime.ofInstant(created.toInstant(), ZoneOffset.UTC))
                .ownerAvatarKey(rs.getString("owner_avatar_key"))
                .build();
    }

    /** Matches Rust {@code CachedHomeFeed} for cross-impl Redis compatibility. */
    @Data
    @JsonInclude(JsonInclude.Include.NON_NULL)
    static class CachedHomeFeed {
        private List<Post> posts;

        // Nanoseconds since the Unix epoch, matching Rust's cache encoding
        // (OffsetDateTime -> i64 nanos) exactly. A signed 64-bit nanosecond
        // count only overflows around the year 2262, so this is safe for the
        // foreseeable lifetime of the service in both implementations.
        private Long nextCursorNanos;
        private UUID nextCursorId;

        static CachedHomeFeed fromPage(List<Post> posts, Optional<CursorUtil.Cursor> next) {
            CachedHomeFeed c = new CachedHomeFeed();
            c.posts = new ArrayList<>(posts);
            next.ifPresent(cursor -> {
                c.nextCursorNanos = cursor.timestamp().toInstant().getEpochSecond() * 1_000_000_000L
                        + cursor.timestamp().getNano();
                c.nextCursorId = cursor.id();
            });
            return c;
        }

        FeedPage toPage() {
            if (posts == null) {
                return null;
            }
            Optional<CursorUtil.Cursor> next = Optional.empty();
            if (nextCursorNanos != null && nextCursorId != null) {
                long seconds = Math.floorDiv(nextCursorNanos, 1_000_000_000L);
                int nanos = (int) Math.floorMod(nextCursorNanos, 1_000_000_000L);
                OffsetDateTime ts = OffsetDateTime.ofInstant(
                        java.time.Instant.ofEpochSecond(seconds, nanos), ZoneOffset.UTC);
                next = Optional.of(new CursorUtil.Cursor(ts, nextCursorId));
            } else if (nextCursorNanos != null || nextCursorId != null) {
                return null;
            }
            return new FeedPage(posts, next);
        }
    }
}
