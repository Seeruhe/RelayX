# Agent-native Proxy Infrastructure Control Plane - Development Plan v0.1

## 0. Product Boundary

This project is not a generic AI Agent platform and not a clone of 3x-ui.

It is a proxy infrastructure control plane for managing `xray-core` first and `sing-box` later.

Core product:

- Manage proxy nodes, clients, credentials, profiles, deployments, subscriptions, usage, logs, audit, rollback.
- Compile declarative Profile IR into runtime-specific config.
- Run trusted Runner agents on VPS nodes.
- Provide AI-assisted operations as a proposal layer.
- Expose A2A compatibility later at the external edge, not in the internal control path.

Core rule:

```text
Proxy management is the product.
AI is a proposal layer.
A2A is an external interoperability boundary.
Runner is a trusted executor.
Profile IR is the long-term moat.
```

## 1. Recommended Development Machine

### Minimum for P0 backend PoC

```text
CPU: 4 vCPU
RAM: 8 GB
Disk: 80 GB SSD
OS: Linux x86_64
```

### Comfortable full-stack development

```text
CPU: 8 vCPU
RAM: 16-32 GB
Disk: 150-200 GB SSD
OS: Linux x86_64
```

### Current 2 vCPU / 4 GB machine profile

This can run backend PoC work, but should avoid heavy Docker stacks and parallel builds.

Use:

```text
Rust control-plane
Rust runner mock
SQLite or one local Postgres
Xray compiler tests
Next.js only when needed
```

Avoid:

```text
Local LLM
ClickHouse
Large docker compose
Multiple Next.js/Rust watch processes
Heavy frontend builds
```

## 2. Runtime Dependencies

Install on development machine:

```bash
rustup
cargo
nodejs >= 22
pnpm
postgresql >= 16
sqlx-cli
just
git
openssl
pkg-config
xray-core
```

Optional later:

```bash
docker
docker compose
nats-server
sing-box
ollama
```

Recommended Rust toolchain:

```bash
rustup toolchain install stable
rustup default stable
cargo install sqlx-cli --no-default-features --features postgres,rustls
cargo install cargo-nextest
```

Recommended Node setup:

```bash
corepack enable
corepack prepare pnpm@latest --activate
```

## 3. Repository Layout

Use a monorepo:

```text
proxy-control/
  Cargo.toml
  package.json
  pnpm-workspace.yaml
  justfile
  .env.example

  apps/
    web/
      Next.js console

  crates/
    domain/
      Profile IR, Task, Artifact, Deployment, Runner command types

    control-plane/
      axum API, auth, node manager, profile service, deploy service

    runner/
      VPS trusted executor

    compiler-xray/
      Profile IR -> Xray config

    compiler-singbox/
      P1: Profile IR -> sing-box config

    subscription/
      subscription artifact generation

    policy/
      compile/deploy policy checks

    storage/
      sqlx repositories, event_outbox, audit append-only writer

    secrets/
      secret references, envelope encryption abstraction

    model-gateway/
      P1: BYOM OpenAI-compatible integration

    a2a-edge/
      P2: A2A external adapter

  migrations/
    Postgres migrations

  docs/
    architecture-v0.1.md
    adr/
      0001-product-boundary.md
      0002-runner-trust-model.md
      0003-profile-ir-source-of-truth.md
      0004-a2a-edge-only.md
    ir/
      profile-ir-v0.md
    runner/
      trust-model.md
```

## 4. P0 Scope

P0 must be small and complete. Do not include AI, A2A, marketplace, payments, React Flow, or sing-box.

P0 objective:

```text
Create one Xray profile, deploy it to one Runner node, verify, observe, subscribe, and rollback safely.
```

P0 feature list:

- Tenant-local single admin mode.
- Node registration.
- Runner heartbeat.
- Node capability snapshot.
- Profile version creation.
- Client credential creation.
- Xray VLESS + REALITY support.
- Xray Shadowsocks support.
- Xray Trojan support.
- Compile Profile IR to Xray JSON.
- Run `xray run -test` before deployment.
- Runner atomic config write.
- Reload/restart core.
- Deployment snapshot.
- Rollback pointer.
- Basic health observation.
- Subscription link generation.
- Raw usage sample.
- Append-only audit.
- Content-addressed artifacts.
- Event outbox.
- Secret isolation.

P0 non-goals:

- A2A.
- AI proposal.
- BYOM.
- Node marketplace.
- Payment.
- Complex billing.
- sing-box adapter.
- Advanced route visual editor.
- Public multi-tenant SaaS hardening.

## 5. P0 Acceptance Criteria

P0 is done only when this flow works end to end:

```text
1. Start Postgres.
2. Start control-plane.
3. Start runner.
4. Runner registers with one-time registration token.
5. Runner sends heartbeat and capability snapshot.
6. Admin creates a Profile with one VLESS REALITY inbound.
7. Admin creates one client credential.
8. Control-plane compiles Profile IR into Xray config.
9. Control-plane stores compiled config as content-addressed artifact.
10. Control-plane creates DeploymentPlan.
11. Runner receives signed deployment command.
12. Runner verifies signature, TTL, nonce, sequence, node scope.
13. Runner writes temp config.
14. Runner runs `xray run -test`.
15. Runner atomically switches active config.
16. Runner reloads or restarts Xray.
17. Runner observes process health.
18. Control-plane marks deployment succeeded.
19. Subscription URL returns usable client config.
20. Audit trail shows actor, task, artifacts, deployment, runner result.
21. Bad config causes failed deployment and rollback.
```

## 6. Core Architecture

### Request path

```text
Next.js Web Console
  -> Rust Control Plane API
  -> Postgres / Event Outbox / Artifact Store
  -> Rust Runner Agent
  -> xray-core
```

### Future A2A path

```text
External Agent / Partner / CLI
  -> A2A Edge Adapter
  -> Internal Task Kernel
  -> Control Plane
  -> Runner
```

The Runner is never exposed as an A2A Agent.

## 7. Core Domain Models

### ProfileVersion

`profile_versions.ir_json` is the only desired-state source of truth.

Other tables are indexes, drafts, templates, or projections.

Required fields:

```text
id
tenant_id
profile_id
version
ir_json
schema_version
compiler_version
target_core_kind
target_core_version
feature_flags
assets_version
input_hash
created_by
created_at
```

### Profile IR

Profile IR represents business intent, not raw Xray or sing-box JSON.

Example shape:

```json
{
  "schema_version": "0.1",
  "runtime": {
    "core": "xray",
    "core_version": "1.x"
  },
  "inbounds": [
    {
      "id": "in_reality_443",
      "protocol": "vless",
      "listen": "0.0.0.0",
      "port": 443,
      "security": {
        "type": "reality",
        "server_name": "example.com"
      },
      "client_group_refs": ["group_default"]
    }
  ],
  "client_groups": [
    {
      "id": "group_default",
      "credential_policy": "vless_uuid",
      "quota_policy_ref": "quota_200gb_30d"
    }
  ],
  "routes": [],
  "dns": {
    "mode": "system"
  }
}
```

Do not store real secrets in Profile IR.

### Secrets

Secrets are referenced by ID.

Examples:

```text
client UUID
Shadowsocks password
Reality private key
subscription token secret
model provider API key
runner node key
```

Artifacts must not contain plaintext secrets unless explicitly marked as sealed and access-controlled.

### Artifact

All important outputs are immutable artifacts.

Required fields:

```text
id
tenant_id
kind
schema_version
media_type
sha256
storage_uri
redaction_status
created_by
created_at
```

Artifact kinds:

```text
ProfileIR
ProfilePatch
CompiledXrayConfig
CompiledSingBoxConfig
SubscriptionArtifact
ValidationReport
DeploymentPlan
DeploymentResult
RollbackPointer
DiagnosisReport
```

### Task

Internal Task Kernel is A2A-compatible in semantics but not implemented on A2A in P0.

States:

```text
created
policy_checking
waiting_approval
queued
leased
running
observing
succeeded
failed
rolled_back
canceled
expired
```

### DeploymentPlan

DeploymentPlan must be immutable.

Required fields:

```text
target_node_id
target_profile_version_id
compiled_config_artifact_id
core_kind
core_version
assets_version
port_diff
secret_diff
reload_or_restart_decision
preflight_checks
health_checks
rollback_pointer_id
created_by
created_at
```

### RollbackPointer

RollbackPointer must include more than the previous config.

Include:

```text
previous_compiled_config_artifact_id
previous_core_version
previous_assets_version
previous_systemd_unit_hash
previous_secret_refs
previous_subscription_state_ref
previous_health_baseline
```

## 8. Database Tables

P0 migrations should include:

```text
tenants
users
api_keys

nodes
node_identities
node_capabilities
runner_sessions
node_heartbeats

profiles
profile_versions

clients
credentials
credential_leases

deployments
deployment_snapshots
deployment_health_checks

tasks
task_steps

artifacts
artifact_blobs

secrets
policy_rules
approvals

event_outbox
idempotency_keys

usage_records
usage_rollups

subscription_tokens
subscription_access_logs

audit_events
model_providers
model_invocations
```

P0 may create only the subset needed for Xray deployment, but table design should not contradict this full shape.

## 9. Event Outbox

P0 should not require NATS.

Use `event_outbox` for reliable local events:

```text
id
tenant_id
event_type
aggregate_type
aggregate_id
payload_json
status
attempts
available_at
created_at
processed_at
```

Later, an outbox worker can publish events to NATS JetStream.

## 10. Runner Trust Model

Runner is a trusted executor. It must not accept external Agent calls.

Hard requirements:

- mTLS or equivalent mutual identity.
- One-time node registration token.
- Control Plane signs canonical Runner commands.
- Runner verifies signature.
- Runner verifies tenant scope and node scope.
- Command includes TTL.
- Command includes nonce.
- Command includes monotonic sequence.
- Runner signs result response.
- Runner only initiates outbound connection when possible.
- Core install/update uses signed manifest and checksum.
- Core version allowlist.
- `install_core` requires higher privilege than `reload_core`.

Runner command examples:

```text
RegisterNode
Heartbeat
ApplyDeploymentPlan
ValidateConfig
ReloadCore
RestartCore
RollbackDeployment
CollectMetrics
CollectLogWindow
```

Runner must execute structured commands, not free shell strings.

## 11. Deployment Engine

Deployment is desired-state reconciliation, not command execution.

Flow:

```text
1. Load target ProfileVersion.
2. Load target NodeCapability snapshot.
3. Run pre-compile policy checks.
4. Compile Profile IR to Xray JSON.
5. Run post-compile policy checks.
6. Store compiled config artifact.
7. Create DeploymentPlan.
8. Create RollbackPointer.
9. Sign Runner command.
10. Runner writes temp config directory.
11. Runner runs `xray run -test`.
12. Runner snapshots current runtime state.
13. Runner atomically switches active symlink.
14. Runner reloads or restarts Xray.
15. Runner observes health window.
16. Runner signs result.
17. Control-plane commits final deployment state.
18. On failure, Runner rolls back using RollbackPointer.
```

## 12. Xray P0 Compiler Scope

Support only:

```text
VLESS + REALITY
Shadowsocks
Trojan
Basic direct/block outbounds
Basic routing placeholder
Basic stats API config
```

Do not support in P0:

```text
complex fallback
complex XHTTP/gRPC/WS matrix
advanced DNS rules
multi-core mixed deployment
sing-box
```

Compiler must be version-aware.

If the target core version does not support a feature, compilation must fail clearly.

Never silently downgrade security-sensitive settings.

## 13. Subscription Service

Separate subscription artifacts from core config artifacts.

Do not put subscription generation inside `compiler-xray`.

Subscription module responsibilities:

```text
read deployed profile/client/credential state
generate vless/trojan/ss links
generate grouped subscription
sign subscription token
rotate token
log access
hide revoked credentials
respect expiry/quota state
```

## 14. Usage Accounting

Core stats are not the billing ledger.

P0:

```text
Runner samples Xray stats
Control-plane stores raw usage_records
Control-plane calculates usage_rollups
Quota decisions use rollups, not live stats only
```

Separate:

```text
raw samples
hourly rollups
daily rollups
quota state
```

## 15. Frontend P0

Use:

```text
Next.js App Router
TypeScript
Tailwind
shadcn/ui
TanStack Query
SSE for task/deployment stream
generated TS API client from OpenAPI
```

P0 pages:

```text
/dashboard
/nodes
/clients
/profiles
/deployments
/tasks
/logs
/settings
```

P0 UI style:

- Dense operational UI.
- No marketing landing page.
- No React Flow yet.
- No AI chat yet.
- Monaco only for read-only config/profile artifact viewing.
- Main editing path is structured Profile editor.

## 16. AI/BYOM P1

AI is a proposal layer.

Supported P1 modes:

```text
OpenAI-compatible endpoint
LiteLLM endpoint
Ollama/vLLM endpoint if OpenAI-compatible
```

AI outputs only typed artifacts:

```text
ProfilePatch
DiagnosisReport
DeploymentPlanDraft
```

AI artifact metadata:

```text
model_provider
model_name
prompt_hash
schema_version
validation_result
policy_result
redaction_status
created_at
```

Red lines:

- No plaintext secrets in model input by default.
- Logs must be redacted before model input.
- No natural-language implicit modification.
- No model direct Runner execution.
- No model bypass of policy checks.

## 17. A2A P2

A2A belongs at the edge.

P2 components:

```text
a2a-edge crate
AgentCard
private agent registry
A2A Task -> Internal Task mapping
A2A Artifact -> Internal Artifact mapping
auth/rate limit
```

Allowed public skills:

```text
validate_proxy_profile
create_deployment_plan
diagnose_node
generate_subscription
```

Not allowed:

```text
write_config
reload_core
restart_core
tail_logs_unbounded
install_core
```

NodeMarketplaceAgent is P3, not P2.

## 18. P1/P2/P3 Roadmap

### P0 - Xray control-plane closure

```text
Rust Control Plane
Rust Runner
Postgres
Next.js Console
Profile IR
XrayAdapter
DeploymentPlan
RollbackPointer
SubscriptionArtifact
Audit/EventOutbox/Secrets
```

### P1 - Productization

```text
client quota/expiry
credential rotation/revoke
usage rollups
task kernel polish
artifact viewer
AI proposal
BYOM Level 1/2
sing-box adapter skeleton
```

### P2 - Agent edge

```text
A2A Edge Adapter
AgentCard
private registry
external task API
CLI
Agentgateway evaluation
```

### P3 - Lease/marketplace

```text
Credential Lease -> NodeLease
NodeMarketplaceAgent
provider offers
abuse detection
reputation
payment/settlement
```

## 19. Local Development Commands

Expected commands after repository exists:

```bash
just setup
just db-up
just migrate
just control-plane
just runner
just web
just test
just lint
```

Example `justfile` targets:

```makefile
setup:
    pnpm install
    cargo fetch

db-up:
    pg_ctl start

migrate:
    sqlx migrate run

control-plane:
    cargo run -p control-plane

runner:
    cargo run -p runner

web:
    pnpm --filter web dev

test:
    cargo nextest run
    pnpm --filter web test

lint:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm --filter web lint
```

## 20. Environment Variables

`.env.example`:

```bash
DATABASE_URL=postgres://proxy:proxy@localhost:5432/proxy_control
CONTROL_PLANE_BIND=127.0.0.1:8080
PUBLIC_BASE_URL=http://127.0.0.1:8080

RUNNER_NODE_ID=dev-node-1
RUNNER_BIND=127.0.0.1:9090
RUNNER_WORK_DIR=/var/lib/proxy-runner
RUNNER_XRAY_BIN=/usr/local/bin/xray

ARTIFACT_STORE_DIR=.data/artifacts
SECRET_MASTER_KEY_DEV=replace-me

WEB_API_BASE_URL=http://127.0.0.1:8080
```

For local non-root dev, use:

```text
RUNNER_WORK_DIR=.data/runner
RUNNER_XRAY_BIN=/path/to/xray
```

## 21. Testing Strategy

### Unit tests

```text
Profile IR validation
Xray compiler
policy checks
artifact hashing
secret reference handling
deployment state machine
runner command signature verification
```

### Integration tests

```text
compile valid VLESS REALITY profile
reject unsupported feature for target version
create deployment plan
runner validates config with fake xray binary
bad config causes failed deployment
rollback pointer restored
subscription hides revoked credential
usage sample rolls up correctly
```

### E2E tests

```text
create node
create client
create profile
deploy profile
fetch subscription
observe deployment success
trigger rollback
verify audit trail
```

## 22. Security Checklist

P0 must enforce:

- No plaintext secrets in Profile IR.
- No free-form shell execution by Runner.
- Signed Runner commands.
- Command TTL.
- Command nonce.
- Command sequence.
- Node scope validation.
- Append-only audit.
- Content-addressed artifacts.
- Redacted logs.
- Subscription token rotation support.
- Idempotency key for deploy APIs.

## 23. Engineering Risks

Highest risks:

```text
Profile IR over-abstracts Xray and sing-box
Runner compromise affects all nodes
core version drift breaks deployments
usage accounting becomes unreliable
AI proposal bypasses structure/policy
subscription client compatibility fragments
```

Mitigations:

```text
version capability matrix
strict compile errors
signed runner commands
event outbox
artifact hashes
secret references
raw usage + rollups
schema-bound AI outputs
subscription compatibility tests
```

## 24. Immediate Implementation Order

Do not start with UI.

Build in this order:

```text
1. Rust workspace scaffold
2. domain crate
3. Profile IR v0 structs
4. Artifact model and hashing
5. Secret reference model
6. Xray compiler for one VLESS REALITY profile
7. Runner command model and signature interface
8. DeploymentPlan model
9. Postgres migrations
10. control-plane minimal API
11. runner minimal apply flow
12. xray run -test integration
13. subscription artifact generation
14. basic Next.js console
15. audit/event_outbox
16. rollback flow
```

## 25. Definition of v0.1 Done

v0.1 is done when:

```text
One developer can run the control-plane locally.
One Runner can register.
One Xray profile can be created.
One client credential can be issued.
The profile compiles into Xray config.
The config passes xray run -test.
The Runner applies it atomically.
The deployment is observable.
The subscription URL works.
A bad deployment rolls back.
Audit and artifacts explain exactly what happened.
```

