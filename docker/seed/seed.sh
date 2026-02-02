#!/bin/bash
set -euo pipefail

echo "Uploading seed images to S3..."
bash docker/seed/upload_media.sh

echo "Waiting for API to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:8080/health | grep -q '"status":"ok"'; then
        echo "API is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "API not ready after 30 seconds"
        exit 1
    fi
    sleep 1
    echo -n "."
done

echo "Clearing existing data..."
docker compose exec -T db psql -U lumine -d lumine -c "DELETE FROM users;"
docker compose exec -T redis redis-cli flushdb

echo "Creating users via API (with proper password hashes)..."
# Create demo user
curl -s -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"demo","email":"demo@example.com","display_name":"Demo User","bio":"Hello from Luminé.","password":"ChangeMe123!"}' \
  > /dev/null

# Create alice user
curl -s -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"alice","email":"alice@example.com","display_name":"Alice","bio":"Coffee, photos, and travel.","password":"ChangeMe123!"}' \
  > /dev/null

# Create bob user
curl -s -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"bob","email":"bob@example.com","display_name":"Bob","bio":"Street photography enthusiast.","password":"ChangeMe123!"}' \
  > /dev/null

# Create cora user
curl -s -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"cora","email":"cora@example.com","display_name":"Cora","bio":"Food, friends, and sunsets.","password":"ChangeMe123!"}' \
  > /dev/null

echo "Creating sample content..."
docker compose exec -T db psql -U lumine -d lumine < docker/seed/seed_content.sql

echo "Seed data loaded successfully!"
