use ed25519_dalek::{SigningKey, VerifyingKey};
use runner::{runner_result_signing_key_from_hex, HttpCommandSource, LocalRunner, OutboundRunner};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let node_id = std::env::var("RUNNER_NODE_ID").unwrap_or_else(|_| "dev-node-1".into());
    let work_dir =
        PathBuf::from(std::env::var("RUNNER_WORK_DIR").unwrap_or_else(|_| ".data/runner".into()));
    let xray_bin =
        PathBuf::from(std::env::var("RUNNER_XRAY_BIN").unwrap_or_else(|_| "xray".into()));
    let Some(control_plane_base_url) = std::env::var("CONTROL_PLANE_BASE_URL").ok() else {
        println!(
            "runner {node_id} ready; work_dir={} xray_bin={} (set CONTROL_PLANE_BASE_URL to start outbound polling)",
            work_dir.display(),
            xray_bin.display()
        );
        return Ok(());
    };

    let verify_key = dev_control_plane_verify_key();
    let mut local = LocalRunner::new(
        node_id.clone(),
        work_dir.clone(),
        xray_bin.clone(),
        verify_key,
    );
    if let Ok(command) = std::env::var("RUNNER_XRAY_RELOAD_CMD") {
        local = local.with_reload_command(PathBuf::from(command));
    }
    if let Ok(command) = std::env::var("RUNNER_XRAY_HEALTH_CMD") {
        local = local.with_health_command(PathBuf::from(command));
    }
    let mut source = HttpCommandSource::new(control_plane_base_url.clone());
    if let Ok(token) = std::env::var("RUNNER_API_TOKEN") {
        source = source.with_runner_token(token);
    }
    if let Ok(key_hex) = std::env::var("RUNNER_RESULT_SIGNING_KEY_HEX") {
        source = source.with_result_signing_key(runner_result_signing_key_from_hex(&key_hex)?);
    }
    if let Ok(registration_token) = std::env::var("NODE_REGISTRATION_TOKEN") {
        let xray_version =
            std::env::var("RUNNER_XRAY_VERSION").unwrap_or_else(|_| "unknown".into());
        source
            .register_node(&node_id, &registration_token, &xray_version)
            .await?;
    }
    source
        .send_heartbeat(
            &node_id,
            serde_json::json!({
                "xray_bin": xray_bin.display().to_string(),
                "work_dir": work_dir.display().to_string(),
                "runner_version": env!("CARGO_PKG_VERSION"),
            }),
        )
        .await?;
    let mut outbound = OutboundRunner::new(local, source);
    let once = std::env::var("RUNNER_ONCE")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let interval = std::env::var("RUNNER_POLL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1_000);

    println!(
        "runner {node_id} polling {control_plane_base_url}; work_dir={} xray_bin={}",
        work_dir.display(),
        xray_bin.display()
    );
    loop {
        match outbound.poll_apply_once().await? {
            Some(result) => println!("deployment {} -> {:?}", result.deployment_id, result.status),
            None => println!("no runner command available for {node_id}"),
        }
        if once {
            break;
        }
        tokio::time::sleep(Duration::from_millis(interval)).await;
    }
    Ok(())
}

fn dev_control_plane_verify_key() -> VerifyingKey {
    VerifyingKey::from(&SigningKey::from_bytes(&[11u8; 32]))
}
