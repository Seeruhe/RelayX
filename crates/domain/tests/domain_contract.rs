use chrono::{Duration, Utc};
use domain::*;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::collections::HashSet;

#[test]
fn artifact_hash_is_content_addressed_sha256() {
    let artifact = Artifact::from_bytes(
        "tenant-dev",
        ArtifactKind::CompiledXrayConfig,
        "application/json",
        br#"{"log":{}}"#,
        "admin",
    );

    assert_eq!(
        artifact.sha256,
        "7f959f83ac84e60f296a95caeab65b3f9c65c33b48fed012ef357acd741cadef"
    );
    assert!(artifact.storage_uri.ends_with(&artifact.sha256));
    assert_eq!(
        artifact.redaction_status,
        RedactionStatus::ContainsNoSecrets
    );
}

#[test]
fn profile_ir_rejects_plaintext_secret_like_fields() {
    let mut ir = ProfileIr::vless_reality_example("group_default", "sec_reality_private");
    ir.inbounds[0].security = Security::Reality {
        server_name: "example.com".into(),
        private_key_ref: SecretRef::new(
            "INLINE:super-secret-private-key",
            SecretKind::RealityPrivateKey,
        ),
        short_ids: vec!["abcd".into()],
    };

    let err = ir.validate().unwrap_err().to_string();
    assert!(
        err.contains("secret references must not contain inline material"),
        "{err}"
    );
}

#[test]
fn signed_runner_command_enforces_signature_ttl_nonce_sequence_and_node_scope() {
    let signing = SigningKey::from_bytes(&[7u8; 32]);
    let verify = VerifyingKey::from(&signing);
    let command = RunnerCommand::new(
        "tenant-dev",
        "node-a",
        42,
        Utc::now() + Duration::seconds(30),
        RunnerCommandKind::Heartbeat {
            capability_snapshot: serde_json::json!({"xray":"1.8.8"}),
        },
    );
    let envelope = SignedRunnerCommand::sign(command, &signing).unwrap();
    let mut seen = HashSet::new();

    let accepted = envelope
        .verify(&verify, "node-a", 41, &mut seen, Utc::now())
        .unwrap();
    assert_eq!(accepted.sequence, 42);

    let replay = envelope
        .verify(&verify, "node-a", 41, &mut seen, Utc::now())
        .unwrap_err()
        .to_string();
    assert!(replay.contains("nonce replay"), "{replay}");

    let mut seen = HashSet::new();
    let wrong_node = envelope
        .verify(&verify, "node-b", 41, &mut seen, Utc::now())
        .unwrap_err()
        .to_string();
    assert!(wrong_node.contains("node scope"), "{wrong_node}");

    let mut seen = HashSet::new();
    let stale = envelope
        .verify(&verify, "node-a", 42, &mut seen, Utc::now())
        .unwrap_err()
        .to_string();
    assert!(stale.contains("sequence"), "{stale}");
}

#[test]
fn signed_deployment_result_enforces_runner_signature_and_node_scope() {
    let runner_signing = SigningKey::from_bytes(&[22u8; 32]);
    let runner_verify = VerifyingKey::from(&runner_signing);
    let result = DeploymentResult {
        deployment_id: "dep-a".into(),
        status: DeploymentStatus::Succeeded,
        message: "applied".into(),
        artifact_sha256: "a".repeat(64),
        observed_at: Utc::now(),
    };

    let envelope = SignedDeploymentResult::sign("node-a", result.clone(), &runner_signing).unwrap();
    let accepted = envelope.verify(&runner_verify, "node-a").unwrap();
    assert_eq!(accepted.deployment_id, "dep-a");

    let wrong_node = envelope
        .verify(&runner_verify, "node-b")
        .unwrap_err()
        .to_string();
    assert!(wrong_node.contains("node scope"), "{wrong_node}");

    let mut tampered = envelope.clone();
    tampered.result.status = DeploymentStatus::Failed;
    let bad_sig = tampered
        .verify(&runner_verify, "node-a")
        .unwrap_err()
        .to_string();
    assert!(bad_sig.contains("signature invalid"), "{bad_sig}");
}
