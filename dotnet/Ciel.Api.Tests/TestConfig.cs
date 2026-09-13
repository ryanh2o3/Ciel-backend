using System.Security.Cryptography;
using Ciel.Api.Config;

namespace Ciel.Api.Tests;

/// <summary>Builds a minimal, valid <see cref="AppConfig"/> for unit tests.</summary>
internal static class TestConfig
{
    public static AppConfig Create(
        long accessTtlMinutes = 15,
        long refreshTtlDays = 30) => new()
    {
        HttpAddr = "0.0.0.0:8080",
        DatabaseUrl = "postgres://test:test@localhost:5432/test",
        RedisUrl = "redis://127.0.0.1/",
        S3Endpoint = "http://127.0.0.1:4566",
        S3Region = "fr-par",
        S3Bucket = "test-bucket",
        QueueEndpoint = "http://127.0.0.1:4566",
        QueueRegion = "fr-par",
        QueueName = "test-queue",
        DbMaxConnections = 5,
        UploadUrlTtlSeconds = 900,
        UploadMaxBytes = 10_485_760,
        PasetoAccessKey = RandomBytes(32),
        PasetoRefreshKey = RandomBytes(32),
        AccessTtlMinutes = accessTtlMinutes,
        RefreshTtlDays = refreshTtlDays,
    };

    private static byte[] RandomBytes(int length)
    {
        var bytes = new byte[length];
        RandomNumberGenerator.Fill(bytes);
        return bytes;
    }
}
