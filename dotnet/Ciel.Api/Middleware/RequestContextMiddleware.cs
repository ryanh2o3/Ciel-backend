using System.Net;
using Ciel.Api.Config;

namespace Ciel.Api.Middleware;

/// <summary>How the original request was served (from trusted forwarded headers only).</summary>
public enum ResolvedScheme
{
    Http,
    Https,

    /// <summary>Direct connection or untrusted peer — do not enforce HTTPS from headers.</summary>
    Unknown,
}

/// <summary>Populated for every request after <see cref="RequestContextMiddleware"/> runs.</summary>
public sealed record RequestPeer(IPAddress ClientIp, ResolvedScheme Scheme);

/// <summary>
/// Trusted-proxy-aware client IP and request scheme for rate limiting and HTTPS
/// checks. Mirrors Rust's <c>request_context_middleware</c> (src/http/middleware/request_context.rs).
/// </summary>
public sealed class RequestContextMiddleware(RequestDelegate next, AppConfig config)
{
    public const string ItemKey = "CielRequestPeer";

    public async Task InvokeAsync(HttpContext context)
    {
        var remote = context.Connection.RemoteIpAddress ?? IPAddress.Loopback;
        var trusted = IsTrustedProxy(remote, config.TrustedProxyCidrs);

        RequestPeer peer;
        if (trusted)
        {
            var clientIp = ParseForwardedFor(context.Request.Headers["X-Forwarded-For"], config.TrustedProxyCidrs) ?? remote;

            // Fail closed: behind a trusted proxy we require an explicit forwarded scheme.
            var scheme = ParseForwardedProto(context.Request.Headers["X-Forwarded-Proto"]) ?? ResolvedScheme.Http;

            peer = new RequestPeer(clientIp, scheme);
        }
        else
        {
            peer = new RequestPeer(remote, ResolvedScheme.Unknown);
        }

        context.Items[ItemKey] = peer;
        await next(context);
    }

    public static RequestPeer? Get(HttpContext context) =>
        context.Items.TryGetValue(ItemKey, out var value) ? value as RequestPeer : null;

    internal static bool IsTrustedProxy(IPAddress addr, IReadOnlyList<IPNetwork> cidrs)
    {
        foreach (var cidr in cidrs)
        {
            if (cidr.Contains(addr))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// Walk `X-Forwarded-For` from the right (the only end a trusted proxy
    /// controls), skipping our own trusted proxies, and take the first hop that
    /// isn't one of them. Taking the leftmost entry would let clients spoof their
    /// IP — and bypass IP rate limits — by sending a forged XFF header.
    /// </summary>
    internal static IPAddress? ParseForwardedFor(string? value, IReadOnlyList<IPNetwork> trusted)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        var hops = value
            .Split(',')
            .Select(s => s.Trim())
            .Select(s => IPAddress.TryParse(s, out var ip) ? ip : null)
            .Where(ip => ip is not null)
            .Select(ip => ip!)
            .ToList();

        for (var i = hops.Count - 1; i >= 0; i--)
        {
            if (!trusted.Any(cidr => cidr.Contains(hops[i])))
            {
                return hops[i];
            }
        }

        // Every hop is a trusted proxy (e.g. internal health checks).
        return hops.Count > 0 ? hops[0] : null;
    }

    internal static ResolvedScheme? ParseForwardedProto(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }

        var token = value.Split(',')[0].Trim().ToLowerInvariant();
        return token switch
        {
            "https" => ResolvedScheme.Https,
            "http" => ResolvedScheme.Http,
            _ => null,
        };
    }
}
