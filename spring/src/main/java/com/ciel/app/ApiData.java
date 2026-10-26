package com.ciel.app;

import com.ciel.web.error.ApiException;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.sql.*;
import java.time.*;
import java.util.*;
import java.util.function.Supplier;
import java.util.regex.Pattern;
import org.postgresql.util.PGobject;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Component;
import org.springframework.transaction.PlatformTransactionManager;
import org.springframework.transaction.support.TransactionTemplate;

/** Parameterized queries and JSON-safe rows for the extended API services. */
@Component
public class ApiData {
  private final JdbcTemplate jdbc;
  private final TransactionTemplate transactions;
  private final ObjectMapper mapper;
  private static final Pattern PARAMETER = Pattern.compile("\\$(\\d+)");
  public ApiData(JdbcTemplate jdbc, PlatformTransactionManager manager, ObjectMapper mapper) {
    this.jdbc = jdbc;
    this.transactions = new TransactionTemplate(manager);
    this.mapper = mapper;
  }
  public <T> T transaction(Supplier<T> work) {
    return transactions.execute(status -> work.get());
  }
  private PreparedStatement prepare(Connection connection, String sql, Object[] args) throws SQLException {
    var matcher = PARAMETER.matcher(sql);
    List<Object> ordered = new ArrayList<>();
    while (matcher.find()) ordered.add(args[Integer.parseInt(matcher.group(1)) - 1]);
    var statement = connection.prepareStatement(matcher.replaceAll("?"));
    for (int i = 0; i < ordered.size(); i++) {
      Object value = ordered.get(i);
      if (value instanceof Instant instant)
        value = OffsetDateTime.ofInstant(instant, ZoneOffset.UTC);
      if (value instanceof UUID[] ids)
        value = connection.createArrayOf("uuid", ids);
      statement.setObject(i + 1, value);
    }
    return statement;
  }
  public List<Map<String, Object>> rows(String sql, Object... args) {
    return jdbc.query(c -> prepare(c, sql, args), (rs, index) -> {
      Map<String, Object> row = new LinkedHashMap<>();
      for (int i = 1; i <= rs.getMetaData().getColumnCount(); i++) {
        Object value = rs.getObject(i);
        if (value instanceof Timestamp time)
          value = time.toInstant().toString();
        else if (value instanceof java.sql.Array array)
          value = Arrays.asList((Object[]) array.getArray());
        else if (value instanceof PGobject pg) {
          try {
            value = mapper.readValue(pg.getValue(), Object.class);
          } catch (Exception ignored) {
            value = pg.getValue();
          }
        }
        row.put(rs.getMetaData().getColumnLabel(i), value);
      }
      return row;
    });
  }
  public Map<String, Object> one(String sql, Object... args) {
    var rows = rows(sql, args);
    return rows.isEmpty() ? null : rows.getFirst();
  }
  public Map<String, Object> required(String sql, String error, Object... args) {
    var row = one(sql, args);
    if (row == null)
      throw ApiException.notFound(error);
    return row;
  }
  public int execute(String sql, Object... args) {
    return jdbc.update(c -> prepare(c, sql, args));
  }
  public static Map<String, Object> object(Object... entries) {
    Map<String, Object> result = new LinkedHashMap<>();
    for (int i = 0; i < entries.length; i += 2) result.put((String) entries[i], entries[i + 1]);
    return result;
  }
  public static int integer(Map<String, Object> row, String key) {
    return ((Number) row.get(key)).intValue();
  }
  public static UUID uuid(Map<String, Object> row, String key) {
    return UUID.fromString(row.get(key).toString());
  }
  public static String text(Map<String, Object> row, String key) {
    Object v = row.get(key);
    return v == null ? null : v.toString();
  }
  public static Instant instant(Map<String, Object> row, String key) {
    return Instant.parse(text(row, key));
  }
  public static int limit(Integer value) {
    int n = value == null ? 30 : value;
    if (n < 1 || n > 200)
      throw ApiException.badRequest("limit must be between 1 and 200");
    return n;
  }
  public static Object[] cursorArgs(String cursor, int limit, Object... prefix) {
    List<Object> args = new ArrayList<>(Arrays.asList(prefix));
    if (cursor != null) {
      var parsed = CursorUtil.parse(cursor).orElseThrow(() -> ApiException.badRequest("invalid cursor"));
      args.add(parsed.timestamp());
      args.add(parsed.id());
    }
    args.add((long) limit + 1);
    return args.toArray();
  }
  public static Map<String, Object> page(List<Map<String, Object>> rows, int limit, String time, String id) {
    String next = null;
    if (rows.size() > limit) {
      var extra = rows.removeLast();
      next = extra.get(time) + "/" + extra.get(id);
    }
    return object("items", rows, "next_cursor", next);
  }
}
