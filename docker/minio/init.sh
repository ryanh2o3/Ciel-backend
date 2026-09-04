#!/bin/bash
set -euo pipefail

echo "Waiting for MinIO..."
until mc alias set local "http://minio:9000" "${MINIO_ROOT_USER}" "${MINIO_ROOT_PASSWORD}" >/dev/null 2>&1; do
  sleep 1
done

echo "Ensuring bucket ${S3_BUCKET} exists..."
mc mb --ignore-existing "local/${S3_BUCKET}"

# Bucket-level CORS (`mc cors set`) is AIStor-only and fails on community MinIO.
# Global CORS defaults to * via MINIO_API_CORS_ALLOW_ORIGIN on the minio service.

echo "MinIO init complete."
