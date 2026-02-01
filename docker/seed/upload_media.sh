#!/bin/bash
set -euo pipefail

if ! docker compose ps localstack >/dev/null 2>&1; then
  echo "LocalStack is not running. Start it with: docker compose up -d"
  exit 1
fi

if [ ! -d "docker/seed/images" ]; then
  echo "Missing docker/seed/images. Create it and add images to upload."
  exit 1
fi

if ! docker compose exec -T localstack sh -c "test -d /seed-images"; then
  echo "LocalStack container is missing /seed-images. Did you mount ./docker/seed/images?"
  exit 1
fi

docker compose exec -T localstack sh -c \
  "awslocal s3 mb s3://picshare-media >/dev/null 2>&1 || true"

docker compose exec -T localstack sh -c \
  "find /seed-images -type f 2>/dev/null | while read -r f; do
     rel=\${f#/seed-images/}
     base=\${rel%.jpg}
     key=\"seed/\$rel\"
     thumb_key=\"seed/\${base}_thumb.jpg\"
     medium_key=\"seed/\${base}_medium.jpg\"

     awslocal s3 cp \"\$f\" \"s3://picshare-media/\$key\"
     awslocal s3 cp \"\$f\" \"s3://picshare-media/\$thumb_key\"
     awslocal s3 cp \"\$f\" \"s3://picshare-media/\$medium_key\"
   done"

echo "Uploaded seed images to s3://picshare-media/seed/"
