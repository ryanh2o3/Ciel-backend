using System.Text.Json;
using Amazon.Runtime;
using Amazon.SQS;
using Amazon.SQS.Model;
using Ciel.Api.Config;
using Ciel.Api.Domain;
using Ciel.Api.Http.Json;

namespace Ciel.Api.Services;

public sealed class QueueService
{
    private readonly IAmazonSQS _client;
    private readonly string _queueName;
    private string? _queueUrl;

    public QueueService(AppConfig config)
    {
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
        var body = JsonSerializer.Serialize(job, CielJsonOptions.Create());

        await _client.SendMessageAsync(new SendMessageRequest
        {
            QueueUrl = url,
            MessageBody = body,
        }, ct);
    }

    private async Task<string> GetQueueUrlAsync(CancellationToken ct)
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
}
