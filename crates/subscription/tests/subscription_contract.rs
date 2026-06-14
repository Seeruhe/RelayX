use domain::*;
use subscription::*;

#[test]
fn grouped_subscription_contains_active_links_and_hides_revoked_credentials() {
    let profile = DeployedProfile::new("profile-a", "node-a.example", 443, "example.com")
        .with_credential(Credential::active_vless(
            "cred-a",
            "group_default",
            "2f4f6f8a-1111-4c4c-9999-111111111111",
            "Alice",
        ))
        .with_credential(Credential::revoked_vless(
            "cred-b",
            "group_default",
            "33333333-3333-4333-9333-333333333333",
            "Bob",
        ));

    let artifact = generate_subscription_artifact("tenant-dev", &profile, "admin").unwrap();
    let decoded = String::from_utf8(artifact.body_base64_decoded()).unwrap();

    assert!(decoded.contains("vless://2f4f6f8a-1111-4c4c-9999-111111111111@node-a.example:443"));
    assert!(decoded.contains("security=reality"));
    assert!(decoded.contains("sni=example.com"));
    assert!(!decoded.contains("33333333-3333-4333-9333-333333333333"));
    assert_eq!(artifact.artifact.kind, ArtifactKind::SubscriptionArtifact);
}
