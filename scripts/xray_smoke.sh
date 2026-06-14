#!/usr/bin/env bash
set -euo pipefail

image="${XRAY_DOCKER_IMAGE:-ghcr.io/xtls/xray-core:latest}"

echo "Checking real Xray image: ${image}"
docker run --rm "$image" version >/dev/null

echo "Running compiler output through real xray run -test"
XRAY_DOCKER_IMAGE="$image" \
  cargo test -p compiler-xray --test real_xray_contract -- --ignored --nocapture

echo "Xray smoke passed"
