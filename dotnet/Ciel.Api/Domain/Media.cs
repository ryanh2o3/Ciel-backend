namespace Ciel.Api.Domain;

public sealed class Media
{
    public Guid Id { get; set; }
    public Guid OwnerId { get; set; }
    public required string OriginalKey { get; set; }
    public required string ThumbKey { get; set; }
    public required string MediumKey { get; set; }
    public int Width { get; set; }
    public int Height { get; set; }
    public long Bytes { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public string? ThumbUrl { get; set; }
    public string? MediumUrl { get; set; }
    public string? OriginalUrl { get; set; }
}

public sealed class MediaJob
{
    public Guid UploadId { get; set; }
    public Guid OwnerId { get; set; }
    public required string OriginalKey { get; set; }
}

public sealed class UploadIntent
{
    public Guid UploadId { get; set; }
    public required string ObjectKey { get; set; }
    public required string UploadUrl { get; set; }
    public long ExpiresInSeconds { get; set; }
    public List<UploadHeader> Headers { get; set; } = [];
}

public sealed class UploadHeader
{
    public required string Name { get; set; }
    public required string Value { get; set; }
}

public sealed class UploadStatus
{
    public required string Status { get; set; }
    public Guid? ProcessedMediaId { get; set; }
}
