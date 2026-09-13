using Ciel.Api.Services;
using Xunit;

namespace Ciel.Api.Tests;

public class PasetoServiceTests
{
    [Fact]
    public void MintAndVerifyAccessTokenRoundTrips()
    {
        var service = new PasetoService(TestConfig.Create());
        var userId = Guid.NewGuid();

        var issued = service.MintAccess(userId);
        Assert.False(string.IsNullOrWhiteSpace(issued.Token));
        Assert.True(issued.ExpiresAt > DateTimeOffset.UtcNow);

        var verified = service.VerifyAccess(issued.Token);
        Assert.Equal(userId, verified);
    }

    [Fact]
    public void MintAndVerifyRefreshTokenRoundTrips()
    {
        var service = new PasetoService(TestConfig.Create());
        var userId = Guid.NewGuid();
        var refreshId = Guid.NewGuid();

        var issued = service.MintRefresh(userId, refreshId);
        var claims = service.VerifyRefresh(issued.Token);

        Assert.NotNull(claims);
        Assert.Equal(userId, claims!.UserId);
        Assert.Equal(refreshId, claims.RefreshId);
    }

    [Fact]
    public void VerifyAccessRejectsRefreshToken()
    {
        var service = new PasetoService(TestConfig.Create());
        var userId = Guid.NewGuid();

        var refresh = service.MintRefresh(userId, Guid.NewGuid());

        // typ=refresh tokens must not be usable as access tokens.
        Assert.Null(service.VerifyAccess(refresh.Token));
    }

    [Fact]
    public void VerifyRefreshRejectsAccessToken()
    {
        var service = new PasetoService(TestConfig.Create());
        var userId = Guid.NewGuid();

        var access = service.MintAccess(userId);

        Assert.Null(service.VerifyRefresh(access.Token));
    }

    [Fact]
    public void VerifyAccessRejectsGarbageToken()
    {
        var service = new PasetoService(TestConfig.Create());
        Assert.Null(service.VerifyAccess("not-a-real-token"));
    }

    [Fact]
    public void VerifyAccessRejectsTokenSignedWithDifferentKey()
    {
        var serviceA = new PasetoService(TestConfig.Create());
        var serviceB = new PasetoService(TestConfig.Create());

        var issued = serviceA.MintAccess(Guid.NewGuid());

        Assert.Null(serviceB.VerifyAccess(issued.Token));
    }

    [Fact]
    public void ExpiredAccessTokenFailsVerification()
    {
        // Negative TTL mints a token whose expiration is already in the past.
        var service = new PasetoService(TestConfig.Create(accessTtlMinutes: -1));
        var issued = service.MintAccess(Guid.NewGuid());

        Assert.Null(service.VerifyAccess(issued.Token));
    }
}
