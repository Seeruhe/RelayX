#!/usr/bin/env bash
set -euo pipefail

pg_container="proxy-control-e2e-postgres-${RANDOM}-${RANDOM}"
pg_port="${E2E_POSTGRES_PORT:-55434}"
api_port="${E2E_CONTROL_PLANE_PORT:-18081}"
image="${POSTGRES_SMOKE_IMAGE:-postgres:16-alpine}"
xray_image="${XRAY_DOCKER_IMAGE:-ghcr.io/xtls/xray-core:latest}"
database_url="postgres://proxy:proxy@127.0.0.1:${pg_port}/proxy_control"
base_url="http://127.0.0.1:${api_port}"
work_dir="$(mktemp -d)"
control_plane_pid=""

cleanup() {
  if [[ -n "${control_plane_pid}" ]]; then
    kill "${control_plane_pid}" >/dev/null 2>&1 || true
    wait "${control_plane_pid}" >/dev/null 2>&1 || true
  fi
  docker rm -f "$pg_container" >/dev/null 2>&1 || true
  rm -rf "$work_dir"
}
trap cleanup EXIT

json_get() {
  python3 -c 'import json,sys; data=json.load(sys.stdin); cur=data
for part in sys.argv[1].split("."):
    cur = cur[int(part)] if isinstance(cur, list) else cur[part]
print(cur)' "$1"
}

request() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local output="$work_dir/response.json"
  local status
  if [[ -n "$body" ]]; then
    status=$(curl -sS -o "$output" -w '%{http_code}' -X "$method" "$base_url$path" \
      -H 'content-type: application/json' \
      --data "$body")
  else
    status=$(curl -sS -o "$output" -w '%{http_code}' -X "$method" "$base_url$path")
  fi
  if [[ "$status" -lt 200 || "$status" -ge 300 ]]; then
    echo "Request failed: $method $path status=$status" >&2
    cat "$output" >&2 || true
    exit 1
  fi
  cat "$output"
}

echo "Building control-plane and runner binaries"
cargo build -p control-plane -p runner >/dev/null

echo "Starting disposable Postgres ${pg_container} on ${pg_port}"
docker run -d --name "$pg_container" \
  -e POSTGRES_USER=proxy \
  -e POSTGRES_PASSWORD=proxy \
  -e POSTGRES_DB=proxy_control \
  -p "${pg_port}:5432" \
  "$image" >/dev/null
for _ in $(seq 1 60); do
  if docker exec "$pg_container" pg_isready -U proxy -d proxy_control >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
docker exec "$pg_container" pg_isready -U proxy -d proxy_control >/dev/null

echo "Applying migration"
docker exec -i "$pg_container" psql -U proxy -d proxy_control -v ON_ERROR_STOP=1 < migrations/0001_p0_schema.sql >/dev/null

echo "Writing Docker-backed xray wrapper"
xray_wrapper="$work_dir/xray-docker-wrapper.sh"
cat > "$xray_wrapper" <<WRAP
#!/usr/bin/env bash
set -euo pipefail
config=""
args=()
while [[ \$# -gt 0 ]]; do
  case "\$1" in
    -config)
      config="\$2"
      args+=("\$1" "/work/config.json")
      shift 2
      ;;
    *)
      args+=("\$1")
      shift
      ;;
  esac
done
if [[ -z "\$config" ]]; then
  exec docker run --rm ${xray_image} "\${args[@]}"
fi
config_dir=\$(dirname "\$config")
exec docker run --rm -v "\$config_dir:/work:ro" ${xray_image} "\${args[@]}"
WRAP
chmod +x "$xray_wrapper"
docker run --rm "$xray_image" version >/dev/null

echo "Starting control-plane on ${base_url}"
DATABASE_URL="$database_url" \
CONTROL_PLANE_BIND="127.0.0.1:${api_port}" \
RUNNER_API_TOKEN="dev-runner-token" \
NODE_REGISTRATION_TOKEN="dev-registration-token" \
  target/debug/control-plane >"$work_dir/control-plane.log" 2>&1 &
control_plane_pid="$!"
for _ in $(seq 1 60); do
  if curl -sS "$base_url/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -sS "$base_url/healthz" >/dev/null

echo "Starting runner once for self-registration and heartbeat"
CONTROL_PLANE_BASE_URL="$base_url" \
RUNNER_NODE_ID="node-e2e" \
RUNNER_API_TOKEN="dev-runner-token" \
NODE_REGISTRATION_TOKEN="dev-registration-token" \
RUNNER_XRAY_VERSION="26.3.27" \
RUNNER_WORK_DIR="$work_dir/runner-register" \
RUNNER_XRAY_BIN="$xray_wrapper" \
RUNNER_ONCE=1 \
  target/debug/runner >"$work_dir/runner-register.log" 2>&1
heartbeat=$(request GET /nodes/node-e2e/heartbeat)
heartbeat_xray=$(printf '%s' "$heartbeat" | json_get capability_snapshot.xray_bin)
if [[ -z "$heartbeat_xray" ]]; then
  echo "runner heartbeat missing xray capability snapshot" >&2
  cat "$work_dir/runner-register.log" >&2 || true
  exit 1
fi

echo "Creating VLESS REALITY profile/client and compiling deployment"
request POST /profiles/vless-reality '{"profile_id":"profile-e2e","server_name":"example.com"}' >/dev/null
request POST /clients '{"client_id":"client-e2e","profile_id":"profile-e2e","display_name":"E2E Alice","kind":"vless","uuid":"2f4f6f8a-1111-4c4c-9999-111111111111"}' >/dev/null
compile_response=$(request POST /deployments/compile '{"profile_id":"profile-e2e","node_id":"node-e2e"}')
artifact_sha=$(printf '%s' "$compile_response" | json_get artifact.sha256)
deployment_id="dep-${artifact_sha:0:12}"

reload_cmd="$work_dir/reload-xray.sh"
health_cmd="$work_dir/health-xray.sh"
cat > "$reload_cmd" <<'RELOAD'
#!/usr/bin/env bash
set -euo pipefail
test -f "$RUNNER_ACTIVE_CONFIG"
printf 'reload:%s:%s\n' "$RUNNER_DEPLOYMENT_ID" "$RUNNER_ACTIVE_CONFIG" >> "$RUNNER_RELOAD_LOG"
RELOAD
cat > "$health_cmd" <<'HEALTH'
#!/usr/bin/env bash
set -euo pipefail
test -L "$RUNNER_ACTIVE_DIR"
test -f "$RUNNER_ACTIVE_CONFIG"
printf 'health:%s\n' "$RUNNER_DEPLOYMENT_ID" >> "$RUNNER_HEALTH_LOG"
HEALTH
chmod +x "$reload_cmd" "$health_cmd"

echo "Running runner once with real xray wrapper for ${deployment_id}"
CONTROL_PLANE_BASE_URL="$base_url" \
RUNNER_NODE_ID="node-e2e" \
RUNNER_API_TOKEN="dev-runner-token" \
RUNNER_WORK_DIR="$work_dir/runner" \
RUNNER_XRAY_BIN="$xray_wrapper" \
RUNNER_XRAY_RELOAD_CMD="$reload_cmd" \
RUNNER_XRAY_HEALTH_CMD="$health_cmd" \
RUNNER_RELOAD_LOG="$work_dir/reload.log" \
RUNNER_HEALTH_LOG="$work_dir/health.log" \
RUNNER_ONCE=1 \
  target/debug/runner >"$work_dir/runner.log" 2>&1

deployment=$(request GET "/deployments/${deployment_id}")
status=$(printf '%s' "$deployment" | json_get status)
if [[ "$status" != "Succeeded" ]]; then
  echo "deployment status expected Succeeded, got ${status}" >&2
  cat "$work_dir/runner.log" >&2 || true
  exit 1
fi
health=$(request GET "/deployments/${deployment_id}/health")
health_status=$(printf '%s' "$health" | json_get status)
if [[ "$health_status" != "healthy" ]]; then
  echo "health status expected healthy, got ${health_status}" >&2
  exit 1
fi
if [[ ! -f "$work_dir/runner/active/config.json" ]]; then
  echo "runner active config missing" >&2
  exit 1
fi
if ! grep -q "reload:${deployment_id}:" "$work_dir/reload.log"; then
  echo "runner reload/restart command was not observed" >&2
  cat "$work_dir/runner.log" >&2 || true
  exit 1
fi
if ! grep -q "health:${deployment_id}" "$work_dir/health.log"; then
  echo "runner process health command was not observed" >&2
  cat "$work_dir/runner.log" >&2 || true
  exit 1
fi
subscription=$(request GET /subscriptions/profile-e2e)
python3 - <<'PY' <<<"$subscription"
import base64,json,sys
body=json.load(sys.stdin)["body_base64"]
plain=base64.b64decode(body).decode()
assert "vless://" in plain and "node-e2e.example" in plain, plain
PY

echo "e2e_smoke deployment=${deployment_id} self_registered=yes status=${status} health=${health_status} active_config=yes reload=yes process_health=yes subscription=ok"
