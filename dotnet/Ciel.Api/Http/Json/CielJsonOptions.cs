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
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            PropertyNameCaseInsensitive = true,
        };

        options.Converters.Add(new Rfc3339DateTimeOffsetConverter());
        options.Converters.Add(new Rfc3339NullableDateTimeOffsetConverter());
        options.Converters.Add(new JsonStringEnumConverter());
        return options;
    }
}
