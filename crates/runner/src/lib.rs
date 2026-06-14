use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use domain::{
    DeploymentResult, DeploymentStatus, RunnerCommandKind, SignedDeploymentResult,
    SignedRunnerCommand,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Mutex;

pub struct LocalRunner {
    node_id: String,
    work_dir: PathBuf,
    xray_bin: PathBuf,
    control_plane_key: VerifyingKey,
    last_sequence: u64,
    seen_nonces: HashSet<String>,
    reload_command: Option<PathBuf>,
    health_command: Option<PathBuf>,
}

impl LocalRunner {
    pub fn new(
        node_id: impl Into<String>,
        work_dir: PathBuf,
        xray_bin: PathBuf,
        control_plane_key: VerifyingKey,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            work_dir,
            xray_bin,
            control_plane_key,
            last_sequence: 0,
            seen_nonces: HashSet::new(),
            reload_command: None,
            health_command: None,
        }
    }

    pub fn with_reload_command(mut self, command: PathBuf) -> Self {
        self.reload_command = Some(command);
        self
    }

    pub fn with_health_command(mut self, command: PathBuf) -> Self {
        self.health_command = Some(command);
        self
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub async fn apply(&mut self, envelope: SignedRunnerCommand) -> Result<DeploymentResult> {
        let command = envelope.verify(
            &self.control_plane_key,
            &self.node_id,
            self.last_sequence,
            &mut self.seen_nonces,
            Utc::now(),
        )?;
        self.last_sequence = command.sequence;
        match command.kind {
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id,
                artifact_sha256,
                config_json,
                rollback_json: _,
            } => {
                self.apply_deployment(&deployment_id, &artifact_sha256, &config_json)
                    .await
            }
            RunnerCommandKind::RollbackDeployment {
                deployment_id,
                rollback_to_deployment_id,
                artifact_sha256,
            } => {
                self.rollback_deployment(
                    &deployment_id,
                    &rollback_to_deployment_id,
                    &artifact_sha256,
                )
                .await
            }
            _ => Err(anyhow!("unsupported local runner command for apply()")),
        }
    }

    async fn apply_deployment(
        &self,
        deployment_id: &str,
        artifact_sha256: &str,
        config_json: &serde_json::Value,
    ) -> Result<DeploymentResult> {
        let release_dir = self.work_dir.join("releases").join(deployment_id);
        let tmp_dir = self.work_dir.join("tmp").join(deployment_id);
        fs::create_dir_all(&tmp_dir).await?;
        let config_path = tmp_dir.join("config.json");
        let rendered = serde_json::to_vec_pretty(config_json)?;
        fs::write(&config_path, rendered).await?;

        let output = Command::new(&self.xray_bin)
            .arg("run")
            .arg("-test")
            .arg("-config")
            .arg(&config_path)
            .output()
            .await?;

        if !output.status.success() {
            let active = self.work_dir.join("active");
            let rolled_back = fs::symlink_metadata(&active).await.is_ok();
            let message = if rolled_back {
                format!(
                    "xray config test failed: status={} stderr={}; rolled back to previous active release",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )
            } else {
                format!(
                    "xray config test failed: status={} stderr={}; no previous active release available",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                )
            };
            return Ok(DeploymentResult {
                deployment_id: deployment_id.into(),
                status: if rolled_back {
                    DeploymentStatus::RolledBack
                } else {
                    DeploymentStatus::Failed
                },
                message,
                artifact_sha256: artifact_sha256.into(),
                observed_at: Utc::now(),
            });
        }

        if release_dir.exists() {
            fs::remove_dir_all(&release_dir).await?;
        }
        fs::create_dir_all(release_dir.parent().expect("release parent")).await?;
        fs::rename(&tmp_dir, &release_dir).await?;
        let previous_active = fs::read_link(self.work_dir.join("active")).await.ok();
        self.switch_active(&release_dir).await?;

        if let Some(command) = &self.reload_command {
            if let Err(error) = self.run_core_command(command, deployment_id).await {
                return self
                    .rollback_after_apply_failure(
                        deployment_id,
                        artifact_sha256,
                        previous_active,
                        format!("xray reload/restart command failed: {error}"),
                    )
                    .await;
            }
        }
        if let Some(command) = &self.health_command {
            if let Err(error) = self.run_core_command(command, deployment_id).await {
                return self
                    .rollback_after_apply_failure(
                        deployment_id,
                        artifact_sha256,
                        previous_active,
                        format!("xray process health check failed: {error}"),
                    )
                    .await;
            }
        }

        let reload_message = if self.reload_command.is_some() {
            "reload/restart command succeeded"
        } else {
            "reload/restart command not configured"
        };
        let health_message = if self.health_command.is_some() {
            "process health check succeeded"
        } else {
            "process health check not configured"
        };
        Ok(DeploymentResult {
            deployment_id: deployment_id.into(),
            status: DeploymentStatus::Succeeded,
            message: format!(
                "deployment applied, active config switched, {reload_message}, {health_message}"
            ),
            artifact_sha256: artifact_sha256.into(),
            observed_at: Utc::now(),
        })
    }

    async fn rollback_deployment(
        &self,
        deployment_id: &str,
        rollback_to_deployment_id: &str,
        artifact_sha256: &str,
    ) -> Result<DeploymentResult> {
        let release_dir = self
            .work_dir
            .join("releases")
            .join(rollback_to_deployment_id);
        if fs::symlink_metadata(&release_dir).await.is_err() {
            return Ok(DeploymentResult {
                deployment_id: deployment_id.into(),
                status: DeploymentStatus::Failed,
                message: format!("rollback target release not found: {rollback_to_deployment_id}"),
                artifact_sha256: artifact_sha256.into(),
                observed_at: Utc::now(),
            });
        }
        self.switch_active(&release_dir).await?;
        Ok(DeploymentResult {
            deployment_id: deployment_id.into(),
            status: DeploymentStatus::RolledBack,
            message: format!("deployment rolled back to {rollback_to_deployment_id}"),
            artifact_sha256: artifact_sha256.into(),
            observed_at: Utc::now(),
        })
    }

    async fn switch_active(&self, release_dir: &PathBuf) -> Result<()> {
        let active = self.work_dir.join("active");
        let next = self.work_dir.join("active.next");
        if next.exists() || fs::symlink_metadata(&next).await.is_ok() {
            let _ = fs::remove_file(&next).await;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(release_dir, &next)?;
        #[cfg(not(unix))]
        std::os::windows::fs::symlink_dir(release_dir, &next)?;
        if active.exists() || fs::symlink_metadata(&active).await.is_ok() {
            fs::remove_file(&active).await?;
        }
        fs::rename(next, active).await?;
        Ok(())
    }

    async fn run_core_command(&self, command_path: &PathBuf, deployment_id: &str) -> Result<()> {
        let active_dir = self.work_dir.join("active");
        let active_config = active_dir.join("config.json");
        let output = Command::new(command_path)
            .env("RUNNER_NODE_ID", &self.node_id)
            .env("RUNNER_DEPLOYMENT_ID", deployment_id)
            .env("RUNNER_ACTIVE_DIR", &active_dir)
            .env("RUNNER_ACTIVE_CONFIG", &active_config)
            .output()
            .await?;
        if !output.status.success() {
            return Err(anyhow!(
                "status={} stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    async fn rollback_after_apply_failure(
        &self,
        deployment_id: &str,
        artifact_sha256: &str,
        previous_active: Option<PathBuf>,
        failure: String,
    ) -> Result<DeploymentResult> {
        let active = self.work_dir.join("active");
        let (status, message) = if let Some(previous_active) = previous_active {
            self.switch_active(&previous_active).await?;
            (
                DeploymentStatus::RolledBack,
                format!("{failure}; rolled back to previous active release"),
            )
        } else {
            if fs::symlink_metadata(&active).await.is_ok() {
                let _ = fs::remove_file(&active).await;
            }
            (
                DeploymentStatus::Failed,
                format!("{failure}; no previous active release available"),
            )
        };
        Ok(DeploymentResult {
            deployment_id: deployment_id.into(),
            status,
            message,
            artifact_sha256: artifact_sha256.into(),
            observed_at: Utc::now(),
        })
    }
}

#[async_trait]
pub trait RunnerCommandSource {
    async fn next_command(
        &mut self,
        node_id: &str,
        last_sequence: u64,
    ) -> Result<Option<SignedRunnerCommand>>;
    async fn submit_result(&mut self, result: DeploymentResult) -> Result<()>;
}

pub struct OutboundRunner<S> {
    local: LocalRunner,
    source: S,
}

impl<S> OutboundRunner<S>
where
    S: RunnerCommandSource + Send,
{
    pub fn new(local: LocalRunner, source: S) -> Self {
        Self { local, source }
    }

    pub async fn poll_apply_once(&mut self) -> Result<Option<DeploymentResult>> {
        let node_id = self.local.node_id().to_owned();
        let last_sequence = self.local.last_sequence();
        let Some(command) = self.source.next_command(&node_id, last_sequence).await? else {
            return Ok(None);
        };
        let result = self.local.apply(command).await?;
        self.source.submit_result(result.clone()).await?;
        Ok(Some(result))
    }
}

pub struct RecordingCommandSource {
    commands: VecDeque<SignedRunnerCommand>,
    results: Arc<Mutex<Vec<DeploymentResult>>>,
}

impl RecordingCommandSource {
    pub fn new(commands: Vec<SignedRunnerCommand>) -> Self {
        Self {
            commands: commands.into(),
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn results_handle(&self) -> Arc<Mutex<Vec<DeploymentResult>>> {
        Arc::clone(&self.results)
    }
}

#[async_trait]
impl RunnerCommandSource for RecordingCommandSource {
    async fn next_command(
        &mut self,
        _node_id: &str,
        last_sequence: u64,
    ) -> Result<Option<SignedRunnerCommand>> {
        while let Some(front) = self.commands.front() {
            if front.command.sequence <= last_sequence {
                self.commands.pop_front();
            } else {
                break;
            }
        }
        Ok(self.commands.pop_front())
    }

    async fn submit_result(&mut self, result: DeploymentResult) -> Result<()> {
        self.results.lock().await.push(result);
        Ok(())
    }
}

pub struct HttpCommandSource {
    base_url: String,
    client: reqwest::Client,
    runner_token: Option<String>,
    last_node_id: Option<String>,
    result_signing_key: SigningKey,
}

impl HttpCommandSource {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            client: reqwest::Client::new(),
            runner_token: None,
            last_node_id: None,
            result_signing_key: dev_runner_result_signing_key(),
        }
    }

    pub fn with_runner_token(mut self, token: impl Into<String>) -> Self {
        self.runner_token = Some(token.into());
        self
    }

    pub fn with_result_signing_key(mut self, signing_key: SigningKey) -> Self {
        self.result_signing_key = signing_key;
        self
    }

    fn add_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.runner_token {
            Some(token) => request.header("x-runner-token", token),
            None => request,
        }
    }

    fn next_url(&self, node_id: &str, last_sequence: u64) -> String {
        format!(
            "{}/runner/nodes/{}/commands/next?last_sequence={}",
            self.base_url, node_id, last_sequence
        )
    }

    fn result_url(&self, node_id: &str) -> String {
        format!("{}/runner/nodes/{}/results", self.base_url, node_id)
    }

    fn heartbeat_url(&self, node_id: &str) -> String {
        format!("{}/runner/nodes/{}/heartbeat", self.base_url, node_id)
    }

    fn register_url(&self) -> String {
        format!("{}/nodes/register", self.base_url)
    }

    pub async fn register_node(
        &self,
        node_id: &str,
        registration_token: &str,
        xray_version: &str,
    ) -> Result<()> {
        let runner_result_public_key_hex =
            hex::encode(VerifyingKey::from(&self.result_signing_key).to_bytes());
        let response = self
            .client
            .post(self.register_url())
            .json(&serde_json::json!({
                "registration_token": registration_token,
                "node_id": node_id,
                "xray_version": xray_version,
                "runner_result_public_key_hex": runner_result_public_key_hex,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "control-plane node registration failed: status={}",
                response.status()
            ));
        }
        Ok(())
    }

    pub async fn send_heartbeat(
        &self,
        node_id: &str,
        capability_snapshot: serde_json::Value,
    ) -> Result<()> {
        let response = self
            .add_auth(
                self.client
                    .post(self.heartbeat_url(node_id))
                    .json(&serde_json::json!({"capability_snapshot": capability_snapshot})),
            )
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "control-plane heartbeat failed: status={}",
                response.status()
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl RunnerCommandSource for HttpCommandSource {
    async fn next_command(
        &mut self,
        node_id: &str,
        last_sequence: u64,
    ) -> Result<Option<SignedRunnerCommand>> {
        self.last_node_id = Some(node_id.to_owned());
        let response = self
            .add_auth(self.client.get(self.next_url(node_id, last_sequence)))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(anyhow!(
                "control-plane command poll failed: status={}",
                response.status()
            ));
        }
        Ok(Some(response.json::<SignedRunnerCommand>().await?))
    }

    async fn submit_result(&mut self, result: DeploymentResult) -> Result<()> {
        let node_id = self.last_node_id.clone().unwrap_or_else(|| {
            std::env::var("RUNNER_NODE_ID").unwrap_or_else(|_| "dev-node-1".into())
        });
        let signed_result =
            SignedDeploymentResult::sign(&node_id, result, &self.result_signing_key)?;
        let response = self
            .add_auth(
                self.client
                    .post(self.result_url(&node_id))
                    .json(&signed_result),
            )
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(anyhow!(
                "control-plane result submit failed: status={}",
                response.status()
            ));
        }
        Ok(())
    }
}

fn dev_runner_result_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[22u8; 32])
}

pub fn runner_result_signing_key_from_hex(raw: &str) -> Result<SigningKey> {
    let bytes = hex::decode(raw)?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("runner result signing key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&bytes))
}
