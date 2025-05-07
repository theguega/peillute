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
RUST_LOG=DEBUG cargo run

# Terminal 2
RUST_LOG=INFO cargo run

# Terminal 3
RUST_LOG=ERROR cargo run
```
Le choix du port ainsi que les id de sites sont optionnels mais peuvent être spécifiés:
```sh
# Terminal 1
RUST_LOG=DEBUG cargo run -- --site-id A --port 8000

# Terminal 2
RUST_LOG=INFO cargo run -- --site-id B --port 8001

# Terminal 3
RUST_LOG=ERROR cargo run -- --site-id C --port 8002
```

## 🛠️ Développement et Tests

### Lancer les tests unitaires :
```sh
cargo test
```

### Formater le code:
```sh
cargo fmt
```