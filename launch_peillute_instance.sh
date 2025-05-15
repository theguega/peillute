#!/usr/bin/env bash

set -e

echo "[*] Launching Peillute instance..."

# Set default log level
LOG_LEVEL=""

# Check for -debug argument
for arg in "$@"; do
    if [ "$arg" == "-debug" ]; then
        LOG_LEVEL="debug"
    fi
done

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "[*] Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"
fi

# Check if cargo-binstall is installed
if ! command -v cargo-binstall &> /dev/null; then
    echo "[*] Installing cargo-binstall..."
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    source "$HOME/.cargo/env"
fi

# Check if dioxus-cli is installed
if ! command -v dx &> /dev/null; then
    echo "[*] Installing dioxus-cli..."
    cargo binstall dioxus-cli -y
fi

# Check if the bundle exists or is outdated
BUNDLE_PATH="target/dx/peillute/release/web/server"
SOURCE_HASH_FILE=".source_hash"

function current_source_hash() {
    find src -type f -name '*.rs' -exec md5sum {} \; | sort -k 2 | md5sum | awk '{print $1}'
}

SHOULD_REBUNDLE=false

if [[ ! -f "$BUNDLE_PATH" ]]; then
    echo "[*] No bundle found. Need to build."
    SHOULD_REBUNDLE=true
else
    CURRENT_HASH=$(current_source_hash)
    if [[ ! -f "$SOURCE_HASH_FILE" ]] || [[ "$CURRENT_HASH" != "$(cat "$SOURCE_HASH_FILE")" ]]; then
        echo "[*] Source has changed. Rebuilding..."
        SHOULD_REBUNDLE=true
    else
        echo "[*] Bundle is up to date."
    fi
fi

if [[ "$SHOULD_REBUNDLE" == true ]]; then
    echo "[*] Bundling project with Dioxus..."
    dx bundle --release --platform web
    current_source_hash > "$SOURCE_HASH_FILE"
fi

echo "[*] Running the Peillute instance..."
cd target/dx/peillute/release/web
RUST_LOG=$LOG_LEVEL ./server
