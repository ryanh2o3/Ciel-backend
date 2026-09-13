using System.Text.Json;
using Amazon.Runtime;
using Amazon.SQS;
using Amazon.SQS.Model;
using Ciel.Api.Config;
using Ciel.Api.Domain;
using Ciel.Api.Http.Json;

namespace Ciel.Api.Services;

public sealed class QueueService : IDisposable
{
    // Building a JsonSerializerOptions (and its converters) on every call is
    // wasteful; the options are immutable and safe to share across requests.
    private static readonly JsonSerializerOptions JsonOptions = CielJsonOptions.Create();

    private readonly IAmazonSQS _client;
    private readonly string _queueName;
    private readonly ILogger<QueueService> _logger;
    private readonly SemaphoreSlim _queueUrlLock = new(1, 1);
    private string? _queueUrl;
    private bool _disposed;

    public QueueService(AppConfig config, ILogger<QueueService> logger)
    {
        _logger = logger;
        var sqsConfig = new AmazonSQSConfig
        {
            ServiceURL = config.QueueEndpoint,
            AuthenticationRegion = config.QueueRegion,
        };

        AWSCredentials credentials = string.IsNullOrEmpty(config.SqsAccessKey)
            ? FallbackCredentialsFactory.GetCredentials()
            : new BasicAWSCredentials(config.SqsAccessKey, config.SqsSecretKey);

        _client = new AmazonSQSClient(credentials, sqsConfig);
        _queueName = config.QueueName;
    }

    public async Task EnqueueMediaJobAsync(MediaJob job, CancellationToken ct)
    {
        var url = await GetQueueUrlAsync(ct);
        var body = JsonSerializer.Serialize(job, JsonOptions);

        try
        {
            await _client.SendMessageAsync(new SendMessageRequest
            {
                QueueUrl = url,
                MessageBody = body,
            }, ct);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "failed to enqueue media job for upload {UploadId}", job.UploadId);
            throw;
        }
    }

    private async Task<string> GetQueueUrlAsync(CancellationToken ct)
    {
        if (_queueUrl is not null)
        {
            return _queueUrl;
        }

        // Guards against a thundering herd of concurrent requests each trying to
        // resolve (and potentially create) the same queue on cold start.
        await _queueUrlLock.WaitAsync(ct);
        try
        {
            if (_queueUrl is not null)
            {
                return _queueUrl;
            }

            try
            {
                var response = await _client.GetQueueUrlAsync(_queueName, ct);
                _queueUrl = response.QueueUrl;
                return _queueUrl;
            }
            catch (QueueDoesNotExistException)
            {
                _logger.LogInformation("queue {QueueName} does not exist yet, creating it", _queueName);
                var created = await _client.CreateQueueAsync(new CreateQueueRequest
                {
                    QueueName = _queueName,
                    Attributes = new Dictionary<string, string>
                    {
                        ["VisibilityTimeout"] = "300",
                    },
                }, ct);
                _queueUrl = created.QueueUrl;
                return _queueUrl;
            }
        }
        finally
        {
            _queueUrlLock.Release();
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _client.Dispose();
        _queueUrlLock.Dispose();
    }
}
