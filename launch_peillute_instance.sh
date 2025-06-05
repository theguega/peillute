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

for arg in "$@"; do
    if [ "$arg" == "-install" ]; then
        if [ "$(uname)" == "Linux" ]; then
            # Détection de la distribution via /etc/os-release
            if [ -f /etc/os-release ]; then
                . /etc/os-release
                distro=$ID
            else
                echo "Impossible de détecter la distribution Linux."
                exit 1
            fi

            case "$distro" in
                debian|ubuntu)
                    echo "Distribution détectée : $distro"
                    sudo apt update
                    sudo apt install -y libwebkit2gtk-4.1-dev \
                        build-essential \
                        curl \
                        wget \
                        file \
                        libxdo-dev \
                        libssl-dev \
                        libayatana-appindicator3-dev \
                        librsvg2-dev
                    ;;
                fedora)
                    echo "Distribution détectée : Fedora"
                    sudo dnf install -y webkit2gtk4.1-devel \
                        gcc-c++ \
                        curl \
                        wget \
                        file \
                        libxdo-devel \
                        openssl-devel \
                        libappindicator-gtk3 \
                        librsvg2-devel
                    ;;
                *)
                    echo "Distribution $distro non supportée par ce script."
                    exit 1
                    ;;
            esac
        fi
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

# Function to run demo
run_demo() {
    echo "[*] Running the Peillute demo..."
    cd target/dx/peillute/release/web

    # Open 10 terminals with different arguments
    if [ "$(uname)" == "Darwin" ]; then
        # macOS
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 0\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002,127.0.0.1:10004 --db-id 1\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10002 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 2\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10003 --peers 127.0.0.1:10000,127.0.0.1:10002 --db-id 3\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10004 --peers 127.0.0.1:10001,127.0.0.1:10006,127.0.0.1:10005 --db-id 4\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10005 --peers 127.0.0.1:10004,127.0.0.1:10009 --db-id 5\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10006 --peers 127.0.0.1:10004,127.0.0.1:10007,127.0.0.1:10008 --db-id 6\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10007 --peers 127.0.0.1:10006,127.0.0.1:10008 --db-id 7\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10008 --peers 127.0.0.1:10006,127.0.0.1:10007 --db-id 8\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10009 --peers 127.0.0.1:10005 --db-id 9\""
    else
        # Linux
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 0; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002,127.0.0.1:10004 --db-id 1; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10002 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 2; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10003 --peers 127.0.0.1:10000,127.0.0.1:10002 --db-id 3; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10004 --peers 127.0.0.1:10001,127.0.0.1:10006,127.0.0.1:10005 --db-id 4; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10005 --peers 127.0.0.1:10004,127.0.0.1:10009 --db-id 5; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10006 --peers 127.0.0.1:10004,127.0.0.1:10007,127.0.0.1:10008 --db-id 6; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10007 --peers 127.0.0.1:10006,127.0.0.1:10008 --db-id 7; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10008 --peers 127.0.0.1:10006,127.0.0.1:10007 --db-id 8; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL ./server --port 10009 --peers 127.0.0.1:10005 --db-id 9; exec bash"
    fi
}

# Function to run demo_cli
run_demo_cli() {
    echo "[*] Running the Peillute demo_cli..."

    # Open 10 terminals with different arguments
    if [ "$(uname)" == "Darwin" ]; then
        # macOS
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 0\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002,127.0.0.1:10004 --db-id 1\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10002 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 2\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10003 --peers 127.0.0.1:10000,127.0.0.1:10002 --db-id 3\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10004 --peers 127.0.0.1:10001,127.0.0.1:10006,127.0.0.1:10005 --db-id 4\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10005 --peers 127.0.0.1:10004,127.0.0.1:10009 --db-id 5\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10006 --peers 127.0.0.1:10004,127.0.0.1:10007,127.0.0.1:10008 --db-id 6\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10007 --peers 127.0.0.1:10006,127.0.0.1:10008 --db-id 7\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10008 --peers 127.0.0.1:10006,127.0.0.1:10007 --db-id 8\""
        sleep 1
        osascript -e "tell app \"Terminal\" to do script \"cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10009 --peers 127.0.0.1:10005 --db-id 9\""
    else
        # Linux
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 0; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002,127.0.0.1:10004 --db-id 1; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10002 --peers 127.0.0.1:10001,127.0.0.1:10003 --db-id 2; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10003 --peers 127.0.0.1:10000,127.0.0.1:10002 --db-id 3; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10004 --peers 127.0.0.1:10001,127.0.0.1:10006,127.0.0.1:10005 --db-id 4; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10005 --peers 127.0.0.1:10004,127.0.0.1:10009 --db-id 5; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10006 --peers 127.0.0.1:10004,127.0.0.1:10007,127.0.0.1:10008 --db-id 6; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10007 --peers 127.0.0.1:10006,127.0.0.1:10008 --db-id 7; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10008 --peers 127.0.0.1:10006,127.0.0.1:10007 --db-id 8; exec bash"
        sleep 1
        gnome-terminal -- bash -c "cd $(pwd) && RUST_LOG=$LOG_LEVEL cargo run -- --port 10009 --peers 127.0.0.1:10005 --db-id 9; exec bash"
    fi
}

# Function to clean db and snapshots
clean_db_and_snapshots() {
    echo "[*] Cleaning db and snapshots..."
    rm -f peillute* 2>/dev/null
    rm -f snapshot* 2>/dev/null
    rm -f target/dx/peillute/release/web/peillute* 2>/dev/null
    rm -f target/dx/peillute/release/web/snapshot* 2>/dev/null
}

# Check for -demo argument
for arg in "$@"; do
    if [ "$arg" == "-clean" ]; then
        clean_db_and_snapshots
        exit 0
    fi
    if [ "$arg" == "-demo" ]; then
        run_demo
        exit 0
    fi
    if [ "$arg" == "-demo_cli" ]; then
        run_demo_cli
        exit 0
    fi
done

echo "[*] Running the Peillute instance..."
cd target/dx/peillute/release/web
RUST_LOG=$LOG_LEVEL ./server
