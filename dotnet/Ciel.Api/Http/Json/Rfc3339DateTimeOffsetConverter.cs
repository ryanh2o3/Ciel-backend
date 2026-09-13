using System.Text.Json;
using System.Text.Json.Serialization;

namespace Ciel.Api.Http.Json;

/// <summary>
/// Serializes <see cref="DateTimeOffset"/> as RFC3339 (matching Rust's
/// <c>time::serde::rfc3339</c>), e.g. <c>2024-01-15T12:00:00.123456Z</c> for
/// UTC values. Cursor encoding (see <see cref="Ciel.Api.Http.CursorCodec"/>)
/// uses the same "yyyy-MM-ddTHH:mm:ss.ffffffZ" shape.
/// </summary>
public sealed class Rfc3339DateTimeOffsetConverter : JsonConverter<DateTimeOffset>
{
    public override DateTimeOffset Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        var value = reader.GetString();
        return DateTimeOffset.Parse(value!, System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.RoundtripKind);
    }

    public override void Write(Utf8JsonWriter writer, DateTimeOffset value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(Format(value));
    }

    public static string Format(DateTimeOffset value)
    {
        var utc = value.ToUniversalTime();
        return utc.ToString("yyyy-MM-ddTHH:mm:ss.ffffffZ", System.Globalization.CultureInfo.InvariantCulture);
    }
}

/// <summary>Nullable variant used for optional timestamp fields.</summary>
public sealed class Rfc3339NullableDateTimeOffsetConverter : JsonConverter<DateTimeOffset?>
{
    public override DateTimeOffset? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.Null)
        {
            return null;
        }

        var value = reader.GetString();
        return DateTimeOffset.Parse(value!, System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.RoundtripKind);
    }

    public override void Write(Utf8JsonWriter writer, DateTimeOffset? value, JsonSerializerOptions options)
    {
        if (value is null)
        {
            writer.WriteNullValue();
        }
        else
        {
            writer.WriteStringValue(Rfc3339DateTimeOffsetConverter.Format(value.Value));
        }
    }
}
