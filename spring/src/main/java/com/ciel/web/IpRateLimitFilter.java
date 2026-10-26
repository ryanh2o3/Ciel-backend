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
        if (p.equals("/account/device/register") && "POST".equals(method)) return new IpLimitConfig("device_register",60,RateWindow.HOUR);
        if (p.startsWith("/invites/validate/") && "GET".equals(method)) return new IpLimitConfig("invite_validate",120,RateWindow.HOUR);
        return null;
    }

    /** Match Rust/.NET: only trust forwarded addresses from configured proxies. */
    private String clientIp(HttpServletRequest request) {
        var trusted = TrustedProxies.parse(props.getTrustedProxyCidrs());
        String remote = request.getRemoteAddr();
        if (!TrustedProxies.contains(trusted, remote)) return remote;
        String xff = request.getHeader("X-Forwarded-For");
        if (xff == null) return remote;
        var hops = new java.util.ArrayList<String>();
        for (String part : xff.split(",")) {
            String hop = part.trim();
            // Reject hostnames; forwarding headers must never trigger DNS lookup.
            if (!hop.matches("[0-9a-fA-F:.]+") || (!hop.contains(":") && !hop.matches("[0-9.]+"))) continue;
            try { hops.add(java.net.InetAddress.getByName(hop).getHostAddress()); }
            catch (java.net.UnknownHostException ignored) { }
        }
        for (int i = hops.size() - 1; i >= 0; i--)
            if (!TrustedProxies.contains(trusted, hops.get(i))) return hops.get(i);
        return hops.isEmpty() ? remote : hops.get(0);
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
