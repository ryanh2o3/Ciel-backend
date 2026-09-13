package com.ciel.app;

import com.ciel.domain.User;
import com.ciel.web.dto.AuthTokenResponse;
import com.ciel.web.error.ApiException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.jdbc.support.rowset.SqlRowSet;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Timestamp;
import java.time.Instant;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.Optional;
import java.util.UUID;

@Service
public class AuthService {

    private static final Logger log = LoggerFactory.getLogger(AuthService.class);

    private final JdbcTemplate jdbc;
    private final PasetoService paseto;
    private final CryptoService crypto;

    public AuthService(JdbcTemplate jdbc, PasetoService paseto, CryptoService crypto) {
        this.jdbc = jdbc;
        this.paseto = paseto;
        this.crypto = crypto;
    }

    /**
     * Validates a PASETO access token and re-checks the account against the
     * database on every call (not just at token mint time), so a user
     * deleted or banned mid-session is rejected within one request — this is
     * the single choke point that every {@code /v1} route under
     * {@link com.ciel.web.auth.AuthUserResolver} and the rate-limit filters
     * goes through, matching Rust's {@code authenticate_access_token} (see
     * {@code src/app/auth.rs}).
     */
    public Optional<UUID> authenticateAccessToken(String token) {
        Optional<UUID> userId = paseto.verifyAccess(token);
        if (userId.isEmpty()) {
            return Optional.empty();
        }
        Integer found = jdbc.query(
                """
                SELECT 1
                FROM users u
                LEFT JOIN user_trust_scores uts ON uts.user_id = u.id
                WHERE u.id = ? AND u.deleted_at IS NULL
                  AND (uts.banned_until IS NULL OR uts.banned_until <= now())
                """,
                rs -> rs.next() ? 1 : null,
                userId.get());
        if (found == null) {
            logIfBanned(userId.get());
            return Optional.empty();
        }
        return userId;
    }

    /** Best-effort audit log distinguishing "banned" from "deleted" on the rejection path above. */
    private void logIfBanned(UUID userId) {
        try {
            Timestamp banned = jdbc.queryForObject(
                    "SELECT banned_until FROM user_trust_scores WHERE user_id = ?", Timestamp.class, userId);
            if (banned != null && banned.toInstant().isAfter(Instant.now())) {
                log.warn("rejected access token for banned user {} (banned_until={})", userId, banned.toInstant());
            }
        } catch (Exception ignored) {
            // No trust-score row or lookup failure: not worth failing the request over a log line.
        }
    }

    public AuthTokenResponse login(String email, String password) {
        SqlRowSet rs = jdbc.queryForRowSet(
                """
                SELECT u.id, u.password_hash, uts.banned_until
                FROM users u
                LEFT JOIN user_trust_scores uts ON uts.user_id = u.id
                WHERE (u.email = ? OR u.handle = ?) AND u.deleted_at IS NULL
                """,
                email, email);
        if (!rs.next()) {
            throw ApiException.unauthorized("invalid credentials");
        }
        UUID userId = (UUID) rs.getObject("id");
        String hash = rs.getString("password_hash");
        if (hash == null || hash.isEmpty() || !crypto.verifyPassword(password, hash)) {
            throw ApiException.unauthorized("invalid credentials");
        }
        Timestamp banned = rs.getTimestamp("banned_until");
        if (banned != null && banned.toInstant().isAfter(java.time.Instant.now())) {
            throw ApiException.forbidden("Your account has been temporarily suspended");
        }
        return issuePair(userId).response();
    }

    @Transactional
    public AuthTokenResponse refresh(String refreshToken) {
        var claims = paseto.verifyRefresh(refreshToken)
                .orElseThrow(() -> ApiException.unauthorized("invalid refresh token"));
        String tokenHash = crypto.sha256Hex(refreshToken);

        SqlRowSet rs = jdbc.queryForRowSet(
                """
                SELECT rt.revoked_at, u.deleted_at, uts.banned_until
                FROM refresh_tokens rt
                JOIN users u ON u.id = rt.user_id
                LEFT JOIN user_trust_scores uts ON uts.user_id = rt.user_id
                WHERE rt.id = ? AND rt.user_id = ? AND rt.token_hash = ?
                  AND rt.expires_at > now()
                FOR UPDATE OF rt
                """,
                claims.refreshId(), claims.userId(), tokenHash);
        if (!rs.next()) {
            throw ApiException.unauthorized("invalid refresh token");
        }
        if (rs.getTimestamp("revoked_at") != null) {
            jdbc.update(
                    "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = ? AND revoked_at IS NULL",
                    claims.userId());
            throw ApiException.unauthorized("invalid refresh token");
        }
        if (rs.getTimestamp("deleted_at") != null) {
            throw ApiException.unauthorized("invalid refresh token");
        }
        Timestamp banned = rs.getTimestamp("banned_until");
        if (banned != null && banned.toInstant().isAfter(java.time.Instant.now())) {
            throw ApiException.forbidden("Your account has been temporarily suspended");
        }

        Issued issued = issuePair(claims.userId());
        jdbc.update(
                """
                UPDATE refresh_tokens SET revoked_at = now(), replaced_by = ?
                WHERE id = ? AND revoked_at IS NULL
                """,
                issued.refreshId(), claims.refreshId());
        return issued.response();
    }

    public void revoke(String refreshToken) {
        var claims = paseto.verifyRefresh(refreshToken);
        if (claims.isEmpty()) {
            return;
        }
        String tokenHash = crypto.sha256Hex(refreshToken);
        jdbc.update(
                """
                UPDATE refresh_tokens SET revoked_at = now()
                WHERE id = ? AND user_id = ? AND token_hash = ? AND revoked_at IS NULL
                """,
                claims.get().refreshId(), claims.get().userId(), tokenHash);
    }

    public Optional<User> getCurrentUser(UUID userId) {
        return jdbc.query(
                """
                SELECT id, handle, email, display_name, bio, avatar_key, created_at
                FROM users WHERE id = ? AND deleted_at IS NULL
                """,
                rs -> rs.next() ? Optional.of(mapUser(rs)) : Optional.empty(),
                userId);
    }

    @Transactional
    public User signup(
            String handle,
            String email,
            String displayName,
            String bio,
            String avatarKey,
            String password,
            String inviteCode) {
        String passwordHash = crypto.hashPassword(password);
        User user = jdbc.query(
                """
                INSERT INTO users (handle, email, display_name, bio, avatar_key, password_hash)
                VALUES (?, ?, ?, ?, ?, ?)
                RETURNING id, handle, email, display_name, bio, avatar_key, created_at
                """,
                rs -> {
                    if (!rs.next()) {
                        throw ApiException.internal("signup failed");
                    }
                    return mapUser(rs);
                },
                handle, email, displayName, bio, avatarKey, passwordHash);

        jdbc.update(
                "INSERT INTO user_trust_scores (user_id) VALUES (?) ON CONFLICT DO NOTHING",
                user.getId());

        int consumed = jdbc.update(
                """
                UPDATE invite_codes
                SET use_count = use_count + 1
                WHERE code = ? AND is_valid
                  AND (expires_at IS NULL OR expires_at > now())
                  AND use_count < max_uses
                """,
                inviteCode);
        if (consumed == 0) {
            throw ApiException.badRequest("invalid invite code");
        }
        return user;
    }

    private record Issued(UUID refreshId, AuthTokenResponse response) {}

    private Issued issuePair(UUID userId) {
        UUID refreshId = UUID.randomUUID();
        var access = paseto.mintAccess(userId);
        var refresh = paseto.mintRefresh(userId, refreshId);
        String hash = crypto.sha256Hex(refresh.token());
        jdbc.update(
                """
                INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
                VALUES (?, ?, ?, ?)
                """,
                refreshId, userId, hash, Timestamp.from(refresh.expiresAt().toInstant()));
        return new Issued(
                refreshId,
                new AuthTokenResponse(
                        access.token(), refresh.token(), access.expiresAt(), refresh.expiresAt()));
    }

    private static User mapUser(ResultSet rs) throws SQLException {
        Timestamp created = rs.getTimestamp("created_at");
        return User.builder()
                .id((UUID) rs.getObject("id"))
                .handle(rs.getString("handle"))
                .email(rs.getString("email"))
                .displayName(rs.getString("display_name"))
                .bio(rs.getString("bio"))
                .avatarKey(rs.getString("avatar_key"))
                .createdAt(created == null ? null : OffsetDateTime.ofInstant(created.toInstant(), ZoneOffset.UTC))
                .build();
    }
}
