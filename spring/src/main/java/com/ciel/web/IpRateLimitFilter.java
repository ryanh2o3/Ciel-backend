package com.ciel.web;

import com.ciel.app.RateLimiter;
import com.ciel.config.AppProperties;
import com.ciel.config.RateWindow;
import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.core.Ordered;
import org.springframework.core.annotation.Order;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;

import java.io.IOException;

/**
 * IP-based rate limiting for unauthenticated endpoints (login, refresh,
 * revoke, signup, health). Mirrors Rust's {@code ip_rate_limit_middleware}
 * (see {@code src/http/middleware/rate_limit.rs} {@code ip_rate_limit_config}),
 * restricted to the routes this API exposes.
 */
@Component
@Order(Ordered.HIGHEST_PRECEDENCE + 2)
public class IpRateLimitFilter extends OncePerRequestFilter {

    private static final Logger log = LoggerFactory.getLogger(IpRateLimitFilter.class);

    private record IpLimitConfig(String action, long limit, RateWindow window) {}

    private final RateLimiter rateLimiter;
    private final AppProperties props;

    public IpRateLimitFilter(RateLimiter rateLimiter, AppProperties props) {
        this.rateLimiter = rateLimiter;
        this.props = props;
    }

    private static String logicalPath(String path) {
        return path.startsWith("/v1") ? path.substring(3) : path;
    }

    private IpLimitConfig ipRateLimitConfig(String path, String method) {
        String p = logicalPath(path);
        if (p.equals("/auth/login") && "POST".equals(method)) {
            return new IpLimitConfig("login", 30, RateWindow.HOUR);
        }
        if (p.equals("/auth/refresh") && "POST".equals(method)) {
            return new IpLimitConfig("auth_refresh", 120, RateWindow.HOUR);
        }
        if (p.equals("/auth/revoke") && "POST".equals(method)) {
            return new IpLimitConfig("auth_revoke", 60, RateWindow.HOUR);
        }
        if (p.equals("/users") && "POST".equals(method)) {
            return new IpLimitConfig("signup", props.getIpSignupRateLimit(), RateWindow.DAY);
        }
        if (p.equals("/health") && "GET".equals(method)) {
            return new IpLimitConfig("health", 60, RateWindow.MINUTE);
        }
        return null;
    }

    /** X-Forwarded-For first hop if present, else the direct remote address. */
    private static String clientIp(HttpServletRequest request) {
        String xff = request.getHeader("X-Forwarded-For");
        if (xff != null && !xff.isBlank()) {
            return xff.split(",")[0].trim();
        }
        return request.getRemoteAddr();
    }

    @Override
    protected void doFilterInternal(
            HttpServletRequest request, HttpServletResponse response, FilterChain filterChain)
            throws ServletException, IOException {
        IpLimitConfig config = ipRateLimitConfig(request.getRequestURI(), request.getMethod());
        if (config == null) {
            filterChain.doFilter(request, response);
            return;
        }

        String ip = clientIp(request);
        try {
            boolean limited = rateLimiter.checkAndIncrementIp(ip, config.action(), config.limit(), config.window());
            if (limited) {
                log.warn("IP rate limit exceeded: ip={} action={}", ip, config.action());
                RateLimitResponses.writeRateLimited(response, config.action(), config.limit());
                return;
            }
            filterChain.doFilter(request, response);
        } catch (Exception e) {
            log.error("failed to check IP rate limit", e);
            RateLimitResponses.writeInternalError(response);
        }
    }
}
