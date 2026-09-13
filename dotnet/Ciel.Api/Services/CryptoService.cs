using System.Security.Cryptography;
using System.Text;
using Konscious.Security.Cryptography;

namespace Ciel.Api.Services;

public sealed class CryptoService
{
    /// <summary>
    /// Argon2id is deliberately expensive (CPU + ~19MB memory per call); running
    /// it inline on a thread-pool-bound async request would starve the pool
    /// under load. <c>DenyChildAttach</c> keeps it off the ambient execution
    /// context (no flowing cancellation/culture) which is what we want for a
    /// pure CPU-bound background computation.
    /// </summary>
    public Task<string> HashPasswordAsync(string password) => Task.Factory.StartNew(
        () => HashPassword(password),
        CancellationToken.None,
        TaskCreationOptions.DenyChildAttach,
        TaskScheduler.Default);

    public Task<bool> VerifyPasswordAsync(string password, string phc) => Task.Factory.StartNew(
        () => VerifyPassword(password, phc),
        CancellationToken.None,
        TaskCreationOptions.DenyChildAttach,
        TaskScheduler.Default);

    private string HashPassword(string password)
    {
        var salt = new byte[16];
        RandomNumberGenerator.Fill(salt);
        var hash = HashWithParams(password, salt, iterations: 2, memoryKb: 19456, parallelism: 1);
        return FormatPhc(hash, salt, iterations: 2, memoryKb: 19456, parallelism: 1);
    }

    private bool VerifyPassword(string password, string phc)
    {
        try
        {
            // PHC format: $argon2id$v=19$m=..,t=..,p=..$<salt>$<hash> — after
            // Split(RemoveEmptyEntries) the leading '$' disappears, so the 5
            // parts are indexed [0]=argon2id [1]=v=19 [2]=params [3]=salt [4]=hash.
            var parts = phc.Split('$', StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length < 5 || !parts[0].Equals("argon2id", StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }

            var paramParts = parts[2].Split(',');
            var memoryKb = ParseParam(paramParts, 'm');
            var iterations = ParseParam(paramParts, 't');
            var parallelism = ParseParam(paramParts, 'p');
            var salt = Convert.FromBase64String(parts[3]);
            var expected = Convert.FromBase64String(parts[4]);
            var actual = HashWithParams(password, salt, iterations, memoryKb, parallelism);
            return CryptographicOperations.FixedTimeEquals(actual, expected);
        }
        catch
        {
            return false;
        }
    }

    public static string Sha256Hex(string value)
    {
        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes(value));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    private static byte[] HashWithParams(
        string password,
        byte[] salt,
        int iterations,
        int memoryKb,
        int parallelism)
    {
        var argon2 = new Argon2id(Encoding.UTF8.GetBytes(password))
        {
            Salt = salt,
            DegreeOfParallelism = parallelism,
            Iterations = iterations,
            MemorySize = memoryKb,
        };
        return argon2.GetBytes(32);
    }

    private static string FormatPhc(byte[] hash, byte[] salt, int iterations, int memoryKb, int parallelism)
    {
        return $"$argon2id$v=19$m={memoryKb},t={iterations},p={parallelism}${Convert.ToBase64String(salt)}${Convert.ToBase64String(hash)}";
    }

    private static int ParseParam(string[] parts, char key)
    {
        foreach (var part in parts)
        {
            if (part.Length > 2 && part[0] == key && part[1] == '=' &&
                int.TryParse(part[2..], out var value))
            {
                return value;
            }
        }

        throw new FormatException($"missing argon2 param {key}");
    }
}
