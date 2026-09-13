using Ciel.Api.Config;

namespace Ciel.Api.Middleware;

public sealed class ServedByMiddleware(RequestDelegate next, AppConfig config)
{
    public async Task InvokeAsync(HttpContext context)
    {
        context.Response.OnStarting(() =>
        {
            context.Response.Headers["X-Ciel-Served-By"] = config.ServedBy;
            return Task.CompletedTask;
        });

        await next(context);
    }
}
