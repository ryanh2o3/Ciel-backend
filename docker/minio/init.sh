#!/bin/bash
set -euo pipefail

echo "Waiting for MinIO..."
until mc alias set local "http://minio:9000" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}" >/dev/null 2>&1; do
  sleep 1
done

echo "Ensuring bucket ${S3_BUCKET} exists..."
mc mb --ignore-existing "local/${S3_BUCKET}"

# CORS for browser clients / future web; native mobile apps ignore CORS.
cat >/tmp/cors.json <<'EOF'
{
  "CORSRules": [
    {
      "AllowedOrigins": ["*"],
      "AllowedMethods": ["GET", "PUT", "HEAD", "POST", "DELETE"],
      "AllowedHeaders": ["*"],
      "ExposeHeaders": ["ETag", "Content-Length", "Content-Type"],
      "MaxAgeSeconds": 3600
    }
  ]
}
EOF
mc cors set /tmp/cors.json "local/${S3_BUCKET}"

echo "MinIO init complete."
