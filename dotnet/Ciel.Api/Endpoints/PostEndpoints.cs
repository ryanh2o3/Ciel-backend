using System.Text;
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
        group.MapGet("/{id}", GetPost).OptionalAuth();
        group.MapPost("/{id}/like", LikePost).RequireAuth();
        group.MapDelete("/{id}/like", UnlikePost).RequireAuth();
        group.MapPost("/{id}/comment", CommentPost).RequireAuth();
        return group;
    }

    private static async Task<IResult> CreatePost(
        ExtendedApiService extended,
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

        if (req.Caption is not null && System.Text.Encoding.UTF8.GetByteCount(req.Caption) > Validation.MaxCaptionLen)
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

        extended.invalidateFollowers(ownerId);
        extended.activity(ownerId,"post_created");
        await media.PopulatePostAvatarUrlsAsync([post], ct);
        return Results.Json(post);
    }

    private static async Task<IResult> GetPost(
        ExtendedApiService extended,
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
        ExtendedApiService extended,
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
        if(created)extended.postEngagement(userId,id,"post_liked",null);
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
        ExtendedApiService extended,
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

        if (req.Body.EnumerateRunes().Count() > maxCommentLen)
        {
            throw ApiException.BadRequest("comment body exceeds 1000 characters");
        }

        var userId = AuthHttpContext.Require(ctx).UserId;
        if (!await posts.CanViewPostAsync(id, userId, ct))
        {
            throw ApiException.NotFound("post not found");
        }

        var comment = await engagement.CommentPostAsync(userId, id, req.Body, ct);
        extended.postEngagement(userId,id,"post_commented",req.Body);
        return Results.Json(comment);
    }
}
