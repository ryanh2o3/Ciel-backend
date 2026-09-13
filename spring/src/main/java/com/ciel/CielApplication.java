package com.ciel;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.boot.context.properties.ConfigurationPropertiesScan;

/**
 * Entry point for the Spring Boot parity port of the Ciel photo social API.
 *
 * <p>This implementation only serves the {@code api} role (HTTP {@code /v1} routes) —
 * per the cross-implementation contract, media processing and hourly cleanup loops
 * remain exclusively owned by the Rust implementation. See
 * {@code Ciel-backend/docs/CONTRACT.md} for the full shared contract.
 */
@SpringBootApplication
@ConfigurationPropertiesScan
public class CielApplication {

    public static void main(String[] args) {
        SpringApplication.run(CielApplication.class, args);
    }
}
