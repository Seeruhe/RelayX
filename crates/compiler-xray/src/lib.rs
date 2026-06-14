use base64::Engine;
use domain::{
    Credential, CredentialMaterial, CredentialStatus, InboundProtocol, ProfileIr, Security,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Profile IR invalid: {0}")]
    InvalidIr(#[from] domain::DomainError),
    #[error("REALITY requires xray-core >= 1.8.0, got {0}")]
    UnsupportedReality(String),
    #[error("missing secret material for {0}")]
    MissingSecret(String),
    #[error("missing active credential for inbound {0}")]
    MissingCredential(String),
    #[error("mixed shadowsocks methods for inbound {0}")]
    MixedShadowsocksMethods(String),
    #[error("{method} requires a base64-encoded {expected_bytes}-byte psk")]
    InvalidShadowsocks2022Psk {
        method: String,
        expected_bytes: usize,
    },
    #[error("unsupported inbound protocol for this compiler")]
    UnsupportedProtocol,
}

#[derive(Debug, Clone)]
pub struct CompileContext {
    pub core_version: String,
    pub secrets: HashMap<String, String>,
    pub credentials: Vec<Credential>,
}

impl CompileContext {
    pub fn new(core_version: &str) -> Self {
        Self {
            core_version: core_version.into(),
            secrets: HashMap::new(),
            credentials: vec![],
        }
    }

    pub fn with_secret(mut self, id: &str, value: &str) -> Self {
        self.secrets.insert(id.into(), value.into());
        self
    }

    pub fn with_credential(mut self, credential: Credential) -> Self {
        self.credentials.push(credential);
        self
    }

    pub fn with_core_version(mut self, core_version: &str) -> Self {
        self.core_version = core_version.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct CompiledXrayConfig {
    pub config_json: Value,
    pub compiler_version: String,
}

pub fn compile_profile_to_xray(
    ir: &ProfileIr,
    ctx: &CompileContext,
) -> Result<CompiledXrayConfig, CompileError> {
    ir.validate()?;
    let mut inbounds = Vec::new();

    for inbound in &ir.inbounds {
        match inbound.protocol {
            InboundProtocol::Vless => {
                let mut clients = Vec::new();
                for group in &inbound.client_group_refs {
                    for credential in ctx.credentials.iter().filter(|c| {
                        c.client_group_id == *group && c.status == CredentialStatus::Active
                    }) {
                        if let CredentialMaterial::VlessUuid { uuid } = &credential.material {
                            clients.push(json!({
                                "id": uuid,
                                "email": credential.display_name,
                                "flow": "xtls-rprx-vision"
                            }));
                        }
                    }
                }

                let mut inbound_json = json!({
                    "tag": inbound.id,
                    "listen": inbound.listen,
                    "port": inbound.port,
                    "protocol": "vless",
                    "settings": {
                        "clients": clients,
                        "decryption": "none"
                    }
                });

                match &inbound.security {
                    Security::Reality {
                        server_name,
                        private_key_ref,
                        short_ids,
                    } => {
                        if !supports_reality(&ctx.core_version) {
                            return Err(CompileError::UnsupportedReality(ctx.core_version.clone()));
                        }
                        let private_key =
                            ctx.secrets.get(&private_key_ref.id).ok_or_else(|| {
                                CompileError::MissingSecret(private_key_ref.id.clone())
                            })?;
                        inbound_json["streamSettings"] = json!({
                            "network": "tcp",
                            "security": "reality",
                            "realitySettings": {
                                "show": false,
                                "dest": format!("{server_name}:443"),
                                "serverNames": [server_name],
                                "privateKey": private_key,
                                "shortIds": short_ids
                            }
                        });
                    }
                    Security::Tls { server_name } => {
                        inbound_json["streamSettings"] = json!({"network":"tcp","security":"tls","tlsSettings":{"serverName": server_name}});
                    }
                    Security::None => {}
                }
                inbounds.push(inbound_json);
            }
            InboundProtocol::Shadowsocks | InboundProtocol::Trojan => {
                let inbound_json = match inbound.protocol {
                    InboundProtocol::Shadowsocks => {
                        let mut selected = None;
                        for group in &inbound.client_group_refs {
                            for credential in ctx.credentials.iter().filter(|c| {
                                c.client_group_id == *group && c.status == CredentialStatus::Active
                            }) {
                                if let CredentialMaterial::ShadowsocksPassword {
                                    method,
                                    password,
                                } = &credential.material
                                {
                                    match &selected {
                                        Some((existing_method, _)) if existing_method != method => {
                                            return Err(CompileError::MixedShadowsocksMethods(
                                                inbound.id.clone(),
                                            ));
                                        }
                                        Some(_) => {}
                                        None => {
                                            selected = Some((method.clone(), password.clone()));
                                        }
                                    }
                                }
                            }
                        }
                        let (method, password) = selected
                            .ok_or_else(|| CompileError::MissingCredential(inbound.id.clone()))?;
                        validate_shadowsocks_method_password(&method, &password)?;
                        json!({
                            "tag": inbound.id,
                            "listen": inbound.listen,
                            "port": inbound.port,
                            "protocol": "shadowsocks",
                            "settings": {
                                "method": method,
                                "password": password,
                                "network": "tcp,udp"
                            }
                        })
                    }
                    InboundProtocol::Trojan => {
                        let mut clients = Vec::new();
                        for group in &inbound.client_group_refs {
                            for credential in ctx.credentials.iter().filter(|c| {
                                c.client_group_id == *group && c.status == CredentialStatus::Active
                            }) {
                                if let CredentialMaterial::TrojanPassword { password } =
                                    &credential.material
                                {
                                    clients.push(json!({
                                        "password": password,
                                        "email": credential.display_name
                                    }));
                                }
                            }
                        }
                        if clients.is_empty() {
                            return Err(CompileError::MissingCredential(inbound.id.clone()));
                        }
                        let mut inbound_json = json!({
                            "tag": inbound.id,
                            "listen": inbound.listen,
                            "port": inbound.port,
                            "protocol": "trojan",
                            "settings": {
                                "clients": clients
                            }
                        });
                        apply_stream_security(&mut inbound_json, &inbound.security)?;
                        inbound_json
                    }
                    InboundProtocol::Vless => unreachable!("VLESS handled above"),
                };
                inbounds.push(inbound_json);
            }
        }
    }

    let config_json = json!({
        "log": {"loglevel": "warning"},
        "inbounds": inbounds,
        "outbounds": [
            {"tag":"direct", "protocol":"freedom"},
            {"tag":"block", "protocol":"blackhole"}
        ],
        "routing": {"rules": []},
        "stats": {},
        "api": {"tag":"api", "services":["StatsService"]},
        "policy": {"system": {"statsInboundUplink": true, "statsInboundDownlink": true}}
    });

    Ok(CompiledXrayConfig {
        config_json,
        compiler_version: env!("CARGO_PKG_VERSION").into(),
    })
}

fn apply_stream_security(
    inbound_json: &mut Value,
    security: &Security,
) -> Result<(), CompileError> {
    match security {
        Security::Tls { server_name } => {
            inbound_json["streamSettings"] = json!({
                "network": "tcp",
                "security": "tls",
                "tlsSettings": {
                    "serverName": server_name
                }
            });
        }
        Security::None => {}
        Security::Reality { .. } => return Err(CompileError::UnsupportedProtocol),
    }
    Ok(())
}

fn validate_shadowsocks_method_password(method: &str, password: &str) -> Result<(), CompileError> {
    let expected_bytes = match method {
        "2022-blake3-aes-128-gcm" => 16,
        "2022-blake3-aes-256-gcm" => 32,
        _ => return Ok(()),
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(password)
        .map_err(|_| CompileError::InvalidShadowsocks2022Psk {
            method: method.into(),
            expected_bytes,
        })?;
    if decoded.len() != expected_bytes {
        return Err(CompileError::InvalidShadowsocks2022Psk {
            method: method.into(),
            expected_bytes,
        });
    }
    Ok(())
}

fn supports_reality(version: &str) -> bool {
    let mut parts = version.split('.').filter_map(|s| s.parse::<u64>().ok());
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    major > 1 || (major == 1 && minor >= 8)
}
