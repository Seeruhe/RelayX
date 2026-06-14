use chrono::Utc;
use domain::{
    Artifact, ArtifactKind, Credential, DeploymentResult, DeploymentStatus, ProfileIr,
    RunnerCommand, RunnerCommandKind, SignedRunnerCommand,
};
use ed25519_dalek::SigningKey;
use sqlx::{postgres::PgPoolOptions, Executor};
use storage::{
    DeploymentPlanRecord, HeartbeatRecord, NodeRecord, PostgresStore, ProfileRecord,
    UsageSampleRecord,
};

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing at a disposable Postgres database"]
async fn postgres_store_runs_p0_flow_against_migrated_schema() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("set TEST_DATABASE_URL to a disposable Postgres database");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap();
    reset_and_migrate(&pool).await;

    let store = PostgresStore::from_pool(pool);
    store
        .register_node(NodeRecord::new(
            "tenant-pg",
            "node-pg",
            "node-pg.example",
            "1.8.8",
        ))
        .await
        .unwrap();
    store
        .record_heartbeat(HeartbeatRecord::new(
            "tenant-pg",
            "node-pg",
            serde_json::json!({"xray_version":"1.8.8","os":"linux"}),
        ))
        .await
        .unwrap();
    assert_eq!(
        store.latest_heartbeat("node-pg").await.unwrap().node_id,
        "node-pg"
    );

    store
        .create_profile(ProfileRecord::new(
            "tenant-pg",
            "profile-pg",
            ProfileIr::vless_reality_example("group_default", "sec_reality_private"),
        ))
        .await
        .unwrap();
    store
        .add_credential(
            "profile-pg",
            Credential::active_vless(
                "client-pg",
                "group_default",
                "2f4f6f8a-1111-4c4c-9999-111111111111",
                "Postgres Alice",
            ),
        )
        .await
        .unwrap();

    let artifact = Artifact::from_bytes(
        "tenant-pg",
        ArtifactKind::CompiledXrayConfig,
        "application/json",
        br#"{"inbounds":[]}"#,
        "test",
    );
    store
        .record_artifact_blob(artifact.clone(), br#"{"inbounds":[]}"#.to_vec())
        .await
        .unwrap();
    assert_eq!(
        store.artifact_bytes(&artifact.id).await.unwrap(),
        br#"{"inbounds":[]}"#
    );

    store
        .record_deployment_plan(DeploymentPlanRecord::new(
            "tenant-pg",
            "dep-pg",
            "node-pg",
            "profile-pg",
            &artifact.id,
        ))
        .await
        .unwrap();
    assert_eq!(
        store.deployment_status("dep-pg").await.unwrap(),
        DeploymentStatus::Pending
    );

    let signed = SignedRunnerCommand::sign(
        RunnerCommand::new(
            "tenant-pg",
            "node-pg",
            1,
            Utc::now() + chrono::Duration::seconds(60),
            RunnerCommandKind::ApplyDeploymentPlan {
                deployment_id: "dep-pg".into(),
                artifact_sha256: artifact.sha256.clone(),
                config_json: serde_json::json!({"inbounds":[]}),
                rollback_json: None,
            },
        ),
        &SigningKey::from_bytes(&[11u8; 32]),
    )
    .unwrap();
    store
        .enqueue_runner_command("node-pg", signed)
        .await
        .unwrap();
    let command = store
        .next_runner_command("node-pg", 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(command.command.sequence, 1);

    store
        .record_deployment_result(DeploymentResult {
            deployment_id: "dep-pg".into(),
            status: DeploymentStatus::Succeeded,
            message: "postgres runner result".into(),
            artifact_sha256: artifact.sha256,
            observed_at: Utc::now(),
        })
        .await
        .unwrap();
    assert_eq!(
        store.deployment_status("dep-pg").await.unwrap(),
        DeploymentStatus::Succeeded
    );
    assert_eq!(
        store
            .latest_deployment_health("dep-pg")
            .await
            .unwrap()
            .status,
        "healthy"
    );

    store
        .record_usage_sample(UsageSampleRecord::new(
            "tenant-pg",
            "node-pg",
            Some("client-pg".into()),
            100,
            200,
            Utc::now(),
        ))
        .await
        .unwrap();
    for bucket in ["hour", "day", "month"] {
        let rollup = store
            .latest_usage_rollup_for_credential("client-pg", bucket)
            .await
            .unwrap();
        assert_eq!(rollup.bucket, bucket);
        assert_eq!(rollup.uplink_bytes, 100);
        assert_eq!(rollup.downlink_bytes, 200);
    }

    let token = store.issue_subscription_token("profile-pg").await.unwrap();
    store
        .verify_subscription_token("profile-pg", &token.token)
        .await
        .unwrap();
    assert!(store.audit_count().await.unwrap() > 0);
    assert!(store.outbox_count().await.unwrap() > 0);
}

async fn reset_and_migrate(pool: &sqlx::PgPool) {
    pool.execute("DROP SCHEMA public CASCADE").await.unwrap();
    pool.execute("CREATE SCHEMA public").await.unwrap();
    let migration = include_str!("../../../migrations/0001_p0_schema.sql");
    for statement in migration.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() {
            pool.execute(statement).await.unwrap();
        }
    }
}
