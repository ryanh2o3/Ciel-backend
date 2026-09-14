package com.ciel.web;

import com.ciel.config.AppProperties;
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
import java.util.List;

/**
 * Security headers, mirroring Rust's {@code security_headers_middleware}
 * (see {@code src/http/middleware/security.rs}): nosniff, deny-framing, XSS
 * protection, a locked-down CSP, a strict referrer policy, a default
 * no-store Cache-Control, HSTS off localhost, and rejection of requests that
 * a <em>trusted</em> proxy explicitly forwarded as plain HTTP.
 *
 * <p>Unlike an earlier version of this filter, {@code X-Forwarded-Proto} is
 * ignored unless the peer is in {@code TRUSTED_PROXY_CIDRS}. Untrusted / empty
 * CIDRs → scheme {@code Unknown} → allow (TLS may terminate at Cloudflare /
 * the ingress). That matches Rust and avoids 403s when Traefik marks the
 * cleartext hop as {@code http}.
 */
@Component
@Order(Ordered.HIGHEST_PRECEDENCE + 1)
public class SecurityHeadersFilter extends OncePerRequestFilter {

    private static final Logger log = LoggerFactory.getLogger(SecurityHeadersFilter.class);

    private static final String HSTS_VALUE = "max-age=31536000; includeSubDomains";
    private static final String CSP_VALUE = "default-src 'none'; frame-ancestors 'none'";
    private static final String CACHE_CONTROL_VALUE = "no-store, no-cache, must-revalidate";

    private final List<TrustedProxies.Cidr> trustedProxyCidrs;

    public SecurityHeadersFilter(AppProperties props) {
        this.trustedProxyCidrs = TrustedProxies.parse(props.getTrustedProxyCidrs());
    }

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

        // Rust: only enforce HTTPS when peer is a trusted proxy and scheme is Http.
        // Unknown (direct / untrusted) → allow.
        if (!local && TrustedProxies.contains(trustedProxyCidrs, request.getRemoteAddr())) {
            String forwardedProto = request.getHeader("X-Forwarded-Proto");
            if (forwardedProto == null || forwardedProto.isBlank()) {
                // Fail closed behind a trusted proxy with no scheme header.
                log.warn("rejected non-HTTPS request (missing X-Forwarded-Proto behind trusted proxy): host={}", host);
                response.setStatus(HttpServletResponse.SC_FORBIDDEN);
                return;
            }
            String scheme = forwardedProto.split(",")[0].trim().toLowerCase();
            if ("http".equals(scheme)) {
                log.warn("rejected non-HTTPS request (forwarded scheme was http): host={}", host);
                response.setStatus(HttpServletResponse.SC_FORBIDDEN);
                return;
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
