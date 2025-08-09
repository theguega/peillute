<p align="center">
  <a href="https://github.com/guegathe/peillute">
    <img src="assets/icon.png" alt="Logo" width="80" height="80">
  </a>
</p>

<h1 align="center">Peillute</h1>

<p align="center">
  A Distributed Cross-Platform Payment App in Rust
  <br />
  <a href="https://guegathe.gitlab.utc.fr/peillute/doc/peillute/"><strong>Explore the docs »</strong></a>
  <br />
  <br />
  <a href="https://gitlab.utc.fr/guegathe/peillute/-/pipelines">
    <img src="https://gitlab.utc.fr/guegathe/peillute/badges/main/pipeline.svg" alt="CI/CD Pipeline">
  </a>
  <a href="https://opensource.org/licenses/MIT">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
  </a>
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/Made%20with-Rust-orange?logo=rust" alt="Made with Rust">
  </a>
</p>

## 📖 About The Project

Peillute is a distributed, cross-platform payment application built with Rust. It's inspired by "Pay'UTC" and designed to explore concepts of distributed systems from the SR05 course at the Université de Technologie de Compiègne.

The core of Peillute is a peer-to-peer network where each node maintains a synchronized database. Communication happens over TCP, and data consistency is ensured using vector clocks and a snapshot mechanism.

This project also emphasizes a "production-like" development workflow, incorporating automatic testing, documentation generation, and CI/CD pipelines.

**Key Features:**

- **Distributed Database:** Each node holds a copy of the database.
- **P2P Networking:** Nodes automatically discover and connect to peers on the network.
- **Data Consistency:** Vector clocks for ordering transactions and snapshots for state consistency.
- **Cross-Platform:** Runs on the command line or as a web application thanks to the [Dioxus](https://dioxuslabs.com/) framework.
- **Modern Workflow:** Includes CI/CD, unit tests, and automated documentation.

<p align="center">
  <img src="assets/doc/peillute_pay_page.jpeg" alt="Pay page" width="49%">
  <img src="assets/doc/peillute_system_info.jpeg" alt="System Info" width="49%">
</p>

---

## 🚀 Getting Started

### Prerequisites

- **Rust & Cargo:** [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
- **Dioxus CLI:** Needed for web and desktop builds.
  ```sh
  cargo install dioxus-cli
  ```
- **Browser:** For the web interface, a Chromium-based browser or Safari is recommended as Dioxus may have issues with Firefox.

### Installation & Launch

The easiest way to get started is to use the provided launch script.

1.  **Clone the repository:**
    ```sh
    git clone https://gitlab.utc.fr/guegathe/peillute.git
    cd peillute
    ```
2.  **Run the launch script:**
    The script can install system dependencies (on Linux) and run the application with various flags.

    ```sh
    # Run with web UI (default)
    ./launch_peillute_instance.sh

    # Run in CLI mode
    ./launch_peillute_instance.sh -cli

    # Run a demo with pre-filled data
    ./launch_peillute_instance.sh -demo

    # For more options, see the script or use a potential -help flag
    ```

<details>
<summary>Manual Installation</summary>

If you prefer a manual setup:

1.  **Install Rust:**
    ```sh
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
2.  **Install `cargo-binstall` (for faster Dioxus CLI installation):**
    ```sh
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    ```
3.  **Install Dioxus CLI:**
    ```sh
    cargo binstall dioxus-cli
    ```
4.  **For Linux/Windows:** Check the [Dioxus Getting Started Guide](https://dioxuslabs.com/learn/0.6/getting_started/#) for any extra dependencies.

</details>

---

## 🛠️ Usage

Peillute can be run as a web application or a command-line tool.

### Web Application

The default `cargo run` command will start the web server.

```sh
# Start the web server
cargo run
```

You can also build a release version with Dioxus:

```sh
# Bundle the app for web
dx bundle --release --platform web

# Run the bundled server
cd target/dx/peillute/release/web
./server
```

### Command-Line Interface (CLI)

Use the `-cli` flag with the launch script for the CLI mode.

```sh
./launch_peillute_instance.sh -cli
```

To run the CLI mode manually, you will need to pass arguments to the application. You can likely see the available options with a help flag.

```sh
cargo run -- --help
```

### Advanced: Simulating a Network

You can simulate a distributed network by running multiple instances and manually specifying their peers.

<p align="center">
  <img src="assets/doc/peillute_network.png" alt="Network" width="80%">
</p>

<details>
<summary>Click to see commands for simulating the network above</summary>

Open multiple terminals and run the following commands:

```sh
# Terminal 1
RUST_LOG=debug cargo run -- --cli-port 10000 --cli-peers 127.0.0.1:10001,127.0.0.1:10003 --cli-db-id 0
# Terminal 2
RUST_LOG=debug cargo run -- --cli-port 10001 --cli-peers 127.0.0.1:10000,127.0.0.1:10002,127.0.0.1:10004 --cli-db-id 1
# Terminal 3
RUST_LOG=debug cargo run -- --cli-port 10002 --cli-peers 127.0.0.1:10001,127.0.0.1:10003 --cli-db-id 2
# Terminal 4
RUST_LOG=debug cargo run -- --cli-port 10003 --cli-peers 127.0.0.1:10000,127.0.0.1:10002 --cli-db-id 3
# Terminal 5
RUST_LOG=debug cargo run -- --cli-port 10004 --cli-peers 127.0.0.1:10001,127.0.0.1:10006,127.0.0.1:10005 --cli-db-id 4
# Terminal 6
RUST_LOG=debug cargo run -- --cli-port 10005 --cli-peers 127.0.0.1:10004,127.0.0.1:10009 --cli-db-id 5
# Terminal 7
RUST_LOG=debug cargo run -- --cli-port 10006 --cli-peers 127.0.0.1:10004,127.0.0.1:10007,127.0.0.1:10008 --cli-db-id 6
# Terminal 8
RUST_LOG=debug cargo run -- --cli-port 10007 --cli-peers 127.0.0.1:10006,127.0.0.1:10008 --cli-db-id 7
# Terminal 9
RUST_LOG=debug cargo run -- --cli-port 10008 --cli-peers 127.0.0.1:10006,127.0.0.1:10007 --cli-db-id 8
# Terminal 10
RUST_LOG=debug cargo run -- --cli-port 10009 --cli-peers 127.0.0.1:10005 --cli-db-id 9
```

</details>

---

## 🔬 Development & Testing

- **Run all tests:**
  ```sh
  cargo test --all-features
  ```
- **Format code:**
  ```sh
  cargo fmt
  ```
- **Generate and open documentation:**
  ```sh
  cargo doc --open
  ```

---

## 📜 License

Distributed under the MIT License. See `LICENSE` file for more information.
