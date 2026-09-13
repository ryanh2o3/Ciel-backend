using Ciel.Api.Domain;
using Ciel.Api.Http;
using Ciel.Api.Http.Auth;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;
using Npgsql;

namespace Ciel.Api.Endpoints;

public static class PostEndpoints
{
    public static RouteGroupBuilder MapPostEndpoints(this RouteGroupBuilder group)
    {
        group.MapPost("/", CreatePost).RequireAuth();
        group.MapGet("/{id:guid}", GetPost).OptionalAuth();
        group.MapPost("/{id:guid}/like", LikePost).RequireAuth();
        group.MapDelete("/{id:guid}/like", UnlikePost).RequireAuth();
        group.MapPost("/{id:guid}/comment", CommentPost).RequireAuth();
        return group;
    }

    private static async Task<IResult> CreatePost(
        PostService posts,
        MediaService media,
        FeedService feed,
        HttpContext ctx,
        CreatePostRequest req,
        CancellationToken ct)
    {
        if (req.MediaIds.Count == 0)
        {
            throw ApiException.BadRequest("at least one media_id is required");
        }

        if (req.MediaIds.Count > 10)
        {
            throw ApiException.BadRequest("maximum 10 images per post");
        }

        if (req.Caption is not null && req.Caption.Length > Validation.MaxCaptionLen)
        {
            throw ApiException.BadRequest("caption must be at most 2200 characters");
        }

        var ownerId = AuthHttpContext.Require(ctx).UserId;
        Post post;
        try
        {
            post = await posts.CreatePostAsync(ownerId, req.MediaIds, req.Caption, ct);
        }
        catch (PostgresException ex) when (ex.SqlState == PostgresErrorCodes.ForeignKeyViolation)
        {
            throw ApiException.BadRequest("invalid media_id");
        }

        await feed.RefreshHomeFeedAsync(ownerId, ct);
        await media.PopulatePostAvatarUrlsAsync([post], ct);
        return Results.Json(post);
    }

    private static async Task<IResult> GetPost(
        PostService posts,
        MediaService media,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var viewerId = AuthHttpContext.OptionalUserId(ctx);
        var post = await posts.GetPostAsync(id, viewerId, ct)
            ?? throw ApiException.NotFound("post not found");
        await media.PopulatePostAvatarUrlsAsync([post], ct);
        return Results.Json(post);
    }

    private static async Task<IResult> LikePost(
        PostService posts,
        EngagementService engagement,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        if (!await posts.CanViewPostAsync(id, userId, ct))
        {
            throw ApiException.NotFound("post not found");
        }

        var created = await engagement.LikePostAsync(userId, id, ct);
        return Results.Json(new LikeResponse { Created = created });
    }

    private static async Task<IResult> UnlikePost(
        EngagementService engagement,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        var deleted = await engagement.UnlikePostAsync(userId, id, ct);
        return deleted ? Results.NoContent() : throw ApiException.NotFound("like not found");
    }

    private static async Task<IResult> CommentPost(
        PostService posts,
        EngagementService engagement,
        HttpContext ctx,
        Guid id,
        CommentRequest req,
        CancellationToken ct)
    {
        const int maxCommentLen = 1000;
        if (string.IsNullOrWhiteSpace(req.Body))
        {
            throw ApiException.BadRequest("comment body cannot be empty");
        }

        if (req.Body.Length > maxCommentLen)
        {
            throw ApiException.BadRequest("comment body exceeds 1000 characters");
        }

        var userId = AuthHttpContext.Require(ctx).UserId;
        if (!await posts.CanViewPostAsync(id, userId, ct))
        {
            throw ApiException.NotFound("post not found");
        }

        var comment = await engagement.CommentPostAsync(userId, id, req.Body, ct);
        return Results.Json(comment);
    }
}
