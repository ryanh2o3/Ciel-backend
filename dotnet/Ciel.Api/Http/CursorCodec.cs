using Ciel.Api.Http.Errors;
using Ciel.Api.Http.Json;

namespace Ciel.Api.Http;

public static class CursorCodec
{
    public static (DateTimeOffset Timestamp, Guid Id)? Parse(string? cursor)
    {
        if (string.IsNullOrWhiteSpace(cursor))
        {
            return null;
        }

        var slash = cursor.IndexOf('/');
        if (slash <= 0 || slash >= cursor.Length - 1)
        {
            throw ApiException.BadRequest("invalid cursor");
        }

        var timestampPart = cursor[..slash];
        var idPart = cursor[(slash + 1)..];

        if (!DateTimeOffset.TryParse(
                timestampPart,
                System.Globalization.CultureInfo.InvariantCulture,
                System.Globalization.DateTimeStyles.RoundtripKind | System.Globalization.DateTimeStyles.AssumeUniversal,
                out var timestamp) ||
            !Guid.TryParse(idPart, out var id))
        {
            throw ApiException.BadRequest("invalid cursor");
        }

        return (timestamp.ToUniversalTime(), id);
    }

    public static string? Encode((DateTimeOffset Timestamp, Guid Id)? cursor)
    {
        if (cursor is null)
        {
            return null;
        }

        var ts = Rfc3339DateTimeOffsetConverter.Format(cursor.Value.Timestamp);
        return $"{ts}/{cursor.Value.Id}";
    }
}
