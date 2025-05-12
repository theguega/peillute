# Distributed Application in Rust

This project is a distributed application in Rust using TCP for communication between nodes.
The goal is to manually implement mechanisms such as vector clocks, replica management, and snapshot taking.

## 🚀 Installation

### 1. Clone the repo
```sh
git clone https://gitlab.utc.fr/guegathe/peillute.git -j8
```

### 2. Install dependencies
Make sure you have Rust, Cargo, Dioxus, and their dependencies installed.

```sh
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Cargo bin-install
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install Dioxus
cargo binstall dioxus-cli
```

### For Linux and Windows users, refer to:
https://dioxuslabs.com/learn/0.6/getting_started/#

## 🚀 Compile and Run

### 1. Compile with Dioxus (merges client and server)
```sh
dx bundle --release --platform web
```

### 2. Run the binary:
```sh
cd target/dx/peillute/release/web
RUST_LOG=info ./server
```

## 🛠️ Development and Testing

### Run unit tests:
```sh
cargo test
```

### Format the code:
```sh
cargo fmt
```
