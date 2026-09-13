namespace Ciel.Api.Middleware;

/// <summary>
/// Security headers middleware that adds essential security headers to all
/// responses and enforces HTTPS when we can infer the original scheme from
/// trusted forwarded headers. Mirrors Rust's <c>security_headers_middleware</c>
/// (src/http/middleware/security.rs).
/// </summary>
public sealed class SecurityHeadersMiddleware(RequestDelegate next, ILogger<SecurityHeadersMiddleware> logger)
{
    private const string HstsValue = "max-age=31536000; includeSubDomains";
    private const string CspValue = "default-src 'none'; frame-ancestors 'none'";
    private const string CacheControlValue = "no-store, no-cache, must-revalidate";

    public async Task InvokeAsync(HttpContext context)
    {
        var host = context.Request.Headers.Host.ToString();

        var isLocal = host.StartsWith("localhost", StringComparison.Ordinal)
            || host.StartsWith("127.0.0.1", StringComparison.Ordinal)
            || host.StartsWith("0.0.0.0", StringComparison.Ordinal)
            || host.StartsWith("[::1]", StringComparison.Ordinal);

        var scheme = RequestContextMiddleware.Get(context)?.Scheme ?? ResolvedScheme.Unknown;

        // Never trust a default "https" when headers are missing (see RequestContextMiddleware).
        // Unknown = direct client or no trusted proxy: assume TLS may terminate at the app.
        if (!isLocal && scheme == ResolvedScheme.Http)
        {
            logger.LogWarning("rejected non-HTTPS request (forwarded scheme was http) for host {Host}", host);
            context.Response.StatusCode = StatusCodes.Status403Forbidden;
            return;
        }

        context.Response.OnStarting(() =>
        {
            var headers = context.Response.Headers;

            if (!isLocal)
            {
                headers["Strict-Transport-Security"] = HstsValue;
            }

            headers["X-Content-Type-Options"] = "nosniff";
            headers["X-Frame-Options"] = "DENY";
            headers["X-XSS-Protection"] = "1; mode=block";
            headers["Content-Security-Policy"] = CspValue;
            headers["Referrer-Policy"] = "no-referrer";

            if (!headers.ContainsKey("Cache-Control"))
            {
                headers["Cache-Control"] = CacheControlValue;
            }

            return Task.CompletedTask;
        });

        await next(context);
    }
}
