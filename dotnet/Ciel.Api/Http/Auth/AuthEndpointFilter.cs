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
