using Ciel.Api.Config;
using Paseto;
using Paseto.Builder;
using Paseto.Cryptography.Key;
using Paseto.Protocol;

namespace Ciel.Api.Services;

public sealed class PasetoService
{
    private readonly PasetoSymmetricKey _accessKey;
    private readonly PasetoSymmetricKey _refreshKey;
    private readonly long _accessTtlMinutes;
    private readonly long _refreshTtlDays;

    public PasetoService(AppConfig config)
    {
        _accessKey = new PasetoSymmetricKey(config.PasetoAccessKey, new Version4());
        _refreshKey = new PasetoSymmetricKey(config.PasetoRefreshKey, new Version4());
        _accessTtlMinutes = config.AccessTtlMinutes;
        _refreshTtlDays = config.RefreshTtlDays;
    }

    public sealed record IssuedToken(string Token, DateTimeOffset ExpiresAt);

    public sealed record RefreshClaims(Guid UserId, Guid RefreshId);

    public IssuedToken MintAccess(Guid userId)
    {
        var now = DateTime.UtcNow;
        var exp = now.AddMinutes(_accessTtlMinutes);
        var token = new PasetoBuilder()
            .Use(ProtocolVersion.V4, Purpose.Local)
            .WithKey(_accessKey)
            .Issuer("ciel")
            .Audience("ciel")
            .Subject(userId.ToString())
            .IssuedAt(now)
            .Expiration(exp)
            .AddClaim("typ", "access")
            .Encode();
        return new IssuedToken(token, new DateTimeOffset(exp, TimeSpan.Zero));
    }

    public IssuedToken MintRefresh(Guid userId, Guid refreshId)
    {
        var now = DateTime.UtcNow;
        var exp = now.AddDays(_refreshTtlDays);
        var token = new PasetoBuilder()
            .Use(ProtocolVersion.V4, Purpose.Local)
            .WithKey(_refreshKey)
            .Issuer("ciel")
            .Audience("ciel")
            .Subject(userId.ToString())
            .TokenIdentifier(refreshId.ToString())
            .IssuedAt(now)
            .Expiration(exp)
            .AddClaim("typ", "refresh")
            .Encode();
        return new IssuedToken(token, new DateTimeOffset(exp, TimeSpan.Zero));
    }

    public Guid? VerifyAccess(string token)
    {
        var payload = Decrypt(token, _accessKey, "access");
        if (payload is null || !payload.TryGetValue("sub", out var sub))
        {
            return null;
        }

        return Guid.TryParse(sub?.ToString(), out var userId) ? userId : null;
    }

    public RefreshClaims? VerifyRefresh(string token)
    {
        var payload = Decrypt(token, _refreshKey, "refresh");
        if (payload is null ||
            !payload.TryGetValue("sub", out var sub) ||
            !payload.TryGetValue("jti", out var jti))
        {
            return null;
        }

        if (!Guid.TryParse(sub?.ToString(), out var userId) ||
            !Guid.TryParse(jti?.ToString(), out var refreshId))
        {
            return null;
        }

        return new RefreshClaims(userId, refreshId);
    }

    private Dictionary<string, object>? Decrypt(string token, PasetoSymmetricKey key, string expectedTyp)
    {
        try
        {
            var validation = new PasetoTokenValidationParameters
            {
                ValidateLifetime = true,
                ValidateIssuer = true,
                ValidateAudience = true,
                ValidIssuer = "ciel",
                ValidAudience = "ciel",
            };

            var result = new PasetoBuilder()
                .Use(ProtocolVersion.V4, Purpose.Local)
                .WithKey(key)
                .Decode(token, validation);

            if (!result.IsValid || result.Paseto?.Payload is null)
            {
                return null;
            }

            var payload = result.Paseto.Payload;
            if (!payload.TryGetValue("typ", out var typObj) ||
                !string.Equals(typObj?.ToString(), expectedTyp, StringComparison.Ordinal))
            {
                return null;
            }

            return payload;
        }
        catch
        {
            return null;
        }
    }
}
