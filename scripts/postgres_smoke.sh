#!/usr/bin/env bash
set -euo pipefail

container_name="proxy-control-postgres-smoke-${RANDOM}-${RANDOM}"
port="${POSTGRES_SMOKE_PORT:-55432}"
image="${POSTGRES_SMOKE_IMAGE:-postgres:16-alpine}"
database_url="postgres://proxy:proxy@127.0.0.1:${port}/proxy_control"

cleanup() {
  docker rm -f "$container_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "Starting disposable Postgres container: ${container_name} on port ${port}"
docker run -d --name "$container_name" \
  -e POSTGRES_USER=proxy \
  -e POSTGRES_PASSWORD=proxy \
  -e POSTGRES_DB=proxy_control \
  -p "${port}:5432" \
  "$image" >/dev/null

for _ in $(seq 1 60); do
  if docker exec "$container_name" pg_isready -U proxy -d proxy_control >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

docker exec "$container_name" pg_isready -U proxy -d proxy_control >/dev/null

echo "Running Postgres-backed storage contract against ${database_url}"
TEST_DATABASE_URL="$database_url" \
  cargo test -p storage --test postgres_store_contract -- --ignored --nocapture

echo "Postgres smoke passed"
