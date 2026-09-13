package com.ciel.app;

import com.ciel.domain.Comment;
import com.ciel.domain.Like;
import com.ciel.domain.Post;
import com.ciel.domain.PostVisibility;
import com.ciel.web.error.ApiException;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.sql.Array;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Timestamp;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@Service
public class PostService {

    private final JdbcTemplate jdbc;

    public PostService(JdbcTemplate jdbc) {
        this.jdbc = jdbc;
    }

    @Transactional
    public Post createPost(UUID ownerId, List<UUID> mediaIds, String caption) {
        if (mediaIds == null || mediaIds.isEmpty()) {
            throw ApiException.badRequest("at least one media_id is required");
        }
        if (mediaIds.size() > 10) {
            throw ApiException.badRequest("maximum 10 images per post");
        }
        if (new HashSet<>(mediaIds).size() != mediaIds.size()) {
            throw ApiException.badRequest("media_ids must be unique");
        }

        String inClause = String.join(",", java.util.Collections.nCopies(mediaIds.size(), "?"));
        List<Object> countArgs = new ArrayList<>();
        countArgs.add(ownerId);
        countArgs.addAll(mediaIds);
        Long owned = jdbc.queryForObject(
                "SELECT COUNT(*) FROM media WHERE owner_id = ? AND id IN (" + inClause + ")",
                Long.class,
                countArgs.toArray());
        if (owned == null || owned != mediaIds.size()) {
            throw ApiException.badRequest("invalid media_id");
        }

        UUID postId;
        try {
            postId = jdbc.queryForObject(
                    """
                    INSERT INTO posts (owner_id, caption, visibility)
                    VALUES (?, ?, ?::post_visibility)
                    RETURNING id
                    """,
                    UUID.class,
                    ownerId,
                    caption,
                    PostVisibility.PUBLIC.asDb());
        } catch (DataIntegrityViolationException e) {
            throw ApiException.badRequest("invalid media_id");
        }

        for (int i = 0; i < mediaIds.size(); i++) {
            try {
                jdbc.update(
                        "INSERT INTO post_media (post_id, media_id, position) VALUES (?, ?, ?)",
                        postId,
                        mediaIds.get(i),
                        (short) i);
            } catch (DataIntegrityViolationException e) {
                throw ApiException.badRequest("invalid media_id");
            }
        }

        return jdbc.query(
                """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.id = ?
                """,
                rs -> {
                    if (!rs.next()) {
                        throw ApiException.internal("create post failed");
                    }
                    return mapPostRow(rs, readUuidList(rs, "media_ids"), false);
                },
                postId);
    }

    public Optional<Post> getPost(UUID postId, Optional<UUID> viewerId) {
        if (viewerId.isPresent()) {
            UUID viewer = viewerId.get();
            return jdbc.query(
                    """
                    SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                           u.avatar_key AS owner_avatar_key,
                           COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                           p.caption, p.visibility::text AS visibility, p.created_at,
                           (SELECT COUNT(*)::bigint FROM likes l WHERE l.post_id = p.id) AS like_count,
                           (SELECT COUNT(*)::bigint FROM comments c WHERE c.post_id = p.id) AS comment_count,
                           EXISTS(SELECT 1 FROM likes l WHERE l.post_id = p.id AND l.user_id = ?) AS liked_by_viewer
                    FROM posts p
                    JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                    WHERE p.id = ?
                      AND (p.visibility = 'public'
                           OR p.owner_id = ?
                           OR (p.visibility = 'followers_only' AND EXISTS (
                               SELECT 1 FROM follows WHERE follower_id = ? AND followee_id = p.owner_id
                           )))
                      AND NOT EXISTS (
                          SELECT 1 FROM blocks
                          WHERE (blocker_id = p.owner_id AND blocked_id = ?)
                             OR (blocker_id = ? AND blocked_id = p.owner_id)
                      )
                    """,
                    rs -> rs.next() ? Optional.of(mapPostRow(rs, readUuidList(rs, "media_ids"), true)) : Optional.empty(),
                    viewer,
                    postId,
                    viewer,
                    viewer,
                    viewer,
                    viewer);
        }
        return jdbc.query(
                """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at,
                       (SELECT COUNT(*)::bigint FROM likes l WHERE l.post_id = p.id) AS like_count,
                       (SELECT COUNT(*)::bigint FROM comments c WHERE c.post_id = p.id) AS comment_count
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.id = ? AND p.visibility = 'public'
                """,
                rs -> rs.next() ? Optional.of(mapPostRow(rs, readUuidList(rs, "media_ids"), false)) : Optional.empty(),
                postId);
    }

    public boolean canViewPost(UUID postId, UUID viewerId) {
        Boolean visible = jdbc.queryForObject(
                """
                SELECT true
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.id = ?
                  AND (p.visibility = 'public'
                       OR p.owner_id = ?
                       OR (p.visibility = 'followers_only' AND EXISTS (
                           SELECT 1 FROM follows WHERE follower_id = ? AND followee_id = p.owner_id
                       )))
                  AND NOT EXISTS (
                      SELECT 1 FROM blocks
                      WHERE (blocker_id = p.owner_id AND blocked_id = ?)
                         OR (blocker_id = ? AND blocked_id = p.owner_id)
                  )
                """,
                Boolean.class,
                postId,
                viewerId,
                viewerId,
                viewerId,
                viewerId);
        return Boolean.TRUE.equals(visible);
    }

    public Optional<Like> likePost(UUID userId, UUID postId) {
        return jdbc.query(
                """
                INSERT INTO likes (user_id, post_id) VALUES (?, ?)
                ON CONFLICT DO NOTHING
                RETURNING id, user_id, post_id, created_at
                """,
                rs -> rs.next() ? Optional.of(mapLike(rs)) : Optional.empty(),
                userId,
                postId);
    }

    public boolean unlikePost(UUID userId, UUID postId) {
        int rows = jdbc.update("DELETE FROM likes WHERE user_id = ? AND post_id = ?", userId, postId);
        return rows > 0;
    }

    public Comment addComment(UUID userId, UUID postId, String body) {
        return jdbc.queryForObject(
                """
                WITH inserted AS (
                    INSERT INTO comments (user_id, post_id, body) VALUES (?, ?, ?)
                    RETURNING id, user_id, post_id, body, created_at
                )
                SELECT i.id, i.user_id, i.post_id, i.body, i.created_at,
                       u.handle AS user_handle, u.display_name AS user_display_name
                FROM inserted i
                JOIN users u ON u.id = i.user_id
                """,
                (rs, rowNum) -> mapComment(rs),
                userId,
                postId,
                body);
    }

    private static Like mapLike(ResultSet rs) throws SQLException {
        return Like.builder()
                .id((UUID) rs.getObject("id"))
                .userId((UUID) rs.getObject("user_id"))
                .postId((UUID) rs.getObject("post_id"))
                .createdAt(toOffset(rs.getTimestamp("created_at")))
                .build();
    }

    private static Comment mapComment(ResultSet rs) throws SQLException {
        return Comment.builder()
                .id((UUID) rs.getObject("id"))
                .userId((UUID) rs.getObject("user_id"))
                .postId((UUID) rs.getObject("post_id"))
                .body(rs.getString("body"))
                .createdAt(toOffset(rs.getTimestamp("created_at")))
                .userHandle(rs.getString("user_handle"))
                .userDisplayName(rs.getString("user_display_name"))
                .build();
    }

    static Post mapPostRow(ResultSet rs, List<UUID> mediaIds, boolean withViewer) throws SQLException {
        String vis = rs.getString("visibility");
        PostVisibility visibility = PostVisibility.fromDb(vis);
        if (visibility == null) {
            throw ApiException.internal("unknown post visibility: " + vis);
        }
        Post.PostBuilder builder = Post.builder()
                .id((UUID) rs.getObject("id"))
                .ownerId((UUID) rs.getObject("owner_id"))
                .ownerHandle(rs.getString("owner_handle"))
                .ownerDisplayName(rs.getString("owner_display_name"))
                .mediaIds(mediaIds)
                .caption(rs.getString("caption"))
                .visibility(visibility)
                .createdAt(toOffset(rs.getTimestamp("created_at")))
                .ownerAvatarKey(rs.getString("owner_avatar_key"));
        if (columnPresent(rs, "like_count")) {
            builder.likeCount(rs.getLong("like_count"));
        }
        if (columnPresent(rs, "comment_count")) {
            builder.commentCount(rs.getLong("comment_count"));
        }
        if (withViewer && columnPresent(rs, "liked_by_viewer")) {
            builder.likedByViewer(rs.getBoolean("liked_by_viewer"));
        }
        return builder.build();
    }

    static List<UUID> readUuidList(ResultSet rs, String column) throws SQLException {
        Array arr = rs.getArray(column);
        if (arr == null) {
            return List.of();
        }
        Object[] raw = (Object[]) arr.getArray();
        List<UUID> ids = new ArrayList<>(raw.length);
        for (Object o : raw) {
            ids.add((UUID) o);
        }
        return ids;
    }

    private static OffsetDateTime toOffset(Timestamp ts) {
        return ts == null ? null : OffsetDateTime.ofInstant(ts.toInstant(), ZoneOffset.UTC);
    }

    private static boolean columnPresent(ResultSet rs, String column) throws SQLException {
        var meta = rs.getMetaData();
        for (int i = 1; i <= meta.getColumnCount(); i++) {
            if (column.equals(meta.getColumnLabel(i))) {
                return true;
            }
        }
        return false;
    }
}
