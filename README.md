# Kër Finance

**La gestion quotidienne d’un multiservices au Sénégal, sur votre ordinateur.**

Kër Finance rassemble la trésorerie Orange Money, Wave, Djamo et espèces, les produits, les ventes, les dettes et l’argent confié par les clients. L’application fonctionne localement, sans connexion aux opérateurs, et protège les données dans une base chiffrée.

Version du projet : **0.5.0** · Application de bureau pour **macOS et Windows** · Montants en **FCFA entiers (XOF)** · Fuseau **Africa/Dakar**.

## Documentation

| Votre besoin | Où commencer |
| --- | --- |
| Installer et démarrer | [Installation](#installation) et [première ouverture](#première-ouverture) |
| Apprendre à utiliser chaque écran | [Guide d’utilisation pas à pas](docs/guide-utilisation.md) |
| Comprendre les calculs, les annulations et les exports | [Référence métier](docs/reference-metier.md) |
| Lancer le code, tester ou créer un installateur | [Développement et distribution](docs/developpement.md) |

## Ce que vous pouvez faire

- **Contrôler la trésorerie** : répartir le capital entre plusieurs comptes/SIM, compter les soldes, comparer le capital réel au capital attendu et expliquer les écarts.
- **Gérer la boutique** : créer un catalogue, vendre plusieurs articles, recevoir du stock et corriger les quantités après comptage.
- **Suivre les dettes** : enregistrer les transferts non encore payés, les échéances et les remboursements répartis sur les dettes les plus anciennes.
- **Garder l’argent des clients** : recevoir des dépôts, restituer tout ou partie du solde et reprendre les montants déjà détenus.
- **Consulter et exporter** : retrouver les inventaires et mouvements, puis produire des rapports PDF, Excel ou CSV.
- **Protéger les données** : utiliser un PIN, le verrouillage automatique, un journal d’audit et des sauvegardes chiffrées restaurables par mot de passe.

## Installation

Les installateurs publiés sont à télécharger dans les [releases GitHub du projet](https://github.com/aloundoye/multiservices/releases). Un brouillon de release est accessible aux mainteneurs jusqu’à sa publication. Les noms ci-dessous correspondent à la version 0.5.0.

| Votre ordinateur | Fichier à choisir |
| --- | --- |
| Mac Apple Silicon : M1, M2, M3, etc. | `Ker-Finance_0.5.0_macos-arm64.dmg` |
| Mac Intel | `Ker-Finance_0.5.0_macos-x64.dmg` |
| Windows 64 bits, architecture x64 | `Ker-Finance_0.5.0_windows-x64-setup.exe` |

**Sur macOS** : ouvrez le fichier DMG, glissez **Kër Finance** dans **Applications**, puis lancez l’application depuis ce dossier. Le projet cible macOS 11 ou plus récent.

**Sur Windows** : lancez le fichier EXE et suivez l’assistant d’installation.

Les DMG du workflow sont signés ad hoc, sans notarisation Apple ; l’EXE Windows n’a pas de signature éditeur. Le système peut donc afficher une alerte à l’ouverture. Node.js, Rust et les outils de compilation servent au développement ; ils ne sont pas nécessaires pour utiliser un installateur.

### Vérifier un téléchargement

Chaque release générée contient un fichier `SHA256SUMS.txt`. Comparez la somme calculée à la ligne du fichier téléchargé.

```bash
# macOS : adapter le nom pour un Mac Intel.
shasum -a 256 Ker-Finance_0.5.0_macos-arm64.dmg
```

```powershell
# Windows PowerShell
Get-FileHash .\Ker-Finance_0.5.0_windows-x64-setup.exe -Algorithm SHA256
```

## Première ouverture

L’assistant comporte trois étapes : **Boutique**, **Capital** et **Sécurité**.

1. Renseignez le nom de votre boutique.
2. Indiquez le capital initial, strictement supérieur à zéro.
3. Répartissez ce montant entre Orange Money, Wave, Djamo et les espèces. Ajoutez les SIM nécessaires : le **reste à répartir doit être égal à 0 FCFA**.
4. Choisissez et confirmez un **PIN de 4 à 12 chiffres** pour ouvrir l’application au quotidien.
5. Choisissez et confirmez un **mot de passe de récupération d’au moins 12 caractères**. Conservez-le hors de l’ordinateur.
6. Cliquez sur **Créer mon espace**, puis faites une copie externe dans **Paramètres → Sauvegardes → Copier vers une clé USB**.

Si les liquidités de départ comprennent de l’argent déjà gardé pour des clients, utilisez ensuite **Dépôts clients → Reprendre les montants** pour le distinguer du capital de la boutique. La [configuration détaillée](docs/guide-utilisation.md#configurer-la-boutique) donne un exemple de répartition.

## Utilisation au quotidien

1. **Ouvrez la boutique** avec votre PIN et consultez le tableau de bord.
2. **Enregistrez les opérations** dans l’écran adapté, au fur et à mesure de la journée.
3. **Faites un inventaire** : relevez chaque solde sur les services et comptez les espèces, argent des clients compris.
4. **Vérifiez l’écart**, expliquez-le si nécessaire et clôturez l’inventaire.
5. **Exportez vos rapports et copiez une sauvegarde externe** avant de terminer la journée. Le bouton **Verrouiller**, en bas à gauche, ferme la session.

### Quel écran utiliser ?

| Opération | Écran / action |
| --- | --- |
| Vendre un article du catalogue | **Produits et ventes → Nouvelle vente** |
| Acheter des articles suivis en stock | **Produits et ventes → Recevoir du stock** sur le produit |
| Saisir une commission, une recette hors catalogue, une dépense ou un mouvement de capital | **Journal boutique → Nouvelle écriture** |
| Noter un transfert mobile qu’un client doit encore payer | **Dettes clients → Ajouter une dette** |
| Encaisser le remboursement d’un client | **Dettes clients → Enregistrer un remboursement** |
| Recevoir ou rendre de l’argent confié | **Dépôts clients → Recevoir un dépôt / Restituer** |
| Contrôler la trésorerie et constater un écart | **Nouvel inventaire** |
| Consulter les clôtures précédentes | **Historique** |
| Exporter une période en PDF, Excel ou CSV | **Rapports** |
| Ajouter une SIM, régler les rappels ou sauvegarder | **Paramètres** |

### Comprendre les chiffres

Les **soldes des comptes affichés sont ceux du dernier inventaire validé**. Les opérations enregistrées ne les mettent pas à jour automatiquement : vérifiez le solde réel sur le service avant de payer ou de restituer de l’argent.

Le **capital attendu** part du capital réel de la dernière clôture, puis tient compte des écritures du journal et des reclassements des dépôts déjà détenus. Au prochain inventaire :

```text
Capital réel net = liquidités comptées + créances restantes − dépôts à restituer
Écart            = capital réel net − capital attendu
```

**Évitez les doubles saisies** : les ventes et achats de produits créent déjà leurs écritures dans le journal. Les dettes, remboursements, dépôts et restitutions se saisissent dans leurs espaces dédiés. Un transfert entre vos propres comptes change la répartition des liquidités ; ce n’est pas une recette de la boutique.

Le [guide d’utilisation](docs/guide-utilisation.md) détaille chaque parcours et propose une journée d’exemple.

## Sauvegardes et récupération

La base SQLite est chiffrée avec SQLCipher. L’accès quotidien utilise le PIN et le coffre du système ; le mot de passe de récupération permet de restaurer une sauvegarde sur un autre ordinateur.

L’application tente une sauvegarde locale à la création de l’espace et après chaque clôture d’inventaire. Elle conserve les **30 copies locales les plus récentes**. Un échec de sauvegarde après clôture n’annule pas l’inventaire : contrôlez le message affiché et créez une copie manuelle si nécessaire.

Dans **Paramètres → Sauvegardes**, vous pouvez créer une copie locale, enregistrer un fichier `.msbackup` sur une clé USB ou restaurer une sauvegarde. Une restauration remplace les données présentes après vérification de la copie et sauvegarde de sécurité de la session ouverte.

Conservez une copie externe et son mot de passe de récupération hors de l’ordinateur. Les exports PDF, Excel et CSV servent aux rapports ; ils ne remplacent pas une sauvegarde `.msbackup`.

Consultez la [procédure de restauration, y compris sur un nouvel ordinateur](docs/guide-utilisation.md#sauvegarder-et-restaurer).

## Lancer le projet depuis le code

Prérequis : **Node.js 24**, **Rust stable**, puis les dépendances système indiquées dans le [guide de développement](docs/developpement.md#prérequis-et-lancement-local).

```bash
git clone https://github.com/aloundoye/multiservices.git
cd multiservices
npm ci
npm run tauri:dev
```

La commande ouvre l’application de bureau. `npm run dev` démarre seulement l’interface Vite ; les opérations métier nécessitent Tauri. Les commandes de test, l’architecture, la génération des installateurs et le processus de release sont documentés dans [Développement et distribution](docs/developpement.md).

## Limites de la version 0.5.0

- Une boutique, un ordinateur et un seul profil gérant ; aucune synchronisation cloud.
- Aucune connexion aux API Orange Money, Wave ou Djamo : les soldes sont relevés manuellement.
- Produits vendus avec paiement intégral, sans crédit ni retour partiel ; la valeur du stock n’est pas ajoutée au capital.
- Dettes issues de transferts Orange Money, Wave ou Djamo ; elles ne servent pas à vendre des produits à crédit.
- Dépôts gratuits, sans intérêts, transferts entre clients ou compensation avec leurs dettes.
- Restauration accessible depuis les paramètres d’une session ouverte ; aucun parcours « PIN oublié » sur l’écran de connexion.

Les [règles métier et de migration](docs/reference-metier.md) précisent les comportements des données existantes, des annulations et des exports détaillés.
