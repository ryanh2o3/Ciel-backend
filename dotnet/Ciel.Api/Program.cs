using Ciel.Api.Config;
using Ciel.Api.Endpoints;
using Ciel.Api.Http.Errors;
using Ciel.Api.Http.Json;
using Ciel.Api.Middleware;
using Ciel.Api.Services;
using Microsoft.AspNetCore.ResponseCompression;
using Microsoft.AspNetCore.Server.Kestrel.Core;
using Npgsql;
using StackExchange.Redis;

var builder = WebApplication.CreateBuilder(args);

// WebApplication.CreateBuilder already wires up environment variables (and
// command line args, appsettings.json, user secrets) as configuration
// sources by default — an extra AddEnvironmentVariables() call here would be
// a redundant no-op.

var appConfig = AppConfig.FromEnvironment(builder.Configuration);
builder.Services.AddSingleton(appConfig);

// Kestrel request body cap, matching UPLOAD_MAX_BYTES / Rust's RequestBodyLimitLayer (10MB).
builder.Services.Configure<KestrelServerOptions>(options =>
{
    options.Limits.MaxRequestBodySize = 10 * 1024 * 1024;
});

// Response compression (gzip/brotli), matching Rust's tower_http CompressionLayer.
builder.Services.AddResponseCompression(options =>
{
    options.EnableForHttps = true;
    options.Providers.Add<BrotliCompressionProvider>();
    options.Providers.Add<GzipCompressionProvider>();
});

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
builder.Services.AddSingleton<TrustService>();
builder.Services.AddSingleton<RateLimiterService>();

builder.Services.AddExceptionHandler<ErrorHandler>();
// Intentionally NOT calling AddProblemDetails(): ErrorHandler.TryHandleAsync
// always returns true, so ASP.NET Core's ProblemDetails writer would never
// run anyway — omitting the registration avoids a second (unused) error
// contract living in the app and keeps `{"error":"..."}` the only shape.

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

// Order mirrors Rust's middleware stack (src/http/mod.rs): request id ->
// trusted-proxy-aware client IP/scheme -> security headers -> served-by ->
// compression -> IP rate limiting -> routed endpoints. Per-user rate
// limiting and ban checks run inside the auth endpoint filters (see
// Http/Auth/AuthEndpointFilter.cs) since that's where AuthUser is resolved.
app.UseExceptionHandler();
app.UseMiddleware<RequestIdMiddleware>();
app.UseMiddleware<RequestContextMiddleware>();
app.UseMiddleware<SecurityHeadersMiddleware>();
app.UseMiddleware<ServedByMiddleware>();
app.UseResponseCompression();
app.UseMiddleware<IpRateLimitMiddleware>();

app.MapHealthEndpoints();

var v1 = app.MapGroup("/v1");
v1.MapGroup("/auth").MapAuthEndpoints();
v1.MapGroup("/users").MapUserEndpoints();
v1.MapGroup("/posts").MapPostEndpoints();
v1.MapGroup("/feed").MapFeedEndpoints();
v1.MapGroup("/media").MapMediaEndpoints();

// Listen address: ASPNETCORE_URLS (set by the deployment environment, e.g.
// Kubernetes) is the source of truth; AppConfig.HttpAddr is read for parity
// with the Rust/Spring stacks' env vars but intentionally not wired to
// app.Urls here. See dotnet/README.md.
app.Run();

public partial class Program;
