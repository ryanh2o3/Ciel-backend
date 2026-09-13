using Ciel.Api.Domain;
using Npgsql;

namespace Ciel.Api.Services;

public sealed class UserService
{
    private readonly NpgsqlDataSource _db;

    public UserService(NpgsqlDataSource db) => _db = db;

    public async Task<(User User, long Followers, long Following, long Posts)?> GetPublicUserWithCountsAsync(
        Guid userId,
        CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, u.created_at,
                   (SELECT COUNT(*) FROM follows WHERE followee_id = u.id) AS followers_count,
                   (SELECT COUNT(*) FROM follows WHERE follower_id = u.id) AS following_count,
                   (SELECT COUNT(*) FROM posts WHERE owner_id = u.id) AS posts_count
            FROM users u WHERE u.id = $1 AND u.deleted_at IS NULL
            """;
        cmd.Parameters.AddWithValue(userId);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        var user = new User
        {
            Id = reader.GetGuid(0),
            Handle = reader.GetString(1),
            Email = reader.GetString(2),
            DisplayName = reader.GetString(3),
            Bio = reader.IsDBNull(4) ? null : reader.GetString(4),
            AvatarKey = reader.IsDBNull(5) ? null : reader.GetString(5),
            CreatedAt = reader.GetFieldValue<DateTimeOffset>(6),
        };

        return (user, reader.GetInt64(7), reader.GetInt64(8), reader.GetInt64(9));
    }
}
