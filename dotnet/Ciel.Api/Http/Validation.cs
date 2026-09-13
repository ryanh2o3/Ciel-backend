using System.Text.RegularExpressions;
using Ciel.Api.Http.Errors;

namespace Ciel.Api.Http;

public static partial class Validation
{
    public const int MaxEmailLen = 254;
    public const int MaxDisplayNameLen = 50;
    public const int MaxBioLen = 500;
    public const int MaxPasswordLen = 128;
    public const int MaxCaptionLen = 2200;

    [GeneratedRegex("^[a-zA-Z0-9_]{3,30}$")]
    private static partial Regex HandleRegex();

    public static void ValidateHandle(string handle)
    {
        if (!HandleRegex().IsMatch(handle))
        {
            throw ApiException.BadRequest("handle can only contain letters, numbers, and underscores");
        }
    }

    public static void ValidateSignup(
        string handle,
        string email,
        string displayName,
        string? bio,
        string password,
        string inviteCode)
    {
        ValidateHandle(handle);

        if (string.IsNullOrWhiteSpace(email))
        {
            throw ApiException.BadRequest("email cannot be empty");
        }

        email = email.Trim();
        if (email.Length > MaxEmailLen)
        {
            throw ApiException.BadRequest("email must be at most 254 characters");
        }

        var parts = email.Split('@');
        if (parts.Length != 2 || parts[0].Length == 0 || !parts[1].Contains('.'))
        {
            throw ApiException.BadRequest("invalid email format");
        }

        if (string.IsNullOrWhiteSpace(displayName))
        {
            throw ApiException.BadRequest("display_name cannot be empty");
        }

        if (displayName.Length > MaxDisplayNameLen)
        {
            throw ApiException.BadRequest("display_name must be at most 50 characters");
        }

        if (bio is not null && bio.Length > MaxBioLen)
        {
            throw ApiException.BadRequest("bio must be at most 500 characters");
        }

        if (password.Trim().Length < 8)
        {
            throw ApiException.BadRequest("password must be at least 8 characters");
        }

        if (password.Length > MaxPasswordLen)
        {
            throw ApiException.BadRequest("password must be at most 128 characters");
        }

        if (!password.Any(char.IsUpper))
        {
            throw ApiException.BadRequest("password must contain at least one uppercase letter");
        }

        if (!password.Any(char.IsLower))
        {
            throw ApiException.BadRequest("password must contain at least one lowercase letter");
        }

        if (!password.Any(char.IsDigit))
        {
            throw ApiException.BadRequest("password must contain at least one digit");
        }

        if (string.IsNullOrWhiteSpace(inviteCode))
        {
            throw ApiException.BadRequest("invite_code is required");
        }
    }

    public static int ParseLimit(int? limit)
    {
        var value = limit ?? 30;
        if (value is < 1 or > 200)
        {
            throw ApiException.BadRequest("limit must be between 1 and 200");
        }

        return value;
    }
}
