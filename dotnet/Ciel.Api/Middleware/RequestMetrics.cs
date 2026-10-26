using System.Collections.Concurrent;
using System.Diagnostics;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace Ciel.Api.Middleware;

public sealed class RequestMetrics
{
    private sealed record Key(string Method, string Path, int Status);
    private sealed class Sample
    {
        public long Count;
        public long Ticks;
    }
    private readonly ConcurrentDictionary<Key, Sample> samples = new();
    public async Task Record(HttpContext context, RequestDelegate next)
    {
        long start = Stopwatch.GetTimestamp();
        try
        {
            await next(context);
        }
        finally
        {
            var route = (context.GetEndpoint() as RouteEndpoint)?.RoutePattern.RawText ?? "unmatched";
            route = Regex.Replace(route, @"\{([^}:]+)(?::[^}]+)?\}", ":$1");
            var sample = samples.GetOrAdd(new(context.Request.Method, route, context.Response.StatusCode),
                                          _ => new());
            Interlocked.Increment(ref sample.Count);
            Interlocked.Add(ref sample.Ticks, Stopwatch.GetTimestamp() - start);
        }
    }
    public string Render()
    {
        var text =
            new StringBuilder("# TYPE http_requests_total counter\n# TYPE http_request_duration_seconds summary\n");
        foreach (var (key, sample) in samples)
        {
            string labels = $"{{method=\"{key.Method}\",path=\"{key.Path}\",status=\"{key.Status}\"}}";
            long count = Interlocked.Read(ref sample.Count);
            text.Append("http_requests_total").Append(labels).Append(' ').Append(count).Append('\n');
            text.Append("http_request_duration_seconds_count").Append(labels).Append(' ').Append(count).Append('\n');
            text.Append("http_request_duration_seconds_sum")
                .Append(labels)
                .Append(' ')
                .Append(((double)Interlocked.Read(ref sample.Ticks) / Stopwatch.Frequency)
                            .ToString(CultureInfo.InvariantCulture))
                .Append('\n');
        }
        return text.ToString();
    }
}
