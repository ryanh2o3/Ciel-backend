using Ciel.Api.Http;
using Ciel.Api.Http.Errors;
using Xunit;

namespace Ciel.Api.Tests;

public class CursorCodecTests
{
    [Fact]
    public void ParseReturnsNullForEmptyCursor()
    {
        Assert.Null(CursorCodec.Parse(null));
        Assert.Null(CursorCodec.Parse(""));
        Assert.Null(CursorCodec.Parse("   "));
    }

    [Fact]
    public void EncodeThenParseRoundTrips()
    {
        var timestamp = DateTimeOffset.Parse("2024-01-15T12:30:45.123456Z").ToUniversalTime();
        var id = Guid.NewGuid();

        var encoded = CursorCodec.Encode((timestamp, id));
        Assert.NotNull(encoded);

        var decoded = CursorCodec.Parse(encoded);
        Assert.NotNull(decoded);
        Assert.Equal(timestamp, decoded.Value.Timestamp);
        Assert.Equal(id, decoded.Value.Id);
    }

    [Fact]
    public void EncodeOfNullReturnsNull()
    {
        Assert.Null(CursorCodec.Encode(null));
    }

    [Theory]
    [InlineData("not-a-cursor")]
    [InlineData("2024-01-15T12:00:00Z/not-a-guid")]
    [InlineData("/550e8400-e29b-41d4-a716-446655440000")]
    [InlineData("2024-01-15T12:00:00Z/")]
    public void ParseThrowsBadRequestForMalformedCursor(string malformed)
    {
        var ex = Assert.Throws<ApiException>(() => CursorCodec.Parse(malformed));
        Assert.Equal(400, ex.StatusCode);
    }

    [Fact]
    public void ParseAcceptsRfc3339MicrosecondPrecision()
    {
        const string cursor = "2024-01-15T12:00:00.123456Z/550e8400-e29b-41d4-a716-446655440000";
        var decoded = CursorCodec.Parse(cursor);

        Assert.NotNull(decoded);
        Assert.Equal(Guid.Parse("550e8400-e29b-41d4-a716-446655440000"), decoded.Value.Id);
        Assert.Equal(TimeSpan.Zero, decoded.Value.Timestamp.Offset);
    }
}
