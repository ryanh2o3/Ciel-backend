using System.Net;
using Ciel.Api.Middleware;
using Xunit;

namespace Ciel.Api.Tests;

/// <summary>
/// Mirrors Rust's request_context.rs unit tests (src/http/middleware/request_context.rs).
/// </summary>
public class RequestContextMiddlewareTests
{
    [Fact]
    public void ForwardedForRightmostUntrustedHop()
    {
        var trusted = new List<IPNetwork> { IPNetwork.Parse("10.0.0.0/8") };

        // 10.0.0.1 is our proxy; 203.0.113.1 is the real client even though a
        // spoofed entry (198.51.100.7) was prepended by the client.
        var ip = RequestContextMiddleware.ParseForwardedFor("198.51.100.7, 203.0.113.1, 10.0.0.1", trusted);

        Assert.Equal(IPAddress.Parse("203.0.113.1"), ip);
    }

    [Fact]
    public void ForwardedForAllTrustedFallsBackToFirst()
    {
        var trusted = new List<IPNetwork> { IPNetwork.Parse("10.0.0.0/8") };

        var ip = RequestContextMiddleware.ParseForwardedFor("10.0.0.2, 10.0.0.1", trusted);

        Assert.Equal(IPAddress.Parse("10.0.0.2"), ip);
    }

    [Fact]
    public void ForwardedProtoHttps()
    {
        Assert.Equal(ResolvedScheme.Https, RequestContextMiddleware.ParseForwardedProto("https, http"));
    }

    [Fact]
    public void TrustedProxyCidrContainsLoopback()
    {
        var cidr = IPNetwork.Parse("127.0.0.1/32");
        Assert.True(RequestContextMiddleware.IsTrustedProxy(IPAddress.Loopback, new List<IPNetwork> { cidr }));
    }

    [Fact]
    public void UntrustedPeerNotInCidr()
    {
        var cidr = IPNetwork.Parse("10.0.0.0/8");
        Assert.False(RequestContextMiddleware.IsTrustedProxy(IPAddress.Parse("192.168.1.1"), new List<IPNetwork> { cidr }));
    }
}
