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

## Docker

```bash
docker build -t ciel-api-dotnet:local -f dotnet/Dockerfile dotnet
docker run --rm -p 8080:8080 --env-file ../.env ciel-api-dotnet:local
```

## Notes

- Enqueues media jobs to SQS; the **Rust worker** processes uploads.
- Does not run hourly cleanup loops (Rust API only).
- Responses include `X-Ciel-Served-By: dotnet`.
