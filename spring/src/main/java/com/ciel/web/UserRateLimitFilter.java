package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.RateLimiter;
import com.ciel.config.TrustLevel;
import com.ciel.web.auth.BearerAuth;
import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.core.Ordered;
import org.springframework.core.annotation.Order;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;

import java.io.IOException;
import java.util.Optional;
import java.util.UUID;

/**
 * Rate limiting for authenticated endpoints. Mirrors Rust's
 * {@code rate_limit_middleware} (see {@code src/http/middleware/rate_limit.rs}):
 * same action-per-path mapping (restricted to the actions this API exposes),
 * same Redis-backed quota via {@link RateLimiter}, same fail-closed behavior
 * on Redis errors, and the same 429 body / headers.
 */
@Component
@Order(Ordered.HIGHEST_PRECEDENCE + 3)
public class UserRateLimitFilter extends OncePerRequestFilter {

    private static final Logger log = LoggerFactory.getLogger(UserRateLimitFilter.class);

    private final RateLimiter rateLimiter;
    private final AuthService authService;
    private final JdbcTemplate jdbc;

    public UserRateLimitFilter(RateLimiter rateLimiter, AuthService authService, JdbcTemplate jdbc) {
        this.rateLimiter = rateLimiter;
        this.authService = authService;
        this.jdbc = jdbc;
    }

    /** Strips the `/v1` prefix so matchers work regardless of context path. */
    private static String logicalPath(String path) {
        return path.startsWith("/v1") ? path.substring(3) : path;
    }

    private static String rateLimitAction(String path, String method) {
        String p = logicalPath(path);
        if (p.equals("/posts") && "POST".equals(method)) {
            return "post";
        }
        if (p.startsWith("/posts/") && p.endsWith("/like") && "POST".equals(method)) {
            return "like";
        }
        if (p.startsWith("/posts/") && p.endsWith("/comment") && "POST".equals(method)) {
            return "comment";
        }
        if (p.equals("/feed") && "GET".equals(method)) {
            return "feed";
        }
        if (p.equals("/feed/refresh") && "POST".equals(method)) {
            return "feed";
        }
        if (p.startsWith("/media") && "GET".equals(method)) {
            return "media_read";
        }
        if (p.startsWith("/media") && ("POST".equals(method) || "DELETE".equals(method))) {
            return "media_upload";
        }
        return null;
    }

    @Override
    protected void doFilterInternal(
            HttpServletRequest request, HttpServletResponse response, FilterChain filterChain)
            throws ServletException, IOException {
        String action = rateLimitAction(request.getRequestURI(), request.getMethod());
        if (action == null) {
            filterChain.doFilter(request, response);
            return;
        }

        Optional<UUID> userId = BearerAuth.extractToken(request.getHeader("Authorization"))
                .flatMap(authService::authenticateAccessToken);
        if (userId.isEmpty()) {
            // No valid session: let the handler / AuthUserResolver reject with 401.
            filterChain.doFilter(request, response);
            return;
        }

        try {
            TrustLevel trustLevel = loadTrustLevel(userId.get());
            RateLimiter.RateLimitInfo info = rateLimiter.checkAndIncrement(userId.get(), action, trustLevel);

            if (info.limited()) {
                RateLimitResponses.writeRateLimited(response, action, info.limit());
                return;
            }

            response.setHeader("X-RateLimit-Limit", Long.toString(info.limit()));
            response.setHeader("X-RateLimit-Remaining", Long.toString(info.remaining()));
            filterChain.doFilter(request, response);
        } catch (Exception e) {
            log.error("failed to check rate limit", e);
            RateLimitResponses.writeInternalError(response);
        }
    }

    private TrustLevel loadTrustLevel(UUID userId) {
        try {
            Integer level = jdbc.queryForObject(
                    "SELECT trust_level FROM user_trust_scores WHERE user_id = ?", Integer.class, userId);
            return level == null ? TrustLevel.NEW : TrustLevel.fromInt(level);
        } catch (org.springframework.dao.EmptyResultDataAccessException e) {
            return TrustLevel.NEW;
        }
    }
}
