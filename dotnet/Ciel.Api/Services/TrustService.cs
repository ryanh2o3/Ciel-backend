using Ciel.Api.Config;
using Npgsql;

namespace Ciel.Api.Services;

/// <summary>
/// Reads user trust/ban state from <c>user_trust_scores</c>. Mirrors the read
/// paths of Rust's <c>TrustService</c> (src/app/trust.rs) used by the rate
/// limit and ban-check middleware; write paths (strikes, activity scoring)
/// are owned by the moderation system and are out of scope for this port.
/// </summary>
public sealed class TrustService(NpgsqlDataSource db, ILogger<TrustService> logger)
{
    public async Task<TrustLevel> GetTrustLevelAsync(Guid userId, CancellationToken ct)
    {
        try
        {
            await using var conn = await db.OpenConnectionAsync(ct);
            await using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT trust_level FROM user_trust_scores WHERE user_id = $1";
            cmd.Parameters.AddWithValue(userId);

            var result = await cmd.ExecuteScalarAsync(ct);
            return result is int level ? TrustLevelExtensions.FromInt32(level) : TrustLevel.New;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "failed to load trust level for user {UserId}", userId);
            throw;
        }
    }

    public async Task<bool> IsBannedAsync(Guid userId, CancellationToken ct)
    {
        try
        {
            await using var conn = await db.OpenConnectionAsync(ct);
            await using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT banned_until FROM user_trust_scores WHERE user_id = $1";
            cmd.Parameters.AddWithValue(userId);

            var result = await cmd.ExecuteScalarAsync(ct);
            return result is DateTimeOffset bannedUntil && bannedUntil > DateTimeOffset.UtcNow;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "failed to check ban status for user {UserId}", userId);
            throw;
        }
    }
}
