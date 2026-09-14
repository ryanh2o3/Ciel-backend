package com.ciel.config;

import lombok.Data;
import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.boot.context.properties.NestedConfigurationProperty;

/**
 * Binds the {@code ciel.*} properties in {@code application.yml} (env-var driven).
 * Registered via {@code @ConfigurationPropertiesScan} on {@code CielApplication}.
 */
@ConfigurationProperties(prefix = "ciel")
@Data
public class AppProperties {

    /** Value reported via the {@code X-Ciel-Served-By} response header. */
    private String servedBy = "spring";

    /** Optional shared secret for moderation endpoints (x-admin-token header). Unused by this parity subset. */
    private String adminToken;

    /**
     * Per-IP signup limit (requests/day) used by {@link com.ciel.web.IpRateLimitFilter}.
     * Matches Rust's {@code IP_SIGNUP_RATE_LIMIT} (see {@code src/config/mod.rs}).
     */
    private long ipSignupRateLimit = 3;

    /**
     * Comma-separated CIDRs whose peers may set {@code X-Forwarded-For} /
     * {@code X-Forwarded-Proto} (env: {@code TRUSTED_PROXY_CIDRS}). Empty means
     * forwarded headers are ignored for HTTPS enforcement (Rust parity).
     */
    private String trustedProxyCidrs = "";

    @NestedConfigurationProperty
    private Paseto paseto = new Paseto();

    @NestedConfigurationProperty
    private S3 s3 = new S3();

    @NestedConfigurationProperty
    private Queue queue = new Queue();

    @NestedConfigurationProperty
    private Upload upload = new Upload();

    @Data
    public static class Paseto {
        /** Base64 of 32 raw bytes. */
        private String accessKey;
        /** Base64 of 32 raw bytes. */
        private String refreshKey;
        private long accessTtlMinutes = 15;
        private long refreshTtlDays = 30;
    }

    @Data
    public static class S3 {
        private String endpoint;
        private String publicEndpoint;
        private String region = "fr-par";
        private String bucket;
        private String accessKey;
        private String secretKey;
        private boolean forcePathStyle = true;
    }

    @Data
    public static class Queue {
        private String endpoint;
        private String region = "fr-par";
        private String name;
        private String accessKey;
        private String secretKey;
    }

    @Data
    public static class Upload {
        private long urlTtlSeconds = 900;
        private long maxBytes = 10_485_760L;
    }
}
