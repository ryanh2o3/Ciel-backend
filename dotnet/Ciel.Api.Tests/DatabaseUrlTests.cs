using Ciel.Api.Config;
using Xunit;

namespace Ciel.Api.Tests;

public class DatabaseUrlTests
{
    [Fact]
    public void ConvertsPostgresUriToNpgsqlKeyValue()
    {
        var result = AppConfig.ToNpgsqlConnectionString(
            "postgres://ciel:change-me-strong-db-password@host.k3d.internal:5432/ciel");

        Assert.Contains("Host=host.k3d.internal", result);
        Assert.Contains("Port=5432", result);
        Assert.Contains("Username=ciel", result);
        Assert.Contains("Password=change-me-strong-db-password", result);
        Assert.Contains("Database=ciel", result);
        Assert.DoesNotContain("postgres://", result);
    }

    [Fact]
    public void LeavesKeyValueStringsUntouched()
    {
        const string raw = "Host=localhost;Username=ciel;Password=x;Database=ciel";
        Assert.Equal(raw, AppConfig.ToNpgsqlConnectionString(raw));
    }
}
