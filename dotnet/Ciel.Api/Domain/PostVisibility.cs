using System.Text.Json.Serialization;

namespace Ciel.Api.Domain;

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum PostVisibility
{
    Public,
    FollowersOnly,
}

public static class PostVisibilityDb
{
    public static PostVisibility? FromDb(string value) => value switch
    {
        "public" => PostVisibility.Public,
        "followers_only" => PostVisibility.FollowersOnly,
        _ => null,
    };

    public static string AsDb(PostVisibility visibility) => visibility switch
    {
        PostVisibility.Public => "public",
        PostVisibility.FollowersOnly => "followers_only",
        _ => throw new ArgumentOutOfRangeException(nameof(visibility)),
    };
}
