# 07 — Peuplement émergent : hameaux, villages et terrains (plaine à champs ouverts)

> Statut : **design du 2026-11-04** ; **les étapes 1 (générateur de carte) et 2 (aptitude par terrain) sont faites** (§8, §10, §11), le reste n'est pas implémenté. Les éléments historiques viennent de connaissances générales d'histoire agraire (plaines du Bassin parisien et de Flandre, vers 1750) : **à vérifier sur sources avant calibration**. Région retenue : **plaine à champs ouverts** (décision du 2026-11-04). Complète `02` §7–8 (extension communautaire), `04` (POC) et `06` (fertilité).

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

1. ~~Générateur de carte~~ : **fait le 2026-11-04** (`sim/src/terrain.rs`, rendu `app/src/terrain.js`, détail §10).
2. ~~Table d'aptitudes et effet sur rendement, travail requis et coût de construction~~ ; mode de carte « aptitude » : **fait le 2026-11-04** (§11).
3. Défrichement (coût, bois récupéré, repousse), drainage, ponts et gués (§3.3, §3.4, §9).
4. Générateur d'état initial par score de site (§5), suppression du plafond, règle « la famille reste » (§4).
5. Outils de mesure (distribution des tailles de hameaux, §6), puis réglage des poids.
6. Plus tard : échange entre pops, outils, saisons, services, navigation, relief (collines, montagnes), régions macro.

## 9. Décisions prises (2026-11-04)

Valeurs de départ **réalistes pour une tile de 250 m (6,25 ha)** et une carte de 200 × 200 (50 × 50 km, 2 500 km²) ; à régler à l'usage. Sauf mention, elles ne sont pas encore dans le code (étapes 2 et 3).

| Sujet | Décision | Repère réaliste |
|---|---|---|
| **Rivières** | 1 fleuve, 3 affluents, 6 ruisseaux sur la carte (**fait**) ; gabarit selon le nombre de tiles d'eau en amont : ruisseau < 70, rivière ≥ 70, fleuve ≥ 240 | un cours tous les 5 à 8 km ; ~2 à 3 % de la carte en eau |
| **Franchissement** (étape 3) | coût d'une tile de route ×3 sur un ruisseau (gué ou passerelle), ×10 sur une rivière (pont de bois ou de pierre), ×40 sur un fleuve (grand pont, enjeu majeur) | un pont de pierre coûte des années de travail d'un village |
| **Défrichement** (étape 3) | ~1 000 pop-jours par tile de forêt (chantier, équipe), qui rapporte un lot de bois | 150 à 250 jours-homme par hectare × 6,25 ha |
| **Bois rapporté** (étape 3) | ~600 stères par tile de forêt, à convertir dans les unités de `poc.toml` | 100 à 150 stères/ha pour un taillis sous futaie ; un ménage brûle 10 à 15 stères par an, une pop (≈ 20 ménages) ~250 |
| **Repousse** (étape 3) | friche → taillis en ~15 ans, taillis → bois exploitable en ~40 ans ; **plus rapide en lisière d'une forêt** (×0,6 de durée, ensemencement par les voisines), plus lente au milieu d'une zone déboisée (×1,5). Le défrichement se fait une tile à la fois, avec la repousse comme contrainte sur la forêt exploitée | succession naturelle des taillis |
| **Opportunités retenues** (étape 3) | **pierre** (carrière en plaine rocailleuse : construction, routes) et **tourbe** (marais : combustible, alternative au bois là où il manque). **Poisson** et étangs reportés (ressource nouvelle) | |
| **Terres en champs ouverts** | une terre peut **englober des terrains différents** (rendement = moyenne de l'emprise, comme aujourd'hui), mais seulement sur des tiles constructibles. Les champs ouverts réels recouvrent plusieurs sols | parcelles en lanières traversant les sols |
| **État initial** | généré d'un coup (§5), pas de pré-simulation | |
| **Palette** | trois terrains agricoles (plaine, riche, rocailleuse), marais, forêt ; colline et montagne avec les régions à relief | |

Question encore ouverte : la **forêt exploitée** (type de terre actuel, `foret_exploitee`) doit-elle s'appliquer sur les tiles de forêt de la carte, ou rester une terre ordinaire sur plaine ? (étape 3, avec le défrichement)

## 10. Étape 1 réalisée : le générateur de carte

| Élément | Où | Contenu |
|---|---|---|
| Terrain | `sim/src/terrain.rs` | `generer_terrain(ParamsTerrain)` : altitude (pente générale + relief doux), écoulement par inondation de priorité (aucune cuvette, chaque cours d'eau mène au bord), gabarit par accumulation, puis terrains attribués par parts dans l'ordre marais, forêt, rocailleuse, riche. Entiers seulement, déterministe |
| Parts par défaut | `ParamsTerrain::defaut` | marais 4 %, forêt 12 %, rocailleuse 15 %, riche 26 %, plaine le reste (~43 %) ; eau ~2,4 % en plus. Les parts sont des paramètres, pas des règles |
| Tile | `sim/src/monde.rs` | `Tuile.terrain`, `Tuile.eau`, `Tuile.aval` ; `Tuile::constructible()` : ni eau, ni forêt, ni marais |
| Effets de jeu | `sim/src/simulation.rs`, `village.rs` | une tile non constructible n'est ni bâtie, ni plantée, ni traversée par une route neuve. Les cours d'eau sont 4-connexes : une route (même en diagonale) ne peut pas les franchir tant qu'il n'y a pas de pont |
| Site du village | `terrain::sites_village` | fenêtre dégagée de rayon 12 (ou 10), sans eau, forêt ni marais, de préférence près d'un point d'eau (≤ 17 tiles) et du centre de la carte ; les sites suivants servent de repli si la disposition en croix n'y tient pas |
| Rendu | `app/src/terrain.js` | une image pour toute la carte (12 px par tile) : bordures déformées par un bruit, textures procédurales par terrain, rivières lissées entre tiles (courbes G1) avec berges ; mode Fertilité sur toute la carte ; panneau d'information (terrain, eau) |
| Tests | `sim/tests/terrain.rs` | déterminisme, parts exactes, chaque cours d'eau va au bord par des côtés avec un gabarit croissant, site du village dégagé et près de l'eau, jamais de bâti ni de chemin sur un terrain fermé après 120 ans |
| Visualisation | `sim-cli --terrain` | parts de chaque terrain et vue d'ensemble en ASCII |

**Limites connues**
- ~~La fertilité reste le champ radial autour du village~~ : levé à l'étape 2 (§11).
- Le village garde sa **disposition en croix** (remplacée à l'étape 4). Comme la fenêtre dégagée exclut l'eau, le point d'eau le plus proche est à 11–17 tiles (3 à 4 km) : à la lisière du terroir, pas au cœur du village.
- Forêts et marais sont fermés (ni défrichement, ni drainage avant l'étape 3) ; les routes ne franchissent aucun cours d'eau (pas de pont).
- La pente générale donne des rivières à peu près parallèles ; le relief n'est pas rendu.
- Le sol est une image de 12 px par tile : un peu flou à fort zoom ; un rendu par chunks à plus forte résolution est possible plus tard.
- Pas de variantes visuelles par tile (fleurs, rochers…) ; les assets de l'atlas ne servent plus pour le sol.

## 11. Étape 2 réalisée : l'aptitude par terrain

| Élément | Où | Contenu |
|---|---|---|
| Table de données | `sim/donnees/poc.toml` | `[terrains.<nom>] cout_construction` ; par type de terre : `aptitude = {…}` (rendement par terrain) et `travail_terrain = {…}` (travail requis par terrain). Terrain absent = 1,0 ; nom inconnu = erreur de chargement |
| Rendement | `Catalogue::rendement_tile` | qualité locale de la tile × aptitude du terrain ; la production d'une terre suit la **moyenne de son emprise** |
| Travail requis | `Catalogue::besoin_mo_terre_milli` | somme par tile de `jours_par_tile_an × travail_terrain` : une terre sur sol difficile réclame plus de bras |
| Coût de construction | `cout_defrichement`, `cout_logement` | le travail des chantiers (défrichement, maison, hameau) est multiplié par `cout_construction` du terrain de chaque tile |
| Choix des tiles | `meilleur_site`, `agrandir`, `lancer_defrichement` | le score d'une tile pour une terre utilise le **rendement pour ce type** (champ ≠ élevage de laine) |
| Fertilité locale | `village.rs` | **plus de déclin radial** autour du village : bruit de 0,85 à 1,3, la distance ne joue que par le trajet. Paramètres `rayon_fertile` / `rayon_sterile` et options CLI supprimés |
| Site du village | `terrain::candidats_centre` | pénalise la plaine rocailleuse à portée, favorise la riche |
| App | `app/src/App.vue`, `PanneauInfos.vue` | mode **Aptitude** avec choix du type de terre ; le panneau d'une tile libre donne le rendement de chaque type, le travail requis et le surcoût de construction |

**Valeurs de départ** (hypothèses à régler à l'usage)

| Terre | plaine | riche | rocailleuse | marécageuse | forêt |
|---|---|---|---|---|---|
| Champ de céréales | 1,0 | 1,25 | 0,6 | 0,15 | 0 |
| Pâturage | 1,0 | 1,1 | 0,7 | 0,5 | 0,2 |
| Verger | 1,0 | 1,1 | 1,0 | 0,1 | 0 |
| Forêt exploitée (taillis planté) | 1,0 | 1,0 | 1,0 | 0,5 | 1,0 |
| Champ de lin | 1,0 | 1,3 | 0,4 | 0,3 | 0 |
| Élevage de laine | 1,0 | 0,85 | 1,2 | 0,3 | 0,1 |

Travail requis par tile : rocailleuse ×1,3 (céréales, lin), marais ×1,4 à 2,0, forêt ×2,0 à 2,5. Coût de construction : plaine et riche ×1, rocailleuse ×1,3, forêt ×2, marais ×2,5.

**Décisions et limites**
- La **forêt exploitée** reste un **taillis planté sur terre ordinaire** (1,0 sur plaine) tant que les tiles de forêt de la carte ne sont pas exploitables (étape 3) : à 0,6 le village manquait de bois. Elle prendra alors sa place sur les tiles de forêt.
- Aucun terrain n'interdit une activité : l'aptitude est un multiplicateur, donc un terrain difficile reste utilisable si le besoin presse (émergence). Forêt et marais restent fermés par `constructible()` jusqu'à l'étape 3.
- Mesure : sur 100 ans et 8 graines, la part de plaine rocailleuse parmi les nouveaux champs (14,5 %) est inférieure à sa part parmi les tiles libres à portée (17,3 %) : l'effet est **faible** car la compacité (`poids_voisins` = 0,25) pèse autant qu'un écart d'aptitude de 0,25. À régler avec les poids de `[extension]` (étape 5).
- Le village de départ n'est pas posé en fonction du terrain de chaque terre (la croix est rigide) ; la composition initiale reste dimensionnée sur un rendement de 1,0.
- La compacité des terres mesurée par les tests monte à ~1,36 (1,35 avant) : elles suivent les plaques de terrain.
- Les rendements dépendent encore uniquement du terrain et de la qualité locale : pas de fertilité dynamique (`06` §4).
