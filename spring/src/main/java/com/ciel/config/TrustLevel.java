package com.ciel.config;

/**
 * Trust levels for users, determining their rate limits and privileges.
 * Mirrors Rust's {@code config::rate_limits::TrustLevel}. Numeric values must
 * match the {@code user_trust_scores.trust_level} column written by both
 * implementations.
 */
public enum TrustLevel {
    NEW(0),      // 0-7 days, < 5 posts
    BASIC(1),    // 7-30 days, 5+ posts, no violations
    TRUSTED(2),  // 30+ days, 50+ posts, active engagement
    VERIFIED(3); // Manual verification or high trust score

    private final int value;

    TrustLevel(int value) {
        this.value = value;
    }

    public int asInt() {
        return value;
    }

    public static TrustLevel fromInt(int value) {
        return switch (value) {
            case 1 -> BASIC;
            case 2 -> TRUSTED;
            case 3 -> VERIFIED;
            default -> NEW;
        };
    }
}
