using Ciel.Api.Http.Errors;
using Ciel.Api.Services;

namespace Ciel.Api.Http.Auth;

public sealed class AuthEndpointFilter : IEndpointFilter
{
    public async ValueTask<object?> InvokeAsync(
        EndpointFilterInvocationContext context,
        EndpointFilterDelegate next)
    {
        var authHeader = context.HttpContext.Request.Headers.Authorization.FirstOrDefault();
        if (string.IsNullOrWhiteSpace(authHeader))
        {
            throw ApiException.Unauthorized("missing Authorization header");
        }

        if (!authHeader.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
        {
            throw ApiException.Unauthorized("invalid Authorization header");
        }

        var token = authHeader["Bearer ".Length..].Trim();
        var authService = context.HttpContext.RequestServices.GetRequiredService<AuthService>();
        var userId = await authService.AuthenticateAccessTokenAsync(token, context.HttpContext.RequestAborted);
        if (userId is null)
        {
            throw ApiException.Unauthorized("invalid token");
        }

        context.HttpContext.Items[AuthHttpContext.ItemKey] = new AuthUser(userId.Value);

        await RateLimitAndBanGate.EnforceAsync(context.HttpContext, userId.Value);

        return await next(context);
    }
}

public sealed class OptionalAuthEndpointFilter : IEndpointFilter
{
    public async ValueTask<object?> InvokeAsync(
        EndpointFilterInvocationContext context,
        EndpointFilterDelegate next)
    {
        var authHeader = context.HttpContext.Request.Headers.Authorization.FirstOrDefault();
        if (!string.IsNullOrWhiteSpace(authHeader) &&
            authHeader.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
        {
            var token = authHeader["Bearer ".Length..].Trim();
            var authService = context.HttpContext.RequestServices.GetRequiredService<AuthService>();
            var userId = await authService.AuthenticateAccessTokenAsync(token, context.HttpContext.RequestAborted);
            if (userId is not null)
            {
                context.HttpContext.Items[AuthHttpContext.ItemKey] = new AuthUser(userId.Value);
                await RateLimitAndBanGate.EnforceAsync(context.HttpContext, userId.Value);
            }
        }

        return await next(context);
    }
}

public static class AuthEndpointExtensions
{
    public static RouteHandlerBuilder RequireAuth(this RouteHandlerBuilder builder) =>
        builder.AddEndpointFilter<AuthEndpointFilter>();

    public static RouteHandlerBuilder OptionalAuth(this RouteHandlerBuilder builder) =>
        builder.AddEndpointFilter<OptionalAuthEndpointFilter>();
}

/// <summary>
/// Ban check + per-user rate limiting for authenticated requests, run from
/// inside the auth endpoint filters (where <see cref="AuthUser"/> is
/// resolved) rather than as global middleware. Mirrors Rust's
/// <c>ban_check_middleware</c> and <c>rate_limit_middleware</c>
/// (src/http/middleware/ban.rs, src/http/middleware/rate_limit.rs), which
/// both key off an optional <c>AuthUser</c> extractor.
/// </summary>
internal static class RateLimitAndBanGate
{
    public static async Task EnforceAsync(HttpContext httpContext, Guid userId)
    {
        var trustService = httpContext.RequestServices.GetRequiredService<TrustService>();
        var ct = httpContext.RequestAborted;

        if (await trustService.IsBannedAsync(userId, ct))
        {
            throw ApiException.Forbidden("Your account has been temporarily suspended");
        }

        var path = httpContext.Request.Path.Value ?? string.Empty;
        var method = httpContext.Request.Method;
        var action = RateLimitActions.ForAuthenticatedRequest(path, method);
        if (action is null)
        {
            return;
        }

        var trustLevel = await trustService.GetTrustLevelAsync(userId, ct);
        var rateLimiter = httpContext.RequestServices.GetRequiredService<RateLimiterService>();
        var info = await rateLimiter.CheckAndIncrementAsync(userId, action, trustLevel);

        if (info.Limited)
        {
            throw ApiException.TooManyRequestsWithHeaders(
                $"Rate limit exceeded for action: {action}. Please try again later.",
                info.Limit,
                0);
        }

        httpContext.Response.Headers["X-RateLimit-Limit"] = info.Limit.ToString();
        httpContext.Response.Headers["X-RateLimit-Remaining"] = info.Remaining.ToString();
    }
}
