# Application Répartie en Rust

Ce projet est une application répartie en Rust utilisant TCP pour la communication entre les nœuds.
L'objectif est d'implémenter manuellement des mécanismes comme les horloges vectorielles, la gestion des réplicats et la prise de snapshots.

## 🚀 Installation

### 1. Cloner le repo
```sh
https://gitlab.utc.fr/guegathe/peillute.git -j8
```

### 2. Installer les dépendances
Assurez-vous d'avoir Rust et Cargo installés, puis exécutez :
```sh
# Check & Test
cargo check && cargo test

# Build
cargo build
```

## 📡 Lancer un nœud

Chaque instance fonctionne comme un nœud sur le réseau local. Exemple pour lancer 3 nœuds :
```sh
# Terminal 1
RUST_LOG=debug cargo run -- --site-id 1 --port 8000 --peers 127.0.0.1:8001,127.0.0.1:8002

# Terminal 2
RUST_LOG=info cargo run -- --site-id 2 --port 8001 --peers 127.0.0.1:8000,127.0.0.1:8002

# Terminal 3
RUST_LOG=error cargo run -- --site-id 3 --port 8002 --peers 127.0.0.1:8000,127.0.0.1:8001
```

## 🛠️ Développement et Tests

### Lancer les tests unitaires :
```sh
cargo test
```
