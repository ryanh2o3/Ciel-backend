using Ciel.Api.Services;
using Xunit;

namespace Ciel.Api.Tests;

public class CryptoServiceTests
{
    [Fact]
    public async Task HashThenVerifyRoundTrips()
    {
        var crypto = new CryptoService();
        const string password = "Sup3rSecretPassw0rd!";

        var hash = await crypto.HashPasswordAsync(password);

        Assert.StartsWith("$argon2id$", hash);
        Assert.True(await crypto.VerifyPasswordAsync(password, hash));
    }

    [Fact]
    public async Task VerifyFailsForWrongPassword()
    {
        var crypto = new CryptoService();
        var hash = await crypto.HashPasswordAsync("correct-password");

        Assert.False(await crypto.VerifyPasswordAsync("wrong-password", hash));
    }

    [Fact]
    public async Task VerifyAcceptsRustGeneratedUnpaddedPhc()
    {
        // Real hash produced by the Rust argon2 crate (unpadded base64 segments).
        const string rustPhc = "$argon2id$v=19$m=19456,t=2,p=1$aNvD0XsklBlRiXk6Pz+W9A$RwEl4P+w33YwAAa2qjmh7zYsNEq8kzBi/3LJfHfAFwI";
        var crypto = new CryptoService();

        Assert.True(await crypto.VerifyPasswordAsync("ChangeMe123!", rustPhc));
        Assert.False(await crypto.VerifyPasswordAsync("wrong-password", rustPhc));
    }

    [Fact]
    public async Task VerifyFailsForMalformedHash()
    {
        var crypto = new CryptoService();
        Assert.False(await crypto.VerifyPasswordAsync("anything", "not-a-phc-string"));
    }

    [Fact]
    public async Task HashingTwiceProducesDifferentSaltsAndHashes()
    {
        var crypto = new CryptoService();
        const string password = "same-password";

        var hashOne = await crypto.HashPasswordAsync(password);
        var hashTwo = await crypto.HashPasswordAsync(password);

        Assert.NotEqual(hashOne, hashTwo);
        Assert.True(await crypto.VerifyPasswordAsync(password, hashOne));
        Assert.True(await crypto.VerifyPasswordAsync(password, hashTwo));
    }

    [Fact]
    public void Sha256HexIsLowercaseAndStable()
    {
        var a = CryptoService.Sha256Hex("hello world");
        var b = CryptoService.Sha256Hex("hello world");

        Assert.Equal(a, b);
        Assert.Equal(a, a.ToLowerInvariant());
        Assert.Equal(64, a.Length);
    }
}
