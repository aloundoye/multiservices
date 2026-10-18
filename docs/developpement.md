# Développement et distribution — Kër Finance

[Retour au README](../README.md) · [Guide d’utilisation](guide-utilisation.md) · [Référence métier](reference-metier.md)

Les commandes ci-dessous s’exécutent à la racine du dépôt. L’application associe une interface React et un moteur Rust dans une fenêtre Tauri ; aucun serveur applicatif ni fichier `.env` n’est nécessaire.

## Sommaire

- [Prérequis et lancement local](#prérequis-et-lancement-local)
- [Architecture](#architecture)
- [Générer un DMG sur macOS](#générer-un-dmg-sur-macos)
- [Générer les installateurs Windows](#générer-les-installateurs-windows)
- [Releases macOS et Windows](#releases-macos-et-windows)

## Prérequis et lancement local

Prérequis : **Node.js 24** avec npm (version utilisée en CI), **Rust stable** avec Cargo, Git pour cloner le dépôt et les [dépendances système de Tauri 2](https://v2.tauri.app/start/prerequisites/).

- Sur macOS : Xcode Command Line Tools ; la configuration du projet cible macOS 11 minimum.
- Sur Windows : Microsoft C++ Build Tools avec la charge de travail C++ pour les applications de bureau, et WebView2.

Pour récupérer le projet :

```bash
git clone https://github.com/aloundoye/multiservices.git
cd multiservices
```

```bash
npm ci
npm run tauri:dev
```

`npm run tauri:dev` lance Vite sur `http://127.0.0.1:1420` puis ouvre la fenêtre native. La première compilation Rust peut prendre plusieurs minutes. `Ctrl+C` arrête le lancement depuis le terminal.

`npm run dev` démarre uniquement l’interface web. Les commandes métier nécessitent le moteur Tauri : ouvrir seulement Vite dans un navigateur ne permet pas d’utiliser la boutique. `npm run preview` prévisualise de la même façon le frontend compilé.

Le mode développement conserve l’identifiant `sn.kerfinance.multiservices` : il peut utiliser les mêmes données que l’application installée pour le même utilisateur système. Utilisez un profil système distinct pour tester avec des données fictives et évitez d’ouvrir les deux versions simultanément.

Vérifications :

```bash
npm run build
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run release:check
```

`npm run test:watch` relance les tests frontend pendant le développement. Le workflow de release exécute les vérifications ci-dessus pour chaque plateforme.

### Stockage local

Tauri détermine le dossier de données à partir de l’identifiant `sn.kerfinance.multiservices`. Il contient `ker-finance.db` (base SQLCipher), `security.json` (enveloppes de clés) et `backups/` (copies `.msbackup`). L’accès quotidien dépend aussi du secret du coffre système. Pour déplacer les données, utilisez la restauration d’une sauvegarde avec le mot de passe de récupération, selon le [guide d’utilisation](guide-utilisation.md#sauvegarder-et-restaurer).

## Architecture

- `src/` : interface React 19 + TypeScript + Vite ;
- `src-tauri/src/domain.rs` : validations et règles comptables ;
- `src-tauri/src/db.rs` : schéma SQLite et transactions métier ;
- `src-tauri/src/accounts.rs` : comptes, validations, soldes et variations ;
- `src-tauri/src/migration_v2.sql` : migration transactionnelle multi-comptes ;
- `src-tauri/src/stock.rs` : catalogue, ventes, réapprovisionnements et mouvements de stock ;
- `src-tauri/src/migration_v3.sql` : tables des produits et liens entre stock et journal ;
- `src-tauri/src/custody.rs` : clients déposants, registre, reprises, annulations et rapprochement ;
- `src-tauri/src/migration_v4.sql` : dépôts clients, idempotence et instantanés aux clôtures ;
- `src-tauri/src/debt_clients.rs` : fiches des dettes, aperçus et remboursements globaux ;
- `src-tauri/src/migration_v5.sql` : répertoire des dettes, reçus et allocations immuables ;
- `src-tauri/src/security.rs` : enveloppes de clés et chiffrement ;
- `src-tauri/src/backup.rs` : sauvegarde, rétention, contrôle et restauration ;
- `src-tauri/src/export.rs` : exports PDF, XLSX et CSV.

Toutes les écritures transitent par des commandes Tauri typées et sont validées en Rust. Le frontend n’accède jamais directement à la base.

## Générer un DMG sur macOS

Prérequis : macOS 11 ou plus récent, Xcode Command Line Tools, Node.js 24 et Rust stable.

Créer l’application macOS et l’image d’installation DMG :

```bash
npm ci
npm run tauri:build:mac
```

La commande construit d’abord le `.app` avec Tauri, puis crée le DMG avec l’outil natif `hdiutil`. Elle est entièrement non interactive et ne dépend pas de Finder.

Les fichiers sont générés dans :

- `src-tauri/target/release/bundle/macos/Kër Finance.app`
- `src-tauri/target/release/bundle/dmg/Kër Finance_0.5.0_<architecture>.dmg`

Le DMG est généré pour l’architecture du Mac qui exécute la commande (`arm64` ou `x86_64`). Pour reproduire la signature ad hoc utilisée en CI, exécutez `APPLE_SIGNING_IDENTITY=- npm run tauri:build:mac`. Une distribution notarisée nécessite une signature Developer ID et une notarisation Apple.

## Générer les installateurs Windows

Exécuter sur Windows avec Node.js 24, Rust stable, Microsoft C++ Build Tools et WebView2 :

```powershell
npm ci
npm run tauri:build:windows
```

L’installateur EXE NSIS est généré dans `src-tauri/target/release/bundle/nsis/`. Pour générer à la fois le MSI et l’EXE NSIS, utilisez `npm run tauri:build` ; les sorties se trouvent dans les sous-dossiers `msi/` et `nsis/` de `src-tauri/target/release/bundle/`.

Les tests comptables utilisent une base SQLite en mémoire et un test séparé vérifie SQLCipher avec une vraie base chiffrée. L’option avancée `cipher_memory_security` reste désactivée : avec OpenSSL statique sous Windows, son allocateur global provoque un `STATUS_STACK_OVERFLOW`. Cette option ne contrôle pas le chiffrement du fichier, qui reste actif via `PRAGMA key`.

## Releases macOS et Windows

Le workflow [Release installers](../.github/workflows/release.yml) se déclenche lorsque vous poussez un tag `vX.Y.Z`. GitHub Actions compile sur des runners macOS et Windows : vous pouvez déclencher les trois builds depuis votre Mac sans y installer de compilateur Windows. Les variantes générées sont :

- `Ker-Finance_X.Y.Z_macos-arm64.dmg` pour les Mac Apple Silicon ;
- `Ker-Finance_X.Y.Z_macos-x64.dmg` pour les Mac Intel ;
- `Ker-Finance_X.Y.Z_windows-x64-setup.exe` pour Windows 64 bits.

Chaque variante exécute les tests frontend et Rust, le build frontend et Clippy avant de générer son installateur. Après réussite des trois builds, les installateurs et `SHA256SUMS.txt` sont joints à un **brouillon de release GitHub**. Les artefacts `installer-macos-arm64`, `installer-macos-x64` et `installer-windows-x64` restent également disponibles pendant 30 jours dans l’onglet Actions. Le workflow vérifie que le tag, les manifestes npm/Tauri/Rust et leurs fichiers de verrouillage portent la même version, et que les trois builds proviennent du même commit.

Pour une prochaine version :

1. Mettez à jour `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et l’entrée du projet dans `src-tauri/Cargo.lock`.
2. Exécutez les vérifications, puis enregistrez et poussez les changements, y compris le workflow de release.
3. Créez et poussez le tag correspondant à la nouvelle version. Par exemple, si la prochaine version choisie est **0.6.0**, après mise à jour des manifestes :

```bash
npm run release:check -- v0.6.0
git tag -a v0.6.0 -m "Kër Finance 0.6.0"
git push origin v0.6.0
```

Chaque version utilise un nouveau tag et une nouvelle release. Ne recréez pas et ne déplacez pas un tag existant. Une fois le workflow présent sur la branche principale, **Actions → Release installers → Run workflow** permet de relancer un tag existant. Sélectionnez la branche contenant le workflow à jour et indiquez le tag à reconstruire. Le code de l’application est toujours extrait du commit de ce tag. Une relance réutilise le brouillon existant et remplace ses fichiers ; elle refuse de modifier une release déjà publiée.

Vérifiez les installateurs sur les systèmes cibles avant de publier le brouillon. Les DMG de ce workflow sont signés ad hoc, sans notarisation Apple ; l’EXE Windows n’est pas signé par un certificat éditeur. La [documentation Tauri sur les releases GitHub](https://v2.tauri.app/distribute/pipelines/github/) décrit la configuration des certificats pour une distribution signée.

La publication du brouillon reste une action manuelle depuis la page de release GitHub.
