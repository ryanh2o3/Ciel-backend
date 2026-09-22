package com.ciel.web;

import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.http.MediaType;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * Liveness matching Rust {@code handlers::health}: {@code {"status":"ok"|"degraded"}}.
 * Pings Postgres and Redis with short timeouts; either failure → {@code degraded}.
 */
@RestController
public class HealthController {

    private final JdbcTemplate jdbc;
    private final StringRedisTemplate redis;

    public HealthController(JdbcTemplate jdbc, StringRedisTemplate redis) {
        this.jdbc = jdbc;
        this.redis = redis;
    }

    @GetMapping(value = "/health", produces = MediaType.APPLICATION_JSON_VALUE)
    public Map<String, String> health() {
        boolean db = pingDb();
        boolean cache = pingRedis();
        return Map.of("status", db && cache ? "ok" : "degraded");
    }

    private boolean pingDb() {
        try {
            Integer one = jdbc.queryForObject("SELECT 1", Integer.class);
            return one != null && one == 1;
        } catch (Exception e) {
            return false;
        }
    }

    private boolean pingRedis() {
        try {
            String pong = redis.execute((org.springframework.data.redis.core.RedisCallback<String>) connection ->
                    connection.ping());
            return pong != null && !pong.isBlank();
        } catch (Exception e) {
            return false;
        }
    }
}
