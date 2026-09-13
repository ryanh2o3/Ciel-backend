package com.ciel.web;

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
 * Security headers, mirroring Rust's {@code security_headers_middleware}
 * (see {@code src/http/middleware/security.rs}): nosniff, deny-framing, XSS
 * protection, a locked-down CSP, a strict referrer policy, a default
 * no-store Cache-Control, HSTS off localhost, and rejection of requests that
 * explicitly forwarded as plain HTTP.
 */
@Component
@Order(Ordered.HIGHEST_PRECEDENCE + 1)
public class SecurityHeadersFilter extends OncePerRequestFilter {

    private static final Logger log = LoggerFactory.getLogger(SecurityHeadersFilter.class);

    private static final String HSTS_VALUE = "max-age=31536000; includeSubDomains";
    private static final String CSP_VALUE = "default-src 'none'; frame-ancestors 'none'";
    private static final String CACHE_CONTROL_VALUE = "no-store, no-cache, must-revalidate";

    private static boolean isLocal(String host) {
        if (host == null) {
            return false;
        }
        return host.startsWith("localhost")
                || host.startsWith("127.0.0.1")
                || host.startsWith("0.0.0.0")
                || host.startsWith("[::1]");
    }

    @Override
    protected void doFilterInternal(
            HttpServletRequest request, HttpServletResponse response, FilterChain filterChain)
            throws ServletException, IOException {
        String host = request.getHeader("Host");
        boolean local = isLocal(host);

        if (!local) {
            String forwardedProto = request.getHeader("X-Forwarded-Proto");
            if (forwardedProto != null) {
                String scheme = forwardedProto.split(",")[0].trim().toLowerCase();
                if ("http".equals(scheme)) {
                    log.warn("rejected non-HTTPS request (forwarded scheme was http): host={}", host);
                    response.setStatus(HttpServletResponse.SC_FORBIDDEN);
                    return;
                }
            }
        }

        if (!local) {
            response.setHeader("Strict-Transport-Security", HSTS_VALUE);
        }
        response.setHeader("X-Content-Type-Options", "nosniff");
        response.setHeader("X-Frame-Options", "DENY");
        response.setHeader("X-XSS-Protection", "1; mode=block");
        response.setHeader("Content-Security-Policy", CSP_VALUE);
        response.setHeader("Referrer-Policy", "no-referrer");
        if (response.getHeader("Cache-Control") == null) {
            response.setHeader("Cache-Control", CACHE_CONTROL_VALUE);
        }

        filterChain.doFilter(request, response);
    }
}
