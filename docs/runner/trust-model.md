# Runner Trust Model

P0 runner safeguards implemented in code:

- Ed25519 signed command envelope
- canonical JSON command bytes for signing
- command TTL validation
- nonce replay rejection
- monotonic sequence validation
- node scope validation
- Ed25519 signed deployment result envelope
- control-plane verifies runner result signature against the node's registered public key before committing deployment status
- temp config write before validation
- `xray run -test` before activation
- atomic `active` symlink switch
- previous active release preserved on failed validation
- structured `RollbackDeployment` command switches `active` back to a known previous release
