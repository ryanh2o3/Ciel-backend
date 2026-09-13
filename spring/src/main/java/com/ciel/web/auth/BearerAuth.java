package com.ciel.web.auth;

import java.util.Optional;

/**
 * Shared {@code Authorization: Bearer <token>} header parsing, used by
 * {@link AuthUserResolver}, {@code PostController#optionalViewer}, and the
 * rate-limiting filters so the exact same parsing rules apply everywhere.
 */
public final class BearerAuth {

    private static final String PREFIX = "Bearer ";

    private BearerAuth() {}

    /** Returns the trimmed token, or empty if the header is missing/not a Bearer token. */
    public static Optional<String> extractToken(String authorizationHeader) {
        if (authorizationHeader == null || !authorizationHeader.regionMatches(true, 0, PREFIX, 0, PREFIX.length())) {
            return Optional.empty();
        }
        return Optional.of(authorizationHeader.substring(PREFIX.length()).trim());
    }
}
