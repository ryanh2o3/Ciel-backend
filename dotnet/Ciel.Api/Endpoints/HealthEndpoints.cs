using Ciel.Api.Http.Dtos;
using Npgsql;
using StackExchange.Redis;

namespace Ciel.Api.Endpoints;

public static class HealthEndpoints
{
    public static WebApplication MapHealthEndpoints(this WebApplication app)
    {
        app.MapGet("/health", async (NpgsqlDataSource db, IConnectionMultiplexer redis, CancellationToken ct) =>
        {
            var dbOk = false;
            var redisOk = false;

            try
            {
                await using var conn = await db.OpenConnectionAsync(ct);
                await using var cmd = conn.CreateCommand();
                cmd.CommandText = "SELECT 1";
                await cmd.ExecuteScalarAsync(ct);
                dbOk = true;
            }
            catch
            {
                // degraded
            }

            try
            {
                _ = await redis.GetDatabase().PingAsync();
                redisOk = true;
            }
            catch
            {
                // degraded
            }

            return Results.Json(new HealthResponse
            {
                Status = dbOk && redisOk ? "ok" : "degraded",
            });
        });

        return app;
    }
}

