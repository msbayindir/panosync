## What does this change?

<!-- One or two sentences. Link the issue if there is one: "Fixes #123". -->

## How was it tested?

<!-- Commands you ran and on which platforms (macOS / Linux X11 or Wayland / Windows). -->

## Checklist

- [ ] `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass
- [ ] Clipboard contents are never logged (byte counts at most)
- [ ] If the wire format or key derivation changed: the protocol version is bumped and `docs/architecture.md` is updated
- [ ] User-facing changes are added to `CHANGELOG.md` under *Unreleased*
