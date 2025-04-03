# Application Répartie en Rust

Ce projet est une application répartie en Rust utilisant TCP pour la communication entre les nœuds.
L'objectif est d'implémenter manuellement des mécanismes comme les horloges vectorielles, la gestion des réplicats et la prise de snapshots.

tokio : Pour la gestion asynchrone et les connexions TCP.
serde et bincode : Pour la sérialisation et la désérialisation des messages entre les nœuds.
clap : Pour la gestion des arguments de la ligne de commande.
tracing : Pour les logs détaillés.

## 🚀 Installation

### 1. Cloner le dépôt
```sh

```

### 2. Installer les dépendances
Assurez-vous d'avoir Rust et Cargo installés, puis exécutez :
```sh
cargo build
```

## 📡 Lancer un nœud

Chaque instance fonctionne comme un nœud sur le réseau local. Pour en lancer un :
```sh
cargo run -- <adresse_ip> <port>
```
Exemple :
```sh
cargo run -- 127.0.0.1 8080
```

## 🛠️ Développement et Tests

### Lancer les tests unitaires :
```sh
cargo test
```

## 📜 Fonctionnalités prévues
- [ ] Communication pair-à-pair via TCP
- [ ] Gestion de la cohérence des réplicats
- [ ] Algorithme d’exclusion mutuelle
- [ ] Implémentation des horloges vectorielles
- [ ] Snapshots distribués
