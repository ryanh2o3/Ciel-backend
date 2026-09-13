package com.ciel.web;

import jakarta.servlet.http.HttpServletResponse;

import java.io.IOException;

/** Shared 429 / 500 response writers for {@link IpRateLimitFilter} and {@link UserRateLimitFilter}. */
final class RateLimitResponses {

    private RateLimitResponses() {}

    static void writeRateLimited(HttpServletResponse response, String action, long limit) throws IOException {
        response.setStatus(429); // 429 Too Many Requests (not in HttpServletResponse.SC_* constants)
        response.setContentType("application/json");
        response.setHeader("X-RateLimit-Limit", Long.toString(limit));
        response.setHeader("X-RateLimit-Remaining", "0");
        response.setHeader("Retry-After", "60");
        response.getWriter().write(
                "{\"error\":\"Rate limit exceeded for action: " + action + ". Please try again later.\"}");
    }

    static void writeInternalError(HttpServletResponse response) throws IOException {
        response.setStatus(HttpServletResponse.SC_INTERNAL_SERVER_ERROR);
        response.setContentType("application/json");
        response.getWriter().write("{\"error\":\"failed to check rate limit\"}");
    }
}
