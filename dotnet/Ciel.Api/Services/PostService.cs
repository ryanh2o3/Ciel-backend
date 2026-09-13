using Ciel.Api.Domain;
using Ciel.Api.Http.Errors;
using Npgsql;

namespace Ciel.Api.Services;

public sealed class PostService
{
    private readonly NpgsqlDataSource _db;
    private readonly ILogger<PostService> _logger;

    public PostService(NpgsqlDataSource db, ILogger<PostService> logger)
    {
        _db = db;
        _logger = logger;
    }

    public async Task<Post> CreatePostAsync(Guid ownerId, List<Guid> mediaIds, string? caption, CancellationToken ct)
    {
        if (mediaIds.Count == 0)
        {
            throw ApiException.BadRequest("at least one media_id is required");
        }

        if (mediaIds.Count > 10)
        {
            throw ApiException.BadRequest("maximum 10 images per post");
        }

        if (mediaIds.Distinct().Count() != mediaIds.Count)
        {
            throw ApiException.BadRequest("media_ids must be unique");
        }

        await using var conn = await _db.OpenConnectionAsync(ct);

        await using (var countCmd = conn.CreateCommand())
        {
            countCmd.CommandText = "SELECT COUNT(*) FROM media WHERE id = ANY($1) AND owner_id = $2";
            countCmd.Parameters.AddWithValue(mediaIds.ToArray());
            countCmd.Parameters.AddWithValue(ownerId);
            var owned = (long)(await countCmd.ExecuteScalarAsync(ct) ?? 0L);
            if (owned != mediaIds.Count)
            {
                throw ApiException.BadRequest("invalid media_id");
            }
        }

        await using var tx = await conn.BeginTransactionAsync(ct);
        Post post;

        await using (var insert = conn.CreateCommand())
        {
            insert.Transaction = tx;
            insert.CommandText = """
                WITH inserted_post AS (
                    INSERT INTO posts (owner_id, caption, visibility)
                    VALUES ($1, $2, $3::post_visibility)
                    RETURNING id, owner_id, caption, visibility::text AS visibility, created_at
                )
                SELECT p.id, p.owner_id, p.caption, p.visibility, p.created_at,
                       u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key
                FROM inserted_post p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                """;
            insert.Parameters.AddWithValue(ownerId);
            insert.Parameters.AddWithValue((object?)caption ?? DBNull.Value);
            insert.Parameters.AddWithValue(PostVisibilityDb.AsDb(PostVisibility.Public));

            await using var reader = await insert.ExecuteReaderAsync(ct);
            if (!await reader.ReadAsync(ct))
            {
                throw ApiException.Internal("failed to create post");
            }

            post = ReadPost(reader, mediaIds);
        }

        for (var i = 0; i < mediaIds.Count; i++)
        {
            await using var link = conn.CreateCommand();
            link.Transaction = tx;
            link.CommandText = "INSERT INTO post_media (post_id, media_id, position) VALUES ($1, $2, $3)";
            link.Parameters.AddWithValue(post.Id);
            link.Parameters.AddWithValue(mediaIds[i]);
            link.Parameters.AddWithValue((short)i);
            await link.ExecuteNonQueryAsync(ct);
        }

        await tx.CommitAsync(ct);
        _logger.LogInformation("post {PostId} created by {OwnerId} with {MediaCount} media items", post.Id, ownerId, mediaIds.Count);
        return post;
    }

    public async Task<Post?> GetPostAsync(Guid postId, Guid? viewerId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();

        if (viewerId is Guid viewer)
        {
            cmd.CommandText = """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at,
                       (SELECT COUNT(*)::bigint FROM likes l WHERE l.post_id = p.id) AS like_count,
                       (SELECT COUNT(*)::bigint FROM comments c WHERE c.post_id = p.id) AS comment_count,
                       EXISTS(SELECT 1 FROM likes l WHERE l.post_id = p.id AND l.user_id = $2) AS liked_by_viewer
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.id = $1
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
                """;
            cmd.Parameters.AddWithValue(postId);
            cmd.Parameters.AddWithValue(viewer);
        }
        else
        {
            cmd.CommandText = """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at,
                       (SELECT COUNT(*)::bigint FROM likes l WHERE l.post_id = p.id) AS like_count,
                       (SELECT COUNT(*)::bigint FROM comments c WHERE c.post_id = p.id) AS comment_count
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.id = $1 AND p.visibility = 'public'
                """;
            cmd.Parameters.AddWithValue(postId);
        }

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        var post = ReadPostWithCounts(reader, viewerId.HasValue);
        return post;
    }

    public async Task<bool> CanViewPostAsync(Guid postId, Guid viewerId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT true
            FROM posts p
            JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
            WHERE p.id = $1
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
            """;
        cmd.Parameters.AddWithValue(postId);
        cmd.Parameters.AddWithValue(viewerId);
        return await cmd.ExecuteScalarAsync(ct) is not null;
    }

    private Post ReadPost(NpgsqlDataReader reader, List<Guid> mediaIds)
    {
        var visibilityRaw = reader.GetString(3);
        var visibility = PostVisibilityDb.FromDb(visibilityRaw)
            ?? throw LogUnknownVisibility(visibilityRaw);

        return new Post
        {
            Id = reader.GetGuid(0),
            OwnerId = reader.GetGuid(1),
            Caption = reader.IsDBNull(2) ? null : reader.GetString(2),
            Visibility = visibility,
            CreatedAt = reader.GetFieldValue<DateTimeOffset>(4),
            OwnerHandle = reader.GetString(5),
            OwnerDisplayName = reader.GetString(6),
            OwnerAvatarKey = reader.IsDBNull(7) ? null : reader.GetString(7),
            MediaIds = mediaIds,
        };
    }

    private Post ReadPostWithCounts(NpgsqlDataReader reader, bool withViewerLike)
    {
        var mediaIds = reader.GetFieldValue<Guid[]>(5).ToList();
        var visibilityRaw = reader.GetString(7);
        var visibility = PostVisibilityDb.FromDb(visibilityRaw)
            ?? throw LogUnknownVisibility(visibilityRaw);

        var post = new Post
        {
            Id = reader.GetGuid(0),
            OwnerId = reader.GetGuid(1),
            OwnerHandle = reader.GetString(2),
            OwnerDisplayName = reader.GetString(3),
            OwnerAvatarKey = reader.IsDBNull(4) ? null : reader.GetString(4),
            MediaIds = mediaIds,
            Caption = reader.IsDBNull(6) ? null : reader.GetString(6),
            Visibility = visibility,
            CreatedAt = reader.GetFieldValue<DateTimeOffset>(8),
            LikeCount = reader.GetInt64(9),
            CommentCount = reader.GetInt64(10),
        };

        if (withViewerLike)
        {
            post.LikedByViewer = reader.GetBoolean(11);
        }

        return post;
    }

    private InvalidOperationException LogUnknownVisibility(string raw)
    {
        _logger.LogError("encountered unknown post visibility value {Visibility} from database", raw);
        return new InvalidOperationException("unknown visibility");
    }
}
