using System.Text.Json.Serialization;

namespace Ciel.Api.Domain;

public sealed class User
{
    public Guid Id { get; set; }
    public required string Handle { get; set; }
    public required string Email { get; set; }
    public required string DisplayName { get; set; }
    public string? Bio { get; set; }

    [JsonIgnore]
    public string? AvatarKey { get; set; }

    public string? AvatarUrl { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
}

public sealed class PublicUser
{
    public Guid Id { get; set; }
    public required string Handle { get; set; }
    public required string DisplayName { get; set; }
    public string? Bio { get; set; }
    public string? AvatarUrl { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public long FollowersCount { get; set; }
    public long FollowingCount { get; set; }
    public long PostsCount { get; set; }
}
