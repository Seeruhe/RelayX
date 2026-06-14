# Architecture v0.1

The control plane treats proxy management as the product. Profile IR is the source of truth, compiled artifacts are immutable, and the runner is a trusted executor rather than an externally exposed agent.

```text
Admin / Web Console
  -> control-plane API
  -> Profile IR / Artifact / Audit / Outbox storage
  -> signed RunnerCommand
  -> runner
  -> xray-core
```

P0 intentionally excludes AI, A2A, marketplace, payments, and local models. Future AI integrations must produce typed proposal artifacts only.
