using Amazon.Runtime;
using Amazon.S3;
using Amazon.S3.Model;
using Ciel.Api.Config;

namespace Ciel.Api.Services;

public sealed class StorageService : IDisposable
{
    private readonly IAmazonS3 _client;
    private readonly string _bucket;
    private readonly string? _publicEndpoint;
    private readonly ILogger<StorageService> _logger;
    private bool _disposed;

    public StorageService(AppConfig config, ILogger<StorageService> logger)
    {
        _logger = logger;
        _bucket = config.S3Bucket;
        _publicEndpoint = config.S3PublicEndpoint;

        var s3Config = new AmazonS3Config
        {
            ServiceURL = config.S3Endpoint,
            ForcePathStyle = config.S3ForcePathStyle,
            AuthenticationRegion = config.S3Region,
        };

        _client = new AmazonS3Client(ResolveCredentials(config), s3Config);
    }

    private static AWSCredentials ResolveCredentials(AppConfig config)
    {
        if (!string.IsNullOrEmpty(config.SqsAccessKey))
        {
            return new BasicAWSCredentials(config.SqsAccessKey, config.SqsSecretKey);
        }

        return FallbackCredentialsFactory.GetCredentials();
    }

    public string Bucket => _bucket;

    public (string Url, List<(string Name, string Value)> Headers) PresignPut(
        string objectKey,
        string contentType,
        long contentLength,
        long expiresSeconds)
    {
        var request = new GetPreSignedUrlRequest
        {
            BucketName = _bucket,
            Key = objectKey,
            Verb = HttpVerb.PUT,
            ContentType = contentType,
            Expires = DateTime.UtcNow.AddSeconds(expiresSeconds),
        };

        string url;
        try
        {
            url = _client.GetPreSignedURL(request);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "failed to presign PUT url for {ObjectKey}", objectKey);
            throw;
        }

        var headers = new List<(string, string)>
        {
            ("Content-Type", contentType),
        };

        return (url, headers.Select(h => (h.Item1, h.Item2)).ToList());
    }

    public string PresignGet(string objectKey, long expiresSeconds)
    {
        var request = new GetPreSignedUrlRequest
        {
            BucketName = _bucket,
            Key = objectKey,
            Verb = HttpVerb.GET,
            Expires = DateTime.UtcNow.AddSeconds(expiresSeconds),
        };

        string url;
        try
        {
            url = _client.GetPreSignedURL(request);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "failed to presign GET url for {ObjectKey}", objectKey);
            throw;
        }

        return RewritePublicEndpoint(url);
    }

    private string RewritePublicEndpoint(string url)
    {
        if (string.IsNullOrWhiteSpace(_publicEndpoint))
        {
            return url;
        }

        if (!Uri.TryCreate(url, UriKind.Absolute, out var original) ||
            !Uri.TryCreate(_publicEndpoint.Contains("://") ? _publicEndpoint : $"http://{_publicEndpoint}", UriKind.Absolute, out var publicBase))
        {
            return url;
        }

        var builder = new UriBuilder(original)
        {
            Scheme = publicBase.Scheme,
            Host = publicBase.Host,
            Port = publicBase.IsDefaultPort ? -1 : publicBase.Port,
        };
        return builder.Uri.ToString();
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _client.Dispose();
    }
}
