use chrono::{Duration, Utc};
use domain::*;
use ed25519_dalek::{SigningKey, VerifyingKey};
use runner::*;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn runner_verifies_signed_command_tests_config_and_switches_active_symlink() {
    let dir = tempdir().unwrap();
    let fake_xray = dir.path().join("xray-fake.sh");
    fs::write(&fake_xray, "#!/usr/bin/env bash\nset -euo pipefail\ngrep -q invalid_config \"$4\" && exit 9 || exit 0\n").unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_xray)
        .status()
        .unwrap();

    let signing = SigningKey::from_bytes(&[9u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let command = RunnerCommand::new(
        "tenant-dev",
        "node-a",
        1,
        Utc::now() + Duration::seconds(60),
        RunnerCommandKind::ApplyDeploymentPlan {
            deployment_id: "dep-1".into(),
            artifact_sha256: "sha-ok".into(),
            config_json: serde_json::json!({"inbounds":[],"outbounds":[{"protocol":"freedom"}]}),
            rollback_json: None,
        },
    );
    let envelope = SignedRunnerCommand::sign(command, &signing).unwrap();
    let mut runner = LocalRunner::new("node-a", dir.path().join("work"), fake_xray, verify);

    let result = runner.apply(envelope).await.unwrap();

    assert_eq!(result.status, DeploymentStatus::Succeeded);
    assert!(dir.path().join("work/releases/dep-1/config.json").exists());
    let active = fs::read_link(dir.path().join("work/active")).unwrap();
    assert!(active.ends_with("releases/dep-1"), "{active:?}");
}

#[tokio::test]
async fn runner_restarts_core_and_checks_process_health_after_switching_active_config() {
    let dir = tempdir().unwrap();
    let fake_xray = dir.path().join("xray-fake.sh");
    fs::write(&fake_xray, "#!/usr/bin/env bash\nexit 0\n").unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_xray)
        .status()
        .unwrap();

    let reload_log = dir.path().join("reload.log");
    let reload_cmd = dir.path().join("reload-xray.sh");
    fs::write(
        &reload_cmd,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\ntest -f \"$RUNNER_ACTIVE_CONFIG\"\necho \"reload:$RUNNER_DEPLOYMENT_ID:$RUNNER_ACTIVE_CONFIG\" >> {}\n",
            reload_log.display()
        ),
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&reload_cmd)
        .status()
        .unwrap();

    let health_log = dir.path().join("health.log");
    let health_cmd = dir.path().join("health-xray.sh");
    fs::write(
        &health_cmd,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\ntest -L \"$RUNNER_ACTIVE_DIR\"\ntest -f \"$RUNNER_ACTIVE_CONFIG\"\necho \"health:$RUNNER_DEPLOYMENT_ID\" >> {}\n",
            health_log.display()
        ),
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&health_cmd)
        .status()
        .unwrap();

    let signing = SigningKey::from_bytes(&[9u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let command = RunnerCommand::new(
        "tenant-dev",
        "node-a",
        1,
        Utc::now() + Duration::seconds(60),
        RunnerCommandKind::ApplyDeploymentPlan {
            deployment_id: "dep-reload".into(),
            artifact_sha256: "sha-reload".into(),
            config_json: serde_json::json!({"inbounds":[],"outbounds":[{"protocol":"freedom"}]}),
            rollback_json: None,
        },
    );
    let envelope = SignedRunnerCommand::sign(command, &signing).unwrap();
    let mut runner = LocalRunner::new("node-a", dir.path().join("work"), fake_xray, verify)
        .with_reload_command(reload_cmd)
        .with_health_command(health_cmd);

    let result = runner.apply(envelope).await.unwrap();

    assert_eq!(result.status, DeploymentStatus::Succeeded);
    assert!(
        result.message.contains("reload/restart command succeeded"),
        "{}",
        result.message
    );
    assert!(
        result.message.contains("process health check succeeded"),
        "{}",
        result.message
    );
    assert!(fs::read_to_string(reload_log)
        .unwrap()
        .contains("reload:dep-reload:"));
    assert_eq!(
        fs::read_to_string(health_log).unwrap(),
        "health:dep-reload\n"
    );
}

#[tokio::test]
async fn bad_config_fails_and_keeps_previous_active_release() {
    let dir = tempdir().unwrap();
    let fake_xray = dir.path().join("xray-fake.sh");
    fs::write(&fake_xray, "#!/usr/bin/env bash\nset -euo pipefail\ngrep -q invalid_config \"$4\" && exit 9 || exit 0\n").unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_xray)
        .status()
        .unwrap();

    let signing = SigningKey::from_bytes(&[10u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let mut runner = LocalRunner::new("node-a", dir.path().join("work"), fake_xray, verify);

    let good = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            1,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-good".into(),
                artifact_sha256: "sha-good".into(),
                config_json: serde_json::json!({"ok": true}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    runner.apply(good).await.unwrap();

    let bad = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            2,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-bad".into(),
                artifact_sha256: "sha-bad".into(),
                config_json: serde_json::json!({"invalid_config": true}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    let result = runner.apply(bad).await.unwrap();

    assert_eq!(result.status, DeploymentStatus::RolledBack);
    assert!(
        result
            .message
            .contains("rolled back to previous active release"),
        "{}",
        result.message
    );
    let active = fs::read_link(dir.path().join("work/active")).unwrap();
    assert!(active.ends_with("releases/dep-good"), "{active:?}");
    assert!(
        result.message.contains("xray config test failed"),
        "{}",
        result.message
    );
}

#[tokio::test]
async fn rollback_command_switches_active_to_previous_release() {
    let dir = tempdir().unwrap();
    let fake_xray = dir.path().join("xray-fake.sh");
    fs::write(&fake_xray, "#!/usr/bin/env bash\nexit 0\n").unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_xray)
        .status()
        .unwrap();

    let signing = SigningKey::from_bytes(&[23u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let mut runner = LocalRunner::new("node-a", dir.path().join("work"), fake_xray, verify);

    let first = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            1,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-first".into(),
                artifact_sha256: "sha-first".into(),
                config_json: serde_json::json!({"version": "first"}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    runner.apply(first).await.unwrap();

    let next = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            2,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-next".into(),
                artifact_sha256: "sha-next".into(),
                config_json: serde_json::json!({"version": "next"}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    runner.apply(next).await.unwrap();
    let active = fs::read_link(dir.path().join("work/active")).unwrap();
    assert!(active.ends_with("releases/dep-next"), "{active:?}");

    let rollback = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            3,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::RollbackDeployment {
                deployment_id: "dep-next".into(),
                rollback_to_deployment_id: "dep-first".into(),
                artifact_sha256: "sha-first".into(),
            },
        ),
        &signing,
    )
    .unwrap();

    let result = runner.apply(rollback).await.unwrap();

    assert_eq!(result.deployment_id, "dep-next");
    assert_eq!(result.status, DeploymentStatus::RolledBack);
    assert_eq!(result.artifact_sha256, "sha-first");
    assert!(result.message.contains("rolled back to dep-first"));
    let active = fs::read_link(dir.path().join("work/active")).unwrap();
    assert!(active.ends_with("releases/dep-first"), "{active:?}");
}

#[tokio::test]
async fn outbound_runner_polls_one_signed_command_applies_it_and_submits_result() {
    let dir = tempdir().unwrap();
    let fake_xray = dir.path().join("xray-fake.sh");
    fs::write(
        &fake_xray,
        "#!/usr/bin/env bash\nset -euo pipefail\ngrep -q invalid_config \"$4\" && exit 9 || exit 0\n",
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_xray)
        .status()
        .unwrap();

    let signing = SigningKey::from_bytes(&[12u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let envelope = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-a",
            1,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-outbound".into(),
                artifact_sha256: "sha-outbound".into(),
                config_json: serde_json::json!({"outbound": true}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    let source = RecordingCommandSource::new(vec![envelope]);
    let results = source.results_handle();
    let local = LocalRunner::new("node-a", dir.path().join("work"), fake_xray, verify);
    let mut outbound = OutboundRunner::new(local, source);

    let result = outbound.poll_apply_once().await.unwrap().unwrap();

    assert_eq!(result.status, DeploymentStatus::Succeeded);
    assert!(dir
        .path()
        .join("work/releases/dep-outbound/config.json")
        .exists());
    let submitted = results.lock().await;
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].deployment_id, "dep-outbound");
}

#[tokio::test]
async fn http_command_source_fetches_no_content_command_and_posts_result() {
    use axum::{
        extract::{Path, Query, State},
        http::HeaderMap,
        http::StatusCode,
        routing::{get, post},
        Json, Router,
    };
    use serde::Deserialize;
    use std::{collections::VecDeque, net::SocketAddr, sync::Arc};
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct TestState {
        commands: Arc<Mutex<VecDeque<SignedRunnerCommand>>>,
        results: Arc<Mutex<Vec<SignedDeploymentResult>>>,
        seen_tokens: Arc<Mutex<Vec<String>>>,
        runner_verify: VerifyingKey,
    }

    #[derive(Deserialize)]
    struct NextQuery {
        last_sequence: u64,
    }

    async fn next(
        State(state): State<TestState>,
        Path(_node_id): Path<String>,
        Query(query): Query<NextQuery>,
        headers: HeaderMap,
    ) -> Result<Json<SignedRunnerCommand>, StatusCode> {
        state.seen_tokens.lock().await.push(
            headers
                .get("x-runner-token")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned(),
        );
        let mut commands = state.commands.lock().await;
        while let Some(front) = commands.front() {
            if front.command.sequence <= query.last_sequence {
                commands.pop_front();
            } else {
                break;
            }
        }
        commands.pop_front().map(Json).ok_or(StatusCode::NO_CONTENT)
    }

    async fn result(
        State(state): State<TestState>,
        Path(node_id): Path<String>,
        headers: HeaderMap,
        Json(result): Json<SignedDeploymentResult>,
    ) -> StatusCode {
        state.seen_tokens.lock().await.push(
            headers
                .get("x-runner-token")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned(),
        );
        if result.verify(&state.runner_verify, &node_id).is_err() {
            return StatusCode::UNAUTHORIZED;
        }
        state.results.lock().await.push(result);
        StatusCode::ACCEPTED
    }

    let signing = SigningKey::from_bytes(&[13u8; 32]);
    let envelope = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-dev",
            "node-http",
            1,
            Utc::now() + Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-http".into(),
                artifact_sha256: "sha-http".into(),
                config_json: serde_json::json!({"http": true}),
                rollback_json: None,
            },
        ),
        &signing,
    )
    .unwrap();
    let state = TestState {
        commands: Arc::new(Mutex::new(VecDeque::from(vec![envelope]))),
        results: Arc::new(Mutex::new(Vec::new())),
        seen_tokens: Arc::new(Mutex::new(Vec::new())),
        runner_verify: VerifyingKey::from(&SigningKey::from_bytes(&[22u8; 32])),
    };
    let app = Router::new()
        .route("/runner/nodes/{node_id}/commands/next", get(next))
        .route("/runner/nodes/{node_id}/results", post(result))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let mut source = HttpCommandSource::new(format!("http://{addr}"))
        .with_runner_token("runner-secret")
        .with_result_signing_key(SigningKey::from_bytes(&[22u8; 32]));
    let command = source.next_command("node-http", 0).await.unwrap().unwrap();
    assert_eq!(command.command.sequence, 1);
    assert!(source.next_command("node-http", 1).await.unwrap().is_none());

    source
        .submit_result(DeploymentResult {
            deployment_id: "dep-http".into(),
            status: DeploymentStatus::Succeeded,
            message: "ok".into(),
            artifact_sha256: "sha-http".into(),
            observed_at: Utc::now(),
        })
        .await
        .unwrap();
    let results = state.results.lock().await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].node_id, "node-http");
    assert_eq!(results[0].result.deployment_id, "dep-http");
    drop(results);
    let seen_tokens = state.seen_tokens.lock().await.clone();
    assert_eq!(
        seen_tokens,
        vec!["runner-secret", "runner-secret", "runner-secret"]
    );
}

#[tokio::test]
async fn http_command_source_sends_heartbeat_with_runner_token() {
    use axum::{
        extract::{Path, State},
        http::{HeaderMap, StatusCode},
        routing::post,
        Json, Router,
    };
    use serde::Deserialize;
    use std::{net::SocketAddr, sync::Arc};
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct HeartbeatState {
        seen: Arc<Mutex<Vec<(String, String, serde_json::Value)>>>,
    }

    #[derive(Deserialize)]
    struct HeartbeatBody {
        capability_snapshot: serde_json::Value,
    }

    async fn heartbeat(
        State(state): State<HeartbeatState>,
        Path(node_id): Path<String>,
        headers: HeaderMap,
        Json(body): Json<HeartbeatBody>,
    ) -> StatusCode {
        let token = headers
            .get("x-runner-token")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        state
            .seen
            .lock()
            .await
            .push((node_id, token, body.capability_snapshot));
        StatusCode::ACCEPTED
    }

    let state = HeartbeatState {
        seen: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/runner/nodes/{node_id}/heartbeat", post(heartbeat))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let source =
        HttpCommandSource::new(format!("http://{addr}")).with_runner_token("runner-secret");
    source
        .send_heartbeat("node-http", serde_json::json!({"xray_version":"1.8.8"}))
        .await
        .unwrap();

    let seen = state.seen.lock().await.clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "node-http");
    assert_eq!(seen[0].1, "runner-secret");
    assert_eq!(seen[0].2["xray_version"], "1.8.8");
}

#[tokio::test]
async fn http_command_source_registers_node_with_registration_token_and_result_public_key() {
    use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
    use serde::Deserialize;
    use std::{net::SocketAddr, sync::Arc};
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct RegistrationState {
        seen: Arc<Mutex<Vec<RegisterBody>>>,
    }

    #[derive(Clone, Deserialize)]
    struct RegisterBody {
        registration_token: String,
        node_id: String,
        xray_version: String,
        runner_result_public_key_hex: String,
    }

    async fn register(
        State(state): State<RegistrationState>,
        Json(body): Json<RegisterBody>,
    ) -> StatusCode {
        state.seen.lock().await.push(body);
        StatusCode::CREATED
    }

    let state = RegistrationState {
        seen: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/nodes/register", post(register))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let signing_key = SigningKey::from_bytes(&[44u8; 32]);
    let source = HttpCommandSource::new(format!("http://{addr}"))
        .with_result_signing_key(signing_key.clone());

    source
        .register_node("node-http", "dev-registration-token", "26.3.27")
        .await
        .unwrap();

    let seen = state.seen.lock().await.clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].registration_token, "dev-registration-token");
    assert_eq!(seen[0].node_id, "node-http");
    assert_eq!(seen[0].xray_version, "26.3.27");
    assert_eq!(
        seen[0].runner_result_public_key_hex,
        hex::encode(VerifyingKey::from(&signing_key).to_bytes())
    );
}

#[test]
fn parses_runner_result_signing_key_from_hex() {
    let key_hex = hex::encode([31u8; 32]);
    let signing = runner_result_signing_key_from_hex(&key_hex).unwrap();
    assert_eq!(VerifyingKey::from(&signing).to_bytes().len(), 32);

    let err = runner_result_signing_key_from_hex("abcd")
        .unwrap_err()
        .to_string();
    assert!(err.contains("32 bytes"), "{err}");
}
