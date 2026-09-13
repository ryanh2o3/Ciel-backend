using Ciel.Api.Http.Dtos;
using Npgsql;
using StackExchange.Redis;

namespace Ciel.Api.Endpoints;

public static class HealthEndpoints
{
    private static readonly TimeSpan PingTimeout = TimeSpan.FromSeconds(2);

    public static WebApplication MapHealthEndpoints(this WebApplication app)
    {
        app.MapGet("/health", async (
            NpgsqlDataSource db,
            IConnectionMultiplexer redis,
            ILogger<Program> logger,
            CancellationToken ct) =>
        {
            var dbOk = false;
            var redisOk = false;

            try
            {
                using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
                timeoutCts.CancelAfter(PingTimeout);

                await using var conn = await db.OpenConnectionAsync(timeoutCts.Token);
                await using var cmd = conn.CreateCommand();
                cmd.CommandText = "SELECT 1";
                await cmd.ExecuteScalarAsync(timeoutCts.Token);
                dbOk = true;
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "health check: database ping failed or timed out");
            }

            try
            {
                using var timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(ct);
                timeoutCts.CancelAfter(PingTimeout);

                _ = await redis.GetDatabase().PingAsync().WaitAsync(timeoutCts.Token);
                redisOk = true;
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "health check: redis ping failed or timed out");
            }

            return Results.Json(new HealthResponse
            {
                Status = dbOk && redisOk ? "ok" : "degraded",
            });
        });

        return app;
    }
}
