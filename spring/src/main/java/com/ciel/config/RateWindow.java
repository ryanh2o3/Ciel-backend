package com.ciel.config;

/** Time window for rate limiting. Mirrors Rust's {@code config::rate_limits::RateWindow}. */
public enum RateWindow {
    MINUTE(60),
    HOUR(3600),
    DAY(86400);

    private final long seconds;

    RateWindow(long seconds) {
        this.seconds = seconds;
    }

    public long seconds() {
        return seconds;
    }
}
