namespace Ciel.Api.Config;

/// <summary>
/// Environment-driven configuration, mirroring Rust's <c>AppConfig::from_env</c>
/// (src/config/mod.rs) so both stacks honor the same env vars against the
/// same Postgres/Redis/S3/SQS infrastructure. See ../Ciel-backend/.env.example.
/// </summary>
public sealed class AppConfig
{
    public required string HttpAddr { get; init; }
    public required string DatabaseUrl { get; init; }
    public required string RedisUrl { get; init; }

    public required string S3Endpoint { get; init; }
    public string? S3PublicEndpoint { get; init; }
    public required string S3Region { get; init; }
    public required string S3Bucket { get; init; }
    public bool S3ForcePathStyle { get; init; }

    public required string QueueEndpoint { get; init; }
    public required string QueueRegion { get; init; }
    public required string QueueName { get; init; }
    public string SqsAccessKey { get; init; } = string.Empty;
    public string SqsSecretKey { get; init; } = string.Empty;

    public int DbMaxConnections { get; init; }

    public string? AdminToken { get; init; }

    public long UploadUrlTtlSeconds { get; init; }
    public long UploadMaxBytes { get; init; }

    public required byte[] PasetoAccessKey { get; init; }
    public required byte[] PasetoRefreshKey { get; init; }
    public long AccessTtlMinutes { get; init; }
    public long RefreshTtlDays { get; init; }

    /// <summary>Value used for the X-Ciel-Served-By response header (contract: rust|spring|dotnet).</summary>
    public string ServedBy { get; init; } = "dotnet";

    public static AppConfig FromEnvironment(IConfiguration configuration)
    {
        string EnvOr(string key, string fallback) => configuration[key] ?? fallback;
        string EnvOrThrow(string key) =>
            configuration[key] ?? throw new InvalidOperationException($"missing required env var: {key}");

        var s3Region = EnvOr("S3_REGION", "fr-par");
        var queueRegion = configuration["QUEUE_REGION"] ?? s3Region;

        return new AppConfig
        {
            HttpAddr = EnvOr("HTTP_ADDR", "0.0.0.0:8080"),
            DatabaseUrl = EnvOrThrow("DATABASE_URL"),
            RedisUrl = EnvOr("REDIS_URL", "redis://127.0.0.1/"),

            S3Endpoint = EnvOrThrow("S3_ENDPOINT"),
            S3PublicEndpoint = configuration["S3_PUBLIC_ENDPOINT"],
            S3Region = s3Region,
            S3Bucket = EnvOrThrow("S3_BUCKET"),
            S3ForcePathStyle = string.Equals(EnvOr("S3_FORCE_PATH_STYLE", "false"), "true", StringComparison.OrdinalIgnoreCase),

            QueueEndpoint = EnvOrThrow("QUEUE_ENDPOINT"),
            QueueRegion = queueRegion,
            QueueName = EnvOrThrow("QUEUE_NAME"),
            SqsAccessKey = EnvOr("SQS_ACCESS_KEY", string.Empty),
            SqsSecretKey = EnvOr("SQS_SECRET_KEY", string.Empty),

            DbMaxConnections = int.Parse(EnvOr("DB_MAX_CONNECTIONS", "25")),

            AdminToken = configuration["ADMIN_TOKEN"],

            UploadUrlTtlSeconds = long.Parse(EnvOr("UPLOAD_URL_TTL_SECONDS", "900")),
            UploadMaxBytes = long.Parse(EnvOr("UPLOAD_MAX_BYTES", "10485760")),

            PasetoAccessKey = DecodeKey(EnvOrThrow("PASETO_ACCESS_KEY"), "PASETO_ACCESS_KEY"),
            PasetoRefreshKey = DecodeKey(EnvOrThrow("PASETO_REFRESH_KEY"), "PASETO_REFRESH_KEY"),
            AccessTtlMinutes = long.Parse(EnvOr("ACCESS_TTL_MINUTES", "15")),
            RefreshTtlDays = long.Parse(EnvOr("REFRESH_TTL_DAYS", "30")),

            ServedBy = EnvOr("CIEL_SERVED_BY", "dotnet"),
        };
    }

    private static byte[] DecodeKey(string value, string keyName)
    {
        byte[] decoded;
        try
        {
            decoded = Convert.FromBase64String(value);
        }
        catch (FormatException ex)
        {
            throw new InvalidOperationException($"invalid {keyName}: {ex.Message}");
        }

        if (decoded.Length != 32)
        {
            throw new InvalidOperationException($"invalid {keyName}: expected 32 bytes, got {decoded.Length}");
        }

        return decoded;
    }
}
