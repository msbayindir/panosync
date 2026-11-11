#!/bin/sh
# Installs the panosync client on macOS or Linux from the GitHub releases.
#
#   curl -fsSL https://raw.githubusercontent.com/msbayindir/panosync/main/install.sh | sh
#
# Environment variables:
#   PANOSYNC_VERSION      release tag to install, e.g. v0.1.0 (default: latest)
#   PANOSYNC_INSTALL_DIR  where to put the binary (default: ~/.local/bin)
#
# The archive is verified against the release's SHA256SUMS before anything is installed.

set -eu

REPO="msbayindir/panosync"
VERSION="${PANOSYNC_VERSION:-latest}"
INSTALL_DIR="${PANOSYNC_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
fail() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Darwin)
            # A shell running under Rosetta reports x86_64 on Apple silicon; prefer the native binary.
            if [ "$arch" = "x86_64" ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = "1" ]; then
                arch="arm64"
            fi
            case "$arch" in
                arm64 | aarch64) echo "aarch64-apple-darwin" ;;
                x86_64) echo "x86_64-apple-darwin" ;;
                *) fail "unsupported macOS architecture: $arch" ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64 | amd64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64 | arm64) echo "aarch64-unknown-linux-gnu" ;;
                *) fail "unsupported Linux architecture: $arch (build from source: cargo install --git https://github.com/$REPO panosync)" ;;
            esac
            ;;
        *) fail "unsupported OS: $os. On Windows, download the .zip from https://github.com/$REPO/releases" ;;
    esac
}

download() { # url destination
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL --proto '=https' --tlsv1.2 -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -q --https-only -O "$2" "$1"
    else
        fail "curl or wget is required"
    fi
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        fail "sha256sum or shasum is required to verify the download"
    fi
}

main() {
    target="$(detect_target)"
    archive="panosync-$target.tar.gz"
    if [ "$VERSION" = "latest" ]; then
        base="https://github.com/$REPO/releases/latest/download"
    else
        base="https://github.com/$REPO/releases/download/$VERSION"
    fi

    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT INT TERM

    say "Downloading $archive ($VERSION)..."
    download "$base/$archive" "$tmp/$archive" || fail "download failed: $base/$archive"
    download "$base/SHA256SUMS" "$tmp/SHA256SUMS" || fail "download failed: $base/SHA256SUMS"

    expected="$(awk -v f="$archive" '$2 == f || $2 == "*" f { print $1 }' "$tmp/SHA256SUMS")"
    [ -n "$expected" ] || fail "$archive is not listed in SHA256SUMS"
    actual="$(sha256 "$tmp/$archive")"
    [ "$expected" = "$actual" ] || fail "checksum mismatch for $archive (expected $expected, got $actual)"
    say "Checksum OK."

    tar -xzf "$tmp/$archive" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    # Copy to a temporary name first so a running panosync is not left with a half-written file.
    cp "$tmp/panosync" "$INSTALL_DIR/.panosync.new"
    chmod 755 "$INSTALL_DIR/.panosync.new"
    mv -f "$INSTALL_DIR/.panosync.new" "$INSTALL_DIR/panosync"

    say "Installed: $INSTALL_DIR/panosync ($("$INSTALL_DIR/panosync" --version))"
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *) say "" ; say "Note: $INSTALL_DIR is not on your PATH. Add it, e.g.:" ; say "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
    esac
    say ""
    say "Next: panosync setup --server wss://<your relay>"
    say "Docs: https://github.com/$REPO#quick-start"
}

main "$@"
