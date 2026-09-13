using System.Text.Json;
using Ciel.Api.Domain;
using Ciel.Api.Http.Json;
using Npgsql;
using StackExchange.Redis;

namespace Ciel.Api.Services;

public sealed class FeedService
{
    private const int FeedCacheTtlSeconds = 30;
    private static readonly JsonSerializerOptions CacheJson = CielJsonOptions.Create();

    private readonly NpgsqlDataSource _db;
    private readonly IConnectionMultiplexer _redis;

    public FeedService(NpgsqlDataSource db, IConnectionMultiplexer redis)
    {
        _db = db;
        _redis = redis;
    }

    public async Task<(List<Post> Posts, (DateTimeOffset Timestamp, Guid Id)? NextCursor)> GetHomeFeedAsync(
        Guid userId,
        (DateTimeOffset Timestamp, Guid Id)? cursor,
        int limit,
        CancellationToken ct)
    {
        var shouldCache = cursor is null;
        var cacheKey = $"feed:home:{userId}";

        if (shouldCache)
        {
            var db = _redis.GetDatabase();
            var cached = await db.StringGetAsync(cacheKey);
            if (!cached.IsNullOrEmpty)
            {
                try
                {
                    var page = JsonSerializer.Deserialize<CachedHomeFeed>(cached!, CacheJson);
                    if (page?.IntoPage() is { } hit)
                    {
                        return hit;
                    }
                }
                catch
                {
                    // ignore corrupt cache
                }
            }
        }

        var limitPlus = limit + 1;
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();

        if (cursor is { } c)
        {
            cmd.CommandText = """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE (p.owner_id = $1
                   OR (p.owner_id IN (
                       SELECT followee_id FROM follows WHERE follower_id = $1
                   ) AND NOT EXISTS (
                       SELECT 1 FROM blocks
                       WHERE (blocker_id = p.owner_id AND blocked_id = $1)
                          OR (blocker_id = $1 AND blocked_id = p.owner_id)
                   )))
                  AND (p.created_at < $2 OR (p.created_at = $2 AND p.id < $3))
                ORDER BY p.created_at DESC, p.id DESC
                LIMIT $4
                """;
            cmd.Parameters.AddWithValue(userId);
            cmd.Parameters.AddWithValue(c.Timestamp);
            cmd.Parameters.AddWithValue(c.Id);
            cmd.Parameters.AddWithValue(limitPlus);
        }
        else
        {
            cmd.CommandText = """
                SELECT p.id, p.owner_id, u.handle AS owner_handle, u.display_name AS owner_display_name,
                       u.avatar_key AS owner_avatar_key,
                       COALESCE(ARRAY(SELECT pm.media_id FROM post_media pm WHERE pm.post_id = p.id ORDER BY pm.position), ARRAY[]::uuid[]) AS media_ids,
                       p.caption, p.visibility::text AS visibility, p.created_at
                FROM posts p
                JOIN users u ON p.owner_id = u.id AND u.deleted_at IS NULL
                WHERE p.owner_id = $1
                   OR (p.owner_id IN (
                       SELECT followee_id FROM follows WHERE follower_id = $1
                   ) AND NOT EXISTS (
                       SELECT 1 FROM blocks
                       WHERE (blocker_id = p.owner_id AND blocked_id = $1)
                          OR (blocker_id = $1 AND blocked_id = p.owner_id)
                   ))
                ORDER BY p.created_at DESC, p.id DESC
                LIMIT $2
                """;
            cmd.Parameters.AddWithValue(userId);
            cmd.Parameters.AddWithValue(limitPlus);
        }

        var posts = await ReadPostsAsync(cmd, ct);
        (DateTimeOffset Timestamp, Guid Id)? nextCursor = null;
        if (posts.Count > limit)
        {
            var extra = posts[^1];
            posts.RemoveAt(posts.Count - 1);
            nextCursor = (extra.CreatedAt, extra.Id);
        }

        if (shouldCache)
        {
            try
            {
                var payload = JsonSerializer.Serialize(CachedHomeFeed.FromPage(posts, nextCursor), CacheJson);
                await _redis.GetDatabase().StringSetAsync(cacheKey, payload, TimeSpan.FromSeconds(FeedCacheTtlSeconds));
            }
            catch
            {
                // best-effort cache write
            }
        }

        return (posts, nextCursor);
    }

    public async Task RefreshHomeFeedAsync(Guid userId, CancellationToken ct)
    {
        _ = ct;
        await _redis.GetDatabase().KeyDeleteAsync($"feed:home:{userId}");
    }

    private static async Task<List<Post>> ReadPostsAsync(NpgsqlCommand cmd, CancellationToken ct)
    {
        var posts = new List<Post>();
        await using var reader = await cmd.ExecuteReaderAsync(ct);
        while (await reader.ReadAsync(ct))
        {
            var visibility = PostVisibilityDb.FromDb(reader.GetString(7))
                ?? throw new InvalidOperationException("unknown visibility");
            posts.Add(new Post
            {
                Id = reader.GetGuid(0),
                OwnerId = reader.GetGuid(1),
                OwnerHandle = reader.GetString(2),
                OwnerDisplayName = reader.GetString(3),
                OwnerAvatarKey = reader.IsDBNull(4) ? null : reader.GetString(4),
                MediaIds = reader.GetFieldValue<Guid[]>(5).ToList(),
                Caption = reader.IsDBNull(6) ? null : reader.GetString(6),
                Visibility = visibility,
                CreatedAt = reader.GetFieldValue<DateTimeOffset>(8),
            });
        }

        return posts;
    }

    private sealed class CachedHomeFeed
    {
        public List<Post> Posts { get; set; } = [];
        public long? NextCursorNanos { get; set; }
        public Guid? NextCursorId { get; set; }

        public static CachedHomeFeed FromPage(List<Post> posts, (DateTimeOffset Timestamp, Guid Id)? nextCursor) =>
            new()
            {
                Posts = posts,
                NextCursorNanos = nextCursor is null
                    ? null
                    : (nextCursor.Value.Timestamp - DateTimeOffset.UnixEpoch).Ticks * 100,
                NextCursorId = nextCursor?.Id,
            };

        public (List<Post> Posts, (DateTimeOffset Timestamp, Guid Id)? NextCursor)? IntoPage()
        {
            if (NextCursorNanos is null && NextCursorId is null)
            {
                return (Posts, null);
            }

            if (NextCursorNanos is null || NextCursorId is null)
            {
                return null;
            }

            var ticks = NextCursorNanos.Value / 100;
            var dt = new DateTimeOffset(ticks, TimeSpan.Zero);
            return (Posts, (dt, NextCursorId.Value));
        }
    }
}
