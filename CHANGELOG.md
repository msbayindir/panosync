# Changelog

All notable changes to this project are documented here.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-11-12

First public release. Speaks **protocol v1** (see [docs/architecture.md](docs/architecture.md)).

### Added

- `panosync`: clipboard sync client for macOS, Linux (X11 and Wayland) and Windows. Polls the clipboard,
  encrypts changes with XChaCha20-Poly1305 and sends them through the relay; writes text from other
  devices to the clipboard. Replay protection, 5-minute clock window, echo prevention, automatic reconnection.
- `panosync-relay`: relay server (WebSocket at `/ws?channel=…`, health check at `/health`). Limits: 1 MB per
  message, 16 devices per channel, 50 messages per 10 seconds per connection.
- Commands `setup`, `start`, `key`, `config` and `pause <minutes>`, with the Turkish aliases `kurulum`,
  `baslat`, `anahtar`, `ayar` and `duraklat`.
- `PANOSYNC_CONFIG_DIR` to use another config directory.
- Prebuilt binaries for macOS (arm64, x86_64), Linux (x86_64, aarch64) and Windows (x86_64), with SHA-256 checksums.
- Relay Docker image `ghcr.io/msbayindir/panosync-relay` for `linux/amd64` and `linux/arm64`.
- `install.sh` for macOS and Linux.
- Homebrew formula in `msbayindir/homebrew-tap` (`brew install msbayindir/tap/panosync`, with `brew services`),
  updated automatically when a release is published.
- cargo-binstall metadata: `cargo binstall --git https://github.com/msbayindir/panosync panosync`.
- systemd user service and launchd agent for the client, and a hardened systemd unit for the relay, in `packaging/`.
- Documentation: `docs/architecture.md` (protocol v1 and threat model), `docs/self-hosting.md` (Coolify,
  Docker, systemd, Caddy, nginx, local network) and a VHS demo (`docs/demo.tape`, `docs/demo.gif`).

[Unreleased]: https://github.com/msbayindir/panosync/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/msbayindir/panosync/releases/tag/v0.1.0
