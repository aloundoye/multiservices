# Kër Finance

Application desktop locale de gestion d’un multiservices au Sénégal : inventaires Orange Money, Wave, Djamo et espèces, produits, stock, ventes, journal de boutique, dettes et dépôts clients, rapports et sauvegardes chiffrées.

## Installer la version 0.5.0

Les installateurs sont disponibles dans les [releases GitHub](https://github.com/aloundoye/multiservices/releases). Les versions en brouillon restent accessibles aux mainteneurs jusqu’à leur publication.

| Système | Fichier à télécharger |
| --- | --- |
| Mac Apple Silicon (puces M1, M2, etc.) | `Ker-Finance_0.5.0_macos-arm64.dmg` |
| Mac Intel | `Ker-Finance_0.5.0_macos-x64.dmg` |
| Windows 64 bits (x64) | `Ker-Finance_0.5.0_windows-x64-setup.exe` |

Sur macOS, ouvrez le DMG et glissez **Kër Finance** dans **Applications**. Sur Windows, lancez l’EXE et suivez l’assistant d’installation. Node.js, Rust et les outils de compilation ne sont nécessaires que pour le développement.

Les applications macOS sont signées ad hoc, sans notarisation Apple ; l’installateur Windows n’a pas de signature éditeur. Le système peut donc afficher une alerte à l’ouverture.

Chaque release contient `SHA256SUMS.txt`. Pour vérifier un téléchargement, comparez la somme calculée à la ligne correspondant au fichier :

```bash
# macOS : adapter le nom du fichier pour un Mac Intel.
shasum -a 256 Ker-Finance_0.5.0_macos-arm64.dmg
```

```powershell
# Windows PowerShell
Get-FileHash .\Ker-Finance_0.5.0_windows-x64-setup.exe -Algorithm SHA256
```

## Fonctionnalités

- capital initial réparti avec contrôle d’égalité exact ;
- inventaires périodiques, comparaison par compte et justification des écarts ;
- capital réel net : liquidités + créances non soldées − dépôts à restituer ;
- dépôts clients gratuits, restitutions partielles et reprise des montants déjà gardés ;
- recettes, commissions, apports, achats, dépenses et retraits de capital ;
- catalogue de produits, suivi du stock, ventes et réapprovisionnements reliés au capital attendu ;
- plusieurs comptes/SIM Orange Money, Wave et Djamo, avec une caisse espèces unique ;
- répertoire des clients des dettes, échéances et remboursement global réparti sur les dettes les plus anciennes ;
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

Les écritures automatiques figurent déjà dans les rapports et exports : ne les ressaisissez pas dans le journal. Les migrations conservent les comptes et données précédentes ; les anciennes sauvegardes restent restaurables.

## Clients des dettes et remboursements

L’espace **Dettes clients** contient trois vues : **Clients**, **Dettes** et **Remboursements**. Enregistrez une fois le nom et le téléphone obligatoires, puis sélectionnez le client pour chaque nouvelle dette. Vous pouvez rechercher les fiches, modifier les coordonnées, archiver une fiche à solde nul et la réactiver. Une fiche présente le reste dû, les remboursements cumulés et les dettes en retard. Les identifiants sont permanents ; les coordonnées enregistrées au moment des dettes et paiements restent inchangées.

Pour un remboursement, choisissez le client, le montant reçu en FCFA entiers, le compte actif ou les espèces, la date et éventuellement une note. L’aperçu montre chaque dette concernée et son solde après paiement. Le bouton **Rembourser ce client** depuis une dette ouvre ce même remboursement global.

- La répartition suit la date du prêt, puis la date d’enregistrement et l’identifiant en cas d’égalité.
- Un paiement antidaté ne concerne que les dettes émises au plus tard à sa date. Les répartitions validées sont figées, même si une dette plus ancienne est ajoutée ensuite.
- Une dette annulée ou soldée est exclue. Le montant ne peut pas dépasser le total remboursable à cette date ; aucun excédent ne devient une avance ou un dépôt.
- Si les soldes changent après l’aperçu, le serveur refuse la validation et demande d’actualiser l’aperçu.
- Le paiement, ses affectations, les soldes et l’audit sont enregistrés ensemble. La même requête rejouée ne crée aucun doublon.

Exemple : **20 000 + 30 000 FCFA** de dettes, puis **35 000 FCFA** reçus → première dette soldée, seconde à **15 000 FCFA**.

Le remboursement réduit les créances sans modifier le capital attendu. À l’inventaire, relevez les liquidités réellement présentes, remboursements compris. **Ne ressaisissez pas les remboursements comme recettes dans le journal.** Le répertoire des dettes reste indépendant des dépôts clients, sans compensation automatique.

### Historique et exports des remboursements

Les anciens paiements sont conservés comme **remboursements unitaires historiques**, avec leurs identifiants, comptes, dates et montants d’origine. Leur solde historique après paiement n’est pas inventé.

Les rapports filtrent les remboursements par leur **propre date**, indépendamment de la date du prêt. Ils présentent un total par reçu et sa répartition détaillée. Dans Excel, la feuille **Remboursements** contient les totaux ; **Répartition remboursements** contient les affectations et **Clients des dettes** les soldes actuels. En CSV, les lignes `remboursement` contiennent le total reçu dans `montant_fcfa` ; les lignes `allocation_remboursement` utilisent uniquement `allocation_fcfa_hors_total`, reliée par `remboursement_id` et `dette_id`. Ne cumulez pas les totaux et leurs affectations. Les soldes des fiches sont les soldes actuels, même pour un rapport sur une période passée.

## Dépôts clients

L’espace **Dépôts clients** suit l’argent confié au gérant : fiche client (nom, téléphone facultatif, identifiant permanent), solde restant, entrées cumulées, restitutions et registre des mouvements. Les cumuls excluent les opérations annulées ; les reprises sont incluses dans les entrées.

- **Recevoir un dépôt** augmente la somme due au client sans augmenter le capital attendu.
- **Restituer** réduit le solde client. Le compte/SIM ou la caisse peut être différent du compte d’origine. Un retrait supérieur au solde client est refusé.
- Un compte actif est obligatoire. La disponibilité réelle doit être vérifiée sur le compte choisi : les soldes de l’application sont les **montants vérifiés au dernier inventaire**, pas des soldes actualisés automatiquement.
- Une annulation exige un motif et ajoute un contre-mouvement lié à l’original. Elle est refusée si elle a déjà eu lieu ou si elle rend le solde client négatif. L’annulation d’une restitution réactive automatiquement un client archivé si de l’argent lui est de nouveau dû.
- L’archivage d’un client est possible uniquement à solde nul. Son identifiant et son historique sont conservés. Les noms, téléphones, comptes et identifiants enregistrés à l’époque ne sont pas réécrits lors d’un renommage.

Le service est gratuit, sans intérêts. Cette version ne gère ni transferts entre clients, ni achats de produits réglés par dépôt, ni compensation avec les dettes clients. Le registre des dépôts est séparé du journal financier : **ne ressaisissez pas les dépôts et restitutions comme recettes ou dépenses**.

### Reprendre l’argent déjà gardé en 0.3.0

1. Créez les fiches des clients concernés.
2. Choisissez **Reprendre les montants** dans Dépôts clients, puis indiquez le montant encore gardé pour chacun. Il n’est pas nécessaire de reconstruire les anciens encaissements.
3. Vérifiez le total et le capital corrigé, puis **Valider la reprise**. Tous les montants sont enregistrés ensemble, sans nouvel encaissement ni dépense.

Une reprise diminue le capital attendu une seule fois, car cet argent était jusque-là compté comme appartenant à la boutique. Une seule reprise non annulée est autorisée par client ; pour la corriger, annulez-la dans le registre et recommencez. Son annulation inverse le reclassement, même après une clôture, si le solde client est suffisant.

Exemple : 1 000 000 FCFA de liquidités, dont 200 000 gardés pour des clients, deviennent **800 000 FCFA de capital attendu + 200 000 FCFA de dépôts**. Après un nouveau dépôt de 50 000 et une restitution de 80 000, les dépôts sont de **170 000**, les liquidités effectivement disponibles de **970 000**, et le capital reste **800 000**. Les soldes vérifiés affichés ne changent qu’au prochain inventaire.

### Inventaires et rapports

À l’inventaire, relevez les **soldes complets, argent des clients compris**. Le moteur calcule :

**Capital réel net = liquidités constatées + créances − dépôts clients restants.**

Le capital attendu part du capital net de la dernière clôture, ajoute les écritures financières suivantes et applique les reclassements non encore intégrés. Chaque clôture fige le total des dépôts, les soldes et libellés par client, ainsi que la séquence du registre. Une date antidatée ne fait jamais appliquer une reprise deux fois. Les anciens inventaires conservent leurs chiffres et affichent **« Dépôts clients non suivis à cette date »**.

Les rapports PDF, Excel et CSV présentent, par client, le solde de départ, les augmentations et diminutions de la période, le solde de fin, le registre détaillé et les dépôts figés aux clôtures. La période utilise les **dates déclarées** ; les reprises et annulations sont incluses. Un mouvement antidaté peut donc changer un rapport de période, voire donner un solde de période négatif si sa date précède les entrées enregistrées. Les inventaires déjà clôturés restent immuables. Le registre conserve aussi la date réelle et l’ordre d’enregistrement, ainsi que l’auteur « Gérant ».

Dans Excel, les feuilles **Dépôts clients**, **Registre dépôts** et **Dépôts aux clôtures** complètent les feuilles financières. En CSV, les lignes `depot_solde_periode`, `depot_mouvement`, `depot_total_cloture` et `depot_client_cloture` sont distinctes du `journal` ; ne les additionnez pas aux recettes ni entre niveaux de détail. Les colonnes supplémentaires contiennent les identifiants, séquences, dates d’enregistrement, liens d’annulation et reclassements de capital.

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

La version 0.5.0 utilise le **schéma SQLite 5** et s’installe par-dessus la 0.4.0. L’identifiant `sn.kerfinance.multiservices` et les chemins de stockage restent inchangés : comptes, produits, ventes, dettes, PIN et historique sont conservés. Gardez le mot de passe de récupération et une copie externe de vos sauvegardes.

Avant de migrer une base existante aux schémas 1, 2, 3 ou 4, l’application crée **et vérifie** une sauvegarde chiffrée `avant-migration-vN-….msbackup`. Si cette étape échoue, elle ne modifie pas la base. Les migrations manquantes sont exécutées conditionnellement, dans une transaction auditée ; les tables du schéma 3 ne sont pas recréées. Au schéma 1, les anciens soldes restent marqués « historique regroupé par service », sans inventer de répartition entre SIM.

Le schéma 4 ajoute les dépôts, les identifiants de requête et les instantanés d’inventaire. Les anciennes clôtures gardent leurs montants sans dépôts historiques inventés. Les opérations de dépôt, le solde client, l’audit et le résultat associé à une requête sont enregistrés dans la même transaction : une requête identique rejouée ne crée aucun doublon, et le même identifiant avec d’autres données est refusé.

Le schéma 5 crée les fiches depuis les anciennes dettes en regroupant uniquement les couples nom/téléphone identiques après normalisation : casse et espaces du nom, espaces/tirets/parenthèses du téléphone. Les noms différents et les indicatifs différents restent séparés. Les dettes, anciens paiements et inventaires ne sont pas réécrits.

Les sauvegardes des **schémas 1 à 5** sont restaurables. La restauration contrôle et migre une copie temporaire avant de remplacer les données actives. Les sauvegardes 4 incluent les dépôts, les reprises, les annulations, les instantanés et les informations d’idempotence. Un mot de passe incorrect, une archive endommagée ou une migration invalide interrompt la restauration. Les sauvegardes 5 incluent aussi les fiches des dettes, les reçus globaux et leurs allocations. Une base migrée au schéma 5 ne doit pas être ouverte avec une ancienne version.

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

Le workflow [Release installers](.github/workflows/release.yml) se déclenche lorsque vous poussez un tag `vX.Y.Z`. GitHub Actions compile sur des runners macOS et Windows : vous pouvez déclencher les trois builds depuis votre Mac sans y installer de compilateur Windows. Les variantes générées sont :

- `Ker-Finance_X.Y.Z_macos-arm64.dmg` pour les Mac Apple Silicon ;
- `Ker-Finance_X.Y.Z_macos-x64.dmg` pour les Mac Intel ;
- `Ker-Finance_X.Y.Z_windows-x64-setup.exe` pour Windows 64 bits.

Chaque variante exécute les tests frontend et Rust, le build frontend et Clippy avant de générer son installateur. Après réussite des trois builds, les installateurs et `SHA256SUMS.txt` sont joints à un **brouillon de release GitHub**. Les artefacts `installer-macos-arm64`, `installer-macos-x64` et `installer-windows-x64` restent également disponibles pendant 30 jours dans l’onglet Actions. Le workflow vérifie que le tag, les manifestes npm/Tauri/Rust et leurs fichiers de verrouillage portent la même version, et que les trois builds proviennent du même commit.

Pour une prochaine version :

1. Mettez à jour `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et l’entrée du projet dans `src-tauri/Cargo.lock`.
2. Exécutez les vérifications, puis enregistrez et poussez les changements, y compris le workflow de release.
3. Créez et poussez le tag correspondant à la nouvelle version. Par exemple, après passage à **0.5.0** :

```bash
npm run release:check -- v0.5.0
git tag -a v0.5.0 -m "Kër Finance 0.5.0"
git push origin v0.5.0
```

La 0.5.0 utilise un nouveau tag `v0.5.0` et une nouvelle release. Le tag et la release `v0.4.0` restent inchangés. Ne recréez pas et ne déplacez pas un tag existant. Une fois le workflow présent sur la branche principale, **Actions → Release installers → Run workflow** permet de relancer un tag existant. Sélectionnez la branche contenant le workflow à jour et indiquez le tag à reconstruire. Le code de l’application est toujours extrait du commit de ce tag. Une relance réutilise le brouillon existant et remplace ses fichiers ; elle refuse de modifier une release déjà publiée.

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
- dépôts gratuits sans transferts entre clients, intérêts ou compensation avec les dettes ;
- les soldes affichés sont ceux du dernier inventaire validé.
