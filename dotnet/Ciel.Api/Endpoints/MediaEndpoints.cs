using Ciel.Api.Config;
using Ciel.Api.Http.Auth;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;

namespace Ciel.Api.Endpoints;

public static class MediaEndpoints
{
    public static RouteGroupBuilder MapMediaEndpoints(this RouteGroupBuilder group)
    {
        group.MapPost("/upload", CreateUpload).RequireAuth();
        group.MapPost("/upload/{id:guid}/complete", CompleteUpload).RequireAuth();
        group.MapGet("/upload/{id:guid}/status", GetUploadStatus).RequireAuth();
        group.MapGet("/{id:guid}", GetMedia).RequireAuth();
        return group;
    }

    private static async Task<IResult> CreateUpload(
        MediaService media,
        AppConfig config,
        HttpContext ctx,
        UploadRequest req,
        CancellationToken ct)
    {
        if (req.Bytes <= 0)
        {
            throw ApiException.BadRequest("bytes must be greater than 0");
        }

        if (req.Bytes > config.UploadMaxBytes)
        {
            throw ApiException.BadRequest("upload exceeds max size");
        }

        var userId = AuthHttpContext.Require(ctx).UserId;
        var intent = await media.CreateUploadAsync(userId, req.ContentType, req.Bytes, ct);
        return Results.Json(intent);
    }

    private static async Task<IResult> CompleteUpload(
        MediaService media,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        var queued = await media.CompleteUploadAsync(id, userId, ct);
        return queued ? Results.StatusCode(StatusCodes.Status202Accepted) : throw ApiException.NotFound("upload not found");
    }

    private static async Task<IResult> GetUploadStatus(
        MediaService media,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        var status = await media.GetUploadStatusAsync(id, userId, ct)
            ?? throw ApiException.NotFound("upload not found");
        return Results.Json(status);
    }

    private static async Task<IResult> GetMedia(
        MediaService media,
        HttpContext ctx,
        Guid id,
        CancellationToken ct)
    {
        var userId = AuthHttpContext.Require(ctx).UserId;
        var item = await media.GetMediaForUserAsync(id, userId, ct)
            ?? throw ApiException.NotFound("media not found");
        return Results.Json(item);
    }
}
