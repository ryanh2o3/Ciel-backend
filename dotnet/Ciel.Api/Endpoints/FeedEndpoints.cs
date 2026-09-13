using Ciel.Api.Domain;
using Ciel.Api.Http;
using Ciel.Api.Http.Auth;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Services;

namespace Ciel.Api.Endpoints;

public static class FeedEndpoints
{
    public static RouteGroupBuilder MapFeedEndpoints(this RouteGroupBuilder group)
    {
        group.MapGet("/", GetFeed).RequireAuth();
        group.MapPost("/refresh", RefreshFeed).RequireAuth();
        return group;
    }

    private static async Task<IResult> GetFeed(
        FeedService feed,
        MediaService media,
        HttpContext ctx,
        int? limit,
        string? cursor,
        CancellationToken ct)
    {
        var parsedLimit = Validation.ParseLimit(limit);
        var parsedCursor = CursorCodec.Parse(cursor);
        var userId = AuthHttpContext.Require(ctx).UserId;

        var (posts, next) = await feed.GetHomeFeedAsync(userId, parsedCursor, parsedLimit, ct);
        await media.PopulatePostAvatarUrlsAsync(posts, ct);

        return Results.Json(new ListResponse<Post>
        {
            Items = posts,
            NextCursor = CursorCodec.Encode(next),
        });
    }

    private static async Task<IResult> RefreshFeed(
        FeedService feed,
        HttpContext ctx,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        await feed.RefreshHomeFeedAsync(userId, ct);
        return Results.NoContent();
    }
}
