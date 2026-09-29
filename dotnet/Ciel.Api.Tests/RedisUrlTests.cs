using Ciel.Api.Config;
using Xunit;

namespace Ciel.Api.Tests;

public class RedisUrlTests
{
    [Fact]
    public void ConvertsRedisUriToStackExchangeOptions()
    {
        var options = AppConfig.BuildRedisConfiguration("redis://ciel-redis:6379");

        Assert.Contains("ciel-redis:6379", options.ToString());
        Assert.False(options.Ssl);
        Assert.Null(options.DefaultDatabase);
    }

    [Fact]
    public void ConvertsRedisUriWithPasswordAndDb()
    {
        var options = AppConfig.BuildRedisConfiguration("redis://:s3cr3t@redis.local:6380/2");

        Assert.Contains("redis.local:6380", options.ToString());
        Assert.Equal("s3cr3t", options.Password);
        Assert.Equal(2, options.DefaultDatabase);
    }

    [Fact]
    public void DefaultsPortForUriWithoutExplicitPort()
    {
        var options = AppConfig.BuildRedisConfiguration("redis://redis/");

        Assert.Contains("redis:6379", options.ToString());
    }

    [Fact]
    public void RedissEnablesTls()
    {
        var options = AppConfig.BuildRedisConfiguration("rediss://redis.example.com");

        Assert.True(options.Ssl);
        Assert.Contains("redis.example.com:6383", options.ToString());
    }

    [Fact]
    public void LeavesStackExchangeStringsUntouched()
    {
        var options = AppConfig.BuildRedisConfiguration("localhost:6380,allowAdmin=true");

        Assert.Contains("localhost:6380", options.ToString());
    }
}
