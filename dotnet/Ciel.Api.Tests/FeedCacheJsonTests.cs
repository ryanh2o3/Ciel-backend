using System.Text.Json;
using Ciel.Api.Domain;
using Ciel.Api.Http.Json;
using Xunit;

namespace Ciel.Api.Tests;

/// <summary>
/// Regression coverage for the feed avatar cache bug: <see cref="Post.OwnerAvatarKey"/>
/// must round-trip through the same <see cref="JsonSerializerOptions"/> used for both
/// the HTTP response body and the Redis <c>feed:home:{userId}</c> cache entry
/// (see Services/FeedService.cs), matching Rust's
/// <c>#[serde(default, skip_serializing_if = "Option::is_none")] owner_avatar_key</c>
/// (src/domain/post.rs) — present on the wire whenever set, omitted when null.
/// </summary>
public class FeedCacheJsonTests
{
    private static readonly JsonSerializerOptions Options = CielJsonOptions.Create();

    [Fact]
    public void OwnerAvatarKeyIsSerializedWhenSet()
    {
        var post = NewPost();
        post.OwnerAvatarKey = "avatars/owner-123.jpg";

        var json = JsonSerializer.Serialize(post, Options);

        Assert.Contains("\"owner_avatar_key\":\"avatars/owner-123.jpg\"", json);
    }

    [Fact]
    public void OwnerAvatarKeyIsOmittedWhenNull()
    {
        var post = NewPost();
        post.OwnerAvatarKey = null;

        var json = JsonSerializer.Serialize(post, Options);

        Assert.DoesNotContain("owner_avatar_key", json);
    }

    [Fact]
    public void OwnerAvatarKeySurvivesCacheRoundTrip()
    {
        var post = NewPost();
        post.OwnerAvatarKey = "avatars/owner-456.jpg";

        // Simulates writing to and reading back from the Redis feed cache
        // (FeedService.GetHomeFeedAsync / CachedHomeFeed).
        var cached = JsonSerializer.Serialize(new List<Post> { post }, Options);
        var roundTripped = JsonSerializer.Deserialize<List<Post>>(cached, Options);

        Assert.NotNull(roundTripped);
        Assert.Equal("avatars/owner-456.jpg", roundTripped![0].OwnerAvatarKey);
    }

    private static Post NewPost() => new()
    {
        Id = Guid.NewGuid(),
        OwnerId = Guid.NewGuid(),
        OwnerHandle = "someone",
        OwnerDisplayName = "Some One",
        MediaIds = [Guid.NewGuid()],
        Caption = "hello",
        CreatedAt = DateTimeOffset.UtcNow,
        Visibility = PostVisibility.Public,
    };
}
