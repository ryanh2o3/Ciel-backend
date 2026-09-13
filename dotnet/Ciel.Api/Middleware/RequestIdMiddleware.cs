namespace Ciel.Api.Middleware;

/// <summary>
/// Generates an <c>X-Request-Id</c> when the client doesn't supply one, and
/// propagates it back on the response. Mirrors Rust's
/// <c>SetRequestIdLayer::x_request_id</c> / <c>PropagateRequestIdLayer</c> (src/http/mod.rs).
/// </summary>
public sealed class RequestIdMiddleware(RequestDelegate next)
{
    public const string HeaderName = "X-Request-Id";

    public async Task InvokeAsync(HttpContext context)
    {
        var requestId = context.Request.Headers[HeaderName].FirstOrDefault();
        if (string.IsNullOrWhiteSpace(requestId))
        {
            requestId = Guid.NewGuid().ToString();
        }

        context.Request.Headers[HeaderName] = requestId;

        context.Response.OnStarting(() =>
        {
            context.Response.Headers[HeaderName] = requestId;
            return Task.CompletedTask;
        });

        await next(context);
    }
}
