# 04 — Périmètre du POC

> **Statut (2026-11-02).** Le moteur `sim/`, les assets et le rendu `app/` sont **alignés** sur le design ci-dessous : tile fixe de 250 m (la carte 200×200 couvre 50 × 50 km), réseau en arêtes à 8 directions (`02` §9), une maison en bois par pop, textile domestique (plus de hameau textile), terres exploitées sur N tiles et budget de jours de travail (`02` §8). Aperçu des assets : `docs/img/apercu-maisons-assets.png`. Reste à faire : plusieurs groupes de hameaux (la croix de départ ne porte qu'un village de ~4 hameaux), plafond des pops et densification (`02` §9.4), saisons.

## Objectif

Une **économie de subsistance** : un village isolé de **1750**, simulé de façon autonome (sans intervention du joueur). Des pops vivent dans des hameaux, travaillent des terres exploitées alentour (des champs de N tiles de 250 m), et filent, tissent et cousent à la maison de quoi vêtir tout le monde avec leur temps libre. La couverture de leurs besoins pilote la croissance ou le déclin de chaque pop, et **les habitants étendent eux-mêmes le village** (défrichement, nouveaux hameaux) quand il le faut. Pas de monnaie, pas de manufacture, pas de firme : le maire est inactif.

**Deux étapes**

| Étape | Contenu |
|---|---|
| **POC-A — subsistance** | Production continue (pas de saisons), circulation des marchandises (production, stock, distribution, consommation), besoins vitaux et de confort, croissance par jauge de pop, budget de jours de travail réparti chaque jour entre les terres, **extension du village par ses habitants** (voir `02` §7). Village initial généré. |
| **POC-B — monnaie et marché** | Salaires, prix, firmes, surplus qui quitte le village (dîme, marché), placement des bâtiments par des firmes. |

**Critères de succès**
- La simulation tourne indéfiniment sans blocage ni dérive absurde.
- Les quantités produites / consommées restent proportionnellement cohérentes (conservation de la matière vérifiée).
- Les dynamiques de subsistance émergent : croissance lente (+0,5 %/an quand le confort manque, +1,5 %/an quand tout est couvert), déclin en cas de manque vital, pénuries localisées, production qui s'ajuste au besoin (le temps de travail va d'abord aux terres dont la ressource manque), **extension du village qui plafonne d'elle-même** (la terre à portée se remplit, la terre lointaine est plus pauvre), population en « dents de scie » autour de ce plafond.
- **Chaque mécanisme est lisible** par le joueur (voir « Ce que voit le joueur »).

## Périmètre

| Élément | POC |
|---|---|
| Carte | 200×200 tiles de 250 m (50 × 50 km), **un village généré** (graine, aléatoire mais cohérent, voir ci-dessous), vue micro uniquement |
| Personnage | Un seul : **le maire**, **inactif** (aucune décision, ne construit pas). Ni leviers (pouvoir / capital / influence), ni postes, ni fortune dans le POC |
| Population | **Pops de 100 hab**, une seule strate (paysans), une seule culture / religion / orientation, attributs fixes. **Une jauge de croissance par pop** : à +100 hab elle se duplique, à −100 elle disparaît (voir `02` §6). La satisfaction du confort servira plus tard à la promotion vers d'autres classes |
| Habitat | Un type : le **hameau** (**1 tile**, **une maison en bois par pop**, de 2 pops au départ à 4 au plus ; un hameau neuf n'a que sa première maison). Plafond de densité d'une tile d'habitat : 10 pops |
| Production | Des **terres exploitées** sans logement (champ, pâturage, verger, forêt, lin, laine), chacune un seul bâtiment sur **N tiles contiguës**, travaillées par les pops des hameaux à portée ; l'**atelier domestique** de chaque hameau (textile, sans poste, avec le temps libre). Ni filature ni atelier de couture indépendants (voir `05`) |
| Monnaie | Aucune (autoconsommation, échanges en nature entre voisins) |
| Construction | Par la **communauté** : défrichement (une terre naît à 4 tiles et s'agrandit par blocs de 4), maisons une à une, fondation de hameaux, par des équipes de pops, avec du bois, à rythme saisonnier (voir `02` §7) |
| Firmes | Aucune en POC-A |
| Terrain | Un seul : **plaine** (herbe) |
| Infrastructure | Une seule : **chemin de terre**, couche d'**arêtes à 8 directions** sur les tiles (`02` §9) ; une route peut traverser des terres |
| Temps | Production **continue**, pas de saisons |
| Niveau technologique | 0 (production primitive) |
| Transport | Le lien **pop → hameau de résidence → lieu de travail** est matérialisé. **Distance = coût du plus court chemin le long des arêtes** (10 par côté, 14 par diagonale), un lieu est desservi si une arête passe par sa tile ; aucune pénalité mais **limite de 100** (10 tiles-équivalents, 2,5 km) pour travailler ou s'approvisionner, et approvisionnement au plus proche. Congestion et livraisons différées : **en sommeil** (capacités très au-dessus des flux) |
| Intermédiaires | Aucun (relation directe fournisseur ↔ client) |

## Ressources

| Catégorie | Ressources POC | Reporté |
|---|---|---|
| Nourriture | grain, viande, fruit — **interchangeables** pour le besoin vital (unités nutritives) | poisson ; nourriture industrielle |
| Textile | lin, laine → tissu → vêtements, **travail domestique** à l'atelier de chaque hameau | coton ; manufacture |
| Bois | bois (chauffage), consommé directement | transformation |
| Travail | les pops : **budget de jours** (12 500 jours-personne par pop et par an) | qualifications |

## Catalogue des assets (identifiants)

**Tiles de 256 px** : `terrain_plaine` (10 variantes d'herbe raccordables) · terres exploitées à bords de parcelle, pour les chantiers et les terres d'une tile : `bat_champ` (3) · `bat_champ_mur` (réservé aux saisons) · `bat_paturage` (2) · `bat_verger` (2) · `bat_foret_exploitee` (3) · `bat_champ_lin` (2) · `bat_elevage_laine` (2) · `bat_chantier_champ` (2) · **textures raccordables** des terres sur N tiles : `bat_champ_plein`, `bat_champ_lin_plein`, `bat_verger_plein`, `bat_foret_exploitee_plein`, `bat_paturage_plein`, `bat_elevage_laine_plein` (2 variantes chacune ; haies et clôtures tracées par le rendu le long du bord de l'emprise).

**Sprites de 128 px** : `bat_maison_bois` (6 variantes, toit de planches, cour et tas de bois, dessinée face au Sud et pivotée vers la route) · `bat_chantier_maison` (3 états : sablières, ossature, charpente).

**Hors atlas** : `sol_chemin.png`, texture de terre battue raccordable qui remplit le trait des routes (tracées en courbes par le rendu, `03` §6).

48 frames, générées par `graphGeneration/` (voir `03` §7) ; aperçu : `docs/img/apercu-maisons-assets.png`. Convention : une maison par pop, sprite exagéré à ~1/3 de tile (`03` §6) ; matériau : bois seul. Reportés : `bat_filature`, `bat_atelier_couture` (manufacture), `bat_quartier_rural` (village de 2–4 tiles).

## Référence 1750 (décision de design, implémentée dans le moteur)

Ordres de grandeur reconstitués de mémoire (Toutain, Braudel…), **à vérifier avant de figer** ; précision attendue ±20 %. Densité moyenne ~47 hab./km², ~75 % d'actifs agricoles.

### Usage du sol pour 100 hab. (1 pop), tile de 6,25 ha

| Usage | % du territoire | ha | Tiles |
|---|---|---|---|
| Labours (jachère incluse) | 40 % | 85 | 13,6 |
| Prés | 7 % | 15 | 2,4 |
| Vignes, vergers, jardins | 3 % | 6 | 1 |
| Bois | 17 % | 36 | 5,8 |
| Landes, friches, communaux | 28 % | 60 | 9,5 |
| Bâti, routes, eaux | 5 % | 11 | 1,7 |
| **Terroir total** | 100 % | ~212 | **~34** |

- 1 pop ≈ 1 km² de labours (85–125 ha). Un champ de 5 km² correspond à ~4 pops **rattachées** (le « 4 pops » du POC), pas à 4 pops à plein temps.
- Un hameau de 2 pops a un terroir de ~64 tiles (4 km², environ 8×8 tiles), dont ~27 de labours.

### Travail agricole (jours-personne par ha et par an)

| Culture | Jours/ha/an | Par tile (6,25 ha) | Pic |
|---|---|---|---|
| Céréales | ~40 | ~250 | moisson juillet-août (~⅓ du travail en 4–6 semaines), battage nov-fév |
| Prairie | ~12 | ~75 | fenaison juin |
| Lin, chanvre | ~100 | ~625 | arrachage, rouissage |
| Vigne | 100–150 | 625–940 | toute l'année |
| Verger, forêt, élevage | *à calibrer* | | |

Le travail des champs n'occupe qu'**environ un tiers** de l'année d'un actif (~2,8 ha de labours par actif, ~7 ha par actif à plein temps) : le reste va à l'élevage, au bois, à la construction et surtout au **textile domestique d'hiver** (`02` §8.3 et §8.4). Dans le POC actuel, 83 % des pops travaillent la terre à plein temps.

### Cellule de marché 1750 (rayon ~12 km, ~450 km², ~21 000 hab., ~7 200 tiles)

| Niveau | Nb | Hab. unit. | Pops | Tiles bâties |
|---|---|---|---|---|
| Ville | 1 | 5 000 | 50 | 8–10 |
| Bourgs | 4 | 800 | 32 | 2 chacun |
| Villages | 15 | 400 | 60 | 2 chacun |
| Hameaux et fermes | ~45 | 150 | 67 | 1 chacun |

- Espacements : hameaux ~2 km, villages 3–4 km, bourgs 8–10 km, villes 25–30 km.
- Proportions ~1:4:15:45 (logique des lieux centraux de Christaller). C'est un **critère de succès mesurable** pour le maillage, jamais un ratio codé (principe n°6).
- Fermes isolées : surtout à l'Ouest (bocage). Les plaines du Nord ont des villages groupés avec champs ouverts. Un paramètre de dispersion du générateur permettra de choisir.

## Calibration

Toutes les valeurs vivent dans `sim/donnees/poc.toml`. Les terres sont calibrées sur la référence 1750 ci-dessus (~34 tiles de terroir par pop) : la production par tile est la ration d'une pop ÷ le nombre de tiles qu'il lui faut.

### Terres exploitées (par tile de 250 m, fertilité 1)

| Terre | Extrant par tile et par jour | Jours-personne par tile et par an | Emprise max | Défrichement (par tile) |
|---|---|---|---|---|
| Champ (grain) | 0,40 grain | 250 | 36 | 10 pop-jours, équipe de 2 |
| Pâturage (viande) | 0,42 viande | 80 | 30 | 5 pop-jours, 1 bois, équipe de 1 |
| Verger (fruit) | 1,0 fruit | 150 | 16 | 12 pop-jours, équipe de 2 |
| Exploitation forestière (bois) | 0,35 bois | 60 | 36 | 4 pop-jours, équipe de 2 |
| Champ de lin | 1,2 lin | 625 | 12 | 10 pop-jours, équipe de 2 |
| Élevage laine | 0,15 laine | 80 | 30 | 5 pop-jours, 1 bois, équipe de 1 |

Une pop fournit **12 500 jours-personne par an** (~50 travailleurs × 250 jours). Pour 20 pops (10 hameaux), les terres demandent ~37 000 jours par an, soit **un tiers du temps des pops** : le reste est du temps libre (textile domestique, chantiers).
Une **maison** coûte 150 pop-jours (équipe de 1) et 100 bois.

### Atelier domestique (tout hameau)

Sans poste : il prend le **temps libre du hameau**. 1 pop-jour par jour de travail requis à plein régime fait **4 vêtements par jour**, soit les besoins de 8 pops. L'atelier ne commande de fibre que pour vêtir les pops de son hameau.

| Étape | Intrants | Extrants (1 pop-jour) |
|---|---|---|
| Tissage | 8 lin + 4 laine | 6 tissu |
| Couture | 6 tissu | 4 vêtements |

### Fertilité, distance et extension
- **Fertilité** d'une tile : bruit lissé (cellule de ~6 km) entre 0,85 et 1,3, qui s'effondre autour du centre du village (pleine jusqu'à 12 tiles, puis déclin linéaire jusqu'à 0,15 à 28 tiles). La fertilité d'une terre est la moyenne de son emprise.
- **Distance maximale** : 100 (10 tiles-équivalents, 2,5 km) entre deux lieux (travail, approvisionnement). À calibrer.
- **Extension** : une terre naît à 4 tiles contiguës et s'agrandit par blocs de 4 jusqu'à son emprise maximale ; au-delà on en fonde une autre. Une maison s'ajoute à un hameau jusqu'à 4 ; hameau plein, ou plus de terre à portée, un hameau neuf se fonde en périphérie du réseau (route neuve de 8 tiles au plus) et des pops sans poste s'y installent. Coûts : `02` §7.
- Avec ces valeurs, un village de 4 hameaux (6 pops au départ) plafonne vers **8 à 15 pops** (800 à 1 500 hab.) en « dents de scie » en quelques siècles : tout le terroir à portée (rayon de 10 tiles) est occupé. C'est l'ordre de grandeur d'un village de 1750 ; aller plus loin demande plusieurs groupes de hameaux.

### Proportions qui en découlent
- Pour **10 hameaux** (20 pops) : ~250 tiles de champ, ~48 de pâturage, ~20 de verger, ~114 de forêt, ~17 de lin, ~67 de laine, soit ~516 tiles de terres pour 20 pops. Le générateur découpe ces tiles en terres de 36 tiles au plus.
- Le générateur démarre à 80 % de la capacité de logement. La croissance est d'abord limitée par le **logement**, puis par la **nourriture** quand la terre à portée est occupée.

### ⚠️ Points de calibration
- **Tranché** : bloc discret de 100 hab (`00` §3), tile de 250 m, une maison par pop, hameau de 2 à 4 pops, plafond de densité 10, recettes textiles, nourriture interchangeable, taux de croissance 1,5 % / 0,5 % / −8 %, distance = coût des arêtes avec limite, budget de jours (`jours_personne_par_pop_an`), extension par la communauté.
- **À calibrer** avec le CLI headless : seuils vitaux, `mortalite_famine`, `distance_max`, coûts de défrichement, jours par tile du verger, de la forêt et de l'élevage (seuls les céréales, la prairie et le lin viennent de la référence 1750), rayons de fertilité, plafond du hameau, taille de la croix de départ.
- **Plus tard** : POC-B (prix, salaires, seuils) ; aléa de récolte ; saisons ; rendements décroissants ; abandon des lieux en déclin.

### Besoins d'une pop (par jour)

| Ressource | Ration normale | Rôle |
|---|---|---|
| Grain | 5 | Nourriture (1 unité nutritive par unité) |
| Viande | 1 | Nourriture **et** part de viande attendue (confort) |
| Fruit | 1 | Nourriture |
| Bois | 2 | Chauffage (vital) |
| Vêtements | 0,5 | Minimum de confort |

| Besoin | Seuil | Effet |
|---|---|---|
| **Vital** : nourriture | 5 unités nutritives (n'importe quelle combinaison de grain, viande, fruit) | Sous le seuil : déclin |
| **Vital** : chauffage | 2 bois | Sous le seuil : déclin |
| **Confort** : viande | 1 | Sous le minimum : insatisfaite |
| **Confort** : vêtements | 0,5 | Sous le minimum : insatisfaite |

| État de la pop | Croissance annuelle |
|---|---|
| Vital et confort couverts (**satisfaite**) | **+1,5 %** |
| Vital couvert, confort insuffisant (**insatisfaite**) | **+0,5 %** (interpolé selon le confort) |
| Vital insuffisant (**manque**) | de +0,5 % à **−8 %** quand le vital tombe à zéro (linéaire) |

### Fertilité, distance et extension
- **Fertilité** d'une tile : bruit lissé entre 0,85 et 1,3, qui s'effondre autour du centre du village (pleine jusqu'à 3 tiles, puis déclin linéaire jusqu'à 0,15 à 7 tiles). Elle multiplie la production des terres exploitées.
- **Distance maximale** : 10 tiles chemin entre deux lieux (travail, approvisionnement). À calibrer.
- **Coûts de création** (travail en pop-jours, équipe, bois) : voir `02` §7.
- Avec ces valeurs, le village plafonne vers **60 à 90 pops** (6 000 à 9 000 hab.) en « dents de scie », après plusieurs siècles. Un terroir plus large plafonne plus haut (170 pops à 4/9, 240 pops à 5/11).

### Proportions qui en découlent
- Un village de **10 hameaux** (dont 3 textiles, 23 pops) demande 1 champ, 1 pâturage, 1 verger, 1 forêt, 1 champ de lin et 1 élevage de laine, soit **19 pops de travail + 3 à l'atelier**, pour 23 pops. Les bras ne sont pas en excès : c'est l'image d'une société agraire.
- Le générateur démarre à 80 % de la capacité de logement. La croissance est d'abord limitée par le **logement**, puis par la **nourriture** à mesure que l'on gagne sur des terres plus pauvres.

### ⚠️ Points de calibration
- **Tranché** : bloc discret de 100 hab (`00` §3), effectifs en pops, hameau 2 pops et hameau textile 3 pops (1 à l'atelier), plafond de densité 10, recettes textiles, nourriture interchangeable, taux de croissance 1,5 % / 0,5 % / −8 %, distance = tiles chemin avec limite, extension par la communauté.
- **À calibrer** avec le CLI headless : seuils vitaux, `mortalite_famine`, `distance_max`, coûts de création, fertilité (rayons du terroir), taille de village de départ.
- **Plus tard** : POC-B (prix, salaires, seuils) ; aléa de récolte ; saisons ; rendements décroissants par pop ; abandon des lieux en déclin.

## Générateur de village

- **Déterministe** (même graine, même village), progressif : un village de 1750 à évolution lente et production quasi stable, puis un deuxième village avec liens économiques, puis un troisième, puis une ville. Simple en POC, il gagnera en finesse à chaque étape.
- **Croix de chemins de terre** de 8 tiles (arêtes), compacte pour rester sous la distance limite. Les **hameaux** sont près du centre, de part et d'autre des bras ; les **terres** sont des blocs de tiles de 3 de large le long d'un bras, au plus près du centre, un bloc de chaque type d'abord pour que chaque ressource ait un producteur proche. La tile nœud de chaque terre est reliée au bras. Si une disposition viole les contraintes d'accès, une autre est tirée de façon déterministe.
- Dimensionné pour être **proche de l'équilibre** : le nombre de tiles de chaque type est déduit des besoins (jamais un ratio codé), pops initiales à 80 % de la capacité de logement, stocks initiaux non nuls, jauges de croissance décalées. Défaut : **4 hameaux** ; au-delà d'environ 6, la croix ne suffit plus.
- Champ de **fertilité** tiré avec la graine, centré sur le village.

## Outillage de test et d'observation

- Crate Rust `sim` **pure** (sans dépendance napi), **CLI headless** : N années en quelques secondes, séries temporelles en CSV.
- **Invariants automatisés** : pas de stock négatif, conservation de la matière (produit = consommé + Δstock + pertes), équilibre malthusien comme oracle (une terre moins productive abaisse la population d'équilibre), pas de dérive sur de longues durées, déterminisme.
- **Déterminisme** : quantités en entiers, itérations ordonnées.
- Paramètres (bâtiments, recettes, besoins) en **fichiers de données**, pas en dur.

## Ce que voit le joueur (POC) — implémenté dans `app/` (voir `app/README.md`)

Chaque indicateur est affiché avec son **facteur limitant** (le moteur enregistre quel `min()` a mordu : main-d'œuvre, intrant, stock plein).

| Niveau | Contenu |
|---|---|
| Hameau | Ses pops et leur lieu de travail, état de chaque pop (satisfaite / insatisfaite / en manque), couverture de chaque ration (reçu/requis, et auprès de quel fournisseur), jauge de croissance, atelier textile sur place |
| Terre exploitée | Production effective %, **fertilité de la tile**, facteur limitant, stocks, pops affectées et leurs hameaux, clients |
| Chantier | Ce qui est construit, travail restant, équipe présente, bois reçu, **motif** (« grain manque, 2 pops sans poste ») |
| Pop sans poste | Pastille distincte dans la liste du hameau |
| Modes de carte | Couverture des besoins, pénuries, utilisation des terres, **fertilité**, portée (tiles chemin) autour d'un lieu |
| Global | Date, vitesse, courbes (pop, production vs consommation), journal d'événements (naissance, disparition, bras réorientés vers…, **chantier lancé / terminé**) |
| Temps | Jour/nuit par teinte globale ; une pop = un sprite, au travail le jour, au logement la nuit |
| Debug | Choc de récolte (« mauvaise récolte »), modifier un paramètre en direct |

## Prochaine étape du POC (décidée le 2026-11-03)

**Jour/nuit et saisons**, ensemble, parce qu'ils se servent mutuellement :

| Sujet | Contenu |
|---|---|
| **Saisons** | Profils mensuels du travail (`02` §8.3, étape 2) : labours et semis en octobre-novembre et mars, moisson en juillet-août (~⅓ du travail des céréales en 4 à 6 semaines), battage de novembre à février, textile domestique l'hiver. Les pops se déplacent d'une terre à l'autre au fil de l'année. Stocks annuels (greniers), soudure, aléa de récolte (`05`) |
| **Jour/nuit** | Cycle visible et lisible : les pops rentrent le soir et partent le matin, teinte de la lumière selon l'heure |
| **Visuels associés** | **Ombres portées** et lumière en rotation : shader de ré-éclairage à partir de l'albedo et des normales (déjà générés pour les maisons, `03` §7), ombres des maisons, des arbres et des clôtures qui tournent avec le soleil ; ambiance des saisons (couleur des champs, neige, feuillage) |

Jusqu'ici les assets sont éclairés au zénith, sans ombre, et la production est continue : les pops restent donc des mois sur la même terre (ce qui est voulu, voir `02` §8.3). Les saisons donnent des raisons de bouger, le jour/nuit un rythme à l'écran.

Reste ensuite : **déclin et abandon** des lieux, **plusieurs groupes de hameaux** (`05`, question 23), puis POC-B : **placement des bâtiments par des firmes**, généralisation de l'extension communautaire.
