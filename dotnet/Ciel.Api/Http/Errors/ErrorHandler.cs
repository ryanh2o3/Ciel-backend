using System.Text.Json;
using Ciel.Api.Http.Json;
using Microsoft.AspNetCore.Diagnostics;

namespace Ciel.Api.Http.Errors;

public sealed class ErrorHandler : IExceptionHandler
{
    private static readonly JsonSerializerOptions JsonOptions = CielJsonOptions.Create();

    public async ValueTask<bool> TryHandleAsync(
        HttpContext httpContext,
        Exception exception,
        CancellationToken cancellationToken)
    {
        if (exception is not ApiException apiException)
        {
            return false;
        }

        foreach (var (name, value) in apiException.Headers)
        {
            httpContext.Response.Headers[name] = value;
        }

        httpContext.Response.StatusCode = apiException.StatusCode;
        await httpContext.Response.WriteAsJsonAsync(
            new ErrorBody { Error = apiException.Message },
            JsonOptions,
            cancellationToken);
        return true;
    }

    private sealed class ErrorBody
    {
        public required string Error { get; set; }
    }
}
