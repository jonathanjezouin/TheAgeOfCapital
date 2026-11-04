# 07 — Peuplement émergent : hameaux, villages et terrains (plaine à champs ouverts)

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

## 3. Carte : terrains, eau et forêts

> Révisé le 2026-11-04 : les sept types de sol d'une première version étaient trop détaillés. Le joueur doit **lire la carte d'un coup d'œil** : peu de terrains, chacun avec une identité visuelle et des conséquences de jeu claires.

### 3.1 Principe : peu de terrains, chacun avec un caractère
Un terrain porte quatre propriétés, toutes en données (`poc.toml`), jamais en règles :

| Propriété | Rôle |
|---|---|
| **Aptitudes** | rendement par type de terre (champ, pré, verger, élevage…), voir `06` §2 |
| **Travail requis** | multiplicateur du travail par unité de surface (traction, drainage, cailloux) |
| **Coût et franchissabilité** | multiplicateur du coût de construction et de route ; infranchissable pour certains terrains |
| **Ressource ou opportunité** | ce qui ne se trouve que là : c'est ce qui rend un détour coûteux *rentable* |

Le dernier point est la clé des futurs choix du joueur : une route à travers un marais coûte plus cher, mais elle ouvre l'accès à la tourbe, au poisson ou à un raccourci. Sans contrepartie, un terrain difficile n'est qu'un obstacle à contourner.

### 3.2 Terrains de la plaine (première version)
| Terrain | Agriculture | Construction et routes | Opportunités (à valider) |
|---|---|---|---|
| **Plaine** | référence (1,0) | référence | — |
| **Plaine riche** (variante) | meilleur rendement des céréales | référence | attire les premiers noyaux ; les terres y sont mises en valeur d'abord |
| **Plaine rocailleuse** | céréales plus faibles ; convient à l'élevage ovin, à la vigne et aux vergers | un peu plus cher (cailloux) | pierre de construction, carrière |
| **Plaine marécageuse** (type Dombes) | quasi inculte au départ ; drainage = investissement tardif | routes et bâtiments nettement plus chers ; ponts, digues | étangs (pêche), tourbe (combustible), roseaux ; urbanisation tardive car pénible |
| **Forêt** | non cultivable tant que non défrichée | traversable, chemins plus lents | bois (chauffage, construction) ; réserve foncière |
| **Colline** (relief, plus tard) | élevage, vigne, vergers | routes plus chères (pente) | minerai, pierre, énergie hydraulique en vallée |
| **Montagne** (plus tard) | quasi nulle | **infranchissable** hors cols, ou très cher | minerai, bois, bois d'œuvre, force hydraulique |

Pour le premier générateur : **plaine, plaine riche, plaine rocailleuse, plaine marécageuse, forêt**, plus l'eau (§3.3). Colline et montagne arrivent avec les régions à relief. Désert, grandes steppes et régions marécageuses étendues relèvent de la **carte macro** (étape 3 de la roadmap) : ce sont des *régions identifiables* qui changent la palette de terrains, pas des cases de plus dans la plaine.

La palette est volontairement **étroite et lisible** : on peut la compléter plus tard (un terrain de plus = une ligne de données et une tile de rendu). Les aptitudes du tableau sont des hypothèses de départ à régler à l'usage.

### 3.3 Eau et rivières
| Élément | Rôle |
|---|---|
| **Rivière** (trois gabarits : ruisseau, rivière, fleuve) | structure le territoire ; **seuls les plus gros gabarits seront un jour navigables**, les petits ne le seront jamais |
| **Franchissement** | un pont coûte cher (plus pour un gabarit plus gros) ; la rivière est donc une coupure qui infléchit routes et terroirs |
| **Point d'eau** | source, rivière ou puits ; attracteur d'habitat (§4.1) ; sur plateau sec, le puits est coûteux |
| **Navigation, moulins, ports** | **hors POC** : la rivière est aujourd'hui cosmétique et structurante (coût de pont, point d'eau). La navigation viendra avec les infrastructures de transport (`02` §9) |

Aucun « village de source » n'est codé : les noyaux se forment aux points d'eau parce que le puits coûte cher ailleurs.

### 3.4 Forêts et défrichement
- Les forêts sont **présentes dès le départ**, placées surtout sur les terrains médiocres (rocailleux, marécageux en lisière) ; une pop doit s'installer près d'elles pour s'approvisionner en bois.
- **Défricher coûte du travail et du temps** (chantier) et **rapporte du bois** (à calibrer : le défrichement fournit un lot de bois au chantier ou au stock du hameau).
- Une tile sans bois **repousse** avec le temps : la tile vide est sélectionnable dans l'app et affiche son temps de repousse. Hypothèse retenue : la repousse est **plus rapide en lisière d'une autre forêt** (ensemencement par les tiles voisines) et plus lente au milieu d'un espace déboisé. *(À confirmer : si c'est l'inverse, il suffit d'inverser le signe dans la donnée.)* Cette fonctionnalité n'est pas retrouvée dans le dépôt `main` : à vérifier avant de l'intégrer au générateur.
- Accès libre pour le POC (pas de communaux, voir §1).

### 3.5 Dérivation et lecture dans le moteur
Le générateur produit des **couches** (substrat, relief doux, hydrographie, forêt) à basse fréquence, déterministes selon la graine, puis **classe chaque tile** dans un terrain (§3.2). Le moteur lit `aptitude[type_de_terre][terrain]` (table dans `poc.toml`) au lieu de `fertilite_ppm` ; la fertilité d'une terre reste la moyenne de son emprise (`06` §5, étape 2). Effets distincts :

| Effet | Où | Exemple (hypothèse) |
|---|---|---|
| **Rendement** | extrants de la terre | blé : plaine riche 1,2 ; plaine 1,0 ; rocailleuse 0,6 ; marais 0,2 |
| **Travail requis** | équipe de la terre | marais ×2, rocailleuse ×1,3 |
| **Coût de construction et de route** | chantiers, segments d'infrastructure | marais ×2–3, pont selon le gabarit |

Les outils (charrue, attelage, drainage) modifieront ces multiplicateurs plus tard : c'est ce qui poussera les pops à en acquérir (§7).

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

Un **générateur d'état initial** pose directement la situation de 1750 ; on observe ensuite son évolution (pas de pré-simulation de décennies : décision du 2026-11-04). Le générateur applique les mêmes règles de site que le jeu (§4) pour placer les hameaux, mais en une seule passe :

- les terres des meilleurs terrains (plaine riche, plaine) sont **déjà exploitées** ; les hameaux sont placés par le score de site (eau, terres, bois, voisins) ;
- quelques noyaux plus gros aux meilleurs points d'eau, beaucoup de hameaux de 1 à 2 pops ailleurs ;
- les terrains médiocres (rocailleux, marécageux, forêts) sont quasi vierges : la conquête de ces terres, avec des rendements décroissants, **émerge** ensuite.

## 6. Critères de succès (à mesurer sur plusieurs graines)

- Distribution des tailles de hameaux à **longue traîne** : majorité de 1 à 2 pops, quelques noyaux de 6 à 16. Médiane et maximum mesurés, jamais imposés.
- Les noyaux sont proches de points d'eau et des meilleurs sols ; les hameaux isolés sont sur terres propres.
- Aucune dépendance au plafond codé : changer les poids du score déplace la distribution, pas une constante.
- Test d'invariants : mêmes résultats à graine égale ; aucun hameau sans accès à l'eau ni à un bien vital.

## 7. Suite : la pop comme acteur (hors cette étape)

La pop cherche sa prospérité (besoins vitaux, puis confort), travaille, et peut acquérir des outils avec son surplus. Cela tient au principe 3 (« la Firme est l'acteur économique ») si une pop paysanne est une **exploitation familiale**, c'est-à-dire une firme implicite. Les firmes à capital et salariés émergent quand une pop accumule assez pour embaucher. Cela demande des **échanges au niveau de la pop** (prix ou troc, monnaie), absents aujourd'hui (les pops consomment le stock de leur hameau). Également reportés : saisons et pointes de travail, services à coût fixe (moulin, four, forge), qui sont les forces de regroupement manquantes. Voir `05`.

## 8. Ordre de travail

1. Générateur de carte : couches, terrains (plaine, plaine riche, rocailleuse, marécageuse, forêt), rivières à trois gabarits, forêts fixes (§3). Rendu de la carte de terrains, validé visuellement.
2. Table d'aptitudes et effet sur rendement, travail requis et coût de construction (§3.5) ; mode de carte « aptitude ».
3. Défrichement (coût, bois récupéré, repousse) et coût de pont (§3.3, §3.4).
4. Générateur d'état initial par score de site (§5), suppression du plafond, règle « la famille reste » (§4).
5. Outils de mesure (distribution des tailles de hameaux, §6), puis réglage des poids.
6. Plus tard : échange entre pops, outils, saisons, services, navigation, relief (collines, montagnes), régions macro.

## 9. Questions ouvertes

- Repousse : plus rapide ou plus lente en lisière d'une autre forêt (hypothèse : plus rapide) ? Durée de départ ?
- Défrichement : quantité de bois rapportée par tile ; lien avec les chantiers.
- Nombre de rivières et de sources pour une carte 200×200 (50 km × 50 km) ; gabarits et coût de pont par gabarit.
- Opportunités des terrains difficiles (§3.2) : lesquelles dans le POC (pierre, tourbe, poisson) ? Sans ressource à aller chercher, une route à travers un marais n'a pas de raison d'être.
- Taille des terres en champs ouverts : voir `02` §10.2 (la terre est un bâtiment multi-tiles ; reste à décider si elle peut englober des tiles de terrains différents, avec rendement moyen, ou si elle doit rester sur un terrain homogène).
