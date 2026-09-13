package com.ciel.app;

import com.ciel.config.AppProperties;
import com.ciel.config.SqsQueueUrl;
import com.ciel.domain.Media;
import com.ciel.domain.Post;
import com.ciel.domain.User;
import com.ciel.web.dto.UploadHeader;
import com.ciel.web.dto.UploadIntent;
import com.ciel.web.dto.UploadStatus;
import com.ciel.web.error.ApiException;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;
import software.amazon.awssdk.services.s3.model.GetObjectRequest;
import software.amazon.awssdk.services.s3.model.PutObjectRequest;
import software.amazon.awssdk.services.s3.presigner.S3Presigner;
import software.amazon.awssdk.services.s3.presigner.model.PresignedGetObjectRequest;
import software.amazon.awssdk.services.s3.presigner.model.PresignedPutObjectRequest;
import software.amazon.awssdk.services.sqs.SqsClient;

import java.net.URI;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Timestamp;
import java.time.Duration;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;

@Service
public class MediaService {

    private static final Logger log = LoggerFactory.getLogger(MediaService.class);
    private static final long PRESIGN_GET_SECONDS = 3600;

    private final JdbcTemplate jdbc;
    private final StringRedisTemplate redis;
    private final S3Presigner presigner;
    private final SqsClient sqsClient;
    private final SqsQueueUrl queueUrl;
    private final AppProperties props;
    private final ObjectMapper objectMapper;

    public MediaService(
            JdbcTemplate jdbc,
            StringRedisTemplate redis,
            S3Presigner presigner,
            SqsClient sqsClient,
            SqsQueueUrl queueUrl,
            AppProperties props,
            ObjectMapper objectMapper) {
        this.jdbc = jdbc;
        this.redis = redis;
        this.presigner = presigner;
        this.sqsClient = sqsClient;
        this.queueUrl = queueUrl;
        this.props = props;
        this.objectMapper = objectMapper;
    }

    public UploadIntent createUpload(UUID ownerId, String contentType, long bytes, long expiresInSeconds) {
        String ext = extensionFromContentType(contentType);
        UUID uploadId = UUID.randomUUID();
        String objectKey = "uploads/" + ownerId + "/" + uploadId + "." + ext;

        jdbc.update(
                """
                INSERT INTO media_uploads (id, owner_id, original_key, content_type, bytes)
                VALUES (?, ?, ?, ?, ?)
                """,
                uploadId,
                ownerId,
                objectKey,
                contentType,
                bytes);

        PutObjectRequest put = PutObjectRequest.builder()
                .bucket(props.getS3().getBucket())
                .key(objectKey)
                .contentType(contentType)
                .contentLength(bytes)
                .build();

        PresignedPutObjectRequest presigned = presigner.presignPutObject(r -> r.signatureDuration(Duration.ofSeconds(expiresInSeconds))
                .putObjectRequest(put));

        List<UploadHeader> headers = new ArrayList<>();
        for (Map.Entry<String, List<String>> e : presigned.signedHeaders().entrySet()) {
            for (String value : e.getValue()) {
                headers.add(new UploadHeader(e.getKey(), value));
            }
        }

        return new UploadIntent(
                uploadId,
                objectKey,
                presigned.url().toString(),
                expiresInSeconds,
                headers);
    }

    public boolean completeUpload(UUID uploadId, UUID ownerId) {
        String originalKey = jdbc.query(
                """
                UPDATE media_uploads
                SET status = 'uploaded', uploaded_at = COALESCE(uploaded_at, now())
                WHERE id = ? AND owner_id = ? AND status IN ('pending', 'uploaded')
                RETURNING original_key
                """,
                rs -> rs.next() ? rs.getString("original_key") : null,
                uploadId,
                ownerId);

        if (originalKey == null) {
            return false;
        }

        enqueueProcessing(uploadId, ownerId, originalKey);
        return true;
    }

    public void enqueueProcessing(UUID uploadId, UUID ownerId, String originalKey) {
        try {
            ObjectNode job = objectMapper.createObjectNode();
            job.put("upload_id", uploadId.toString());
            job.put("owner_id", ownerId.toString());
            job.put("original_key", originalKey);
            String body = objectMapper.writeValueAsString(job);
            sqsClient.sendMessage(r -> r.queueUrl(queueUrl.get()).messageBody(body));
        } catch (Exception e) {
            throw ApiException.internal("failed to enqueue media job");
        }
    }

    public Optional<UploadStatus> getUploadStatus(UUID uploadId, UUID ownerId) {
        return jdbc.query(
                """
                SELECT status::text AS status, processed_media_id
                FROM media_uploads WHERE id = ? AND owner_id = ?
                """,
                rs -> rs.next()
                        ? Optional.of(new UploadStatus(rs.getString("status"), (UUID) rs.getObject("processed_media_id")))
                        : Optional.empty(),
                uploadId,
                ownerId);
    }

    public Optional<Media> getMedia(UUID mediaId) {
        return jdbc.query(
                """
                SELECT id, owner_id, original_key, thumb_key, medium_key, width, height, bytes, created_at
                FROM media WHERE id = ?
                """,
                rs -> rs.next() ? Optional.of(mapMediaWithUrls(rs)) : Optional.empty(),
                mediaId);
    }

    public Optional<Media> getMediaForUser(UUID mediaId, UUID viewerId) {
        return jdbc.query(
                """
                SELECT m.id, m.owner_id, m.original_key, m.thumb_key, m.medium_key,
                       m.width, m.height, m.bytes, m.created_at
                FROM media m
                WHERE m.id = ?
                  AND (m.owner_id = ?
                       OR EXISTS (
                           SELECT 1 FROM post_media pm
                           JOIN posts p ON p.id = pm.post_id
                           WHERE pm.media_id = m.id
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
                       )
                       OR EXISTS (
                           SELECT 1 FROM stories s
                           WHERE s.media_id = m.id
                             AND s.expires_at > now()
                             AND (s.visibility = 'public'
                                  OR s.user_id = ?
                                  OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only')
                                      AND EXISTS (SELECT 1 FROM follows WHERE follower_id = ? AND followee_id = s.user_id)
                                      AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = ?)))
                             AND NOT EXISTS (
                                 SELECT 1 FROM blocks
                                 WHERE (blocker_id = s.user_id AND blocked_id = ?)
                                    OR (blocker_id = ? AND blocked_id = s.user_id)
                             )
                       ))
                """,
                rs -> rs.next() ? Optional.of(mapMediaWithUrls(rs)) : Optional.empty(),
                mediaId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId,
                viewerId);
    }

    public void populateUserAvatarUrl(User user) {
        if (user.getAvatarKey() != null) {
            user.setAvatarUrl(generatePresignedGetUrl(user.getAvatarKey(), 14_400).orElse(null));
        }
    }

    public void populatePostAvatarUrls(List<Post> posts) {
        for (Post post : posts) {
            if (post.getOwnerAvatarKey() != null) {
                post.setOwnerAvatarUrl(generatePresignedGetUrl(post.getOwnerAvatarKey(), 14_400).orElse(null));
            }
        }
    }

    private Media mapMediaWithUrls(ResultSet rs) throws SQLException {
        String originalKey = rs.getString("original_key");
        String thumbKey = rs.getString("thumb_key");
        String mediumKey = rs.getString("medium_key");
        Timestamp created = rs.getTimestamp("created_at");
        return Media.builder()
                .id((UUID) rs.getObject("id"))
                .ownerId((UUID) rs.getObject("owner_id"))
                .originalKey(originalKey)
                .thumbKey(thumbKey)
                .mediumKey(mediumKey)
                .width(rs.getInt("width"))
                .height(rs.getInt("height"))
                .bytes(rs.getLong("bytes"))
                .createdAt(created == null ? null : OffsetDateTime.ofInstant(created.toInstant(), ZoneOffset.UTC))
                .originalUrl(generatePresignedGetUrl(originalKey, PRESIGN_GET_SECONDS).orElse(null))
                .thumbUrl(generatePresignedGetUrl(thumbKey, PRESIGN_GET_SECONDS).orElse(null))
                .mediumUrl(generatePresignedGetUrl(mediumKey, PRESIGN_GET_SECONDS).orElse(null))
                .build();
    }

    Optional<String> generatePresignedGetUrl(String key, long expiresInSeconds) {
        if (key == null || key.isBlank()) {
            return Optional.empty();
        }
        String cacheKey = "presigned:" + key;
        try {
            String cached = redis.opsForValue().get(cacheKey);
            if (cached != null) {
                return Optional.of(cached);
            }
        } catch (Exception e) {
            log.warn("presigned cache read failed: {}", e.toString());
        }

        GetObjectRequest get = GetObjectRequest.builder()
                .bucket(props.getS3().getBucket())
                .key(key)
                .build();
        PresignedGetObjectRequest presigned = presigner.presignGetObject(r -> r.signatureDuration(Duration.ofSeconds(expiresInSeconds))
                .getObjectRequest(get));
        String url = rewritePublicEndpoint(presigned.url().toString());

        long cacheTtl = Math.max(0, expiresInSeconds - 300);
        if (cacheTtl > 0) {
            try {
                redis.opsForValue().set(cacheKey, url, Duration.ofSeconds(cacheTtl));
            } catch (Exception e) {
                log.warn("presigned cache write failed: {}", e.toString());
            }
        }
        return Optional.of(url);
    }

    private String rewritePublicEndpoint(String url) {
        String publicEndpoint = props.getS3().getPublicEndpoint();
        if (publicEndpoint == null || publicEndpoint.isBlank()) {
            return url;
        }
        try {
            URI original = URI.create(url);
            URI pub = publicEndpoint.contains("://") ? URI.create(publicEndpoint) : URI.create("http://" + publicEndpoint);
            return new URI(
                            pub.getScheme(),
                            original.getUserInfo(),
                            pub.getHost(),
                            pub.getPort(),
                            original.getPath(),
                            original.getQuery(),
                            original.getFragment())
                    .toString();
        } catch (Exception e) {
            return url;
        }
    }

    private static String extensionFromContentType(String contentType) {
        return switch (contentType) {
            case "image/jpeg" -> "jpg";
            case "image/png" -> "png";
            case "image/webp" -> "webp";
            default -> throw ApiException.badRequest("unsupported content type");
        };
    }
}
