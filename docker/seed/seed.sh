#!/bin/bash
set -euo pipefail

echo "Uploading seed images to S3..."
bash docker/seed/upload_media.sh

echo "Seeding database..."
docker compose exec -T db psql -U picshare -d picshare < docker/seed/seed.sql

echo "Seed data loaded successfully!"
