use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("secret references must not contain inline material: {0}")]
    InlineSecretRef(String),
    #[error("runner command expired")]
    CommandExpired,
    #[error("runner command node scope mismatch")]
    NodeScope,
    #[error("runner command sequence must be greater than previous sequence")]
    Sequence,
    #[error("runner command nonce replay")]
    NonceReplay,
    #[error("runner command signature invalid")]
    Signature,
    #[error("canonical serialization failed: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    ProfileIR,
    ProfilePatch,
    CompiledXrayConfig,
    CompiledSingBoxConfig,
    SubscriptionArtifact,
    ValidationReport,
    DeploymentPlan,
    DeploymentResult,
    RollbackPointer,
    DiagnosisReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedactionStatus {
    ContainsNoSecrets,
    Redacted,
    SealedContainsSecrets,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub tenant_id: String,
    pub kind: ArtifactKind,
    pub schema_version: String,
    pub media_type: String,
    pub sha256: String,
    pub storage_uri: String,
    pub redaction_status: RedactionStatus,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

impl Artifact {
    pub fn from_bytes(
        tenant_id: impl Into<String>,
        kind: ArtifactKind,
        media_type: impl Into<String>,
        bytes: &[u8],
        created_by: impl Into<String>,
    ) -> Self {
        let sha256 = hex::encode(Sha256::digest(bytes));
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.into(),
            kind,
            schema_version: "0.1".into(),
            media_type: media_type.into(),
            storage_uri: format!("artifact://sha256/{sha256}"),
            sha256,
            redaction_status: RedactionStatus::ContainsNoSecrets,
            created_by: created_by.into(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretKind {
    ClientUuid,
    ShadowsocksPassword,
    TrojanPassword,
    RealityPrivateKey,
    SubscriptionTokenSecret,
    RunnerNodeKey,
    ModelProviderApiKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRef {
    pub id: String,
    pub kind: SecretKind,
}

impl SecretRef {
    pub fn new(id: impl Into<String>, kind: SecretKind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }

    pub fn validate_reference_only(&self) -> Result<(), DomainError> {
        let lowered = self.id.to_ascii_lowercase();
        if lowered.starts_with("inline:") || lowered.contains("private_key") && self.id.len() > 48 {
            return Err(DomainError::InlineSecretRef(self.id.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runtime {
    pub core: String,
    pub core_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InboundProtocol {
    Vless,
    Shadowsocks,
    Trojan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Security {
    Reality {
        server_name: String,
        private_key_ref: SecretRef,
        short_ids: Vec<String>,
    },
    Tls {
        server_name: String,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inbound {
    pub id: String,
    pub protocol: InboundProtocol,
    pub listen: String,
    pub port: u16,
    pub security: Security,
    pub client_group_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientGroup {
    pub id: String,
    pub credential_policy: String,
    pub quota_policy_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsConfig {
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileIr {
    pub schema_version: String,
    pub runtime: Runtime,
    pub inbounds: Vec<Inbound>,
    pub client_groups: Vec<ClientGroup>,
    #[serde(default)]
    pub routes: Vec<serde_json::Value>,
    pub dns: DnsConfig,
}

impl ProfileIr {
    pub fn vless_reality_example(group_id: &str, private_key_ref: &str) -> Self {
        Self {
            schema_version: "0.1".into(),
            runtime: Runtime {
                core: "xray".into(),
                core_version: "1.x".into(),
            },
            inbounds: vec![Inbound {
                id: "in_reality_443".into(),
                protocol: InboundProtocol::Vless,
                listen: "0.0.0.0".into(),
                port: 443,
                security: Security::Reality {
                    server_name: "example.com".into(),
                    private_key_ref: SecretRef::new(private_key_ref, SecretKind::RealityPrivateKey),
                    short_ids: vec!["abcd".into()],
                },
                client_group_refs: vec![group_id.into()],
            }],
            client_groups: vec![ClientGroup {
                id: group_id.into(),
                credential_policy: "vless_uuid".into(),
                quota_policy_ref: None,
            }],
            routes: vec![],
            dns: DnsConfig {
                mode: "system".into(),
            },
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        for inbound in &self.inbounds {
            if let Security::Reality {
                private_key_ref, ..
            } = &inbound.security
            {
                private_key_ref.validate_reference_only()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CredentialMaterial {
    VlessUuid { uuid: String },
    ShadowsocksPassword { method: String, password: String },
    TrojanPassword { password: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub client_group_id: String,
    pub display_name: String,
    pub status: CredentialStatus,
    pub material: CredentialMaterial,
}

impl Credential {
    pub fn active_vless(id: &str, group: &str, uuid: &str, display_name: &str) -> Self {
        Self {
            id: id.into(),
            client_group_id: group.into(),
            display_name: display_name.into(),
            status: CredentialStatus::Active,
            material: CredentialMaterial::VlessUuid { uuid: uuid.into() },
        }
    }

    pub fn revoked_vless(id: &str, group: &str, uuid: &str, display_name: &str) -> Self {
        let mut c = Self::active_vless(id, group, uuid, display_name);
        c.status = CredentialStatus::Revoked;
        c
    }

    pub fn is_active(&self) -> bool {
        self.status == CredentialStatus::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunnerCommand {
    pub tenant_id: String,
    pub node_id: String,
    pub sequence: u64,
    pub nonce: String,
    pub expires_at: DateTime<Utc>,
    pub kind: RunnerCommandKind,
}

impl RunnerCommand {
    pub fn new(
        tenant_id: impl Into<String>,
        node_id: impl Into<String>,
        sequence: u64,
        expires_at: DateTime<Utc>,
        kind: RunnerCommandKind,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            node_id: node_id.into(),
            sequence,
            nonce: Uuid::new_v4().to_string(),
            expires_at,
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum RunnerCommandKind {
    RegisterNode {
        registration_token: String,
    },
    Heartbeat {
        capability_snapshot: serde_json::Value,
    },
    ApplyDeploymentPlan {
        deployment_id: String,
        artifact_sha256: String,
        config_json: serde_json::Value,
        rollback_json: Option<serde_json::Value>,
    },
    ValidateConfig {
        config_json: serde_json::Value,
    },
    ReloadCore,
    RestartCore,
    RollbackDeployment {
        deployment_id: String,
        rollback_to_deployment_id: String,
        artifact_sha256: String,
    },
    CollectMetrics,
    CollectLogWindow {
        lines: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedRunnerCommand {
    pub command: RunnerCommand,
    pub signature_hex: String,
}

impl SignedRunnerCommand {
    pub fn sign(command: RunnerCommand, signing_key: &SigningKey) -> Result<Self, DomainError> {
        let canonical =
            serde_json::to_vec(&command).map_err(|e| DomainError::Serialization(e.to_string()))?;
        let signature = signing_key.sign(&canonical);
        Ok(Self {
            command,
            signature_hex: hex::encode(signature.to_bytes()),
        })
    }

    pub fn verify(
        &self,
        verifying_key: &VerifyingKey,
        expected_node_id: &str,
        last_sequence: u64,
        seen_nonces: &mut HashSet<String>,
        now: DateTime<Utc>,
    ) -> Result<RunnerCommand, DomainError> {
        if self.command.node_id != expected_node_id {
            return Err(DomainError::NodeScope);
        }
        if self.command.expires_at <= now {
            return Err(DomainError::CommandExpired);
        }
        if self.command.sequence <= last_sequence {
            return Err(DomainError::Sequence);
        }
        if !seen_nonces.insert(self.command.nonce.clone()) {
            return Err(DomainError::NonceReplay);
        }
        let sig_bytes: [u8; 64] = hex::decode(&self.signature_hex)
            .map_err(|_| DomainError::Signature)?
            .try_into()
            .map_err(|_| DomainError::Signature)?;
        let sig = Signature::from_bytes(&sig_bytes);
        let canonical = serde_json::to_vec(&self.command)
            .map_err(|e| DomainError::Serialization(e.to_string()))?;
        verifying_key
            .verify(&canonical, &sig)
            .map_err(|_| DomainError::Signature)?;
        Ok(self.command.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Succeeded,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentResult {
    pub deployment_id: String,
    pub status: DeploymentStatus,
    pub message: String,
    pub artifact_sha256: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedDeploymentResult {
    pub node_id: String,
    pub result: DeploymentResult,
    pub signature_hex: String,
}

impl SignedDeploymentResult {
    pub fn sign(
        node_id: impl Into<String>,
        result: DeploymentResult,
        signing_key: &SigningKey,
    ) -> Result<Self, DomainError> {
        let node_id = node_id.into();
        let canonical = serde_json::to_vec(&(node_id.as_str(), &result))
            .map_err(|e| DomainError::Serialization(e.to_string()))?;
        let signature = signing_key.sign(&canonical);
        Ok(Self {
            node_id,
            result,
            signature_hex: hex::encode(signature.to_bytes()),
        })
    }

    pub fn verify(
        &self,
        verifying_key: &VerifyingKey,
        expected_node_id: &str,
    ) -> Result<DeploymentResult, DomainError> {
        if self.node_id != expected_node_id {
            return Err(DomainError::NodeScope);
        }
        let sig_bytes: [u8; 64] = hex::decode(&self.signature_hex)
            .map_err(|_| DomainError::Signature)?
            .try_into()
            .map_err(|_| DomainError::Signature)?;
        let sig = Signature::from_bytes(&sig_bytes);
        let canonical = serde_json::to_vec(&(self.node_id.as_str(), &self.result))
            .map_err(|e| DomainError::Serialization(e.to_string()))?;
        verifying_key
            .verify(&canonical, &sig)
            .map_err(|_| DomainError::Signature)?;
        Ok(self.result.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentPlan {
    pub target_node_id: String,
    pub target_profile_version_id: String,
    pub compiled_config_artifact_id: String,
    pub core_kind: String,
    pub core_version: String,
    pub assets_version: String,
    pub rollback_pointer_id: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeployedProfile {
    pub profile_id: String,
    pub host: String,
    pub port: u16,
    pub reality_server_name: String,
    pub credentials: Vec<Credential>,
}

impl DeployedProfile {
    pub fn new(profile_id: &str, host: &str, port: u16, reality_server_name: &str) -> Self {
        Self {
            profile_id: profile_id.into(),
            host: host.into(),
            port,
            reality_server_name: reality_server_name.into(),
            credentials: vec![],
        }
    }

    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credentials.push(credential);
        self
    }
}
