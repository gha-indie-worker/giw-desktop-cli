# GHA Indie Worker — giw-desktop-cli

Workstation-local IndieBuild CLI and client of `giw-desktop-daemon`.

- The CLI is a client, not a supervisor or second control loop.
- Never accept bearer tokens as CLI arguments; read approved local token files instead.
- Refuse non-loopback daemon endpoints.
- Do not accept or forward arbitrary executable paths, shell fragments, environment values, or Cloudflare credentials.
- Preserve protocol-version fail-closed behavior.
- Rust-first tooling; no Python scripts.
- Shared cross-runtime contracts belong in `gha-indie-worker-interfaces` with TypeSpec/JSON Schema peer authority and TJSV parity.
- Resolve conflicts semantically; do not rebase, stash, reset, or force-push shared history.

For fleet-wide policy, read `ORESoftware/my-ai` `AGENTS.md` and `SHARED.md`.
