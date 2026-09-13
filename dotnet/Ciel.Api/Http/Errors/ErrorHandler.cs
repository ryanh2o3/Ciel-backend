using System.Text.Json;
using Ciel.Api.Http.Json;
using Microsoft.AspNetCore.Diagnostics;
using Microsoft.AspNetCore.Http;

namespace Ciel.Api.Http.Errors;

/// <summary>
/// Central exception -&gt; HTTP response mapping, matching Rust's <c>AppError</c>
/// (src/http/error.rs): every response body is <c>{"error":"..."}</c> (snake_case
/// via <see cref="CielJsonOptions"/>), and truly unexpected exceptions collapse to
/// a 500 with the generic message "internal error" (never leaking exception
/// details to clients). This handler always returns <c>true</c> so ASP.NET Core's
/// built-in ProblemDetails machinery never gets a chance to produce a competing
/// response shape.
/// </summary>
public sealed class ErrorHandler(ILogger<ErrorHandler> logger) : IExceptionHandler
{
    private static readonly JsonSerializerOptions JsonOptions = CielJsonOptions.Create();

    public async ValueTask<bool> TryHandleAsync(
        HttpContext httpContext,
        Exception exception,
        CancellationToken cancellationToken)
    {
        var (statusCode, message, headers) = Classify(exception);

        if (statusCode >= 500)
        {
            logger.LogError(
                exception,
                "unhandled exception while processing {Method} {Path} -> {StatusCode}",
                httpContext.Request.Method,
                httpContext.Request.Path,
                statusCode);
        }
        else
        {
            logger.LogWarning(
                exception,
                "request error while processing {Method} {Path} -> {StatusCode}: {Message}",
                httpContext.Request.Method,
                httpContext.Request.Path,
                statusCode,
                message);
        }

        foreach (var (name, value) in headers)
        {
            httpContext.Response.Headers[name] = value;
        }

        httpContext.Response.StatusCode = statusCode;
        await httpContext.Response.WriteAsJsonAsync(
            new ErrorBody { Error = message },
            JsonOptions,
            cancellationToken);
        return true;
    }

    private static (int StatusCode, string Message, IReadOnlyDictionary<string, string> Headers) Classify(
        Exception exception) => exception switch
    {
        ApiException apiException => (apiException.StatusCode, apiException.Message, apiException.Headers),

        // Malformed request line/headers, unsupported content type, request body
        // too large, etc. Axum/Rust maps these to 400 as well.
        BadHttpRequestException badRequest => (StatusCodes.Status400BadRequest, BadRequestMessage(badRequest), Empty),

        // Minimal API JSON body binding failures surface as JsonException.
        JsonException => (StatusCodes.Status400BadRequest, "invalid request body", Empty),

        _ => (StatusCodes.Status500InternalServerError, "internal error", Empty),
    };

    private static string BadRequestMessage(BadHttpRequestException exception) =>
        exception.StatusCode == StatusCodes.Status413PayloadTooLarge
            ? "request body too large"
            : "invalid request";

    private static readonly IReadOnlyDictionary<string, string> Empty = new Dictionary<string, string>();

    private sealed class ErrorBody
    {
        public required string Error { get; set; }
    }
}
