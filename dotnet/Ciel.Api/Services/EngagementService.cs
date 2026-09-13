using Ciel.Api.Domain;
using Npgsql;

namespace Ciel.Api.Services;

public sealed class EngagementService
{
    private readonly NpgsqlDataSource _db;

    public EngagementService(NpgsqlDataSource db) => _db = db;

    public async Task<bool> LikePostAsync(Guid userId, Guid postId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            INSERT INTO likes (user_id, post_id) VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            RETURNING id
            """;
        cmd.Parameters.AddWithValue(userId);
        cmd.Parameters.AddWithValue(postId);
        var result = await cmd.ExecuteScalarAsync(ct);
        return result is not null;
    }

    public async Task<bool> UnlikePostAsync(Guid userId, Guid postId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = "DELETE FROM likes WHERE user_id = $1 AND post_id = $2";
        cmd.Parameters.AddWithValue(userId);
        cmd.Parameters.AddWithValue(postId);
        return await cmd.ExecuteNonQueryAsync(ct) > 0;
    }

    public async Task<Comment> CommentPostAsync(Guid userId, Guid postId, string body, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            WITH inserted AS (
                INSERT INTO comments (user_id, post_id, body) VALUES ($1, $2, $3)
                RETURNING id, user_id, post_id, body, created_at
            )
            SELECT i.id, i.user_id, i.post_id, i.body, i.created_at,
                   u.handle AS user_handle, u.display_name AS user_display_name
            FROM inserted i
            JOIN users u ON u.id = i.user_id
            """;
        cmd.Parameters.AddWithValue(userId);
        cmd.Parameters.AddWithValue(postId);
        cmd.Parameters.AddWithValue(body);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            throw new InvalidOperationException("failed to create comment");
        }

        return new Comment
        {
            Id = reader.GetGuid(0),
            UserId = reader.GetGuid(1),
            PostId = reader.GetGuid(2),
            Body = reader.GetString(3),
            CreatedAt = reader.GetFieldValue<DateTimeOffset>(4),
            UserHandle = reader.GetString(5),
            UserDisplayName = reader.GetString(6),
        };
    }
}
