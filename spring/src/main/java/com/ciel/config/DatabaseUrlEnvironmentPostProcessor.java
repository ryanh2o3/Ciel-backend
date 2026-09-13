package com.ciel.config;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.env.EnvironmentPostProcessor;
import org.springframework.core.env.ConfigurableEnvironment;
import org.springframework.core.env.MapPropertySource;

import java.net.URI;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Translates the Rust-style {@code DATABASE_URL} (e.g.
 * {@code postgres://user:pass@host:5432/dbname}) and {@code HTTP_ADDR}
 * (e.g. {@code 0.0.0.0:8080}) environment variables into Spring Boot
 * properties before the context starts, so the same env vars used by the
 * Rust and .NET implementations work here without extra config.
 *
 * <p>Explicit {@code SPRING_DATASOURCE_*} / {@code SERVER_PORT} env vars
 * always take precedence — this only fills in gaps.
 */
public class DatabaseUrlEnvironmentPostProcessor implements EnvironmentPostProcessor {

    @Override
    public void postProcessEnvironment(ConfigurableEnvironment environment, SpringApplication application) {
        Map<String, Object> overrides = new LinkedHashMap<>();

        String databaseUrl = environment.getProperty("DATABASE_URL");
        if (databaseUrl != null && !databaseUrl.isBlank() && environment.getProperty("SPRING_DATASOURCE_URL") == null) {
            try {
                URI uri = new URI(databaseUrl);
                String host = uri.getHost();
                int port = uri.getPort() > 0 ? uri.getPort() : 5432;
                String path = uri.getPath() == null || uri.getPath().isBlank() ? "/ciel" : uri.getPath();
                StringBuilder jdbcUrl = new StringBuilder("jdbc:postgresql://").append(host).append(':').append(port).append(path);
                if (uri.getQuery() != null && !uri.getQuery().isBlank()) {
                    jdbcUrl.append('?').append(uri.getQuery());
                }
                overrides.put("spring.datasource.url", jdbcUrl.toString());

                String userInfo = uri.getUserInfo();
                if (userInfo != null && !userInfo.isBlank()) {
                    String[] parts = userInfo.split(":", 2);
                    overrides.put("spring.datasource.username", parts[0]);
                    if (parts.length > 1) {
                        overrides.put("spring.datasource.password", parts[1]);
                    }
                }
            } catch (Exception ignored) {
                // Malformed DATABASE_URL: fall back to spring.datasource.* / application.yml defaults.
            }
        }

        String httpAddr = environment.getProperty("HTTP_ADDR");
        if (httpAddr != null && !httpAddr.isBlank() && environment.getProperty("SERVER_PORT") == null) {
            String portPart = httpAddr.contains(":") ? httpAddr.substring(httpAddr.lastIndexOf(':') + 1) : httpAddr;
            try {
                overrides.put("server.port", Integer.parseInt(portPart.trim()));
            } catch (NumberFormatException ignored) {
                // Malformed HTTP_ADDR: fall back to server.port / application.yml defaults.
            }
        }

        if (!overrides.isEmpty()) {
            environment.getPropertySources().addFirst(new MapPropertySource("cielEnvUrlOverrides", overrides));
        }
    }
}
