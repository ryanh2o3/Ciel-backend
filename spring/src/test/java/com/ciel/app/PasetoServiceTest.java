package com.ciel.app;

import com.ciel.config.AppProperties;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.security.SecureRandom;
import java.time.OffsetDateTime;
import java.util.Base64;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Mint/verify coverage for {@link PasetoService}, mirroring the guarantees
 * Rust's PASETO layer relies on (see {@code src/app/auth.rs}): access and
 * refresh tokens round-trip, a token of the wrong {@code typ} is rejected,
 * and expired tokens are rejected.
 */
class PasetoServiceTest {

    private static PasetoService newService(long accessTtlMinutes, long refreshTtlDays) {
        AppProperties props = new AppProperties();
        props.getPaseto().setAccessKey(randomKeyBase64());
        props.getPaseto().setRefreshKey(randomKeyBase64());
        props.getPaseto().setAccessTtlMinutes(accessTtlMinutes);
        props.getPaseto().setRefreshTtlDays(refreshTtlDays);
        return new PasetoService(props, new ObjectMapper());
    }

    private static String randomKeyBase64() {
        byte[] key = new byte[32];
        new SecureRandom().nextBytes(key);
        return Base64.getEncoder().encodeToString(key);
    }

    @Test
    void mintsAndVerifiesAccessToken() {
        PasetoService paseto = newService(15, 30);
        UUID userId = UUID.randomUUID();

        PasetoService.IssuedAccess access = paseto.mintAccess(userId);

        assertThat(paseto.verifyAccess(access.token())).contains(userId);
        assertThat(access.expiresAt()).isAfter(OffsetDateTime.now());
    }

    @Test
    void mintsAndVerifiesRefreshToken() {
        PasetoService paseto = newService(15, 30);
        UUID userId = UUID.randomUUID();
        UUID refreshId = UUID.randomUUID();

        PasetoService.IssuedRefresh refresh = paseto.mintRefresh(userId, refreshId);
        var verified = paseto.verifyRefresh(refresh.token());

        assertThat(verified).isPresent();
        assertThat(verified.get().userId()).isEqualTo(userId);
        assertThat(verified.get().refreshId()).isEqualTo(refreshId);
    }

    @Test
    void rejectsAccessTokenPresentedAsRefresh() {
        PasetoService paseto = newService(15, 30);
        PasetoService.IssuedAccess access = paseto.mintAccess(UUID.randomUUID());

        assertThat(paseto.verifyRefresh(access.token())).isEmpty();
    }

    @Test
    void rejectsRefreshTokenPresentedAsAccess() {
        PasetoService paseto = newService(15, 30);
        PasetoService.IssuedRefresh refresh = paseto.mintRefresh(UUID.randomUUID(), UUID.randomUUID());

        assertThat(paseto.verifyAccess(refresh.token())).isEmpty();
    }

    @Test
    void rejectsExpiredAccessToken() {
        // A zero-minute TTL mints a token whose `exp` is effectively "now",
        // so it is already expired by the time verifyAccess runs.
        PasetoService paseto = newService(0, 30);
        PasetoService.IssuedAccess access = paseto.mintAccess(UUID.randomUUID());

        assertThat(paseto.verifyAccess(access.token())).isEmpty();
    }

    @Test
    void rejectsTokenSignedWithADifferentKey() {
        PasetoService a = newService(15, 30);
        PasetoService b = newService(15, 30);
        PasetoService.IssuedAccess access = a.mintAccess(UUID.randomUUID());

        assertThat(b.verifyAccess(access.token())).isEmpty();
    }

    @Test
    void rejectsGarbageToken() {
        PasetoService paseto = newService(15, 30);
        assertThat(paseto.verifyAccess("not-a-real-token")).isEmpty();
    }
}
