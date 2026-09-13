namespace Ciel.Api.Http.Dtos;

public sealed class LoginRequest
{
    public required string Email { get; set; }
    public required string Password { get; set; }
}

public sealed class RefreshRequest
{
    public required string RefreshToken { get; set; }
}

public sealed class RevokeRequest
{
    public required string RefreshToken { get; set; }
}

public sealed class AuthTokenResponse
{
    public required string AccessToken { get; set; }
    public required string RefreshToken { get; set; }
    public DateTimeOffset AccessExpiresAt { get; set; }
    public DateTimeOffset RefreshExpiresAt { get; set; }
}

public sealed class CreateUserRequest
{
    public required string Handle { get; set; }
    public required string Email { get; set; }
    public required string DisplayName { get; set; }
    public string? Bio { get; set; }
    public string? AvatarKey { get; set; }
    public required string Password { get; set; }
    public required string InviteCode { get; set; }
}

public sealed class CreatePostRequest
{
    public List<Guid> MediaIds { get; set; } = [];
    public string? Caption { get; set; }
}

public sealed class CommentRequest
{
    public required string Body { get; set; }
}

public sealed class LikeResponse
{
    public bool Created { get; set; }
}

public sealed class UploadRequest
{
    public required string ContentType { get; set; }
    public long Bytes { get; set; }
}

public sealed class ListResponse<T>
{
    public List<T> Items { get; set; } = [];
    public string? NextCursor { get; set; }
}

public sealed class HealthResponse
{
    public required string Status { get; set; }
}

public sealed class PaginationQuery
{
    public int? Limit { get; set; }
    public string? Cursor { get; set; }
}
