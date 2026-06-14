CREATE TABLE tenants (
  id text PRIMARY KEY,
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  email text NOT NULL,
  role text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE api_keys (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  key_hash text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);

CREATE TABLE nodes (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  display_name text NOT NULL,
  status text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE node_identities (
  node_id text PRIMARY KEY REFERENCES nodes(id),
  public_key text NOT NULL,
  registration_token_hash text,
  registered_at timestamptz
);

CREATE TABLE node_registration_tokens (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  token text NOT NULL UNIQUE,
  status text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  consumed_at timestamptz,
  used_by_node_id text REFERENCES nodes(id)
);

CREATE TABLE node_capabilities (
  id bigserial PRIMARY KEY,
  node_id text NOT NULL REFERENCES nodes(id),
  xray_version text NOT NULL,
  capabilities_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE runner_sessions (
  id text PRIMARY KEY,
  node_id text NOT NULL REFERENCES nodes(id),
  started_at timestamptz NOT NULL DEFAULT now(),
  ended_at timestamptz
);

CREATE TABLE runner_commands (
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  node_id text NOT NULL REFERENCES nodes(id),
  sequence bigint NOT NULL,
  envelope_json jsonb NOT NULL,
  status text NOT NULL DEFAULT 'pending',
  leased_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE(node_id, sequence)
);

CREATE TABLE node_heartbeats (
  id bigserial PRIMARY KEY,
  node_id text NOT NULL REFERENCES nodes(id),
  session_id text REFERENCES runner_sessions(id),
  payload_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE profiles (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  name text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE profile_versions (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  profile_id text NOT NULL REFERENCES profiles(id),
  version integer NOT NULL,
  ir_json jsonb NOT NULL,
  schema_version text NOT NULL,
  compiler_version text,
  target_core_kind text NOT NULL,
  target_core_version text NOT NULL,
  feature_flags jsonb NOT NULL DEFAULT '{}',
  assets_version text NOT NULL DEFAULT 'dev',
  input_hash text NOT NULL,
  created_by text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE(profile_id, version)
);

CREATE TABLE clients (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  display_name text NOT NULL,
  status text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE credentials (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  client_id text NOT NULL REFERENCES clients(id),
  client_group_id text NOT NULL,
  kind text NOT NULL,
  secret_ref text NOT NULL,
  status text NOT NULL,
  expires_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE credential_leases (
  id text PRIMARY KEY,
  credential_id text NOT NULL REFERENCES credentials(id),
  lease_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz
);

CREATE TABLE credential_quotas (
  credential_id text PRIMARY KEY REFERENCES credentials(id),
  quota_bytes bigint NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE profile_credentials (
  profile_id text NOT NULL REFERENCES profiles(id),
  credential_id text NOT NULL REFERENCES credentials(id),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (profile_id, credential_id)
);

CREATE TABLE artifacts (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  kind text NOT NULL,
  schema_version text NOT NULL,
  media_type text NOT NULL,
  sha256 text NOT NULL,
  storage_uri text NOT NULL,
  redaction_status text NOT NULL,
  created_by text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE(tenant_id, sha256)
);

CREATE TABLE artifact_blobs (
  sha256 text PRIMARY KEY,
  bytes bytea NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deployments (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  node_id text NOT NULL REFERENCES nodes(id),
  profile_version_id text NOT NULL REFERENCES profile_versions(id),
  status text NOT NULL,
  compiled_config_artifact_id text REFERENCES artifacts(id),
  rollback_pointer_id text,
  created_at timestamptz NOT NULL DEFAULT now(),
  finished_at timestamptz
);

CREATE TABLE rollback_pointers (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  deployment_id text NOT NULL REFERENCES deployments(id),
  previous_deployment_id text REFERENCES deployments(id),
  previous_compiled_config_artifact_id text REFERENCES artifacts(id),
  target_compiled_config_artifact_id text NOT NULL REFERENCES artifacts(id),
  previous_core_version text,
  previous_assets_version text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deployment_snapshots (
  id text PRIMARY KEY,
  deployment_id text NOT NULL REFERENCES deployments(id),
  snapshot_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE deployment_health_checks (
  id bigserial PRIMARY KEY,
  deployment_id text NOT NULL REFERENCES deployments(id),
  status text NOT NULL,
  payload_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE tasks (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  state text NOT NULL,
  task_type text NOT NULL,
  payload_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE task_steps (
  id bigserial PRIMARY KEY,
  task_id text NOT NULL REFERENCES tasks(id),
  step_name text NOT NULL,
  state text NOT NULL,
  payload_json jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE secrets (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  kind text NOT NULL,
  ciphertext bytea NOT NULL,
  key_id text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  destroyed_at timestamptz
);

CREATE TABLE policy_rules (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  rule_json jsonb NOT NULL,
  enabled boolean NOT NULL DEFAULT true
);

CREATE TABLE approvals (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  task_id text REFERENCES tasks(id),
  state text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE event_outbox (
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  event_type text NOT NULL,
  aggregate_type text NOT NULL,
  aggregate_id text NOT NULL,
  payload_json jsonb NOT NULL,
  status text NOT NULL DEFAULT 'pending',
  attempts integer NOT NULL DEFAULT 0,
  available_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  processed_at timestamptz
);

CREATE TABLE idempotency_keys (
  tenant_id text NOT NULL REFERENCES tenants(id),
  key text NOT NULL,
  response_json jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, key)
);

CREATE TABLE usage_records (
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  node_id text NOT NULL REFERENCES nodes(id),
  credential_id text REFERENCES credentials(id),
  uplink_bytes bigint NOT NULL,
  downlink_bytes bigint NOT NULL,
  sampled_at timestamptz NOT NULL
);

CREATE TABLE usage_rollups (
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  credential_id text REFERENCES credentials(id),
  bucket text NOT NULL,
  bucket_start timestamptz NOT NULL,
  uplink_bytes bigint NOT NULL,
  downlink_bytes bigint NOT NULL,
  UNIQUE(tenant_id, credential_id, bucket, bucket_start)
);

CREATE TABLE subscription_tokens (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  profile_id text NOT NULL REFERENCES profiles(id),
  token_hash text NOT NULL,
  status text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  rotated_at timestamptz
);

CREATE TABLE subscription_access_logs (
  id bigserial PRIMARY KEY,
  token_id text REFERENCES subscription_tokens(id),
  remote_addr text,
  user_agent text,
  status text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit_events (
  id bigserial PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  actor text NOT NULL,
  action text NOT NULL,
  subject text NOT NULL,
  payload_json jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE model_providers (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  kind text NOT NULL,
  endpoint text NOT NULL,
  api_key_secret_ref text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE model_invocations (
  id text PRIMARY KEY,
  tenant_id text NOT NULL REFERENCES tenants(id),
  provider_id text REFERENCES model_providers(id),
  model_name text NOT NULL,
  prompt_hash text NOT NULL,
  artifact_id text REFERENCES artifacts(id),
  created_at timestamptz NOT NULL DEFAULT now()
);
