using System.Text.Json;
using Ciel.Api.Domain;
using Ciel.Api.Http.Dtos;
using Ciel.Api.Http.Json;
using Xunit;

namespace Ciel.Api.Tests;

/// <summary>Wire-format parity with Rust serde (null keys + RFC3339).</summary>
public class JsonParityTests
{
    private static readonly JsonSerializerOptions Options = CielJsonOptions.Create();

    [Fact]
    public void ListResponseIncludesNullNextCursor()
    {
        var json = JsonSerializer.Serialize(new ListResponse<string>
        {
            Items = ["a"],
            NextCursor = null,
        }, Options);

        Assert.Contains("\"next_cursor\":null", json);
        Assert.Contains("\"items\"", json);
    }

    [Fact]
    public void UserIncludesNullBioAndAvatarUrl()
    {
        var user = new User
        {
            Id = Guid.NewGuid(),
            Handle = "demo",
            Email = "demo@example.com",
            DisplayName = "Demo",
            Bio = null,
            AvatarUrl = null,
            CreatedAt = DateTimeOffset.Parse("2024-01-15T12:00:00Z"),
        };

        var json = JsonSerializer.Serialize(user, Options);

        Assert.Contains("\"bio\":null", json);
        Assert.Contains("\"avatar_url\":null", json);
        Assert.DoesNotContain("avatar_key", json);
        Assert.Contains("\"created_at\":\"2024-01-15T12:00:00", json);
    }

    [Fact]
    public void UploadStatusIncludesNullProcessedMediaId()
    {
        var json = JsonSerializer.Serialize(new UploadStatus
        {
            Status = "pending",
            ProcessedMediaId = null,
        }, Options);

        Assert.Contains("\"processed_media_id\":null", json);
        Assert.Contains("\"status\":\"pending\"", json);
    }

    [Fact]
    public void PostCaptionNullIsIncluded()
    {
        var post = new Post
        {
            Id = Guid.NewGuid(),
            OwnerId = Guid.NewGuid(),
            MediaIds = [],
            Caption = null,
            CreatedAt = DateTimeOffset.UtcNow,
            Visibility = PostVisibility.Public,
        };

        var json = JsonSerializer.Serialize(post, Options);

        Assert.Contains("\"caption\":null", json);
        Assert.DoesNotContain("owner_avatar_key", json);
    }
}
