# giw-desktop-cli

Desktop/local IndieBuild control client.

`giw-desktop-cli` talks only to `giw-desktop-daemon`. It does **not** supervise workers, launch
`cloudflared`, run keep-awake helpers, or execute update commands itself. That keeps CLI, Flutter,
and the native Rust desktop application on one control plane instead of creating competing process
owners.

The installed binary is `giw-desktop`.

## Commands

```bash
giw-desktop status
giw-desktop reconcile
giw-desktop processes

giw-desktop process start build-server
giw-desktop process stop build-server
giw-desktop process restart build-server

giw-desktop tunnel start
giw-desktop tunnel stop

giw-desktop keep-awake on
giw-desktop keep-awake off

giw-desktop update
```

The default daemon endpoint is `http://127.0.0.1:18440`. The CLI rejects non-loopback daemon URLs.
Override the endpoint with `--daemon-url` or `GIW_DESKTOP_URL` for alternate loopback ports only.

## Authentication

The CLI reads the daemon token from `~/.giw/desktop/token` by default. Override the file location
with `--token-file` or `GIW_DESKTOP_TOKEN_FILE`.

There is intentionally no `--token` option: bearer tokens should not appear in shell history or the
process list.

## Contract

The CLI currently supports desktop protocol version 1. When a daemon reports a newer incompatible
protocol version, the CLI fails closed and asks the operator to update rather than guessing at new
semantics.

Process names come from `giw-desktop-infra/.giw-desktop.yaml`; clients cannot send executable paths,
argv, shell fragments, environment values, Cloudflare credentials, or update commands to the daemon.

## Relationship to the existing cloud CLI

`gha-indie-worker-cli` remains the broader hosted/cloud product CLI. `giw-desktop-cli` is deliberately
narrow: it is the workstation-local control surface for the desktop daemon. The two CLIs can later
share generated contract/client code, but their trust boundaries and operating targets remain distinct.
