using Ciel.Api.Config;
using Ciel.Api.Endpoints;
using Ciel.Api.Http.Errors;
using Ciel.Api.Http.Json;
using Ciel.Api.Middleware;
using Ciel.Api.Services;
using Npgsql;
using StackExchange.Redis;

var builder = WebApplication.CreateBuilder(args);

builder.Configuration.AddEnvironmentVariables();

var appConfig = AppConfig.FromEnvironment(builder.Configuration);
builder.Services.AddSingleton(appConfig);

var dataSourceBuilder = new NpgsqlDataSourceBuilder(appConfig.DatabaseUrl);
dataSourceBuilder.ConnectionStringBuilder.MaxPoolSize = appConfig.DbMaxConnections;
var dataSource = dataSourceBuilder.Build();
builder.Services.AddSingleton(dataSource);

builder.Services.AddSingleton<IConnectionMultiplexer>(_ =>
    ConnectionMultiplexer.Connect(appConfig.RedisUrl));

builder.Services.AddSingleton<StorageService>();
builder.Services.AddSingleton<QueueService>();
builder.Services.AddSingleton<CryptoService>();
builder.Services.AddSingleton<PasetoService>();
builder.Services.AddSingleton<AuthService>();
builder.Services.AddSingleton<UserService>();
builder.Services.AddSingleton<PostService>();
builder.Services.AddSingleton<FeedService>();
builder.Services.AddSingleton<EngagementService>();
builder.Services.AddSingleton<MediaService>();

builder.Services.AddExceptionHandler<ErrorHandler>();
builder.Services.AddProblemDetails();

builder.Services.ConfigureHttpJsonOptions(options =>
{
    var ciel = CielJsonOptions.Create();
    options.SerializerOptions.PropertyNamingPolicy = ciel.PropertyNamingPolicy;
    options.SerializerOptions.DictionaryKeyPolicy = ciel.DictionaryKeyPolicy;
    options.SerializerOptions.DefaultIgnoreCondition = ciel.DefaultIgnoreCondition;
    options.SerializerOptions.PropertyNameCaseInsensitive = ciel.PropertyNameCaseInsensitive;
    foreach (var converter in ciel.Converters)
    {
        options.SerializerOptions.Converters.Add(converter);
    }
});

var app = builder.Build();

app.UseExceptionHandler();
app.UseMiddleware<ServedByMiddleware>();

app.MapHealthEndpoints();

var v1 = app.MapGroup("/v1");
v1.MapGroup("/auth").MapAuthEndpoints();
v1.MapGroup("/users").MapUserEndpoints();
v1.MapGroup("/posts").MapPostEndpoints();
v1.MapGroup("/feed").MapFeedEndpoints();
v1.MapGroup("/media").MapMediaEndpoints();

if (!string.IsNullOrWhiteSpace(appConfig.HttpAddr) &&
    appConfig.HttpAddr.Contains(':', StringComparison.Ordinal))
{
    var parts = appConfig.HttpAddr.Split(':', 2);
    app.Urls.Add($"http://{parts[0]}:{parts[1]}");
}

app.Run();

public partial class Program;
