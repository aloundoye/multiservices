# Référence métier — Kër Finance

[Retour au README](../README.md) · [Guide d’utilisation](guide-utilisation.md) · [Développement](developpement.md)

Ce document rassemble les règles détaillées des comptes, du stock, des dettes, des dépôts et des migrations de la version **0.5.0**. Pour apprendre à utiliser les écrans, commencez par le guide d’utilisation.

## Sommaire

- [Comptes et SIM](#comptes-et-sim)
- [Produits et ventes](#produits-et-ventes)
- [Clients des dettes et remboursements](#clients-des-dettes-et-remboursements)
- [Dépôts clients](#dépôts-clients)
- [Mise à niveau des données et sauvegardes](#mise-à-niveau-des-données-et-sauvegardes)

## Comptes et SIM

À l’ouverture, un compte par service est proposé. Ajoutez les SIM nécessaires et répartissez le capital entre elles et la caisse. La somme doit être exactement égale au capital initial ; les montants sont des FCFA entiers positifs ou nuls.

Ensuite, **Paramètres → Comptes et SIM** permet de créer, renommer, archiver et réactiver les comptes. Chaque compte a un nom obligatoire et un identifiant facultatif. Le nom est unique dans son service, sans distinction de casse, y compris parmi les comptes archivés. Le service et l’ID permanent ne changent jamais.

- Un compte ajouté ne modifie aucun capital. Il affiche « Pas encore relevé » jusqu’au prochain inventaire, où son solde devient obligatoire.
- Un transfert entre deux SIM change seulement la répartition. Ne l’enregistrez pas comme une recette. Un apport réel de capital doit en revanche être inscrit au journal.
- Les inventaires demandent un solde pour chaque compte actif et affichent les sous-totaux par service. La variation du premier relevé d’un compte est indisponible, et non supposée nulle.
- Pour archiver un compte utilisé, son dernier solde doit être nul et aucune opération ne doit avoir été enregistrée sur ce compte depuis. Un compte jamais utilisé peut être archivé directement. La caisse ne peut pas être archivée ni dupliquée.
- Les anciennes dettes d’un compte archivé peuvent être remboursées sur une SIM active ou en espèces. Les contre-écritures gardent le compte et le libellé de l’écriture originale, même après archivage ou renommage.
- L’historique et les exports conservent les noms et identifiants au moment des opérations. Dans les exports CSV, les lignes `solde_compte` détaillent les lignes `inventaire` : ne les additionnez pas ensemble. Dans Excel, la synthèse et les soldes détaillés sont dans des feuilles distinctes.

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
- Si les soldes changent après l’aperçu, le moteur comptable refuse la validation et demande d’actualiser l’aperçu.
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

### Reprendre l’argent déjà gardé

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

## Mise à niveau des données et sauvegardes

La version 0.5.0 utilise le **schéma SQLite 5** et s’installe par-dessus la 0.4.0. L’identifiant `sn.kerfinance.multiservices` et les chemins de stockage restent inchangés : comptes, produits, ventes, dettes, PIN et historique sont conservés. Gardez le mot de passe de récupération et une copie externe de vos sauvegardes.

Avant de migrer une base existante aux schémas 1, 2, 3 ou 4, l’application crée **et vérifie** une sauvegarde chiffrée `avant-migration-vN-….msbackup`. Si cette étape échoue, elle ne modifie pas la base. Les migrations manquantes sont exécutées conditionnellement, dans une transaction auditée ; les tables du schéma 3 ne sont pas recréées. Au schéma 1, les anciens soldes restent marqués « historique regroupé par service », sans inventer de répartition entre SIM.

Le schéma 4 ajoute les dépôts, les identifiants de requête et les instantanés d’inventaire. Les anciennes clôtures gardent leurs montants sans dépôts historiques inventés. Les opérations de dépôt, le solde client, l’audit et le résultat associé à une requête sont enregistrés dans la même transaction : une requête identique rejouée ne crée aucun doublon, et le même identifiant avec d’autres données est refusé.

Le schéma 5 crée les fiches depuis les anciennes dettes en regroupant uniquement les couples nom/téléphone identiques après normalisation : casse et espaces du nom, espaces/tirets/parenthèses du téléphone. Les noms différents et les indicatifs différents restent séparés. Les dettes, anciens paiements et inventaires ne sont pas réécrits.

Les sauvegardes des **schémas 1 à 5** sont restaurables. La restauration contrôle et migre une copie temporaire avant de remplacer les données actives. Les sauvegardes 4 incluent les dépôts, les reprises, les annulations, les instantanés et les informations d’idempotence. Un mot de passe incorrect, une archive endommagée ou une migration invalide interrompt la restauration. Les sauvegardes 5 incluent aussi les fiches des dettes, les reçus globaux et leurs allocations. Une base migrée au schéma 5 ne doit pas être ouverte avec une ancienne version.
