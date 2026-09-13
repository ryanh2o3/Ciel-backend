using System.Text.Json.Serialization;

namespace Ciel.Api.Domain;

public sealed class Post
{
    public Guid Id { get; set; }
    public Guid OwnerId { get; set; }
    public string? OwnerHandle { get; set; }
    public string? OwnerDisplayName { get; set; }
    public List<Guid> MediaIds { get; set; } = [];

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public Media? PrimaryMedia { get; set; }

    public string? Caption { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public PostVisibility Visibility { get; set; }

    // Rust's `Post::owner_avatar_key` uses `skip_serializing_if = "Option::is_none"`,
    // so it IS included on the wire (and in the Redis feed cache) whenever present.
    // A previous unconditional [JsonIgnore] here dropped it from the cache entirely,
    // breaking avatar URL population on cache hits.
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? OwnerAvatarKey { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? OwnerAvatarUrl { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public long? LikeCount { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public long? CommentCount { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? LikedByViewer { get; set; }
}
