#!/bin/bash
set -euo pipefail

echo "Waiting for database..."
until pg_isready -h db -U lumine -d lumine >/dev/null 2>&1; do
  sleep 1
done

echo "Running migrations..."
for file in /migrations/*.sql; do
  echo "Applying ${file}..."
  psql -h db -U lumine -d lumine -v ON_ERROR_STOP=1 -f "$file"
done

echo "Migrations complete."
