# Kër Finance

Application desktop locale de gestion d’un multiservices au Sénégal : inventaires Orange Money, Wave, Djamo et espèces, produits, stock, ventes, journal de boutique, dettes clients, rapports et sauvegardes chiffrées.

## Installer la version 0.3.0

Les installateurs sont disponibles dans les [releases GitHub](https://github.com/aloundoye/multiservices/releases). Les versions en brouillon restent accessibles aux mainteneurs jusqu’à leur publication.

| Système | Fichier à télécharger |
| --- | --- |
| Mac Apple Silicon (puces M1, M2, etc.) | `Ker-Finance_0.3.0_macos-arm64.dmg` |
| Mac Intel | `Ker-Finance_0.3.0_macos-x64.dmg` |
| Windows 64 bits (x64) | `Ker-Finance_0.3.0_windows-x64-setup.exe` |

Sur macOS, ouvrez le DMG et glissez **Kër Finance** dans **Applications**. Sur Windows, lancez l’EXE et suivez l’assistant d’installation. Node.js, Rust et les outils de compilation ne sont nécessaires que pour le développement.

Les applications macOS sont signées ad hoc, sans notarisation Apple ; l’installateur Windows n’a pas de signature éditeur. Le système peut donc afficher une alerte à l’ouverture.

Chaque release contient `SHA256SUMS.txt`. Pour vérifier un téléchargement, comparez la somme calculée à la ligne correspondant au fichier :

```bash
# macOS : adapter le nom du fichier pour un Mac Intel.
shasum -a 256 Ker-Finance_0.3.0_macos-arm64.dmg
```

```powershell
# Windows PowerShell
Get-FileHash .\Ker-Finance_0.3.0_windows-x64-setup.exe -Algorithm SHA256
```

## Fonctionnalités

- capital initial réparti avec contrôle d’égalité exact ;
- inventaires périodiques, comparaison par compte et justification des écarts ;
- capital réel incluant les créances non soldées ;
- recettes, commissions, apports, achats, dépenses et retraits de capital ;
- catalogue de produits, suivi du stock, ventes et réapprovisionnements reliés au capital attendu ;
- plusieurs comptes/SIM Orange Money, Wave et Djamo, avec une caisse espèces unique ;
- dettes Orange Money/Wave/Djamo avec échéances et remboursements partiels sur le compte choisi ;
- contre-écritures et journal d’audit immuable ;
- rapports PDF, Excel et CSV ;
- base SQLite chiffrée par SQLCipher ;
- clé quotidienne protégée par le PIN et le coffre système (Trousseau macOS ou Gestionnaire d’identifiants Windows) ;
- sauvegardes automatiques chiffrées et restauration par mot de passe de récupération.

## Produits et ventes

Le catalogue permet de saisir le prix de vente et le stock initial déjà détenu (sans dépense supplémentaire). Le gérant peut ensuite :

- vendre plusieurs produits à la fois avec un prix ajustable et un compte d’encaissement précis ;
- recevoir du stock en enregistrant automatiquement le montant total payé comme achat ;
- corriger les quantités après comptage, avec un motif et sans mouvement d’argent ;
- annuler une opération complète, depuis son détail ou le journal, avec une correction simultanée du stock et du budget.

Les ventes augmentent le **Capital attendu** du montant encaissé et les achats le diminuent. Les soldes vérifiés restent ceux du dernier inventaire financier. Le stock est suivi en unités entières et sa valeur n’est pas ajoutée au capital. L’historique conserve les noms et tarifs d’origine. Un produit à stock nul peut être archivé ; annuler une ancienne vente le réactive si des articles reviennent en stock.

Les écritures automatiques figurent déjà dans les rapports et exports : ne les ressaisissez pas dans le journal. La migration vers le schéma 3 conserve les comptes et données précédentes ; les anciennes sauvegardes restent restaurables.

## Architecture

- `src/` : interface React 19 + TypeScript + Vite ;
- `src-tauri/src/domain.rs` : validations et règles comptables ;
- `src-tauri/src/db.rs` : schéma SQLite et transactions métier ;
- `src-tauri/src/accounts.rs` : comptes, validations, soldes et variations ;
- `src-tauri/src/migration_v2.sql` : migration transactionnelle multi-comptes ;
- `src-tauri/src/stock.rs` : catalogue, ventes, réapprovisionnements et mouvements de stock ;
- `src-tauri/src/migration_v3.sql` : tables des produits et liens entre stock et journal ;
- `src-tauri/src/security.rs` : enveloppes de clés et chiffrement ;
- `src-tauri/src/backup.rs` : sauvegarde, rétention, contrôle et restauration ;
- `src-tauri/src/export.rs` : exports PDF, XLSX et CSV.

Toutes les écritures transitent par des commandes Tauri typées et sont validées en Rust. Le frontend n’accède jamais directement à la base.

## Comptes et SIM

À l’ouverture, un compte par service est proposé. Ajoutez les SIM nécessaires et répartissez le capital entre elles et la caisse. La somme doit être exactement égale au capital initial ; les montants sont des FCFA entiers positifs ou nuls.

Ensuite, **Paramètres → Comptes et SIM** permet de créer, renommer, archiver et réactiver les comptes. Chaque compte a un nom obligatoire et un identifiant facultatif. Le nom est unique dans son service, sans distinction de casse, y compris parmi les comptes archivés. Le service et l’ID permanent ne changent jamais.

- Un compte ajouté ne modifie aucun capital. Il affiche « Pas encore relevé » jusqu’au prochain inventaire, où son solde devient obligatoire.
- Un transfert entre deux SIM change seulement la répartition. Ne l’enregistrez pas comme une recette. Un apport réel de capital doit en revanche être inscrit au journal.
- Les inventaires demandent un solde pour chaque compte actif et affichent les sous-totaux par service. La variation du premier relevé d’un compte est indisponible, et non supposée nulle.
- Pour archiver un compte utilisé, son dernier solde doit être nul et aucune opération ne doit avoir été enregistrée sur ce compte depuis. Un compte jamais utilisé peut être archivé directement. La caisse ne peut pas être archivée ni dupliquée.
- Les anciennes dettes d’un compte archivé peuvent être remboursées sur une SIM active ou en espèces. Les contre-écritures gardent le compte et le libellé de l’écriture originale, même après archivage ou renommage.
- L’historique et les exports conservent les noms et identifiants au moment des opérations. Dans les exports CSV, les lignes `solde_compte` détaillent les lignes `inventaire` : ne les additionnez pas ensemble. Dans Excel, la synthèse et les soldes détaillés sont dans des feuilles distinctes.

## Mise à niveau des données et sauvegardes

La version 0.3.0 utilise le **schéma SQLite 3**. Les bases en version 1 ou 2 sont migrées au prochain déverrouillage. Conservez une sauvegarde chiffrée externe avant une mise à niveau.

Depuis le schéma 1, l’application crée et vérifie d’abord une sauvegarde chiffrée `avant-migration-v1-….msbackup`. Si cette étape échoue, les données restent en version 1. La migration ajoute les comptes Orange Money — Principal, Wave — Principal, Djamo — Principal et Espèces. Les anciens soldes sont marqués « historique regroupé par service » : aucune répartition ancienne entre SIM n’est inventée.

Le passage au schéma 3 ajoute les produits, ventes, réapprovisionnements et mouvements de stock. Les montants, dates, références, corrections, écarts et comptes existants sont conservés. La migration est atomique, auditée et ne s’exécute qu’une fois.

Les sauvegardes au schéma 3 contiennent les comptes, leurs relevés et libellés historiques, ainsi que les produits et opérations de stock. Une sauvegarde au schéma 1 ou 2 est déchiffrée, contrôlée puis migrée dans une copie temporaire avant de remplacer les données actives. Un mot de passe incorrect ou une migration invalide interrompt la restauration. Ne rouvrez pas une base au schéma 3 avec une ancienne version de l’application.

## Développement

Prérequis : Node.js 24 (version utilisée en CI), Rust stable et les [prérequis Tauri 2](https://v2.tauri.app/start/prerequisites/).

```bash
npm ci
npm run tauri:dev
```

Vérifications :

```bash
npm run build
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run release:check
```

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
- `src-tauri/target/release/bundle/dmg/Kër Finance_0.3.0_<architecture>.dmg`

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

Le workflow [Release installers](.github/workflows/release.yml) se déclenche lorsque vous poussez un tag `vX.Y.Z`. GitHub Actions compile sur des runners macOS et Windows : vous pouvez déclencher les trois builds depuis votre Mac sans y installer de compilateur Windows. Les variantes générées sont :

- `Ker-Finance_X.Y.Z_macos-arm64.dmg` pour les Mac Apple Silicon ;
- `Ker-Finance_X.Y.Z_macos-x64.dmg` pour les Mac Intel ;
- `Ker-Finance_X.Y.Z_windows-x64-setup.exe` pour Windows 64 bits.

Chaque variante exécute les tests frontend et Rust, le build frontend et Clippy avant de générer son installateur. Après réussite des trois builds, les installateurs et `SHA256SUMS.txt` sont joints à un **brouillon de release GitHub**. Les artefacts `installer-macos-arm64`, `installer-macos-x64` et `installer-windows-x64` restent également disponibles pendant 30 jours dans l’onglet Actions. Le workflow vérifie que le tag, les manifestes npm/Tauri/Rust et leurs fichiers de verrouillage portent la même version, et que les trois builds proviennent du même commit.

Pour une prochaine version :

1. Mettez à jour `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et l’entrée du projet dans `src-tauri/Cargo.lock`.
2. Exécutez les vérifications, puis enregistrez et poussez les changements, y compris le workflow de release.
3. Créez et poussez le tag correspondant à la nouvelle version. Par exemple, après passage à **0.3.1** :

```bash
npm run release:check -- v0.3.1
git tag -a v0.3.1 -m "Kër Finance 0.3.1"
git push origin v0.3.1
```

Le tag `v0.3.0` existe déjà : ne le recréez pas et ne le déplacez pas. Une fois le workflow présent sur la branche principale, **Actions → Release installers → Run workflow** permet de relancer un tag existant. Sélectionnez la branche contenant le workflow à jour et indiquez le tag à reconstruire. Le code de l’application est toujours extrait du commit de ce tag. Une relance réutilise le brouillon existant et remplace ses fichiers ; elle refuse de modifier une release déjà publiée.

Vérifiez les installateurs sur les systèmes cibles avant de publier le brouillon. Les DMG de ce workflow sont signés ad hoc, sans notarisation Apple ; l’EXE Windows n’est pas signé par un certificat éditeur. La [documentation Tauri sur les releases GitHub](https://v2.tauri.app/distribute/pipelines/github/) décrit la configuration des certificats pour une distribution signée.

La publication du brouillon reste une action manuelle depuis la page de release GitHub.

## Sécurité et récupération

Au premier démarrage, le gérant choisit :

1. un PIN numérique de 4 à 12 chiffres pour l’usage quotidien ;
2. un mot de passe de récupération d’au moins 12 caractères.

Le mot de passe de récupération doit être conservé hors du PC. Il est indispensable pour restaurer une sauvegarde sur un autre ordinateur. Kër Finance ne possède aucun serveur capable de le récupérer.

## Limites actuelles

- un seul PC, une seule boutique et un seul profil gérant ;
- pas de synchronisation cloud ;
- pas de connexion aux API Orange Money, Wave ou Djamo ;
- ventes de produits payées intégralement, sans crédit ni retour partiel ;
- les soldes affichés sont ceux du dernier inventaire validé.
