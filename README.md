# Distributed Application in Rust

This project is a distributed application in Rust using TCP for communication between nodes. The goal is to manually implement mechanisms such as vector clocks, replica management, and snapshot taking.

## Project documentation

All documentation can be found here : [peillute](https://guegathe.gitlab.utc.fr/peillute/doc/peillute/)

## 🚀 Installation

### Prerequisites

Make sure you have the following installed on your system:
- Rust
- Cargo
- Dioxus

### 1. Clone the Repository

```sh
git clone https://gitlab.utc.fr/guegathe/peillute.git -j8
```

### 2. Automatically Install Dependencies and Run Peillute Instance

```sh
./launch_peillute_instance.sh

# To be more verbose:
./launch_peillute_instance.sh -debug
```

### 3. Manually Install Dependencies

If you prefer to install dependencies manually, follow these steps:

#### Install Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Install Cargo bin-install

```sh
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
```

#### Install Dioxus

```sh
cargo binstall dioxus-cli
```

#### For Linux and Windows Users

Refer to the [Dioxus Getting Started Guide](https://dioxuslabs.com/learn/0.6/getting_started/#) for additional setup instructions.

## 🚀 Compile and Run

### 1. Compile and Run Without the UI

```sh
cargo run --release
```

### Use Arguments to Specify the Port and Peers

```sh
# create a non perfect network with manual peers :
# terminal 1 :
RUST_LOG=debug cargo run --features server -- --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10002
# terminal 2 :
RUST_LOG=debug cargo run --features server -- --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002
# terminal 3 :
RUST_LOG=debug cargo run --features server -- --port 10002 --peers 127.0.0.1:10000,127.0.0.1:10001
# terminal 4 :
RUST_LOG=debug cargo run --features server -- --port 10003 --peers 127.0.0.1:10001,127.0.0.1:10002
```

### 2. Compile with Dioxus (Merges Client and Server)

```sh
dx bundle --release --platform web
```

### 3. Run the Binary

Manually run the server:

```sh
# one instance
cd target/dx/peillute/release/web
./server

# create a non perfect network with manual peers :
# terminal 1 :
RUST_LOG=debug ./server --port 10000 --peers 127.0.0.1:10001,127.0.0.1:10002
# terminal 2 :
RUST_LOG=debug ./server --port 10001 --peers 127.0.0.1:10000,127.0.0.1:10002
# terminal 3 :
RUST_LOG=debug ./server --port 10002 --peers 127.0.0.1:10000,127.0.0.1:10001
# terminal 4 :
RUST_LOG=debug ./server --port 10003 --peers 127.0.0.1:10001,127.0.0.1:10002
```

## 🛠️ Development and Testing

### Run Unit Tests

```sh
cargo test --all-features
```

### Format the Code

```sh
cargo fmt
```

### Generate the documentation

```sh
cargo doc
```

## 📜 License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.
