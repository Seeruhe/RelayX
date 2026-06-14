use compiler_xray::{compile_profile_to_xray, CompileContext};
use domain::{
    ClientGroup, Credential, CredentialMaterial, CredentialStatus, DnsConfig, Inbound,
    InboundProtocol, ProfileIr, Runtime, Security,
};
use std::{fs, process::Command};

#[test]
#[ignore = "requires Docker and XRAY_DOCKER_IMAGE, default ghcr.io/xtls/xray-core:latest"]
fn compiled_shadowsocks_config_passes_real_xray_test() {
    let compiled = compile_profile_to_xray(&shadowsocks_profile(), &context()).unwrap();
    let temp_dir = std::env::temp_dir().join(format!(
        "proxy-control-xray-smoke-{}-{}",
        std::process::id(),
        monotonic_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();
    let config_path = temp_dir.join("config.json");
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&compiled.config_json).unwrap(),
    )
    .unwrap();

    let image = std::env::var("XRAY_DOCKER_IMAGE")
        .unwrap_or_else(|_| "ghcr.io/xtls/xray-core:latest".into());
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/work:ro", temp_dir.display()),
            &image,
            "run",
            "-test",
            "-config",
            "/work/config.json",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::remove_dir_all(&temp_dir).ok();
    assert!(
        output.status.success(),
        "xray run -test failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("Configuration OK") || stderr.contains("Configuration OK"),
        "xray did not report Configuration OK\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

fn context() -> CompileContext {
    CompileContext::new("26.3.27").with_credential(Credential {
        id: "cred-ss-active".into(),
        client_group_id: "group_ss".into(),
        display_name: "SS Alice".into(),
        status: CredentialStatus::Active,
        material: CredentialMaterial::ShadowsocksPassword {
            method: "2022-blake3-aes-128-gcm".into(),
            password: "MDEyMzQ1Njc4OWFiY2RlZg==".into(),
        },
    })
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
            listen: "127.0.0.1".into(),
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

fn monotonic_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
