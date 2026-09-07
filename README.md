# Kër Finance

Application desktop locale de gestion d’un multiservices au Sénégal : inventaires Orange Money, Wave, Djamo et espèces, journal de boutique, dettes clients, rapports et sauvegardes chiffrées.

## Fonctionnalités

- capital initial réparti avec contrôle d’égalité exact ;
- inventaires périodiques, comparaison par compte et justification des écarts ;
- capital réel incluant les créances non soldées ;
- recettes, commissions, apports, achats, dépenses et retraits de capital ;
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

Les écritures automatiques figurent déjà dans les rapports et exports : ne les ressaisissez pas dans le journal. La migration vers le schéma 3 conserve les comptes et données précédentes, avec sauvegarde chiffrée préalable et prise en charge des anciennes sauvegardes.

## Architecture

- `src/` : interface React 19 + TypeScript + Vite ;
- `src-tauri/src/domain.rs` : validations et règles comptables ;
- `src-tauri/src/db.rs` : schéma SQLite et transactions métier ;
- `src-tauri/src/accounts.rs` : comptes, validations, soldes et variations ;
- `src-tauri/src/migration_v2.sql` : migration transactionnelle multi-comptes ;
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

Le schéma SQLite passe de 1 à 2 au prochain déverrouillage. Avant tout changement, l’application crée et vérifie une sauvegarde chiffrée `avant-migration-v1-….msbackup` dans son dossier de sauvegardes. Si cette étape échoue, la migration s’arrête et les données restent en version 1.

La migration crée les comptes Orange Money — Principal, Wave — Principal, Djamo — Principal et Espèces. Les montants, dates, références, corrections et écarts restent inchangés. Les anciens soldes sont explicitement marqués « historique regroupé par service » : aucune répartition ancienne entre SIM n’est inventée. La migration est atomique, auditée et ne s’exécute qu’une fois.

Les sauvegardes version 2 contiennent tous les comptes, y compris archivés, leurs relevés et leurs libellés historiques. Une sauvegarde version 1 est déchiffrée, contrôlée puis migrée dans une copie temporaire avant de remplacer les données actives. Un mot de passe incorrect ou une migration invalide interrompt la restauration. Ne rouvrez pas une base version 2 avec l’ancienne application ; conservez une copie externe de la sauvegarde préalable.

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

Les tests comptables utilisent une base SQLite en mémoire et un test séparé vérifie SQLCipher avec une vraie base chiffrée. L’option avancée `cipher_memory_security` reste désactivée : avec OpenSSL statique sous Windows, son allocateur global provoque un `STATUS_STACK_OVERFLOW`. Cette option ne contrôle pas le chiffrement du fichier, qui reste actif via `PRAGMA key`.

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
