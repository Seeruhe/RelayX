setup:
    cargo fetch
    if command -v pnpm >/dev/null 2>&1; then pnpm install; else echo "pnpm not installed; skipping web install"; fi

db-up:
    echo "For disposable verified Postgres integration run: just postgres-smoke"

migrate:
    sqlx migrate run

postgres-smoke:
    ./scripts/postgres_smoke.sh

xray-smoke:
    ./scripts/xray_smoke.sh

runner-xray-smoke:
    ./scripts/runner_xray_smoke.sh

e2e-smoke:
    ./scripts/e2e_smoke.sh

control-plane:
    cargo run -p control-plane

runner:
    cargo run -p runner

web:
    pnpm --filter web dev

test:
    cargo test --workspace
    if command -v pnpm >/dev/null 2>&1; then pnpm --filter web lint; else echo "pnpm not installed; skipping web lint"; fi

lint:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    if command -v pnpm >/dev/null 2>&1; then pnpm --filter web lint; else echo "pnpm not installed; skipping web lint"; fi
