using Ciel.Api.Http;
using Ciel.Api.Http.Auth;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;

namespace Ciel.Api.Endpoints;

public static class AuthEndpoints
{
    public static RouteGroupBuilder MapAuthEndpoints(this RouteGroupBuilder group)
    {
        group.MapPost("/login", Login);
        group.MapPost("/refresh", Refresh);
        group.MapPost("/revoke", Revoke);
        group.MapGet("/me", GetMe).RequireAuth();
        return group;
    }

    private static async Task<IResult> Login(AuthService auth, LoginRequest req, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(req.Email) || string.IsNullOrWhiteSpace(req.Password))
        {
            throw ApiException.BadRequest("email and password are required");
        }

        if (req.Password.Length > Validation.MaxPasswordLen)
        {
            throw ApiException.BadRequest("password must be at most 128 characters");
        }

        var tokens = await auth.LoginAsync(req.Email.Trim(), req.Password, ct);
        return tokens is null
            ? throw ApiException.Unauthorized("invalid credentials")
            : Results.Json(tokens);
    }

    private static async Task<IResult> Refresh(AuthService auth, RefreshRequest req, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(req.RefreshToken))
        {
            throw ApiException.BadRequest("refresh_token is required");
        }

        var tokens = await auth.RefreshAsync(req.RefreshToken, ct);
        return tokens is null
            ? throw ApiException.Unauthorized("invalid refresh token")
            : Results.Json(tokens);
    }

    private static async Task<IResult> Revoke(AuthService auth, RefreshRequest req, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(req.RefreshToken))
        {
            throw ApiException.BadRequest("refresh_token is required");
        }

        await auth.RevokeAsync(req.RefreshToken, ct);
        return Results.NoContent();
    }

    private static async Task<IResult> GetMe(
        AuthService auth,
        MediaService media,
        HttpContext ctx,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        var user = await auth.GetCurrentUserAsync(userId, ct)
            ?? throw ApiException.NotFound("user not found");
        await media.PopulateUserAvatarUrlAsync(user, ct);
        return Results.Json(user);
    }
}
