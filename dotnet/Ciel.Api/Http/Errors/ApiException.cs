namespace Ciel.Api.Http.Errors;

public sealed class ApiException : Exception
{
    public int StatusCode { get; }
    public IReadOnlyDictionary<string, string> Headers { get; }

    private ApiException(int statusCode, string message, IReadOnlyDictionary<string, string>? headers = null)
        : base(message)
    {
        StatusCode = statusCode;
        Headers = headers ?? new Dictionary<string, string>();
    }

    public static ApiException BadRequest(string message) => new(StatusCodes.Status400BadRequest, message);
    public static ApiException Unauthorized(string message) => new(StatusCodes.Status401Unauthorized, message);
    public static ApiException Forbidden(string message) => new(StatusCodes.Status403Forbidden, message);
    public static ApiException NotFound(string message) => new(StatusCodes.Status404NotFound, message);
    public static ApiException Conflict(string message) => new(StatusCodes.Status409Conflict, message);
    public static ApiException Internal(string message) => new(StatusCodes.Status500InternalServerError, message);
}
