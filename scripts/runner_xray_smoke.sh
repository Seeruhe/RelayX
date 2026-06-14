#!/usr/bin/env bash
set -euo pipefail

image="${XRAY_DOCKER_IMAGE:-ghcr.io/xtls/xray-core:latest}"

echo "Checking real Xray image: ${image}"
docker run --rm "$image" version >/dev/null

echo "Running LocalRunner through real xray run -test wrapper"
XRAY_DOCKER_IMAGE="$image" \
  cargo test -p runner --test real_xray_runner_contract -- --ignored --nocapture

echo "Runner real-Xray smoke passed"
