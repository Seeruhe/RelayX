# ADR 0002: Runner trust model

Runner accepts structured signed commands from the control plane. It verifies signature, TTL, nonce, monotonic sequence, and node scope before touching runtime config. It never executes free-form shell strings.
