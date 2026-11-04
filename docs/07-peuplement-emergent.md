# 07 — Peuplement émergent : hameaux, villages et sols (plaine à champs ouverts)

> Statut : **notes de design du 2026-11-04**, rien n'est implémenté. Les éléments historiques viennent de connaissances générales d'histoire agraire (plaines du Bassin parisien et de Flandre, vers 1750) : **à vérifier sur sources avant calibration**. Région retenue : **plaine à champs ouverts** (décision du 2026-11-04). Complète `02` §7–8 (extension communautaire), `04` (POC) et `06` (fertilité).

## 1. Objectif

Voir émerger, sans le coder, une campagne fidèle : beaucoup de petits hameaux de 1 à 2 pops, quelques noyaux de plusieurs pops (jusqu'à ~16 = 1 600 hab., l'échelle d'un gros village) et, plus tard, des bourgs. **Aucune taille de hameau n'est une règle** : on la mesure (§6).

Principe : un noyau se forme là où les **forces de regroupement** dépassent les **forces de dispersion**. Chaque force est un coût ou un gain mesurable dans le moteur, jamais un seuil.

| Force | Sens | État |
|---|---|---|
| Distance aux champs (trajet quotidien, congestion) | disperse | existe (`distance_max`, flux unifié) |
| Terroir fini, rendements décroissants | plafonne le groupe | partiel (fertilité par tile) |
| Point d'eau (source, rivière, puits) | regroupe | **à faire** (§3) |
| Les enfants restent près des parents | regroupe | **à faire** (§4) |
| Services à coût fixe (moulin, four, forge, église) | regroupe au-delà d'un seuil de clientèle | reporté (firmes, POC-B) |
| Entraide aux pointes de travail (moisson), assolement collectif | regroupe | reporté (saisons) |
| Sécurité | regroupe | reporté |

Point d'attention : sans saisons ni services, le POC n'a que **deux** forces de regroupement (eau et famille). Il ne faut pas lui demander plus que des noyaux modestes ; les gros villages et les bourgs viendront avec les firmes et les saisons.

## 2. Spécificités de la plaine à champs ouverts

- **Habitat groupé** : villages compacts au centre d'un terroir de champs ouverts (parcelles en lanières, sans clôtures), forêts rares et lointaines. C'est l'inverse du bocage dispersé.
- **Eau** : sur plateau, la nappe est profonde (puits coûteux). Les villages se tiennent aux **sources du pied de coteau**, au bord des rivières, ou sur les rares sols à nappe peu profonde. C'est le premier attracteur naturel.
- **Bois rare** : il pousse sur les sols médiocres (argile lourde, sable, pentes), pas sur les bons limons déjà défrichés. Le bois est un facteur de localisation (construction, chauffage).
- **Collectif** (assolement triennal commun, vaine pâture, communaux) : explique la cohésion du village, mais suppose saisons et droits d'usage. **Reporté** ; en attendant, l'accès est libre (simplification assumée : pas de seigneur, pas de droits d'usage, extension libre).

## 3. Carte : couches et sols

Le générateur produit quelques **couches de terrain**, déterministes (graine), dont les types de sol sont **dérivés**. Cela correspond à l'étape 2 de `06` §5.

### 3.1 Couches
| Couche | Génération | Rôle |
|---|---|---|
| **Substrat** | larges bandes et plages à basse fréquence (plateau limoneux, bandes de craie, poches d'argile ou de sable) | fixe le sol de base |
| **Relief doux** | altitude à basse fréquence, pente dérivée | écoulement, coteaux, sources |
| **Hydrographie** | une rivière principale (cours déduit du relief), quelques affluents, **sources** là où le substrat perméable surplombe une couche imperméable en pied de coteau | humidité, alluvions, points d'eau, moulins |
| **Forêt** | présente dès le départ, placée surtout sur les sols médiocres et les pentes ; **non extensible gratuitement** : défricher coûte cher en travail ; repousse lente | bois, réserve foncière |

### 3.2 Types de sol (7 suffisent)
| Sol | Où | Aptitudes (qualitatif) |
|---|---|---|
| **Limon profond** (loess) | plateaux | meilleur blé, betterave plus tard ; nappe profonde (puits coûteux) |
| **Limon sableux / terre franche** | bas de pente | bon blé et orge, labour facile |
| **Argile lourde** | poches humides | sol « froid », dur à labourer avec les outils de 1750 (traction), mauvaise terre à blé, bonne prairie ou forêt |
| **Craie** | coteaux, bandes | pauvre, mince : moutons, seigle, orge ; sources en pied |
| **Sable** | poches, terrasses | pauvre : seigle, lande, forêt ; labour léger, rapide |
| **Alluvions** | fond de vallée | prés, chanvre, lin, potager ; humide voire inondable ; excellent mais risqué pour les céréales |
| **Tourbe / marais** | zones basses de la vallée | pâture pauvre, tourbe (combustible), inutilisable sans drainage |

### 3.3 De l'aptitude à l'activité
Le moteur lit `aptitude[type_de_terre][sol]` (table dans `poc.toml`, ppm de 1,0, voir `06` §5) au lieu de `fertilite_ppm`. La fertilité d'une terre reste la moyenne de son emprise. Deux effets distincts, pour ne pas tout fondre dans un rendement :

| Effet | Où | Exemple |
|---|---|---|
| **Rendement** : multiplie la production | extrants de la terre | blé sur limon = 1,0 ; sur craie = 0,6 ; sur argile = 0,5 |
| **Travail requis** : multiplie le travail par unité de surface | équipe de la terre | argile lourde = ×1,6 à la main ; sable = ×0,8 |

Le second effet permet d'**encourager ou de décourager** une activité par le sol *et* par la technique : plus tard, charrue à versoir et attelage réduiront le travail requis sur argile, et rendront cette terre intéressante. C'est un exemple de pression qui pousse les pops à acquérir des outils (§7), sans script. La table des aptitudes est une donnée, pas une règle.

### 3.4 Eau
Chaque tile reçoit une **distance à un point d'eau** (source, rivière) et une **profondeur de nappe** (haute en vallée, basse sur plateau). Un habitat sans eau à portée n'est pas viable, ou nécessite un puits (chantier coûteux, proportionnel à la profondeur). Aucun « village de source » n'est codé : ils ressortent du coût du puits.

## 4. Règles de peuplement

### 4.1 Choix du site (remplace `meilleur_site` seul)
Un score continu, sans seuil dur : **terres cultivables à portée (pondérées par aptitude), eau (distance ou coût du puits), bois à portée, proximité de voisins** (bonus faible : entraide, sécurité, services futurs), moins le coût de construction. Les poids sont des données à régler sur plusieurs graines.

### 4.2 Plus de plafond codé
Supprimer `capacite_max_pops` (aujourd'hui 4). La taille d'un noyau ressort de coûts marginaux croissants :
- **trajet moyen aux champs** qui s'allonge à mesure que les terres proches sont occupées ;
- **eau** : un point d'eau a un débit, un puits profond coûte plus cher par pop ;
- **congestion** des chemins d'accès (déjà modélisée).

### 4.3 Coûts de logement
Garder une maison supplémentaire et un site neuf à **coûts voisins** (maison ~150 pop-jours, hameau neuf = site + première maison, comparable). Le choix entre « s'ajouter à un noyau » et « fonder ailleurs » est tranché par le score (§4.1), pas par un coût artificiellement favorable à la dispersion : si fonder est toujours moins cher, il n'y a jamais de village.

### 4.4 La famille reste
Une nouvelle pop (croissance, `00`) cherche d'abord un logement **dans son hameau, puis dans les hameaux proches** (déjà en place) ; elle ne **fonde** un site neuf que si le score local descend sous celui d'un site disponible ailleurs. C'est le moteur de croissance sur place d'un noyau.

### 4.5 Champs et entrées
Les terres sont des bâtiments multi-tiles à entrée unique (principe 1 du `CLAUDE.md`) : une tile éloignée d'une terre reste accessible par l'entrée proche. Cela réduit le coût de la dispersion et justifie les hameaux isolés aux premiers temps.

## 5. État initial de la carte

Plaine à champs ouverts en 1750 : les bons limons sont **déjà exploités** (terres en place), les hameaux et noyaux existent déjà aux sites favorables (eau). Le joueur observe leur croissance, le remplissage des vides, puis la conquête de sols de plus en plus marginaux (argile, sable, craie, marais) : les rendements décroissants **émergent**. Le générateur place l'état initial en simulant quelques décennies de peuplement avec les mêmes règles (§4), plutôt qu'en posant des hameaux à la main.

## 6. Critères de succès (à mesurer sur plusieurs graines)

- Distribution des tailles de hameaux à **longue traîne** : majorité de 1 à 2 pops, quelques noyaux de 6 à 16. Médiane et maximum mesurés, jamais imposés.
- Les noyaux sont proches de points d'eau et des meilleurs sols ; les hameaux isolés sont sur terres propres.
- Aucune dépendance au plafond codé : changer les poids du score déplace la distribution, pas une constante.
- Test d'invariants : mêmes résultats à graine égale ; aucun hameau sans accès à l'eau ni à un bien vital.

## 7. Suite : la pop comme acteur (hors cette étape)

La pop cherche sa prospérité (besoins vitaux, puis confort), travaille, et peut acquérir des outils avec son surplus. Cela tient au principe 3 (« la Firme est l'acteur économique ») si une pop paysanne est une **exploitation familiale**, c'est-à-dire une firme implicite. Les firmes à capital et salariés émergent quand une pop accumule assez pour embaucher. Cela demande des **échanges au niveau de la pop** (prix ou troc, monnaie), absents aujourd'hui (les pops consomment le stock de leur hameau). Également reportés : saisons et pointes de travail, services à coût fixe (moulin, four, forge), qui sont les forces de regroupement manquantes. Voir `05`.

## 8. Ordre de travail

1. Générateur de carte : couches, sols, hydrographie, forêts fixes (§3).
2. Table d'aptitudes et effet sur rendement et travail (§3.3) ; mode de carte « aptitude ».
3. Choix de site par score, suppression du plafond, règle « la famille reste » (§4).
4. Outils de mesure (distribution des tailles, §6), puis réglage des poids.
5. Plus tard : échange entre pops, outils, saisons, services.

## 9. Questions ouvertes

- Rivière : navigable (infrastructure de transport, `02` §9) ou seulement hydrographie ?
- Nombre de rivières et de sources pour une carte 200×200 (50 km × 50 km) ?
- Défrichement des forêts : coût de départ, repousse, accès libre ou communal ?
- Taille des terres en champs ouverts (parcelles en lanières) : la terre multi-tiles les représente-t-elle suffisamment ?
- Échelle de l'état initial simulé (§5) : combien d'années de pré-simulation ?
