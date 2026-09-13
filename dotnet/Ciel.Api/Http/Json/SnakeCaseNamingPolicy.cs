using System.Text;
using System.Text.Json;

namespace Ciel.Api.Http.Json;

public sealed class SnakeCaseNamingPolicy : JsonNamingPolicy
{
    public static SnakeCaseNamingPolicy Instance { get; } = new();

    public override string ConvertName(string name)
    {
        if (string.IsNullOrEmpty(name))
        {
            return name;
        }

        var buffer = new StringBuilder(name.Length + 5);
        for (var i = 0; i < name.Length; i++)
        {
            var c = name[i];
            if (char.IsUpper(c))
            {
                if (i > 0)
                {
                    buffer.Append('_');
                }

                buffer.Append(char.ToLowerInvariant(c));
            }
            else
            {
                buffer.Append(c);
            }
        }

        return buffer.ToString();
    }
}
