using Ciel.Api.Config;
using Ciel.Api.Services;
using Xunit;

namespace Ciel.Api.Tests;

/// <summary>
/// Mirrors Rust's rate_limit.rs unit tests (src/http/middleware/rate_limit.rs)
/// to keep the /v1-prefixed route -> action mapping identical across stacks.
/// </summary>
public class RateLimitActionsTests
{
    [Fact]
    public void PostCreateRespectsV1Prefix()
    {
        Assert.Equal("post", RateLimitActions.ForAuthenticatedRequest("/v1/posts", "POST"));
    }

    [Fact]
    public void LikeUnderV1()
    {
        Assert.Equal(
            "like",
            RateLimitActions.ForAuthenticatedRequest("/v1/posts/550e8400-e29b-41d4-a716-446655440000/like", "POST"));
    }

    [Fact]
    public void MediaReadVsUpload()
    {
        Assert.Equal("media_read", RateLimitActions.ForAuthenticatedRequest("/v1/media/some-id", "GET"));
        Assert.Equal("media_upload", RateLimitActions.ForAuthenticatedRequest("/v1/media/upload", "POST"));
    }

    [Fact]
    public void UnmatchedRouteReturnsNull()
    {
        Assert.Null(RateLimitActions.ForAuthenticatedRequest("/v1/users/123", "GET"));
    }

    [Fact]
    public void IpLimitLoginUnderV1()
    {
        var config = RateLimitActions.ForIpRequest("/v1/auth/login", "POST", 10);
        Assert.NotNull(config);
        Assert.Equal("login", config!.Value.Action);
        Assert.Equal(RateWindow.Hour, config.Value.Window);
    }

    [Fact]
    public void IpLimitSignupUsesConfiguredLimit()
    {
        var config = RateLimitActions.ForIpRequest("/v1/users", "POST", 7);
        Assert.NotNull(config);
        Assert.Equal("signup", config!.Value.Action);
        Assert.Equal(7L, config.Value.Limit);
        Assert.Equal(RateWindow.Day, config.Value.Window);
    }

    [Fact]
    public void IpLimitHealthUnprefixed()
    {
        var config = RateLimitActions.ForIpRequest("/health", "GET", 10);
        Assert.NotNull(config);
        Assert.Equal("health", config!.Value.Action);
    }
}
