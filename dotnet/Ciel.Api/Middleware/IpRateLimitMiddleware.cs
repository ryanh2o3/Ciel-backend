using Ciel.Api.Config;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;

namespace Ciel.Api.Middleware;

/// <summary>
/// IP-based rate limiting for unauthenticated endpoints (login, signup, etc).
/// Mirrors Rust's <c>ip_rate_limit_middleware</c> (src/http/middleware/rate_limit.rs).
/// </summary>
public sealed class IpRateLimitMiddleware(RequestDelegate next, ILogger<IpRateLimitMiddleware> logger)
{
    public async Task InvokeAsync(HttpContext context, AppConfig config, RateLimiterService rateLimiter)
    {
        var path = context.Request.Path.Value ?? string.Empty;
        var method = context.Request.Method;

        var ipConfig = RateLimitActions.ForIpRequest(path, method, config.IpSignupRateLimit);
        if (ipConfig is null)
        {
            await next(context);
            return;
        }

        var (action, limit, window) = ipConfig.Value;
        var ip = RequestContextMiddleware.Get(context)?.ClientIp.ToString()
            ?? context.Connection.RemoteIpAddress?.ToString()
            ?? "unknown";

        var isLimited = await rateLimiter.CheckAndIncrementIpAsync(ip, action, limit, window);
        if (isLimited)
        {
            logger.LogWarning("IP rate limit exceeded for {Ip} action {Action}", ip, action);
            throw ApiException.TooManyRequestsWithHeaders(
                "Too many attempts from your IP address. Please try again later.",
                limit,
                0);
        }

        await next(context);
    }
}
