package com.ciel.app;

import com.ciel.config.AppProperties;
import com.ciel.web.error.ApiException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.paseto4j.commons.SecretKey;
import org.paseto4j.commons.Version;
import org.paseto4j.version4.Paseto;
import org.springframework.stereotype.Component;

import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.time.temporal.ChronoUnit;
import java.util.Base64;
import java.util.Optional;
import java.util.UUID;

@Component
public class PasetoService {

    private final byte[] accessKey;
    private final byte[] refreshKey;
    private final long accessTtlMinutes;
    private final long refreshTtlDays;
    private final ObjectMapper mapper;

    public PasetoService(AppProperties props, ObjectMapper mapper) {
        this.accessKey = decodeKey(props.getPaseto().getAccessKey(), "PASETO_ACCESS_KEY");
        this.refreshKey = decodeKey(props.getPaseto().getRefreshKey(), "PASETO_REFRESH_KEY");
        this.accessTtlMinutes = props.getPaseto().getAccessTtlMinutes();
        this.refreshTtlDays = props.getPaseto().getRefreshTtlDays();
        this.mapper = mapper;
    }

    public record IssuedAccess(String token, OffsetDateTime expiresAt) {}

    public record IssuedRefresh(String token, OffsetDateTime expiresAt) {}

    public IssuedAccess mintAccess(UUID userId) {
        Instant now = Instant.now();
        Instant exp = now.plus(accessTtlMinutes, ChronoUnit.MINUTES);
        String payload = claimsJson(userId, null, "access", now, exp);
        String token = Paseto.encrypt(new SecretKey(accessKey, Version.V4), payload, "");
        return new IssuedAccess(token, OffsetDateTime.ofInstant(exp, ZoneOffset.UTC));
    }

    public IssuedRefresh mintRefresh(UUID userId, UUID refreshId) {
        Instant now = Instant.now();
        Instant exp = now.plus(refreshTtlDays, ChronoUnit.DAYS);
        String payload = claimsJson(userId, refreshId, "refresh", now, exp);
        String token = Paseto.encrypt(new SecretKey(refreshKey, Version.V4), payload, "");
        return new IssuedRefresh(token, OffsetDateTime.ofInstant(exp, ZoneOffset.UTC));
    }

    public Optional<UUID> verifyAccess(String token) {
        return decrypt(token, accessKey, "access").map(n -> UUID.fromString(n.get("sub").asText()));
    }

    public Optional<RefreshClaims> verifyRefresh(String token) {
        return decrypt(token, refreshKey, "refresh")
                .map(n -> new RefreshClaims(
                        UUID.fromString(n.get("sub").asText()),
                        UUID.fromString(n.get("jti").asText())));
    }

    public record RefreshClaims(UUID userId, UUID refreshId) {}

    private Optional<JsonNode> decrypt(String token, byte[] key, String expectedTyp) {
        try {
            String json = Paseto.decrypt(new SecretKey(key, Version.V4), token, "");
            JsonNode node = mapper.readTree(json);
            if (!"ciel".equals(text(node, "iss")) || !"ciel".equals(text(node, "aud"))) {
                return Optional.empty();
            }
            if (!expectedTyp.equals(text(node, "typ"))) {
                return Optional.empty();
            }
            Instant exp = Instant.parse(node.get("exp").asText());
            if (Instant.now().isAfter(exp)) {
                return Optional.empty();
            }
            return Optional.of(node);
        } catch (Exception e) {
            return Optional.empty();
        }
    }

    private String claimsJson(UUID userId, UUID jti, String typ, Instant iat, Instant exp) {
        try {
            ObjectNode n = mapper.createObjectNode();
            n.put("iss", "ciel");
            n.put("aud", "ciel");
            n.put("sub", userId.toString());
            n.put("iat", iat.toString());
            n.put("exp", exp.toString());
            n.put("typ", typ);
            if (jti != null) {
                n.put("jti", jti.toString());
            }
            return mapper.writeValueAsString(n);
        } catch (Exception e) {
            throw new IllegalStateException("failed to build paseto claims", e);
        }
    }

    private static String text(JsonNode n, String field) {
        JsonNode v = n.get(field);
        return v == null || v.isNull() ? null : v.asText();
    }

    private static byte[] decodeKey(String b64, String name) {
        if (b64 == null || b64.isBlank()) {
            throw ApiException.internal("missing " + name);
        }
        byte[] raw = Base64.getDecoder().decode(b64.trim().getBytes(StandardCharsets.UTF_8));
        if (raw.length != 32) {
            throw ApiException.internal(name + " must decode to 32 bytes");
        }
        return raw;
    }
}
