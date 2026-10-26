using System.Reflection;
using System.Runtime.CompilerServices;
using Ciel.Api.Http.Errors;

namespace Ciel.Api.Http;

/// <summary>System.Text.Json enforces missing required members; also reject explicit nulls.</summary>
public sealed class RequiredRequestFields : IEndpointFilter
{
    public ValueTask<object?> InvokeAsync(EndpointFilterInvocationContext context, EndpointFilterDelegate next)
    {
        foreach (var argument in context.Arguments)
        {
            if (argument?.GetType().Namespace != "Ciel.Api.Http.Dtos") continue;
            foreach (var property in argument.GetType().GetProperties())
                if (property.IsDefined(typeof(RequiredMemberAttribute)) && property.GetValue(argument) is null)
                    throw ApiException.BadRequest("invalid request body");
        }
        return next(context);
    }
}
