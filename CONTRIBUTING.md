# Contributing

Thanks for your interest in panosync! Bug reports, fixes and small focused improvements are very welcome.

## Before you start

- For anything bigger than a small fix, please open an issue first so we can agree on the approach.
  panosync aims to stay small; features that make it significantly bigger or harder to set up may be declined.
- **Changes to the cryptography, key derivation or message format need an issue first**, and they must
  bump the protocol version (see the versioning rule in [docs/architecture.md](docs/architecture.md)).
- Security problems: please follow [SECURITY.md](SECURITY.md) instead of opening an issue.

## Development

You need Rust 1.88 or newer.

```bash
cargo build
cargo test                    # includes an end-to-end test against a real relay and CLI tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

CI runs `cargo fmt --check`, clippy (warnings are errors) and the tests on Linux, macOS and Windows.

On Linux you may need the X11/Wayland development headers your distribution uses for clipboard access.
The tests do not touch the real clipboard.

## Conventions

- **Never log clipboard contents.** Log byte counts at most. This is the one hard rule.
- Code, comments and user-facing text are in English. Commands keep their Turkish aliases.
  Each module starts with a short comment explaining what it does; keep it accurate.
- Add tests for new behaviour. Keep dependencies to a minimum.
- Add a line to `CHANGELOG.md` under *Unreleased*.

## License

By contributing, you agree that your contributions are dual licensed under MIT OR Apache-2.0,
as described in the [README](README.md#license).
