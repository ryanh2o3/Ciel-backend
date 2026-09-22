using System.Text.Json;
using System.Text.Json.Serialization;

namespace Ciel.Api.Http.Json;

public static class CielJsonOptions
{
    public static JsonSerializerOptions Create()
    {
        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = SnakeCaseNamingPolicy.Instance,
            DictionaryKeyPolicy = SnakeCaseNamingPolicy.Instance,
            // Rust includes null Option fields unless skip_serializing_if is set
            // (User.bio, ListResponse.next_cursor, Media.*_url, UploadStatus.processed_media_id).
            // Post optional engagement/avatar fields use [JsonIgnore(WhenWritingNull)] instead.
            DefaultIgnoreCondition = JsonIgnoreCondition.Never,
            PropertyNameCaseInsensitive = true,
        };

        options.Converters.Add(new Rfc3339DateTimeOffsetConverter());
        options.Converters.Add(new Rfc3339NullableDateTimeOffsetConverter());
        options.Converters.Add(new JsonStringEnumConverter());
        return options;
    }
}
