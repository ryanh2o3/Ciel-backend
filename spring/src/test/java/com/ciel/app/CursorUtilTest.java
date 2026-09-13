package com.ciel.app;

import com.ciel.web.error.ApiException;
import org.junit.jupiter.api.Test;

import java.time.OffsetDateTime;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/** Mirrors the round-trip / bad-cursor expectations of Rust's cursor pagination (see {@code src/http/handlers.rs}). */
class CursorUtilTest {

    @Test
    void roundTripsTimestampAndId() {
        OffsetDateTime ts = OffsetDateTime.parse("2026-01-01T12:34:56.789Z");
        UUID id = UUID.randomUUID();

        String encoded = CursorUtil.encode(ts, id);
        var parsed = CursorUtil.parse(encoded);

        assertThat(parsed).isPresent();
        assertThat(parsed.get().timestamp()).isEqualTo(ts);
        assertThat(parsed.get().id()).isEqualTo(id);
    }

    @Test
    void nullOrBlankCursorIsEmpty() {
        assertThat(CursorUtil.parse(null)).isEmpty();
        assertThat(CursorUtil.parse("")).isEmpty();
        assertThat(CursorUtil.parse("   ")).isEmpty();
    }

    @Test
    void cursorWithoutSlashIsBadRequest() {
        assertThatThrownBy(() -> CursorUtil.parse("not-a-cursor"))
                .isInstanceOf(ApiException.class)
                .hasMessage("invalid cursor");
    }

    @Test
    void cursorWithInvalidTimestampIsBadRequest() {
        assertThatThrownBy(() -> CursorUtil.parse("not-a-date/" + UUID.randomUUID()))
                .isInstanceOf(ApiException.class)
                .hasMessage("invalid cursor");
    }

    @Test
    void cursorWithInvalidUuidIsBadRequest() {
        String tsPart = CursorUtil.encode(OffsetDateTime.now(), UUID.randomUUID()).split("/")[0];
        assertThatThrownBy(() -> CursorUtil.parse(tsPart + "/not-a-uuid"))
                .isInstanceOf(ApiException.class)
                .hasMessage("invalid cursor");
    }

    @Test
    void encodeOptionalReturnsNullForNullCursor() {
        assertThat(CursorUtil.encodeOptional(null)).isNull();
    }
}
