package com.ciel.app;

import com.ciel.web.error.ApiException;

import java.time.OffsetDateTime;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.Optional;
import java.util.UUID;

public final class CursorUtil {

    private static final DateTimeFormatter RFC3339 = DateTimeFormatter.ISO_OFFSET_DATE_TIME;

    private CursorUtil() {}

    public record Cursor(OffsetDateTime timestamp, UUID id) {}

    public static Optional<Cursor> parse(String cursor) {
        if (cursor == null || cursor.isBlank()) {
            return Optional.empty();
        }
        int slash = cursor.indexOf('/');
        if (slash <= 0 || slash >= cursor.length() - 1) {
            throw ApiException.badRequest("invalid cursor");
        }
        String ts = cursor.substring(0, slash);
        String idPart = cursor.substring(slash + 1);
        try {
            OffsetDateTime timestamp = OffsetDateTime.parse(ts, RFC3339);
            UUID id = UUID.fromString(idPart);
            return Optional.of(new Cursor(timestamp, id));
        } catch (DateTimeParseException | IllegalArgumentException e) {
            throw ApiException.badRequest("invalid cursor");
        }
    }

    public static String encode(OffsetDateTime timestamp, UUID id) {
        return RFC3339.format(timestamp) + "/" + id;
    }

    public static String encodeOptional(Cursor cursor) {
        return cursor == null ? null : encode(cursor.timestamp(), cursor.id());
    }
}
