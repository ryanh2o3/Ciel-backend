# Ciel API (.NET 8)

ASP.NET Core implementation of the Ciel `/v1` HTTP API. Shares Postgres schema (Rust migrations), Redis keys, PASETO keys, and S3/SQS infrastructure with the Rust and Spring stacks.

## Run locally

Requires the same environment variables as the Rust API (see `../.env.example`). Minimum:

```bash
export DATABASE_URL=postgres://ciel:ciel@127.0.0.1:5432/ciel
export REDIS_URL=redis://127.0.0.1/
export S3_ENDPOINT=http://127.0.0.1:4566
export S3_BUCKET=ciel-media
export QUEUE_ENDPOINT=http://127.0.0.1:4566
export QUEUE_NAME=ciel-media-jobs
export PASETO_ACCESS_KEY=<43-char-base64-32-bytes>
export PASETO_REFRESH_KEY=<43-char-base64-32-bytes>
export CIEL_SERVED_BY=dotnet
```

```bash
cd dotnet/Ciel.Api
dotnet run
```

Health: `GET http://localhost:8080/health`

### Listen address

The server binds to whatever **`ASPNETCORE_URLS`** says (ASP.NET Core's standard
listen-address variable), e.g. `ASPNETCORE_URLS=http://0.0.0.0:8080`. In
Kubernetes/production this is set by the deployment manifest — there is no
code-level default beyond what the base image / hosting environment provides.
`HTTP_ADDR` (used by the Rust/Spring stacks) is still read into `AppConfig` for
parity but is intentionally **not** wired to `app.Urls`, to avoid two
competing sources of truth for the bind address.

### Trusted proxies / rate limiting

Set `TRUSTED_PROXY_CIDRS` (comma-separated CIDRs, e.g.
`10.0.0.0/8,172.16.0.0/12`) so `X-Forwarded-For` / `X-Forwarded-Proto` are
honored only when the immediate peer is one of your load balancers — this
matches the Rust stack's `TRUSTED_PROXY_CIDRS` handling
(src/http/middleware/request_context.rs) and determines both the client IP
used for IP-based rate limiting and whether HTTPS is enforced. `IP_SIGNUP_RATE_LIMIT`
(default `10`) caps signups per IP per day.

## Tests

```bash
cd dotnet
dotnet test
```

`Ciel.Api.Tests` covers `CursorCodec`, `PasetoService` (mint/verify), `CryptoService`
(Argon2id hash/verify round trip — this is a regression test for a real off-by-one
bug in PHC-string parsing), the feed-cache JSON contract for `owner_avatar_key`,
rate-limit route→action mapping, and trusted-proxy `X-Forwarded-For` parsing.

## Docker

```bash
docker build -t ciel-api-dotnet:local -f dotnet/Dockerfile dotnet
docker run --rm -p 8080:8080 --env-file ../.env ciel-api-dotnet:local
```

## Notes

- Enqueues media jobs to SQS; the **Rust worker** processes uploads.
- Does not run hourly cleanup loops (Rust API only).
- Responses include `X-Ciel-Served-By: dotnet`.
