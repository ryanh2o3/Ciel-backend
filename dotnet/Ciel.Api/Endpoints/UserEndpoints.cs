using Ciel.Api.Domain;
using Ciel.Api.Http;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;
using Npgsql;

namespace Ciel.Api.Endpoints;

public static class UserEndpoints
{
    public static RouteGroupBuilder MapUserEndpoints(this RouteGroupBuilder group)
    {
        group.MapPost("/", CreateUser);
        group.MapGet("/{id:guid}", GetUser);
        return group;
    }

    private static async Task<IResult> CreateUser(
        AuthService auth,
        MediaService media,
        CreateUserRequest req,
        CancellationToken ct)
    {
        Validation.ValidateSignup(
            req.Handle,
            req.Email,
            req.DisplayName,
            req.Bio,
            req.Password,
            req.InviteCode);

        try
        {
            var user = await auth.SignupAsync(
                req.Handle,
                req.Email.Trim(),
                req.DisplayName,
                req.Bio,
                req.AvatarKey,
                req.Password,
                req.InviteCode,
                ct);
            await media.PopulateUserAvatarUrlAsync(user, ct);
            return Results.Json(user);
        }
        catch (PostgresException ex) when (ex.SqlState == PostgresErrorCodes.UniqueViolation)
        {
            if (ex.ConstraintName?.Contains("users_handle_key") == true)
            {
                throw ApiException.Conflict("Handle already taken");
            }

            if (ex.ConstraintName?.Contains("users_email_key") == true)
            {
                throw ApiException.Conflict("Email already taken");
            }

            throw;
        }
    }

    private static async Task<IResult> GetUser(
        UserService users,
        MediaService media,
        Guid id,
        CancellationToken ct)
    {
        var result = await users.GetPublicUserWithCountsAsync(id, ct)
            ?? throw ApiException.NotFound("user not found");

        var (user, followers, following, posts) = result;
        await media.PopulateUserAvatarUrlAsync(user, ct);

        var publicUser = new PublicUser
        {
            Id = user.Id,
            Handle = user.Handle,
            DisplayName = user.DisplayName,
            Bio = user.Bio,
            AvatarUrl = user.AvatarUrl,
            CreatedAt = user.CreatedAt,
            FollowersCount = followers,
            FollowingCount = following,
            PostsCount = posts,
        };

        return Results.Json(publicUser);
    }
}
