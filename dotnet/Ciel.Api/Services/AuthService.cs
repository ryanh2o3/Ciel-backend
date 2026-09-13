using Ciel.Api.Domain;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Errors;
using Npgsql;

namespace Ciel.Api.Services;

public sealed class AuthService
{
    private readonly NpgsqlDataSource _db;
    private readonly PasetoService _paseto;
    private readonly CryptoService _crypto;
    private readonly ILogger<AuthService> _logger;

    public AuthService(NpgsqlDataSource db, PasetoService paseto, CryptoService crypto, ILogger<AuthService> logger)
    {
        _db = db;
        _paseto = paseto;
        _crypto = crypto;
        _logger = logger;
    }

    public async Task<AuthTokenResponse?> LoginAsync(string identifier, string password, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT u.id, u.password_hash, uts.banned_until
            FROM users u
            LEFT JOIN user_trust_scores uts ON uts.user_id = u.id
            WHERE (u.email = $1 OR u.handle = $1) AND u.deleted_at IS NULL
            """;
        cmd.Parameters.AddWithValue(identifier);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        var userId = reader.GetGuid(0);
        var passwordHash = reader.IsDBNull(1) ? string.Empty : reader.GetString(1);
        var bannedUntil = reader.IsDBNull(2) ? (DateTimeOffset?)null : reader.GetFieldValue<DateTimeOffset>(2);

        if (string.IsNullOrEmpty(passwordHash) || !await _crypto.VerifyPasswordAsync(password, passwordHash))
        {
            _logger.LogWarning("login failed for identifier {Identifier}: bad credentials", identifier);
            return null;
        }

        if (bannedUntil is not null && bannedUntil > DateTimeOffset.UtcNow)
        {
            _logger.LogWarning("login rejected for banned user {UserId}", userId);
            throw ApiException.Forbidden("Your account has been temporarily suspended");
        }

        _logger.LogInformation("user {UserId} logged in", userId);
        return await IssuePairAsync(userId, ct);
    }

    public async Task<AuthTokenResponse?> RefreshAsync(string refreshToken, CancellationToken ct)
    {
        var claims = _paseto.VerifyRefresh(refreshToken);
        if (claims is null)
        {
            return null;
        }

        var tokenHash = CryptoService.Sha256Hex(refreshToken);
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var tx = await conn.BeginTransactionAsync(ct);

        await using var select = conn.CreateCommand();
        select.Transaction = tx;
        select.CommandText = """
            SELECT rt.revoked_at, u.deleted_at, uts.banned_until
            FROM refresh_tokens rt
            JOIN users u ON u.id = rt.user_id
            LEFT JOIN user_trust_scores uts ON uts.user_id = rt.user_id
            WHERE rt.id = $1 AND rt.user_id = $2 AND rt.token_hash = $3 AND rt.expires_at > now()
            FOR UPDATE OF rt
            """;
        select.Parameters.AddWithValue(claims.RefreshId);
        select.Parameters.AddWithValue(claims.UserId);
        select.Parameters.AddWithValue(tokenHash);

        DateTimeOffset? revokedAt = null;
        DateTimeOffset? deletedAt = null;
        DateTimeOffset? bannedUntil = null;

        await using (var reader = await select.ExecuteReaderAsync(ct))
        {
            if (!await reader.ReadAsync(ct))
            {
                await tx.RollbackAsync(ct);
                return null;
            }

            revokedAt = reader.IsDBNull(0) ? null : reader.GetFieldValue<DateTimeOffset>(0);
            deletedAt = reader.IsDBNull(1) ? null : reader.GetFieldValue<DateTimeOffset>(1);
            bannedUntil = reader.IsDBNull(2) ? null : reader.GetFieldValue<DateTimeOffset>(2);
        }

        if (revokedAt is not null)
        {
            await using var revokeAll = conn.CreateCommand();
            revokeAll.Transaction = tx;
            revokeAll.CommandText = """
                UPDATE refresh_tokens SET revoked_at = now()
                WHERE user_id = $1 AND revoked_at IS NULL
                """;
            revokeAll.Parameters.AddWithValue(claims.UserId);
            await revokeAll.ExecuteNonQueryAsync(ct);
            await tx.CommitAsync(ct);
            return null;
        }

        if (deletedAt is not null)
        {
            await tx.RollbackAsync(ct);
            return null;
        }

        if (bannedUntil is not null && bannedUntil > DateTimeOffset.UtcNow)
        {
            await tx.RollbackAsync(ct);
            _logger.LogWarning("refresh rejected for banned user {UserId}", claims.UserId);
            throw ApiException.Forbidden("Your account has been temporarily suspended");
        }

        var issued = await IssuePairAsync(claims.UserId, conn, tx, ct);

        await using var rotate = conn.CreateCommand();
        rotate.Transaction = tx;
        rotate.CommandText = """
            UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $1
            WHERE id = $2 AND revoked_at IS NULL
            """;
        rotate.Parameters.AddWithValue(issued.RefreshId);
        rotate.Parameters.AddWithValue(claims.RefreshId);
        await rotate.ExecuteNonQueryAsync(ct);

        await tx.CommitAsync(ct);
        return issued.Response;
    }

    public async Task RevokeAsync(string refreshToken, CancellationToken ct)
    {
        var claims = _paseto.VerifyRefresh(refreshToken);
        if (claims is null)
        {
            return;
        }

        var tokenHash = CryptoService.Sha256Hex(refreshToken);
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            UPDATE refresh_tokens SET revoked_at = now()
            WHERE id = $1 AND user_id = $2 AND token_hash = $3 AND revoked_at IS NULL
            """;
        cmd.Parameters.AddWithValue(claims.RefreshId);
        cmd.Parameters.AddWithValue(claims.UserId);
        cmd.Parameters.AddWithValue(tokenHash);
        await cmd.ExecuteNonQueryAsync(ct);
    }

    public async Task<Guid?> AuthenticateAccessTokenAsync(string token, CancellationToken ct)
    {
        var userId = _paseto.VerifyAccess(token);
        if (userId is null)
        {
            return null;
        }

        // Note: this intentionally does NOT filter on banned_until — ban status is
        // checked separately (AuthEndpointFilter) so banned users get a distinct
        // 403 "temporarily suspended" instead of a generic 401 "invalid token",
        // matching Rust's separate ban_check_middleware (src/http/middleware/ban.rs).
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT true FROM users WHERE id = $1 AND deleted_at IS NULL";
        cmd.Parameters.AddWithValue(userId.Value);
        var result = await cmd.ExecuteScalarAsync(ct);
        return result is null ? null : userId;
    }

    public async Task<User?> GetCurrentUserAsync(Guid userId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            SELECT id, handle, email, display_name, bio, avatar_key, created_at
            FROM users WHERE id = $1 AND deleted_at IS NULL
            """;
        cmd.Parameters.AddWithValue(userId);

        await using var reader = await cmd.ExecuteReaderAsync(ct);
        if (!await reader.ReadAsync(ct))
        {
            return null;
        }

        return ReadUser(reader);
    }

    public async Task<User> SignupAsync(
        string handle,
        string email,
        string displayName,
        string? bio,
        string? avatarKey,
        string password,
        string inviteCode,
        CancellationToken ct)
    {
        var passwordHash = await _crypto.HashPasswordAsync(password);
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var tx = await conn.BeginTransactionAsync(ct);

        User user;
        await using (var insert = conn.CreateCommand())
        {
            insert.Transaction = tx;
            insert.CommandText = """
                INSERT INTO users (handle, email, display_name, bio, avatar_key, password_hash)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, handle, email, display_name, bio, avatar_key, created_at
                """;
            insert.Parameters.AddWithValue(handle);
            insert.Parameters.AddWithValue(email);
            insert.Parameters.AddWithValue(displayName);
            insert.Parameters.AddWithValue((object?)bio ?? DBNull.Value);
            insert.Parameters.AddWithValue((object?)avatarKey ?? DBNull.Value);
            insert.Parameters.AddWithValue(passwordHash);

            await using var reader = await insert.ExecuteReaderAsync(ct);
            if (!await reader.ReadAsync(ct))
            {
                throw ApiException.Internal("signup failed");
            }

            user = ReadUser(reader);
        }

        await using (var trust = conn.CreateCommand())
        {
            trust.Transaction = tx;
            trust.CommandText = """
                INSERT INTO user_trust_scores (user_id) VALUES ($1) ON CONFLICT DO NOTHING
                """;
            trust.Parameters.AddWithValue(user.Id);
            await trust.ExecuteNonQueryAsync(ct);
        }

        await using (var inviteSelect = conn.CreateCommand())
        {
            inviteSelect.Transaction = tx;
            inviteSelect.CommandText = """
                SELECT code, created_by, is_valid, expires_at, use_count, max_uses
                FROM invite_codes
                WHERE code = $1
                FOR UPDATE
                """;
            inviteSelect.Parameters.AddWithValue(inviteCode);

            await using var reader = await inviteSelect.ExecuteReaderAsync(ct);
            if (!await reader.ReadAsync(ct))
            {
                throw ApiException.BadRequest("Invalid invite code");
            }

            var isValid = reader.GetBoolean(2);
            var expiresAt = reader.GetFieldValue<DateTimeOffset>(3);
            var useCount = reader.GetInt32(4);
            var maxUses = reader.GetInt32(5);
            var createdBy = reader.IsDBNull(1) ? (Guid?)null : reader.GetGuid(1);
            await reader.CloseAsync();

            if (!isValid)
            {
                throw ApiException.BadRequest("Invite code has been revoked");
            }

            if (expiresAt < DateTimeOffset.UtcNow)
            {
                throw ApiException.BadRequest("Invite code has expired");
            }

            if (useCount >= maxUses)
            {
                throw ApiException.BadRequest("Invite code has been fully used");
            }

            var isFullyUsed = useCount + 1 >= maxUses;

            await using var inviteUpdate = conn.CreateCommand();
            inviteUpdate.Transaction = tx;
            inviteUpdate.CommandText = """
                UPDATE invite_codes
                SET used_by = $1, used_at = NOW(), use_count = use_count + 1, is_valid = $2
                WHERE code = $3
                """;
            inviteUpdate.Parameters.AddWithValue(user.Id);
            inviteUpdate.Parameters.AddWithValue(!isFullyUsed);
            inviteUpdate.Parameters.AddWithValue(inviteCode);
            await inviteUpdate.ExecuteNonQueryAsync(ct);

            if (createdBy is Guid inviterId)
            {
                await using var relationship = conn.CreateCommand();
                relationship.Transaction = tx;
                relationship.CommandText = """
                    INSERT INTO invite_relationships (inviter_id, invitee_id, invite_code)
                    VALUES ($1, $2, $3)
                    """;
                relationship.Parameters.AddWithValue(inviterId);
                relationship.Parameters.AddWithValue(user.Id);
                relationship.Parameters.AddWithValue(inviteCode);
                await relationship.ExecuteNonQueryAsync(ct);

                await using var trust = conn.CreateCommand();
                trust.Transaction = tx;
                trust.CommandText = """
                    UPDATE user_trust_scores
                    SET successful_invites = successful_invites + 1,
                        trust_points = trust_points + 10
                    WHERE user_id = $1
                    """;
                trust.Parameters.AddWithValue(inviterId);
                await trust.ExecuteNonQueryAsync(ct);
            }
        }

        await tx.CommitAsync(ct);
        _logger.LogInformation("user {UserId} signed up with handle {Handle}", user.Id, user.Handle);
        return user;
    }

    private async Task<AuthTokenResponse> IssuePairAsync(Guid userId, CancellationToken ct)
    {
        await using var conn = await _db.OpenConnectionAsync(ct);
        await using var tx = await conn.BeginTransactionAsync(ct);
        var issued = await IssuePairAsync(userId, conn, tx, ct);
        await tx.CommitAsync(ct);
        return issued.Response;
    }

    private async Task<(Guid RefreshId, AuthTokenResponse Response)> IssuePairAsync(
        Guid userId,
        NpgsqlConnection conn,
        NpgsqlTransaction tx,
        CancellationToken ct)
    {
        var refreshId = Guid.NewGuid();
        var access = _paseto.MintAccess(userId);
        var refresh = _paseto.MintRefresh(userId, refreshId);
        var tokenHash = CryptoService.Sha256Hex(refresh.Token);

        await using var insert = conn.CreateCommand();
        insert.Transaction = tx;
        insert.CommandText = """
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            """;
        insert.Parameters.AddWithValue(refreshId);
        insert.Parameters.AddWithValue(userId);
        insert.Parameters.AddWithValue(tokenHash);
        insert.Parameters.AddWithValue(refresh.ExpiresAt);
        await insert.ExecuteNonQueryAsync(ct);

        return (refreshId, new AuthTokenResponse
        {
            AccessToken = access.Token,
            RefreshToken = refresh.Token,
            AccessExpiresAt = access.ExpiresAt,
            RefreshExpiresAt = refresh.ExpiresAt,
        });
    }

    private static User ReadUser(NpgsqlDataReader reader) => new()
    {
        Id = reader.GetGuid(0),
        Handle = reader.GetString(1),
        Email = reader.GetString(2),
        DisplayName = reader.GetString(3),
        Bio = reader.IsDBNull(4) ? null : reader.GetString(4),
        AvatarKey = reader.IsDBNull(5) ? null : reader.GetString(5),
        CreatedAt = reader.GetFieldValue<DateTimeOffset>(6),
    };
}
