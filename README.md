# Kër Finance

Application desktop locale de gestion d’un multiservices au Sénégal : inventaires Orange Money, Wave, Djamo et espèces, journal de boutique, dettes clients, rapports et sauvegardes chiffrées.

## Fonctionnalités

- capital initial réparti avec contrôle d’égalité exact ;
- inventaires périodiques, comparaison par compte et justification des écarts ;
- capital réel incluant les créances non soldées ;
- recettes, commissions, apports, achats, dépenses et retraits de capital ;
- dettes Orange Money/Wave avec échéances et remboursements partiels ;
- contre-écritures et journal d’audit immuable ;
- rapports PDF, Excel et CSV ;
- base SQLite chiffrée par SQLCipher ;
- clé quotidienne protégée par le PIN et le coffre système (Trousseau macOS ou Gestionnaire d’identifiants Windows) ;
- sauvegardes automatiques chiffrées et restauration par mot de passe de récupération.

## Architecture

- `src/` : interface React 19 + TypeScript + Vite ;
- `src-tauri/src/domain.rs` : validations et règles comptables ;
- `src-tauri/src/db.rs` : schéma SQLite et transactions métier ;
- `src-tauri/src/security.rs` : enveloppes de clés et chiffrement ;
- `src-tauri/src/backup.rs` : sauvegarde, rétention, contrôle et restauration ;
- `src-tauri/src/export.rs` : exports PDF, XLSX et CSV.

Toutes les écritures transitent par des commandes Tauri typées et sont validées en Rust. Le frontend n’accède jamais directement à la base.

## Développement

Prérequis : Node.js 20+, Rust stable et les [prérequis Tauri 2](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri:dev
```

Vérifications :

```bash
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Utiliser et installer sur macOS

Prérequis : macOS 11 ou plus récent, Xcode Command Line Tools, Node.js 20+ et Rust stable.

Lancer l’application en développement :

```bash
npm install
npm run tauri:dev
```

Créer l’application macOS et l’image d’installation DMG :

```bash
npm run tauri:build:mac
```

La commande construit d’abord le `.app` avec Tauri, puis crée le DMG avec l’outil natif `hdiutil`. Elle est entièrement non interactive et ne dépend pas de Finder.

Les fichiers sont générés dans :

- `src-tauri/target/release/bundle/macos/Kër Finance.app`
- `src-tauri/target/release/bundle/dmg/Kër Finance_0.1.0_<architecture>.dmg`

Le build local n’est pas signé par Apple. Il fonctionne sur la machine de développement ; pour le distribuer à d’autres personnes sans alerte Gatekeeper, il faudra ajouter une signature Developer ID et une notarisation Apple.

## Générer les installateurs Windows

Exécuter sur Windows 10/11 avec Microsoft C++ Build Tools et WebView2 :

```powershell
npm ci
npm run tauri:build
```

Les installateurs MSI et NSIS sont générés sous `src-tauri/target/release/bundle/`.

La configuration Windows réserve une pile de 32 Mio au binaire et au test SQLCipher dédié. Les tests comptables utilisent une base SQLite en mémoire et s’exécutent séquentiellement dans GitHub Actions. Cette séparation évite les erreurs `STATUS_STACK_OVERFLOW` de SQLCipher/OpenSSL dans les builds MSVC non optimisés sans retirer la vérification du chiffrement.

## Sécurité et récupération

Au premier démarrage, le gérant choisit :

1. un PIN numérique de 4 à 12 chiffres pour l’usage quotidien ;
2. un mot de passe de récupération d’au moins 12 caractères.

Le mot de passe de récupération doit être conservé hors du PC. Il est indispensable pour restaurer une sauvegarde sur un autre ordinateur. Kër Finance ne possède aucun serveur capable de le récupérer.

## Limites de la V1

- un seul PC, une seule boutique et un seul profil gérant ;
- pas de synchronisation cloud ;
- pas de connexion aux API Orange Money, Wave ou Djamo ;
- pas de gestion de stock ;
- les soldes affichés sont ceux du dernier inventaire validé.
