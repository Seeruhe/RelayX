use compiler_xray::*;
use domain::*;

fn context() -> CompileContext {
    CompileContext::new("1.8.8")
        .with_secret("sec_reality_private", "REALITY_PRIVATE_KEY")
        .with_credential(Credential::active_vless(
            "cred-alice",
            "group_default",
            "2f4f6f8a-1111-4c4c-9999-111111111111",
            "Alice",
        ))
}

fn shadowsocks_profile() -> ProfileIr {
    ProfileIr {
        schema_version: "0.1".into(),
        runtime: Runtime {
            core: "xray".into(),
            core_version: "1.x".into(),
        },
        inbounds: vec![Inbound {
            id: "in_ss_8388".into(),
            protocol: InboundProtocol::Shadowsocks,
            listen: "0.0.0.0".into(),
            port: 8388,
            security: Security::None,
            client_group_refs: vec!["group_ss".into()],
        }],
        client_groups: vec![ClientGroup {
            id: "group_ss".into(),
            credential_policy: "shadowsocks_password".into(),
            quota_policy_ref: None,
        }],
        routes: vec![],
        dns: DnsConfig {
            mode: "system".into(),
        },
    }
}

fn trojan_profile() -> ProfileIr {
    ProfileIr {
        schema_version: "0.1".into(),
        runtime: Runtime {
            core: "xray".into(),
            core_version: "1.x".into(),
        },
        inbounds: vec![Inbound {
            id: "in_trojan_443".into(),
            protocol: InboundProtocol::Trojan,
            listen: "0.0.0.0".into(),
            port: 443,
            security: Security::Tls {
                server_name: "trojan.example.com".into(),
            },
            client_group_refs: vec!["group_trojan".into()],
        }],
        client_groups: vec![ClientGroup {
            id: "group_trojan".into(),
            credential_policy: "trojan_password".into(),
            quota_policy_ref: None,
        }],
        routes: vec![],
        dns: DnsConfig {
            mode: "system".into(),
        },
    }
}

#[test]
fn compiles_vless_reality_profile_to_xray_json_without_leaking_secret_refs() {
    let ir = ProfileIr::vless_reality_example("group_default", "sec_reality_private");
    let compiled = compile_profile_to_xray(&ir, &context()).unwrap();
    let json = compiled.config_json;

    assert_eq!(json["inbounds"][0]["protocol"], "vless");
    assert_eq!(json["inbounds"][0]["streamSettings"]["security"], "reality");
    assert_eq!(
        json["inbounds"][0]["streamSettings"]["realitySettings"]["privateKey"],
        "REALITY_PRIVATE_KEY"
    );
    assert_eq!(
        json["inbounds"][0]["settings"]["clients"][0]["id"],
        "2f4f6f8a-1111-4c4c-9999-111111111111"
    );

    let rendered = serde_json::to_string(&json).unwrap();
    assert!(!rendered.contains("sec_reality_private"));
}

#[test]
fn rejects_reality_for_unsupported_core_version() {
    let ir = ProfileIr::vless_reality_example("group_default", "sec_reality_private");
    let err = compile_profile_to_xray(&ir, &context().with_core_version("1.7.5"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("REALITY requires xray-core >= 1.8.0"), "{err}");
}

#[test]
fn compiles_shadowsocks_profile_to_xray_inbound() {
    let ctx = CompileContext::new("1.8.8")
        .with_credential(Credential {
            id: "cred-ss-active".into(),
            client_group_id: "group_ss".into(),
            display_name: "SS Alice".into(),
            status: CredentialStatus::Active,
            material: CredentialMaterial::ShadowsocksPassword {
                method: "2022-blake3-aes-128-gcm".into(),
                password: "MDEyMzQ1Njc4OWFiY2RlZg==".into(),
            },
        })
        .with_credential(Credential {
            id: "cred-ss-revoked".into(),
            client_group_id: "group_ss".into(),
            display_name: "SS Bob".into(),
            status: CredentialStatus::Revoked,
            material: CredentialMaterial::ShadowsocksPassword {
                method: "2022-blake3-aes-128-gcm".into(),
                password: "cmV2b2tlZC1wYXNzd29yZA==".into(),
            },
        });

    let compiled = compile_profile_to_xray(&shadowsocks_profile(), &ctx).unwrap();
    let inbound = &compiled.config_json["inbounds"][0];
    assert_eq!(inbound["tag"], "in_ss_8388");
    assert_eq!(inbound["protocol"], "shadowsocks");
    assert_eq!(inbound["port"], 8388);
    assert_eq!(inbound["settings"]["method"], "2022-blake3-aes-128-gcm");
    assert_eq!(inbound["settings"]["password"], "MDEyMzQ1Njc4OWFiY2RlZg==");
    assert_eq!(inbound["settings"]["network"], "tcp,udp");
    assert!(!serde_json::to_string(inbound)
        .unwrap()
        .contains("cmV2b2tlZC1wYXNzd29yZA=="));
}

#[test]
fn rejects_invalid_2022_shadowsocks_psk_before_xray_runtime() {
    let ctx = CompileContext::new("1.8.8").with_credential(Credential {
        id: "cred-ss-active".into(),
        client_group_id: "group_ss".into(),
        display_name: "SS Alice".into(),
        status: CredentialStatus::Active,
        material: CredentialMaterial::ShadowsocksPassword {
            method: "2022-blake3-aes-128-gcm".into(),
            password: "ss-password".into(),
        },
    });

    let err = compile_profile_to_xray(&shadowsocks_profile(), &ctx)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("2022-blake3-aes-128-gcm requires a base64-encoded 16-byte psk"),
        "{err}"
    );
}

#[test]
fn compiles_trojan_profile_to_xray_inbound() {
    let ctx = CompileContext::new("1.8.8")
        .with_credential(Credential {
            id: "cred-trojan-active".into(),
            client_group_id: "group_trojan".into(),
            display_name: "Trojan Alice".into(),
            status: CredentialStatus::Active,
            material: CredentialMaterial::TrojanPassword {
                password: "trojan-password".into(),
            },
        })
        .with_credential(Credential {
            id: "cred-trojan-other-group".into(),
            client_group_id: "group_other".into(),
            display_name: "Other".into(),
            status: CredentialStatus::Active,
            material: CredentialMaterial::TrojanPassword {
                password: "other-password".into(),
            },
        });

    let compiled = compile_profile_to_xray(&trojan_profile(), &ctx).unwrap();
    let inbound = &compiled.config_json["inbounds"][0];
    assert_eq!(inbound["tag"], "in_trojan_443");
    assert_eq!(inbound["protocol"], "trojan");
    assert_eq!(
        inbound["settings"]["clients"][0]["password"],
        "trojan-password"
    );
    assert_eq!(inbound["settings"]["clients"][0]["email"], "Trojan Alice");
    assert_eq!(inbound["streamSettings"]["security"], "tls");
    assert_eq!(
        inbound["streamSettings"]["tlsSettings"]["serverName"],
        "trojan.example.com"
    );
    assert!(!serde_json::to_string(inbound)
        .unwrap()
        .contains("other-password"));
}
