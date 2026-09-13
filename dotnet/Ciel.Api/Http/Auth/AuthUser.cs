namespace Ciel.Api.Http.Auth;

public sealed record AuthUser(Guid UserId);

public static class AuthHttpContext
{
    public const string ItemKey = "CielAuthUser";

    public static AuthUser Require(HttpContext context)
    {
        if (context.Items.TryGetValue(ItemKey, out var value) && value is AuthUser user)
        {
            return user;
        }

        throw Errors.ApiException.Unauthorized("missing Authorization header");
    }

    public static Guid? OptionalUserId(HttpContext context)
    {
        if (context.Items.TryGetValue(ItemKey, out var value) && value is AuthUser user)
        {
            return user.UserId;
        }

        return null;
    }
}
