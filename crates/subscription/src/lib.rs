use anyhow::Result;
use base64::Engine;
use domain::{Artifact, ArtifactKind, CredentialMaterial, DeployedProfile};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

#[derive(Debug, Clone)]
pub struct SubscriptionArtifact {
    pub artifact: Artifact,
    pub body_base64: String,
}

impl SubscriptionArtifact {
    pub fn body_base64_decoded(&self) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.body_base64)
            .unwrap_or_default()
    }
}

pub fn generate_subscription_artifact(
    tenant_id: &str,
    profile: &DeployedProfile,
    actor: &str,
) -> Result<SubscriptionArtifact> {
    let mut links = Vec::new();
    for credential in profile.credentials.iter().filter(|c| c.is_active()) {
        match &credential.material {
            CredentialMaterial::VlessUuid { uuid } => {
                let name =
                    utf8_percent_encode(&credential.display_name, NON_ALPHANUMERIC).to_string();
                links.push(format!(
                    "vless://{}@{}:{}?encryption=none&flow=xtls-rprx-vision&security=reality&sni={}&fp=chrome&type=tcp#{}",
                    uuid, profile.host, profile.port, profile.reality_server_name, name
                ));
            }
            CredentialMaterial::TrojanPassword { password } => {
                let name =
                    utf8_percent_encode(&credential.display_name, NON_ALPHANUMERIC).to_string();
                links.push(format!(
                    "trojan://{}@{}:{}?security=tls&sni={}#{}",
                    password, profile.host, profile.port, profile.reality_server_name, name
                ));
            }
            CredentialMaterial::ShadowsocksPassword { method, password } => {
                let userinfo = base64::engine::general_purpose::STANDARD_NO_PAD
                    .encode(format!("{method}:{password}"));
                let name =
                    utf8_percent_encode(&credential.display_name, NON_ALPHANUMERIC).to_string();
                links.push(format!(
                    "ss://{}@{}:{}#{}",
                    userinfo, profile.host, profile.port, name
                ));
            }
        }
    }
    let plaintext = links.join("\n");
    let body_base64 = base64::engine::general_purpose::STANDARD.encode(plaintext.as_bytes());
    let artifact = Artifact::from_bytes(
        tenant_id,
        ArtifactKind::SubscriptionArtifact,
        "text/plain+base64",
        body_base64.as_bytes(),
        actor,
    );
    Ok(SubscriptionArtifact {
        artifact,
        body_base64,
    })
}
