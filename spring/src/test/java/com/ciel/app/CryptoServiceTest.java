package com.ciel.app;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Covers {@link CryptoService}'s Argon2 hash/verify round trip (offloaded to
 * a dedicated executor — see item 8 of the parity review) and the SHA-256
 * hex helper used for refresh-token hashing.
 */
class CryptoServiceTest {

    private final CryptoService crypto = new CryptoService();

    @AfterEach
    void tearDown() {
        crypto.shutdown();
    }

    @Test
    void hashAndVerifyRoundTrip() {
        String hash = crypto.hashPassword("correct horse battery staple");
        assertThat(crypto.verifyPassword("correct horse battery staple", hash)).isTrue();
    }

    @Test
    void verifyRejectsWrongPassword() {
        String hash = crypto.hashPassword("correct horse battery staple");
        assertThat(crypto.verifyPassword("wrong password", hash)).isFalse();
    }

    @Test
    void verifyRejectsMalformedHash() {
        assertThat(crypto.verifyPassword("anything", "not-a-real-hash")).isFalse();
    }

    @Test
    void sha256HexIsDeterministicAndHex() {
        String a = crypto.sha256Hex("some-refresh-token");
        String b = crypto.sha256Hex("some-refresh-token");

        assertThat(a).isEqualTo(b);
        assertThat(a).hasSize(64);
        assertThat(a).matches("[0-9a-f]{64}");
    }
}
