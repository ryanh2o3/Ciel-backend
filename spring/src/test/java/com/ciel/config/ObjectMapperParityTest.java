package com.ciel.config;

import com.ciel.domain.User;
import com.ciel.web.dto.ListResponse;
import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.fasterxml.jackson.databind.SerializationFeature;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.List;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.*;

/** Wire-format parity for the HTTP ObjectMapper (same settings as {@link InfraConfig}). */
class ObjectMapperParityTest {

    private ObjectMapper mapper;

    @BeforeEach
    void setUp() {
        mapper = new ObjectMapper();
        mapper.registerModule(new JavaTimeModule());
        mapper.setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE);
        mapper.disable(SerializationFeature.WRITE_DATES_AS_TIMESTAMPS);
        mapper.setSerializationInclusion(JsonInclude.Include.ALWAYS);
    }

    @Test
    void datesAreRfc3339Strings() throws Exception {
        User user = User.builder()
                .id(UUID.randomUUID())
                .handle("demo")
                .email("demo@example.com")
                .displayName("Demo")
                .bio(null)
                .avatarUrl(null)
                .createdAt(OffsetDateTime.of(2024, 1, 15, 12, 0, 0, 0, ZoneOffset.UTC))
                .build();

        String json = mapper.writeValueAsString(user);

        assertTrue(
                json.contains("\"created_at\":\"2024-01-15T12:00:00Z\"")
                        || json.contains("\"created_at\":\"2024-01-15T12:00:00.000Z\""),
                json);
        assertFalse(json.matches("(?s).*\"created_at\":\\d+.*"), json);
        assertTrue(json.contains("\"bio\":null"), json);
        assertTrue(json.contains("\"avatar_url\":null"), json);
        assertFalse(json.contains("avatar_key"), json);
    }

    @Test
    void listResponseIncludesNullNextCursor() throws Exception {
        String json = mapper.writeValueAsString(new ListResponse<>(List.of("a"), null));
        assertTrue(json.contains("\"next_cursor\":null"), json);
        assertTrue(json.contains("\"items\""), json);
    }
}
