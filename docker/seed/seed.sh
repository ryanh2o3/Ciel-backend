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
docker compose exec -T db psql -U ciel -d ciel -c "DELETE FROM users;"
docker compose exec -T redis redis-cli flushdb

echo "Creating bootstrap user (demo) directly in DB..."
# Insert demo user with a pre-computed Argon2 hash for "ChangeMe123!"
# Then create invite codes so we can register the remaining users via the API.
docker compose exec -T db psql -U ciel -d ciel <<'SQL'
-- Create the demo user directly (bypassing invite requirement)
INSERT INTO users (handle, email, display_name, bio, password_hash)
VALUES (
    'demo',
    'demo@example.com',
    'Demo User',
    'Hello from Ciel.',
    -- Argon2id hash of "ChangeMe123!" (m=19456, t=2, p=1)
    '$argon2id$v=19$m=19456,t=2,p=1$aNvD0XsklBlRiXk6Pz+W9A$RwEl4P+w33YwAAa2qjmh7zYsNEq8kzBi/3LJfHfAFwI'
)
ON CONFLICT (handle) DO NOTHING;

-- Create invite codes owned by demo for the other seed users
INSERT INTO invite_codes (code, created_by, expires_at, invite_type, max_uses, use_count, is_valid)
SELECT code, u.id, NOW() + INTERVAL '1 year', 'standard', 1, 0, TRUE
FROM users u,
     (VALUES ('SEED-ALICE-0001'), ('SEED-BOB-00001'), ('SEED-CORA-0001')) AS v(code)
WHERE u.handle = 'demo'
ON CONFLICT DO NOTHING;
SQL

echo "Creating users via API..."
# Create alice
curl -sf -X POST http://localhost:8080/v1/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"alice","email":"alice@example.com","display_name":"Alice","bio":"Coffee, photos, and travel.","password":"ChangeMe123!","invite_code":"SEED-ALICE-0001"}' \
  > /dev/null || echo "  alice: already exists or failed"

# Create bob
curl -sf -X POST http://localhost:8080/v1/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"bob","email":"bob@example.com","display_name":"Bob","bio":"Street photography enthusiast.","password":"ChangeMe123!","invite_code":"SEED-BOB-00001"}' \
  > /dev/null || echo "  bob: already exists or failed"

# Create cora
curl -sf -X POST http://localhost:8080/v1/users \
  -H "Content-Type: application/json" \
  -d '{"handle":"cora","email":"cora@example.com","display_name":"Cora","bio":"Food, friends, and sunsets.","password":"ChangeMe123!","invite_code":"SEED-CORA-0001"}' \
  > /dev/null || echo "  cora: already exists or failed"

echo "Creating sample content..."
docker compose exec -T db psql -U ciel -d ciel < docker/seed/seed_content.sql

echo "Setting trust levels to Verified (3) for seeded users..."
docker compose exec -T db psql -U ciel -d ciel <<'SQL'
INSERT INTO user_trust_scores (user_id, trust_level, trust_points, account_age_days)
SELECT id, 3, 1000, 365
FROM users
WHERE handle IN ('demo', 'alice', 'bob', 'cora')
ON CONFLICT (user_id) DO UPDATE
SET trust_level = EXCLUDED.trust_level,
    trust_points = EXCLUDED.trust_points,
    account_age_days = EXCLUDED.account_age_days,
    updated_at = NOW();
SQL

echo "Seed data loaded successfully!"
