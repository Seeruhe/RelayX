use chrono::Utc;
use domain::{
    Artifact, ArtifactKind, DeploymentStatus, RunnerCommand, RunnerCommandKind, SignedRunnerCommand,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use runner::LocalRunner;
use std::{fs, os::unix::fs::PermissionsExt};

#[tokio::test]
#[ignore = "requires Docker and XRAY_DOCKER_IMAGE, default ghcr.io/xtls/xray-core:latest"]
async fn local_runner_applies_config_after_real_xray_test_passes() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = write_xray_docker_wrapper(dir.path());
    let signing = SigningKey::from_bytes(&[11u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let config_json = serde_json::json!({
        "log": {"loglevel": "warning"},
        "inbounds": [{
            "tag":"in_ss_8388",
            "listen":"127.0.0.1",
            "port":8388,
            "protocol":"shadowsocks",
            "settings":{
                "method":"2022-blake3-aes-128-gcm",
                "password":"MDEyMzQ1Njc4OWFiY2RlZg==",
                "network":"tcp,udp"
            }
        }],
        "outbounds":[{"tag":"direct","protocol":"freedom"}],
        "routing":{"rules":[]}
    });
    let artifact = Artifact::from_bytes(
        "tenant-dev",
        ArtifactKind::CompiledXrayConfig,
        "application/json",
        serde_json::to_vec(&config_json).unwrap().as_slice(),
        "test",
    );
    let envelope = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-real-xray",
            1,
            Utc::now() + chrono::Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-real-xray".into(),
                artifact_sha256: artifact.sha256.clone(),
                config_json,
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();

    let mut runner = LocalRunner::new("node-real-xray", dir.path().join("work"), wrapper, verify);
    let result = runner.apply(envelope).await.unwrap();
    assert_eq!(result.status, DeploymentStatus::Succeeded);
    assert!(dir
        .path()
        .join("work")
        .join("active")
        .join("config.json")
        .exists());
}

fn write_xray_docker_wrapper(dir: &std::path::Path) -> std::path::PathBuf {
    let wrapper = dir.join("xray-docker-wrapper.sh");
    let image = std::env::var("XRAY_DOCKER_IMAGE")
        .unwrap_or_else(|_| "ghcr.io/xtls/xray-core:latest".into());
    fs::write(
        &wrapper,
        format!(
            r#"#!/usr/bin/env bash
set -euo pipefail
config=""
args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -config)
      config="$2"
      args+=("$1" "/work/config.json")
      shift 2
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done
if [[ -z "$config" ]]; then
  exec docker run --rm {image} "${{args[@]}}"
fi
config_dir=$(dirname "$config")
exec docker run --rm -v "$config_dir:/work:ro" {image} "${{args[@]}}"
"#
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&wrapper).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&wrapper, perms).unwrap();
    wrapper
}
