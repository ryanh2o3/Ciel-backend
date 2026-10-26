using Npgsql;
using Ciel.Api.Http.Errors;
using System.Text.Json;
using System.Globalization;

namespace Ciel.Api.Services;

public sealed class Row : Dictionary<string, object?>
{
    public object? get(string key) => this.GetValueOrDefault(key);
    public void put(string key, object? value) => this[key] = value;
    public object? remove(string key)
    {
        var value = get(key);
        Remove(key);
        return value;
    }
}

/// <summary>Scoped query access; transactions use one connection for row locks and writes.</summary>
public sealed class ApiData(NpgsqlDataSource source)
{
    private NpgsqlConnection? connection;
    private NpgsqlTransaction? currentTransaction;
    public T transaction<T>(Func<T> work)
    {
        if (currentTransaction is not null)
            return work();
        using var conn = source.OpenConnection();
        using var tx = conn.BeginTransaction();
        connection = conn;
        currentTransaction = tx;
        try
        {
            var value = work();
            tx.Commit();
            return value;
        }
        finally
        {
            connection = null;
            currentTransaction = null;
        }
    }
    public void transaction(Action work) => transaction(() =>
                                                        {
                                                            work();
                                                            return true;
                                                        });
    private NpgsqlCommand command(NpgsqlConnection conn, string sql, object?[] args)
    {
        var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        cmd.Transaction = currentTransaction;
        foreach (var value in args)
            cmd.Parameters.Add(new NpgsqlParameter { Value = value ?? DBNull.Value });
        return cmd;
    }
    public List<Row> rows(string sql, params object?[] args)
    {
        using var owned = connection is null ? source.OpenConnection() : null;
        using var cmd = command(connection ?? owned!, sql, args);
        using var reader = cmd.ExecuteReader();
        var result = new List<Row>();
        while (reader.Read())
        {
            var row = new Row();
            for (int i = 0; i < reader.FieldCount; i++)
            {
                object? value = reader.IsDBNull(i) ? null : reader.GetValue(i);
                if (value is DateTime time)
                    value = time.ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss.FFFFFFF'Z'",
                                                            CultureInfo.InvariantCulture);
                else if (value is DateTimeOffset offset)
                    value = offset.ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss.FFFFFFF'Z'",
                                                              CultureInfo.InvariantCulture);
                else if (value is string json && reader.GetDataTypeName(i) is "json" or "jsonb")
                    value = JsonSerializer.Deserialize<JsonElement>(json);
                row.put(reader.GetName(i), value);
            }
            result.Add(row);
        }
        return result;
    }
    public Row? one(string sql, params object?[] args) => rows(sql, args).FirstOrDefault();
    public Row single(string sql, params object?[] args) => one(sql, args) ??
                                                            throw ApiException.Internal("internal server error");
    public Row required(string sql, string error, params object?[] args) => one(sql, args) ??
                                                                            throw ApiException.NotFound(error);
    public int execute(string sql, params object?[] args)
    {
        using var owned = connection is null ? source.OpenConnection() : null;
        using var cmd = command(connection ?? owned!, sql, args);
        return cmd.ExecuteNonQuery();
    }
    public static Row obj(params object?[] entries)
    {
        var row = new Row();
        for (int i = 0; i < entries.Length; i += 2)
            row[(string)entries[i]!] = entries[i + 1];
        return row;
    }
    public static int integer(Row row, string key) => Convert.ToInt32(row.get(key), CultureInfo.InvariantCulture);
    public static Guid uuid(Row row, string key) => Guid.Parse(row.get(key)!.ToString()!);
    public static string? text(Row row, string key) => row.get(key)?.ToString();
    public static DateTimeOffset instant(Row row, string key) => DateTimeOffset.Parse(text(row, key)!,
                                                                                      CultureInfo.InvariantCulture);
    public static int limit(int? value)
    {
        int n = value ?? 30;
        if (n < 1 || n > 200)
            throw ApiException.BadRequest("limit must be between 1 and 200");
        return n;
    }
    public static object?[] cursorArgs(string? cursor, int limit, params object?[] prefix)
    {
        var args = prefix.ToList();
        if (cursor is not null)
        {
            var parts = cursor.Split('/', 2);
            if (parts.Length != 2 ||
                !DateTimeOffset.TryParse(parts[0], CultureInfo.InvariantCulture, DateTimeStyles.None, out var time) ||
                !Guid.TryParse(parts[1], out var id))
                throw ApiException.BadRequest("invalid cursor");
            args.Add(time.ToUniversalTime());
            args.Add(id);
        }
        args.Add((long)limit + 1);
        return args.ToArray();
    }
    public static Row page(List<Row> rows, int limit, string time, string id)
    {
        string? next = null;
        if (rows.Count > limit)
        {
            var extra = rows[^1];
            rows.RemoveAt(rows.Count - 1);
            next = extra.get(time) + "/" + extra.get(id);
        }
        return obj("items", rows, "next_cursor", next);
    }
}
