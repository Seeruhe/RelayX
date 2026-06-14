# Profile IR v0

Profile IR captures business intent:

- runtime target (`xray` first)
- inbound protocol and port
- security mode (REALITY in P0)
- client group references
- DNS/routing placeholders

Secrets are always referenced by ID; inline secret material is rejected by domain validation.
