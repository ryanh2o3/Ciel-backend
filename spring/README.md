# Ciel Spring Boot API

Spring Boot parity port of the Ciel photo social HTTP API. This stack serves `/v1` routes only — media processing and cleanup remain owned by the Rust implementation.

## Contract

Shared behavior, auth, Redis keys, cursor encoding, and env vars are defined in [`../docs/CONTRACT.md`](../docs/CONTRACT.md).

## Build & run

```bash
# Compile
mvn -q package -DskipTests

# Run (requires Postgres, Redis, S3, SQS, PASETO keys — same as Rust)
export DATABASE_URL=postgres://ciel:ciel@127.0.0.1:5432/ciel
export REDIS_URL=redis://127.0.0.1:6379
export PASETO_ACCESS_KEY=...
export PASETO_REFRESH_KEY=...
export CIEL_SERVED_BY=spring
java -jar target/ciel-backend-spring.jar
```

Health: `GET /health` → `{"status":"ok"}` (or `"degraded"` if DB/Redis ping fails)

## Docker

```bash
docker build -t ciel-backend-spring .
docker run --env-file ../.env -p 8080:8080 ciel-backend-spring
```

Responses include `X-Ciel-Served-By: spring` when `CIEL_SERVED_BY=spring`.
