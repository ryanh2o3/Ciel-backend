using Ciel.Api.Config;
using Ciel.Api.Domain;
using Ciel.Api.Http.Errors;
using Npgsql;
using StackExchange.Redis;

namespace Ciel.Api.Services;

public sealed class MediaService
{
    private readonly NpgsqlDataSource _db;
    private readonly IConnectionMultiplexer _redis;
    private readonly StorageService _storage;
    private readonly QueueService _queue;
    private readonly AppConfig _config;
    private readonly ILogger<MediaService> _logger;

    public MediaService(
        NpgsqlDataSource db,
        IConnectionMultiplexer redis,
        StorageService storage,
        QueueService queue,
        AppConfig config,
        ILogger<MediaService> logger)
    {
        _db = db;
        _redis = redis;
        _storage = storage;
        _queue = queue;
        _config = config;
        _logger = logger;
    }

    public async Task<UploadIntent> CreateUploadAsync(
        Guid ownerId,
        string contentType,
        long bytes,
        CancellationToken ct)
    {
        var ext = ExtensionFromContentType(contentType);
        if (ext is null)
        {
            _logger.LogWarning("rejected upload with unsupported content type {ContentType} for owner {OwnerId}", contentType, ownerId);
            throw ApiException.BadRequest("invalid upload request");
        }

        var uploadId = Guid.NewGuid();
        var objectKey = $"uploads/{ownerId}/{uploadId}.{ext}";

        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO media_uploads (id, owner_id, original_key, content_type, bytes)
            VALUES ($1, $2, $3, $4, $5)
            """;
        cmd.Parameters.AddWithValue(uploadId);
        cmd.Parameters.AddWithValue(ownerId);
        cmd.Parameters.AddWithValue(objectKey);
        cmd.Parameters.AddWithValue(contentType);
        cmd.Parameters.AddWithValue(bytes);
        await cmd.ExecuteNonQueryAsync(ct);

        var (url, headers) = _storage.PresignPut(
            objectKey,
            contentType,
            bytes,
            _config.UploadUrlTtlSeconds);

        return new UploadIntent
        {
            UploadId = uploadId,
            ObjectKey = objectKey,
            UploadUrl = url,
            ExpiresInSeconds = _config.UploadUrlTtlSeconds,
            Headers = headers.Select(h => new UploadHeader { Name = h.Item1, Value = h.Item2 }).ToList(),
        };
    }

    public async Task<bool> CompleteUploadAsync(Guid uploadId, Guid ownerId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            UPDATE media_uploads
            SET status = 'uploaded', uploaded_at = COALESCE(uploaded_at, now())
            WHERE id = $1 AND owner_id = $2 AND status IN ('pending', 'uploaded')
            RETURNING original_key
            """;
        cmd.Parameters.AddWithValue(uploadId);
        cmd.Parameters.AddWithValue(ownerId);

        var originalKey = await cmd.ExecuteScalarAsync(ct) as string;
        if (originalKey is null)
        {
            return false;
        }

        await _queue.EnqueueMediaJobAsync(new MediaJob
        {
            UploadId = uploadId,
            OwnerId = ownerId,
            OriginalKey = originalKey,
        }, ct);

        return true;
    }

    public async Task<UploadStatus?> GetUploadStatusAsync(Guid uploadId, Guid ownerId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT status::text AS status, processed_media_id
            FROM media_uploads WHERE id = $1 AND owner_id = $2
            """;
        cmd.Parameters.AddWithValue(uploadId);
        cmd.Parameters.AddWithValue(ownerId);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        return new UploadStatus
        {
            Status = reader.GetString(0),
            ProcessedMediaId = reader.IsDBNull(1) ? null : reader.GetGuid(1),
        };
    }

    public async Task<Media?> GetMediaForUserAsync(Guid mediaId, Guid viewerId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT m.id, m.owner_id, m.original_key, m.thumb_key, m.medium_key,
                   m.width, m.height, m.bytes, m.created_at
            FROM media m
            WHERE m.id = $1
              AND (m.owner_id = $2
                   OR EXISTS (
                       SELECT 1 FROM post_media pm
                       JOIN posts p ON p.id = pm.post_id
                       WHERE pm.media_id = m.id
                         AND (p.visibility = 'public'
                              OR p.owner_id = $2
                              OR (p.visibility = 'followers_only' AND EXISTS (
                                  SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = p.owner_id
                              )))
                         AND NOT EXISTS (
                             SELECT 1 FROM blocks
                             WHERE (blocker_id = p.owner_id AND blocked_id = $2)
                                OR (blocker_id = $2 AND blocked_id = p.owner_id)
                         )
                   )
                   OR EXISTS (
                       SELECT 1 FROM stories s
                       WHERE s.media_id = m.id
                         AND s.expires_at > $3
                         AND (s.visibility = 'public'
                              OR s.user_id = $2
                              OR ((s.visibility = 'friends_only' OR s.visibility = 'close_friends_only')
                                  AND EXISTS (SELECT 1 FROM follows WHERE follower_id = $2 AND followee_id = s.user_id)
                                  AND EXISTS (SELECT 1 FROM follows WHERE follower_id = s.user_id AND followee_id = $2)))
                         AND NOT EXISTS (
                             SELECT 1 FROM blocks
                             WHERE (blocker_id = s.user_id AND blocked_id = $2)
                                OR (blocker_id = $2 AND blocked_id = s.user_id)
                         )
                   ))
            """;
        cmd.Parameters.AddWithValue(mediaId);
        cmd.Parameters.AddWithValue(viewerId);
        cmd.Parameters.AddWithValue(DateTimeOffset.UtcNow);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        return await PopulateUrlsAsync(ReadMedia(reader), ct);
    }

    public async Task<string?> GeneratePresignedGetUrlAsync(string? objectKey, long expiresSeconds, CancellationToken ct)
    {
        _ = ct;
        if (string.IsNullOrWhiteSpace(objectKey))
        {
            return null;
        }

        var cacheKey = $"presigned:{objectKey}";
        var db = _redis.GetDatabase();

        try
        {
            var cached = await db.StringGetAsync(cacheKey);
            if (!cached.IsNullOrEmpty)
            {
                return cached.ToString();
            }
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "presigned URL cache read failed for {ObjectKey}; falling back to S3", objectKey);
        }

        string url;
        try
        {
            url = _storage.PresignGet(objectKey, expiresSeconds);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "presigned URL generation failed for {ObjectKey}", objectKey);
            return null;
        }

        try
        {
            var cacheTtl = Math.Max(expiresSeconds - 300, 1);
            await db.StringSetAsync(cacheKey, url, TimeSpan.FromSeconds(cacheTtl));
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "presigned URL cache write failed for {ObjectKey}", objectKey);
        }

        return url;
    }

    public async Task PopulateUserAvatarUrlAsync(User user, CancellationToken ct)
    {
        user.AvatarUrl = await GeneratePresignedGetUrlAsync(user.AvatarKey, 14400, ct);
    }

    /// <summary>
    /// Populates `owner_avatar_url` on each post from `owner_avatar_key`.
    /// Distinct keys are resolved once (dedupe) and in parallel (Task.WhenAll)
    /// rather than sequentially per-post, since many posts in a feed page
    /// typically share the same handful of authors/avatars.
    /// </summary>
    public async Task PopulatePostAvatarUrlsAsync(IList<Post> posts, CancellationToken ct)
    {
        var keys = posts
            .Select(p => p.OwnerAvatarKey)
            .Where(k => k is not null)
            .Select(k => k!)
            .Distinct()
            .ToList();

        if (keys.Count == 0)
        {
            return;
        }

        var resolved = await Task.WhenAll(keys.Select(async key => (key, url: await GeneratePresignedGetUrlAsync(key, 14400, ct))));
        var urlByKey = resolved.ToDictionary(r => r.key, r => r.url);

        foreach (var post in posts)
        {
            if (post.OwnerAvatarKey is not null && urlByKey.TryGetValue(post.OwnerAvatarKey, out var url))
            {
                post.OwnerAvatarUrl = url;
            }
        }
    }

    /// <summary>Resolves original/thumb/medium URLs in parallel, deduping when keys collide.</summary>
    private async Task<Media> PopulateUrlsAsync(Media media, CancellationToken ct)
    {
        var keys = new[] { media.OriginalKey, media.ThumbKey, media.MediumKey }.Distinct().ToList();
        var resolved = await Task.WhenAll(keys.Select(async key => (key, url: await GeneratePresignedGetUrlAsync(key, 3600, ct))));
        var urlByKey = resolved.ToDictionary(r => r.key, r => r.url);

        media.OriginalUrl = urlByKey[media.OriginalKey];
        media.ThumbUrl = urlByKey[media.ThumbKey];
        media.MediumUrl = urlByKey[media.MediumKey];
        return media;
    }

    private static Media ReadMedia(NpgsqlDataReader reader) => new()
    {
        Id = reader.GetGuid(0),
        OwnerId = reader.GetGuid(1),
        OriginalKey = reader.GetString(2),
        ThumbKey = reader.GetString(3),
        MediumKey = reader.GetString(4),
        Width = reader.GetInt32(5),
        Height = reader.GetInt32(6),
        Bytes = reader.GetInt64(7),
        CreatedAt = reader.GetFieldValue<DateTimeOffset>(8),
    };

    private static string? ExtensionFromContentType(string contentType) => contentType switch
    {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => null,
    };
}
