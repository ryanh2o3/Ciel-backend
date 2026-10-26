using Ciel.Api.Config;
using Ciel.Api.Http.Auth;
using Ciel.Api.Http.Errors;
using Ciel.Api.Services;
using System.Text.Json;
using System.Security.Cryptography;
using System.Text;

namespace Ciel.Api.Endpoints;

public static class ExtendedEndpoints
{
    public static RouteGroupBuilder MapExtendedEndpoints(this RouteGroupBuilder group)
    {
        group
            .MapMethods("/users/{id}", new[] { "PATCH" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.updateProfile(user!.Value, id, s(body, "display_name"),
                                                                  s(body, "bio"), s(body, "avatar_key")));
                        })
            .RequireAuth();
        group
            .MapMethods("/account", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            api.deleteAccount(user!.Value);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/posts", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.userPosts(id, user, limit, cursor));
                        })
            .OptionalAuth();
        group
            .MapMethods("/users/{id}/follow", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.social(user!.Value, id, "follow"));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/unfollow", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.social(user!.Value, id, "unfollow"));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/block", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.social(user!.Value, id, "block"));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/unblock", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.social(user!.Value, id, "unblock"));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/relationship", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.relationship(user!.Value, id));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/followers", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.socialList(id, true, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/following", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.socialList(id, false, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/posts/{id}", new[] { "PATCH" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.updatePost(user!.Value, id, s(body, "caption")));
                        })
            .RequireAuth();
        group
            .MapMethods("/posts/{id}", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.deletePost(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/posts/{id}/likes", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.engagements(user!.Value, id, true, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/posts/{id}/comments", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.engagements(user!.Value, id, false, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/posts/{id}/comments/{comment_id}", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var commentId = ParseId(request, "comment_id");
                            api.deleteComment(user!.Value, id, commentId);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/media/{id}", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.deleteMedia(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/notifications", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.notifications(user!.Value, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/notifications/{id}/read", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.markNotification(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/search/users", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            var q = request.Request.Query["q"].FirstOrDefault();
                            return Results.Json(api.search(true, q, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/search/posts", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            var q = request.Request.Query["q"].FirstOrDefault();
                            return Results.Json(api.search(false, q, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/moderation/users/{id}/flag", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.flag(user!.Value, id, false, s(body, "reason")));
                        })
            .RequireAuth();
        group
            .MapMethods("/moderation/posts/{id}/flag", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.flag(user!.Value, id, true, s(body, "reason")));
                        })
            .RequireAuth();
        group
            .MapMethods("/moderation/posts/{id}/takedown", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            Admin(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            api.takedown(user, id, true, s(body, "reason"));
                            return Results.NoContent();
                        })
            .OptionalAuth();
        group
            .MapMethods("/moderation/comments/{id}/takedown", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            Admin(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            api.takedown(user, id, false, s(body, "reason"));
                            return Results.NoContent();
                        })
            .OptionalAuth();
        group
            .MapMethods("/moderation/audit", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            Admin(request);
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.audit(limit, cursor));
                        })
            .OptionalAuth();
        group
            .MapMethods("/account/trust-score", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            return Results.Json(api.trust(user!.Value));
                        })
            .RequireAuth();
        group
            .MapMethods("/account/rate-limits", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            return Results.Json(api.quotas(user!.Value));
                        })
            .RequireAuth();
        group
            .MapMethods("/account/device/register", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            api.registerDevice(user, s(body, "fingerprint"),
                                               request.Request.Headers.UserAgent.FirstOrDefault());
                            return Results.NoContent();
                        })
            .OptionalAuth();
        group
            .MapMethods("/account/devices", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            return Results.Json(api.devices(user!.Value));
                        })
            .RequireAuth();
        group
            .MapMethods("/invites", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            return Results.Json(api.invites(user!.Value));
                        })
            .RequireAuth();
        group
            .MapMethods("/invites", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.createInvite(user!.Value, days(body)));
                        })
            .RequireAuth();
        group
            .MapMethods("/invites/stats", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            return Results.Json(api.inviteStats(user!.Value));
                        })
            .RequireAuth();
        group
            .MapMethods("/invites/{code}/revoke", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var code = request.Request.RouteValues["code"]!.ToString()!;
                            api.revokeInvite(user!.Value, code);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group.MapMethods("/invites/validate/{code}", new[] { "GET" },
                         (HttpContext request, ExtendedApiService api) =>
                         {
                             var user = AuthHttpContext.OptionalUserId(request);
                             var code = request.Request.RouteValues["code"]!.ToString()!;
                             return Results.Json(api.validateInvite(code));
                         });
        group
            .MapMethods("/admin/invites", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            Admin(request);
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.createInvite(null, days(body)));
                        })
            .OptionalAuth();
        group
            .MapMethods("/stories", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.createStory(user!.Value, uuidBody(body, "media_id"),
                                                                s(body, "caption"), required(body, "visibility")));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/stories", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.stories(user!.Value, id, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.getStory(user!.Value, id));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.deleteStory(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/viewers", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.viewers(user!.Value, id, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/reactions", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.react(user!.Value, id, required(body, "emoji")));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/reactions", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.reactions(user!.Value, id, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/reactions", new[] { "DELETE" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.removeReaction(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/seen", new[] { "POST" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            api.seen(user!.Value, id);
                            return Results.NoContent();
                        })
            .RequireAuth();
        group
            .MapMethods("/feed/stories", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.stories(user!.Value, null, limit, cursor));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/metrics", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            return Results.Json(api.metrics(user!.Value, id));
                        })
            .RequireAuth();
        group
            .MapMethods("/stories/{id}/highlights", new[] { "POST" },
                        async (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var body = await request.Request.ReadFromJsonAsync<Dictionary<string, JsonElement>>() ??
                                       throw ApiException.BadRequest("invalid request body");
                            return Results.Json(api.highlight(user!.Value, id, required(body, "highlight_name")));
                        })
            .RequireAuth();
        group
            .MapMethods("/users/{id}/highlights", new[] { "GET" },
                        (HttpContext request, ExtendedApiService api) =>
                        {
                            var user = AuthHttpContext.OptionalUserId(request);
                            var id = ParseId(request, "id");
                            var limit = ParseLimit(request);
                            var cursor = request.Request.Query["cursor"].FirstOrDefault();
                            return Results.Json(api.highlights(id, limit, cursor));
                        })
            .RequireAuth();
        return group;
    }
    private static Guid ParseId(HttpContext request,
                                string key) => Guid.TryParse(request.Request.RouteValues[key]?.ToString(), out var id)
                                                   ? id
                                                   : throw ApiException.BadRequest("invalid request");
    private static int? ParseLimit(HttpContext request)
    {
        string? value = request.Request.Query["limit"].FirstOrDefault();
        if (value is null)
            return null;
        return int.TryParse(value, out var n) ? n
                                              : throw ApiException.BadRequest("invalid request");
    }
    private static string? s(Dictionary<string, JsonElement> body, string key)
    {
        if (!body.TryGetValue(key, out var value) || value.ValueKind == JsonValueKind.Null)
            return null;
        return value.ValueKind == JsonValueKind.String ? value.GetString()
                                                       : throw ApiException.BadRequest("invalid request body");
    }
    private static string required(Dictionary<string, JsonElement> body,
                                   string key) => s(body, key) ?? throw ApiException.BadRequest("invalid request body");
    private static Guid uuidBody(Dictionary<string, JsonElement> body,
                                 string key) => Guid.TryParse(required(body, key), out var id)
                                                    ? id
                                                    : throw ApiException.BadRequest("invalid request body");
    private static long days(Dictionary<string, JsonElement> body) =>
        !body.TryGetValue("days_valid", out var value) ? 7
        : value.TryGetInt64(out var days)              ? days
                                                       : throw ApiException.BadRequest("invalid request body");
    internal static void Admin(HttpContext request)
    {
        var expected = request.RequestServices.GetRequiredService<AppConfig>().AdminToken;
        if (string.IsNullOrEmpty(expected))
            throw ApiException.Forbidden("admin token not configured");
        string? provided = request.Request.Headers["x-admin-token"].FirstOrDefault();
        if (provided is null)
            throw ApiException.Forbidden("missing admin token");
        if (!CryptographicOperations.FixedTimeEquals(Encoding.UTF8.GetBytes(expected),
                                                     Encoding.UTF8.GetBytes(provided)))
            throw ApiException.Forbidden("invalid admin token");
    }
}
