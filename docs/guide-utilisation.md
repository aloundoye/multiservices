# Guide d’utilisation — Kër Finance

[Retour au README](../README.md) · [Référence métier](reference-metier.md) · [Développement](developpement.md)

Ce guide accompagne le gérant dans l’utilisation de la version **0.5.0**. Les montants sont saisis en **FCFA entiers**, sans décimales. Les opérations restent sur l’ordinateur ; les soldes Orange Money, Wave et Djamo doivent être consultés sur les services concernés.

## Sommaire

- [Configurer la boutique](#configurer-la-boutique)
- [Prendre ses repères](#prendre-ses-repères)
- [Gérer les comptes et SIM](#gérer-les-comptes-et-sim)
- [Gérer les produits et les ventes](#gérer-les-produits-et-les-ventes)
- [Tenir le journal de boutique](#tenir-le-journal-de-boutique)
- [Suivre les dettes et remboursements](#suivre-les-dettes-et-remboursements)
- [Gérer les dépôts clients](#gérer-les-dépôts-clients)
- [Faire un inventaire](#faire-un-inventaire)
- [Corriger une erreur](#corriger-une-erreur)
- [Consulter et exporter les rapports](#consulter-et-exporter-les-rapports)
- [Sauvegarder et restaurer](#sauvegarder-et-restaurer)
- [Exemple de journée](#exemple-de-journée)
- [Questions fréquentes](#questions-fréquentes)

## Configurer la boutique

Installez l’application avec le fichier adapté à votre ordinateur, comme expliqué dans le [README](../README.md#installation), puis ouvrez **Kër Finance**.

### 1. Identifier la boutique

Dans l’étape **Boutique**, saisissez un nom d’au moins deux caractères, puis cliquez sur **Continuer**. Ce nom apparaîtra dans les rapports. La devise est le franc CFA (XOF) et le fuseau est **Africa/Dakar**.

### 2. Répartir les liquidités de départ

Dans l’étape **Capital**, indiquez le montant de départ, puis renseignez les sommes réellement présentes sur les comptes et dans la caisse. Un compte par service est proposé ; ajoutez vos autres SIM si nécessaire. Les soldes peuvent être nuls, mais le capital initial doit être supérieur à zéro.

Exemple pour **1 000 000 FCFA** :

| Compte | Solde de départ |
| --- | ---: |
| Orange Money — Principal | 400 000 |
| Wave — Principal | 300 000 |
| Djamo — Principal | 100 000 |
| Espèces | 200 000 |
| **Total** | **1 000 000** |

Le **reste à répartir** doit être exactement **0 FCFA** pour continuer.

Si ces liquidités comprennent de l’argent déjà confié par des clients, incluez-le dans les soldes réels de départ, puis utilisez **Reprendre les montants** après la création de l’espace. Cette reprise retirera l’argent des clients du capital propre de la boutique, sans enregistrer un nouvel encaissement. La valeur des marchandises déjà en stock ne fait pas partie de cette répartition.

### 3. Protéger l’espace

1. Choisissez un **PIN numérique de 4 à 12 chiffres**, puis confirmez-le.
2. Choisissez un **mot de passe de récupération d’au moins 12 caractères**, puis confirmez-le.
3. Conservez ce mot de passe dans un endroit sûr, hors de l’ordinateur.
4. Cliquez sur **Créer mon espace**.
5. Ouvrez **Paramètres → Sauvegardes → Copier vers une clé USB** pour conserver votre première copie externe.

Le PIN ouvre la boutique au quotidien. Le mot de passe de récupération sert à restaurer les sauvegardes ; il ne remplace pas le PIN sur l’écran de connexion.

## Prendre ses repères

Le menu de gauche donne accès aux écrans suivants :

| Écran | Utilité |
| --- | --- |
| **Tableau de bord** | Voir le capital attendu, le dernier capital réel, les créances, l’argent gardé et le prochain inventaire. |
| **Nouvel inventaire** | Saisir les soldes réels et clôturer le contrôle de trésorerie. |
| **Historique** | Consulter les clôtures et ajouter une correction liée. |
| **Produits et ventes** | Gérer le catalogue, les ventes, les achats et les quantités. |
| **Journal boutique** | Saisir les recettes hors catalogue, commissions, dépenses, apports et retraits de capital. |
| **Dettes clients** | Suivre les transferts mobiles non encore payés et leurs remboursements. |
| **Dépôts clients** | Suivre l’argent que des clients vous confient et que vous devez leur rendre. |
| **Rapports** | Consulter une période et exporter en PDF, Excel ou CSV. |
| **Paramètres** | Configurer les comptes, les rappels, le verrouillage et les sauvegardes ; consulter l’audit. |

Le **capital attendu** évolue avec les écritures enregistrées. Le **dernier capital réel** et la répartition des liquidités restent ceux de la dernière clôture. Une vente peut donc augmenter le capital attendu sans modifier le solde espèces affiché.

Dans **Paramètres → Général**, vous pouvez modifier le nom, choisir un rappel d’inventaire allant d’une heure à une journée, et un verrouillage automatique après 5 à 60 minutes. Cliquez sur **Enregistrer** pour appliquer les réglages. Le bouton **Verrouiller**, en bas à gauche, permet aussi de fermer immédiatement la session.

## Gérer les comptes et SIM

1. Ouvrez **Paramètres → Comptes et SIM**.
2. Cliquez sur **Ajouter un compte**.
3. Choisissez Orange Money, Wave ou Djamo.
4. Renseignez un nom distinct et, si vous le souhaitez, un numéro ou identifiant.
5. Enregistrez, puis relevez son solde au prochain inventaire.

Un compte ajouté affiche **Pas encore relevé** : sa création n’ajoute pas d’argent au capital. La caisse espèces est unique.

Vous pouvez renommer un compte et archiver une SIM inutilisée. Pour archiver un compte déjà utilisé, son dernier solde doit être nul et aucune opération ne doit avoir été enregistrée dessus depuis ce relevé. La caisse ne peut pas être archivée. L’historique conserve les noms utilisés au moment des opérations.

Un transfert entre vos propres comptes change seulement la répartition des liquidités. Ne créez pas une recette ou une dépense pour ce transfert. Si vous apportez réellement de l’argent supplémentaire à la boutique, saisissez un **Apport de capital** dans le journal.

## Gérer les produits et les ventes

### Ajouter un produit

Dans **Produits et ventes → Ajouter un produit**, renseignez son nom, son prix de vente et la **Quantité déjà en stock**, puis enregistrez.

La quantité initiale décrit les marchandises que vous possédez déjà : elle ne crée pas d’achat dans le journal. Pour un nouvel achat, créez le produit avec un stock à **0**, puis utilisez **Recevoir du stock**.

### Recevoir du stock

1. Dans le **Catalogue**, utilisez le bouton **Recevoir du stock** du produit, représenté par un camion.
2. Saisissez la quantité reçue et le **montant total payé**.
3. Choisissez le compte ou les espèces utilisés, la date et éventuellement une note.
4. Cliquez sur **Enregistrer**.

Le stock augmente et un achat du montant total est automatiquement ajouté au journal. Ce montant diminue le capital attendu. **Ne saisissez pas une deuxième fois cet achat dans le journal.**

### Enregistrer une vente

1. Cliquez sur **Nouvelle vente**.
2. Choisissez un produit, la quantité et le prix unitaire. Le tarif du catalogue est proposé et peut être adapté pour cette vente.
3. Utilisez **Ajouter un article** si le client achète plusieurs produits.
4. Vérifiez le total, le compte d’encaissement et la date.
5. Enregistrez après paiement intégral.

Les quantités vendues sont retirées du stock. Une recette est automatiquement inscrite au journal et augmente le capital attendu. Le stock disponible ne peut pas être dépassé. Cette version ne gère pas les ventes de produits à crédit.

### Vérifier les quantités

Utilisez **Compter le stock**, sur la ligne du produit, pour saisir la quantité réellement comptée et un motif. Ce comptage corrige les unités sans mouvement d’argent. Consultez **Mouvements de stock** pour retrouver les entrées, sorties et corrections.

Un produit ne peut être archivé que si son stock est nul. Sa valeur en marchandises n’est pas ajoutée au capital financier.

## Tenir le journal de boutique

Ouvrez **Journal boutique → Nouvelle écriture**, puis choisissez le type :

| Type d’écriture | Exemple | Effet sur le capital attendu |
| --- | --- | --- |
| **Recette boutique** | Prestation ou vente hors catalogue | Augmentation |
| **Commission mobile** | Commission réellement acquise | Augmentation |
| **Apport de capital** | Argent ajouté par le propriétaire | Augmentation |
| **Achat** | Achat hors stock suivi | Diminution |
| **Dépense** | Électricité, transport, frais de boutique | Diminution |
| **Retrait de capital** | Argent repris par le propriétaire | Diminution |

Renseignez un montant positif, le compte utilisé, la date et, si utile, une référence et une note. Vérifiez l’impact affiché, puis cliquez sur **Enregistrer**.

Pour une commission, saisissez le montant de la commission acquise. Les sommes échangées lors des opérations mobiles habituelles ne deviennent pas toutes des recettes. Les ventes et réapprovisionnements du catalogue figurent déjà dans le journal. Les dettes, remboursements, dépôts et restitutions se saisissent dans leurs espaces dédiés.

## Suivre les dettes et remboursements

Une dette correspond ici à un **transfert Orange Money, Wave ou Djamo que le client doit encore payer**. La somme reste une créance de la boutique. Les prêts en espèces et les ventes de produits à crédit ne sont pas couverts par cet écran.

### Enregistrer une dette

1. Dans **Dettes clients → Clients**, cliquez sur **Ajouter un client** et saisissez son nom et son téléphone, tous deux obligatoires.
2. Cliquez sur **Ajouter une dette**, puis sélectionnez le client. Vous pouvez aussi créer sa fiche depuis ce formulaire.
3. Choisissez le **Compte émetteur** et saisissez le **Montant transféré**.
4. Indiquez la date du transfert, éventuellement une échéance et une note.
5. Cliquez sur **Enregistrer la dette**.

Ne saisissez pas le même transfert comme dépense dans le journal. À l’inventaire, l’application ajoute la créance restante aux liquidités réellement présentes.

### Enregistrer un remboursement

1. Cliquez sur **Enregistrer un remboursement** ou, depuis le détail d’une dette, sur **Rembourser ce client**.
2. Choisissez le client, le montant reçu et la date du remboursement.
3. Dans **Reçu sur**, choisissez une SIM active ou les espèces.
4. Vérifiez la **Répartition avant validation**, puis cliquez sur **Valider le remboursement**.

Le paiement rembourse d’abord les dettes les plus anciennes du client. Pour deux dettes de **20 000** puis **30 000 FCFA**, un remboursement de **35 000 FCFA** solde la première et laisse **15 000 FCFA** sur la seconde.

Un paiement antidaté ne peut rembourser que les dettes émises au plus tard à sa date. Le montant ne peut pas dépasser le total remboursable ; l’excédent n’est pas transformé en dépôt. Si les données ont changé depuis l’aperçu, cliquez sur **Actualiser l’aperçu** et revérifiez la répartition.

Le remboursement réduit les créances sans modifier le capital attendu. **Ne le saisissez pas comme recette dans le journal.** Au prochain inventaire, comptez l’argent reçu dans le solde réel du compte ou de la caisse. L’onglet **Remboursements → Voir la répartition** permet de consulter le détail d’un reçu. Une fiche client ne peut être archivée qu’à solde nul.

## Gérer les dépôts clients

Un dépôt est de l’argent confié à la boutique et **dû au client**. Il ne fait pas partie de votre capital. Le répertoire des déposants est séparé de celui des dettes ; aucune compensation automatique n’existe entre les deux.

### Recevoir ou restituer un montant

1. Dans **Dépôts clients**, cliquez sur **Ajouter un client** : son nom est obligatoire, son téléphone est facultatif.
2. Cliquez sur **Recevoir un dépôt** et renseignez le client, le montant, le compte d’encaissement, la date et éventuellement une note.
3. Pour rendre l’argent, utilisez **Restituer**, choisissez le montant et le compte de restitution, puis enregistrez.

Une restitution peut être partielle et utiliser un autre compte que celui d’origine. Elle ne peut pas dépasser le solde gardé pour ce client. Vérifiez aussi l’argent réellement disponible sur le compte choisi : le solde affiché par l’application date du dernier inventaire.

Les dépôts et restitutions n’augmentent ni ne diminuent le capital attendu. **Ne les ajoutez pas comme recettes ou dépenses dans le journal.** Le **Registre des mouvements** conserve leur détail. Un client peut être archivé après restitution intégrale de son solde.

### Reprendre l’argent déjà gardé

Si vous gardiez des sommes avant de les suivre dans l’application :

1. Créez les fiches des clients concernés.
2. Cliquez sur **Reprendre les montants**.
3. Indiquez, pour chaque client, le montant **encore détenu**. Il n’est pas nécessaire de reconstituer les anciens encaissements.
4. Vérifiez le total et le capital corrigé, puis cliquez sur **Valider la reprise**.

Exemple : sur **1 000 000 FCFA** de liquidités initialement comptées comme capital, **200 000 FCFA** appartiennent aux clients. La reprise établit **800 000 FCFA** de capital attendu et **200 000 FCFA** de dépôts à restituer, sans nouvel encaissement.

Une seule reprise non annulée est autorisée par client. Utilisez **Recevoir un dépôt** pour un nouvel encaissement, et **Reprendre les montants** uniquement pour l’argent déjà présent et jusque-là compté dans le capital.

## Faire un inventaire

Faites un inventaire à la fréquence adaptée à la boutique, notamment à la fin de la journée.

1. Ouvrez **Nouvel inventaire** ou **Faire l’inventaire** depuis le tableau de bord.
2. Consultez chaque service et **revérifiez les soldes préremplis**. Saisissez un montant pour chaque compte actif, même s’il est à zéro.
3. Comptez les espèces et incluez **tout l’argent présent, y compris les dépôts des clients**. L’application les déduit elle-même du capital net.
4. Contrôlez les liquidités, les créances ouvertes, les dépôts à restituer, le capital réel net et le capital attendu.
5. Si l’écart est non nul, vérifiez vos saisies et les opérations oubliées. Choisissez ensuite une **Catégorie de l’écart** et renseignez l’**Explication obligatoire**.
6. Cliquez sur **Clôturer l’inventaire**, puis contrôlez le résultat de la sauvegarde automatique.

Le calcul est le suivant :

```text
Capital réel net = liquidités constatées + créances ouvertes − dépôts à restituer
Écart            = capital réel net − capital attendu
```

Un écart positif indique un **surplus**, un écart négatif un **manquant**. Une clôture conserve ses chiffres et ne peut plus être modifiée. Son capital réel devient la référence du prochain contrôle. Une catégorie d’écart explique le constat ; elle ne remplace pas une opération oubliée à enregistrer avant clôture.

Si la sauvegarde échoue après la clôture, l’inventaire reste enregistré. Créez une copie manuelle dans **Paramètres → Sauvegardes** et vérifiez sa réussite.

## Corriger une erreur

L’application conserve la trace des opérations. Utilisez la correction correspondant à l’erreur :

| Erreur | Procédure |
| --- | --- |
| Écriture du journal | Cliquez sur l’icône **Créer une contre-écriture**, renseignez le motif, puis validez. L’original reste visible. |
| Vente ou réapprovisionnement | **Produits et ventes → Ventes et achats → Détails → Annuler cette opération**. L’annulation complète corrige le stock et le capital attendu. |
| Quantité de stock | Utilisez **Compter le stock**, indiquez la quantité réellement comptée et le motif. |
| Dépôt, restitution ou reprise | **Dépôts clients → Registre des mouvements**, ouvrez le détail, puis **Annuler ce mouvement** avec un motif. |
| Dette | Ouvrez son détail, puis **Annuler par correction**, avec un motif. Le reste dû est mis à zéro et l’historique est conservé. |
| Inventaire déjà clôturé | **Historique**, ouvrez le détail, puis **Ajouter une correction liée**. Choisissez le sens, le montant, le compte et le motif. L’inventaire d’origine reste intact. |

Pour annuler une vente réellement effectuée, tous les articles doivent avoir été repris et le client intégralement remboursé. Pour annuler un achat, les articles doivent avoir été rendus et l’argent récupéré. Les retours partiels ne sont pas gérés.

L’annulation d’un dépôt est refusée si elle rendrait le solde du client négatif. Annuler une dette ne signifie pas que le client a payé : utilisez **Enregistrer un remboursement** pour un paiement réel. Cette annulation supprime la créance restante sans créer de remboursement ni ajuster automatiquement le capital attendu ; vérifiez les conséquences au prochain inventaire.

## Consulter et exporter les rapports

1. Ouvrez **Rapports**.
2. Renseignez les dates **Du** et **Au**.
3. Cliquez sur **Actualiser** pour consulter la période.
4. Choisissez **PDF**, **Excel** ou **CSV**, puis le dossier de destination.

Le PDF sert à lire ou imprimer un rapport ; Excel à parcourir les feuilles détaillées ; CSV à réutiliser les données dans un tableur ou un autre outil.

Les remboursements sont filtrés par leur date de paiement, même si la dette est plus ancienne. Les **Créances actuelles** et les soldes des fiches de dettes restent des valeurs actuelles, même dans un rapport portant sur une période passée. Les dépôts sont présentés séparément des recettes et dépenses.

Les exports contiennent des totaux et des détails : ne les additionnez pas ensemble. Par exemple, un reçu de remboursement et ses affectations aux dettes décrivent le même argent. La [référence métier](reference-metier.md) explique les feuilles Excel et les types de lignes CSV.

## Sauvegarder et restaurer

### Créer une copie

Dans **Paramètres → Sauvegardes** :

- **Sauvegarde locale** crée une copie dans le dossier de l’application.
- **Copier vers une clé USB** permet de choisir un emplacement externe et d’enregistrer un fichier `.msbackup`.

Une sauvegarde locale est tentée après la création de l’espace et à chaque clôture d’inventaire. Les **30 copies locales les plus récentes** sont conservées. Les opérations enregistrées après la dernière copie ne seront pas présentes dans cette copie : créez-en une nouvelle après une séance de saisie importante.

Conservez régulièrement une copie sur un support distinct de l’ordinateur et gardez son mot de passe de récupération en lieu sûr. Un export PDF, Excel ou CSV ne permet pas de restaurer la boutique.

### Restaurer dans une session ouverte

1. Ouvrez **Paramètres → Sauvegardes → Restaurer une sauvegarde**.
2. Choisissez le fichier `.msbackup`.
3. Saisissez **le mot de passe de récupération associé à cette sauvegarde**.
4. Choisissez un **nouveau PIN de 4 à 12 chiffres**.
5. Cliquez sur **Restaurer**, puis confirmez le remplacement des données.
6. Vérifiez la boutique et les dernières opérations présentes après restauration.

La copie est contrôlée et, si nécessaire, migrée avant remplacement des données actives. Une sauvegarde de sécurité des données présentes est créée dans la session ouverte. Un mot de passe incorrect ou une copie invalide interrompt la restauration. La session utilise ensuite le nouveau PIN ; le mot de passe de récupération de la sauvegarde restaurée reste celui à conserver.

### Restaurer sur un nouvel ordinateur

La version 0.5.0 propose la restauration depuis les **Paramètres**, accessibles après ouverture d’un espace :

1. Installez Kër Finance et copiez votre fichier `.msbackup` sur le nouvel ordinateur.
2. Lors du premier lancement, créez un **espace provisoire** avec l’assistant. Le capital provisoire doit être positif et entièrement réparti ; cet espace sera remplacé par la sauvegarde.
3. Ouvrez **Paramètres → Sauvegardes** et suivez la procédure de restauration ci-dessus.
4. Utilisez le mot de passe de la **sauvegarde d’origine**, puis choisissez votre nouveau PIN.

Copier uniquement la base de données ne suffit pas pour ouvrir la boutique sur un autre ordinateur : l’accès quotidien dépend aussi du coffre système. Utilisez le fichier `.msbackup` et son mot de passe de récupération.

## Exemple de journée

Cet exemple suppose une boutique sans dette ni dépôt au départ, et **1 000 000 FCFA** de capital réel au dernier inventaire.

| Opération | Où la saisir | Effet sur le capital attendu |
| --- | --- | ---: |
| Vente de 2 articles à 2 500 FCFA | **Produits et ventes → Nouvelle vente** | +5 000 |
| Commission mobile de 1 000 FCFA | **Journal boutique → Commission mobile** | +1 000 |
| Dépense de 2 000 FCFA | **Journal boutique → Dépense** | −2 000 |
| Transfert mobile de 20 000 FCFA non encore payé | **Dettes clients → Ajouter une dette** | 0 |
| Remboursement partiel de 5 000 FCFA | **Dettes clients → Enregistrer un remboursement** | 0 |
| Dépôt client reçu de 50 000 FCFA | **Dépôts clients → Recevoir un dépôt** | 0 |

Le capital attendu est **1 004 000 FCFA**. La créance restante est **15 000 FCFA** et le dépôt à restituer est **50 000 FCFA**.

Si les soldes réellement constatés totalisent **1 039 000 FCFA**, argent du client compris :

```text
Capital réel net = 1 039 000 + 15 000 − 50 000 = 1 004 000 FCFA
Écart            = 1 004 000 − 1 004 000      = 0 FCFA
```

Vous pouvez clôturer l’inventaire équilibré, puis créer une copie externe. Ne ressaisissez dans le journal ni la vente déjà enregistrée avec les produits, ni la dette, ni le remboursement, ni le dépôt.

## Questions fréquentes

### Pourquoi le solde Wave ou espèces n’a-t-il pas changé après une opération ?

Les soldes affichés sont les montants vérifiés au dernier inventaire. Consultez le service ou comptez la caisse pour connaître le solde réel, puis saisissez-le dans **Nouvel inventaire**.

### Pourquoi la répartition du capital est-elle refusée ?

Le capital initial doit être positif et la somme de tous les comptes et espèces doit lui être exactement égale. Vérifiez que le **reste à répartir** est à zéro et que chaque solde est un entier positif ou nul.

### Pourquoi une vente ou une restitution est-elle refusée ?

Pour une vente, vérifiez les quantités disponibles, les prix et le compte choisi. Pour une restitution, le montant doit rester inférieur ou égal au solde dû au client et le compte doit être actif. Le formulaire affiche le motif du refus.

### Puis-je utiliser la même boutique sur deux ordinateurs ?

Cette version ne synchronise pas les ordinateurs. Une restauration sert à reprendre les données d’une copie ; elle ne fusionne pas deux historiques. Utilisez un seul ordinateur comme référence pour la gestion courante.

### Que faire si j’oublie mon PIN ?

L’écran de connexion ne propose pas de récupération du PIN dans cette version. Si vous disposez d’une sauvegarde et de son mot de passe de récupération, vous pouvez la restaurer sur un autre ordinateur avec un nouveau PIN, selon la procédure ci-dessus. Vous retrouverez les données présentes dans cette copie.

### Que faire si j’oublie le mot de passe de récupération ?

Kër Finance ne possède pas de serveur capable de le récupérer. Le PIN ne suffit pas pour restaurer une sauvegarde sur un autre ordinateur. Conservez ce mot de passe dès la configuration initiale.
