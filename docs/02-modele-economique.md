# 02 — Modèle de simulation économique

## 1. La tile

- **Usage unique** par tile : terrain naturel, bâtiment de production, quartier d'habitation. L'**infrastructure n'occupe pas de tile** : c'est une couche d'arêtes posée par-dessus (§9).
- La **tile d'habitat contient ses rues locales**, comprises dans sa densité (15 à 25 % de voirie, voir §9.3). Les voies majeures (route royale, rail, autoroute) sont des arêtes qui la traversent. *(Le code POC-A embarque encore un atelier textile dans le « hameau textile » et des tiles de chemin orthogonales ; la cible supprime l'un et l'autre, voir §8 et §9.)*
- **Surface réelle fixe : 1 tile = 250 m × 250 m = 6,25 ha** (16 tiles/km²). Décision du 2026-11-02, qui remplace l'ancienne « unité comptable à surface variable ». Ce qui varie selon le type, c'est l'**emprise** (nombre de tiles) et la **densité** (pops par tile), pas la surface de la tile.
  - **Distances réelles** : `distance_max` = 10 tiles = 2,5 km ≈ 30–50 min à pied, le rayon de travail réaliste d'un habitat en 1750. Un village de ~2,5 km de rayon (~20 km² à ~50 hab/km²) plafonne naturellement vers 600–1 000 hab. : au-delà, un nouveau hameau se justifie (voir §8.5).
  - **Densités** : hameau ~30 hab/ha (200 hab = 1 tile) ; bourg ~60 hab/ha ; ville 100–200 hab/ha ; **plafond de 10 pops par tile = 160 hab/ha**, réaliste pour les villes de 1750 à 1950.

| Type | Emprise | Population |
|---|---|---|
| Terre exploitée (champ, pré, verger, forêt, lin, élevage…) | **N tiles contiguës**, **un seul bâtiment actif** (un seul nœud) ; extensible tile par tile | — |
| Hameau (POC) | 1 tile | capacité 2 pops = 200 hab |
| Village, bourg (futur) | 2–4 tiles | 4–15 pops |
| Quartier urbain (futur) | 1 tile par quartier | jusqu'à 10 pops par tile |
| Terrain non exploité (landes, friches, bois, eaux) | tile passive | — |

- **Une tile d'habitat est une capacité de logement**, occupée par des **pops de 100 habitants** (voir `00`). Ex. : Rouen ≈ 50 000 hab ≈ 500 pops ≈ 50–80 tiles. Il n'y a plus de « bassin de peuplement » agrégé en une tile : un village est composé de plusieurs tiles.
- La capacité varie avec le type de bâtiment et la densité ; elle n'est pas un nombre fixe d'habitants. **Plafond de densité : 10 pops par tile pour le bâti bas** (~160 hab/ha). Des immeubles pourront le relever (§9.4).
- **Fertilité** : le terrain reste unique (plaine), mais chaque tile porte une `fertilite` (ppm de 1,0) qui multiplie la production des terres exploitées. Elle est tirée par le générateur (bruit lissé, appauvri avec la distance au centre). C'est ce qui fait de la terre, même « toute exploitable », un facteur limitant : on défriche d'abord la meilleure terre accessible, puis de moins en moins productive.
- `valeur_fonciere` existe dès le départ mais n'est pas utilisée dans le POC (futur moteur de la ségrégation spatiale).

### Le ratio habitat / production : indicateur émergent

Avec la tile fixe de 250 m, la campagne de 1750 compte **~1 tile d'habitat pour ~60 tiles de terroir** (un hameau isolé : 1 tile bâtie pour ~64 tiles de terroir, dont ~27 de labours, landes et bois inclus). Les ratios des stades suivants (bourg proto-industriel, ville industrielle, métropole) sont **à recalculer** avec cette tile : le ratio s'inverse avec l'urbanisation.

**Ne jamais le coder.** Il doit émerger de la calibration (production extensive à la campagne, productivité par tile qui explose avec `niveau_tech`). En économie de subsistance, le hameau fait aussi son textile (travail domestique) : le ratio se mesure entre hameaux et terres exploitées, et c'est l'apparition d'ateliers puis de villes qui le fera basculer. Les proportions 1750 de référence sont dans `04`.

## 2. Les acteurs économiques

### Firme
- Possède une ou plusieurs tiles, emploie, achète, vend, dégage profit ou perte, réinvestit.
- **Mono-secteur** : une firme ne gère qu'un type de production.
- Statut : privée, publique, coopérative…
- Règles (Option A — firmes autonomes à règles simples), actives à partir du **POC-B** (monnaie, voir `04`) ; en POC-A les firmes produisent et livrent sans profit :

```
profit  > seuil        → réinvestir (construire un nouveau bâtiment)
capital > seuil        → racheter une firme vulnérable (peu rentable)
perte prolongée        → faillite, tile abandonnée
```

### Groupe
- Possède plusieurs firmes (c'est ainsi qu'un grand groupe internalise transport, approvisionnement, etc.).
- Dans le POC : structure présente, **pas de logique propre**.

### Pop
- Unité de population : **100 habitants, fixe**. Attributs discrets : tile de résidence, tile d'emploi (nullable), strate, culture, religion, orientation (voir `00`).
- Dans le POC : une seule strate (paysan), une seule culture et religion, attributs fixes.
- **Bloc discret** (décision, voir `00` §3) : une pop a une seule résidence et un seul emploi ; la main-d'œuvre d'un bâtiment est la somme des pops affectées (effectifs exprimés en pops). Résidence ≠ travail : consommation au quartier, production au lieu d'emploi.
- **Consommation** : chaque jour, une pop consomme sur le stock de son hameau (5 grain, 1 viande, 1 fruit, 2 bois, 0,5 vêtement, voir `04`). **Pas de monnaie côté pops dans le POC-A** : elles consomment ce que leur hameau reçoit. **Le producteur mange d'abord** : le fournisseur le plus proche d'un hameau textile est son propre atelier (distance 0).

### Personnages (lien avec la vision)
Les firmes et institutions sont dirigées par des personnages, créés par des **postes** (voir `00`). Dans le POC : **un seul personnage, le maire**, coquille vide **inactive** (présent dans les données, aucune décision ; il ne construit pas).

## 3. Flux unifié

Tout circule dans le même modèle : marchandises **et main-d'œuvre**.

- Un quartier **produit** de la main-d'œuvre et **consomme** des biens.
- Un bâtiment de production **consomme** main-d'œuvre + intrants, **produit** des extrants.
- **Distance** entre deux lieux = **longueur du plus court chemin le long des arêtes**, en unités de coût : 10 par côté, 14 par diagonale, 0 sur une même tile (cible, voir §9 ; le code POC-A compte encore des tiles de chemin). Un lieu est **desservi** si une arête passe par sa tile. **Aucune pénalité selon la distance** : on s'approvisionne simplement au plus proche, et l'on ne va ni travailler ni s'approvisionner **au-delà de la limite** (`distance_max` = 100 unités, soit 10 tiles-équivalents ou 2,5 km, à calibrer).
- Un quartier met sa population à disposition dans son périmètre ; une usine met ses ressources à disposition dans son périmètre.
- Les **pops** sont l'unité de ce flux : chaque pop est affectée à un seul lieu de travail (résolution par le marché journalier ci-dessous).

### Lien économique
Créé entre deux tiles quand :
1. l'une produit ce que l'autre consomme ;
2. elles sont reliées par une infrastructure continue ;
3. la distance est inférieure ou égale à `distance_max`.

Sélection du fournisseur : **appel d'offres local** — le client choisit, à portée (`distance_max`), le fournisseur au **coût livré** le plus bas. Les chemins sont **précalculés** à la création du lien et recalculés uniquement si le réseau change.

**POC-A (sans prix)** : coût livré = distance seule, donc le fournisseur le plus proche qui a du stock. **En pénurie, rationnement au pro rata des demandes** (règle simple, sans biais d'ordre de traitement).

### Transport (POC)
- **Implicite** : personne ne « réalise » le transport ; son coût est calculé et absorbé dans le prix de transaction.
- Préparé pour l'extension : `operateur_id` nullable sur le lien (`null` = transport implicite ; plus tard une firme de transport, éventuellement incluse au contrat).
- Pas d'intermédiaires (marchés, négociants) dans le POC : relation **directe** fournisseur ↔ client.

### Contrats
- **Spot** : achat ponctuel, prix du moment, pas de garantie (`contrat_id = null` par défaut).
- **Cadre** : volume régulier, prix négocié, priorité d'accès au stock.
- Champ `qualite` prévu (différenciation avant standardisation) — non exploité dans le POC.
- Mécanisme de formation des prix : **à approfondir** (voir `05`).

### Main-d'œuvre

**POC-A : budget de jours, entraide du hameau, sans marché** *(implémenté, voir §8.3 ; à remplacer par §10.4 : un lieu par pop et par jour).* Une pop n'a plus de poste : chaque jour, le temps des pops de chaque hameau (1 pop = 1 pop-jour, hors équipes de chantier) est réparti entre les terres à portée (`distance_max`), au prorata du temps disponible dans chaque hameau, les terres dont la ressource est en **tension** (ration couverte à moins de 97 %) d'abord. Une terre demande `N tiles × jours par tile et par an ÷ 12 500` pop-jours par jour ; si son stock de sortie est presque plein, elle ne demande que de quoi le remplir. Ce qui reste est le **temps libre** du hameau, que l'atelier domestique utilise. L'« emploi » d'une pop n'est qu'un affichage : sa terre principale, ou « sans poste » si elle est presque inoccupée.

| Événement | Règle |
|---|---|
| Départ | Le générateur ne fixe aucun emploi ; le travail se répartit dès le premier jour |
| Naissance | La nouvelle pop se loge dans son hameau, ou le plus proche qui a de la place (§6) ; son temps rejoint le budget du hameau |
| Disparition d'une pop | Son temps quitte le budget du hameau |
| **Chaque jour** | Équipes de chantier (poste fixe, trajet complet), puis répartition du temps entre les terres, puis temps libre |
| **Chantier** | Une équipe de pops non engagées les plus proches, au plus la moitié d'un hameau (les terres ont besoin de bras) ; pour fonder un hameau, des pops viennent de plus loin et s'installent sur le site (§7) |
| Accès | Le trajet domicile-travail charge les arêtes en proportion du temps donné. Si la congestion empêche d'arriver (en sommeil dans le POC), le temps correspondant est perdu |

Boucle qui se corrige seule : la tension sur une ressource fait aller le temps vers les terres qui la produisent ; si le temps ne suffit pas ou si la terre manque, l'extension prend le relais (§7). Le **travail saisonnier** (profils mensuels) reste hors POC.

**POC-B et suivants : modèle « journalier »** (mobilité, salaires)
- Historiquement dominant jusqu'au tournant du XXᵉ siècle ; aucun contrat durable côté travailleur.
- Chaque jour, une firme publie une demande (volume, taux journalier offert) ; les quartiers accessibles répondent selon disponibilité et coût → **mini-marché spot quotidien**.
- La main-d'œuvre **ne se stocke pas** : ce qui n'est pas employé reste au quartier (chômage latent).
- Une firme en expansion doit **payer plus** (attirer des quartiers lointains) ou **rester limitée** (quartiers proches épuisés, segments saturés) → pression émergente vers cités ouvrières, tramways, délocalisation.
- Extensions futures en **surcouche** sans refactorisation : contrats hebdo/mensuels prioritaires, licenciement, syndicats (`salaire_plancher` sur une zone).

## 4. Infrastructure et congestion

- Chaque segment porte un **volume journalier agrégé** (marchandises + main-d'œuvre).
- La sollicitation est imputée au **tick du transfert**.
- Modèle à deux couches :

```
charge_segment        = Σ volumes transférés via ce segment ce jour
facteur_congestion    = f(charge_segment / capacite_nominale)
cout_transport_effectif = cout_base × (1 + α × facteur_congestion²)
```

| Bande de charge | Effet |
|---|---|
| `charge ≤ capacite_nominale` | Transfert immédiat, coût standard |
| `capacite_nominale < charge ≤ capacite_max` | Transfert immédiat, **surcoût** congestion |
| `charge > capacite_max` | **Livraison différée** à J+1 (`file_attente` du segment) + surcoût |

- Les contrats sont traités **par priorité** (valeur, ancienneté, type) : les premiers passent au coût normal, **le surcoût ne s'applique qu'aux éléments en excès / différés**.
- Courbe **quadratique** : faible pénalité sous le seuil, forte au-delà → effets de bascule.
- La livraison différée est **dans le POC** : elle crée des ruptures de stock en cascade.
- À éviter : routes distinctes par direction, files par entité individuelle, saturation binaire.
- La congestion est un **signal d'investissement** (amélioration de l'infrastructure).

## 5. Temps et séquence du tick

### Horloge
- Horloge de base au **quart d'heure** ; vitesse nominale **1 s réelle = 15 min de jeu**, accélérable jusqu'à une journée en quelques secondes.
- **Résolution économique : la journée.** Pas de transferts horaires, pas de pointe matin/soir : ce qui compte à l'échelle 1750–1950 est la capacité journalière d'une route.

### Fréquences de mise à jour

| Événement | Fréquence |
|---|---|
| Contrats, main-d'œuvre, production, stocks, congestion | Quotidienne |
| Prix (offre / demande) — POC-B | Hebdomadaire |
| Décisions des firmes (investir, racheter, contrats) — POC-B | Mensuelle |
| Construction de nouveaux bâtiments | Saisonnière |
| Démographie des quartiers | Annuelle |

### Séquence quotidienne (option B : recruter avant de produire)

```
J ouverture
  1. Livraison des différés de J-1 (file_attente des segments)
  2. Résolution des contrats d'approvisionnement (marchandises)
  3. Flux de main-d'œuvre (POC-A : trajets des pops vers leur poste ; POC-B : marché journalier)
  4. Calcul de la production effective
  5. Mise à jour des stocks et bilans journaliers
  6. [fin de mois] Décisions stratégiques des firmes
J clôture
```

Les étapes 2 et 3 sont résolues sur le même tick ; la production est calculée après les deux, contraintes combinées **multiplicativement** :

```
production_effective = production_max
  × min(1, main_oeuvre_obtenue / main_oeuvre_requise)
  × min(1, stock_input / input_requis)
```

### Exemple de cinématique
- **Jour 1** : l'usine A a tous ses travailleurs → en fin de journée, +10 unités dans son stock.
- **Jour 2** : l'usine B achète 5 unités à A (transfert imputé au segment ce jour-là), consomme les 5, produit 15 unités de B.

### Stocks
Chaque tile a un **plafond de stock** (`capacite_max`) : une production qui ne trouve plus de place s'arrête (sinon dérive). Fruit et viande se **périment** (perte journalière simple). Le village démarre avec des **stocks initiaux** non nuls.

## 6. Besoins, satisfaction et démographie (POC)

### Couverture des besoins (chaque jour, par hameau)
> ⚠️ Le stock devient **personnel** et la couverture se calcule **par pop** (§10.4, non implémenté).
Chaque pop tire sa ration normale sur le stock de son hameau (`04`). On compare ce qu'elle a **reçu** aux besoins :

```
nutrition  = Σ reçu_r × valeur_nutritive_r        (grain, viande, fruit interchangeables)
vital      = min( nutrition / 5 ,  reçu_bois / 2 )          plafonné à 1
confort    = min( reçu_viande / 1 ,  reçu_vêtements / 0,5 )  plafonné à 1
```
- **Vital** : nourriture (n'importe quelle combinaison) et chauffage. Le manque est une menace de déclin.
- **Confort** : part de viande attendue et minimum de vêtements. Au-dessous : **insatisfaite** ; au-dessus : **satisfaite**.
- La couverture est calculée par hameau : les pops d'un même hameau reçoivent la même part.
- **Pas de compensation** entre vital et confort : un hameau qui a tout sauf le bois reste en manque.

### Croissance : une jauge par pop
Taux annuel de la pop, selon l'état de son hameau :

```
si vital = 1 :  taux = 0,5 % + (1,5 % − 0,5 %) × confort          # insatisfaite → satisfaite
sinon        :  taux = 0,5 % − (0,5 % + 8 %) × (1 − vital)         # jusqu'à −8 % si vital = 0
jauge_pop   += 100 hab × taux / 365                               # chaque jour
≥ +100 hab : la pop se duplique (hameau d'abord, sinon le plus proche avec de la place ; sinon la jauge attend, plafonnée)
≤ −100 hab : la pop disparaît
```
- Le taux net est multiplié par `acceleration` (observation du POC) ; les valeurs de départ sont dans `sim/donnees/poc.toml`.
- Une jauge plutôt qu'un dé quotidien : même espérance mathématique, mais déterministe, rejouable et explicable. Les jauges initiales sont décalées pour éviter des duplications synchrones.
- La population est bornée par la capacité de logement. Quand tous les logements à portée sont pleins, la jauge attend, plafonnée, et **fait bâtir un hameau** (voir §7).
- La main-d'œuvre se compte **en pops** : une pop employée fournit une unité, une pop sans poste ne produit pas. Âge et condition individuelle ne sont pas modélisés (équivalence 1 pop ≈ 50 travailleurs réels : calibration seulement, voir `00` §3).
- Hors POC : mobilité, promotion de classe (par le confort), bascule des attributs (voir `00`).

État d'un hameau (agrégat des pops de la tile) :
```
pops, capacité, vital_jour, confort_jour, couverture par ressource, vital_annuel, confort_annuel
```

## 7. Extension du village par ses habitants (POC-A)

Aucune firme, aucune monnaie : la **communauté** étend le village, à rythme **saisonnier** (tous les 90 jours). Les décisions se prennent là où il y a un manque.

### Quand
| Situation | Réaction |
|---|---|
| **Des pops attendent un logement** (jauge pleine) | Une **maison** de plus dans leur hameau, tant qu'il n'a pas atteint 4 maisons ; sinon un **hameau** neuf (une seule fondation à la fois) |
| **Une ressource est en tension** (ration < 97 % dans un hameau) et des bras restent sans poste | Défricher pour elle, près des hameaux qui en manquent d'abord : **agrandir** une terre du même type de 4 tiles (la plus fertile, la plus proche), sinon **fonder** une terre de 4 tiles |
| Plus de terre libre à portée des hameaux | Un **hameau** se fonde en périphérie du réseau (à portée de tous les producteurs) et des pops sans poste s'y installent |
| Les producteurs de la ressource sont **bloqués par un intrant manquant** | On ne les multiplie pas : la tension est reportée sur l'intrant |
| Un chantier est déjà en cours pour cette ressource | On attend |

Au plus `chantiers_max` chantiers en même temps.

### Où
Un site de terre est **libre** (tile vide, sans arête), à **moins de `distance_max`** du hameau demandeur, et **desservi** : sa tile « nœud » est reliée au réseau par une **route neuve** qui contourne les terres et les chantiers (traversée interdite, §10.1) et peut traverser les tiles d'habitat et dont le coût entre dans la limite de distance. Un agrandissement prend des tiles libres voisines (par un côté) de la terre, les plus fertiles d'abord, sans route. Un site de hameau doit trouver à portée de chaque ration (hors ce que son atelier produit lui-même), de la fibre pour cet atelier et du bois de construction ; il est le plus proche du hameau demandeur, sur la terre la plus pauvre, avec au plus 8 tiles de route neuve.

### Coût
Un chantier est un lieu de travail temporaire : une **équipe** de pops y fournit un nombre de **pop-jours**, et le **matériau** (bois) arrive par le réseau, consommé au prorata du travail. Sans bois, le chantier n'avance pas.

| Création | Travail (pop-jours) | Équipe | Bois | Durée minimale |
|---|---|---|---|---|
| Défricher une tile de champ ou de lin | 10 | 2 | 0 | 5 j par tile |
| … de pâturage ou d'élevage | 5 | 1 | 1 | 5 j par tile |
| … de verger | 12 | 2 | 0 | 6 j par tile |
| … de forêt | 4 | 2 | 0 | 2 j par tile |
| **Une maison** (fonder un hameau en bâtit la première) | 150 | 1 | 100 | 150 j |

(Coûts par tile, multipliés par le nombre de tiles du chantier, 4 en général ; valeurs de départ à calibrer.)

### Équipe
Les pops non engagées les plus proches, au plus la moitié des pops d'un hameau. Un chantier incomplet recrute à nouveau à chaque saison. Pour une fondation de hameau, les pops viennent de plus loin que `distance_max` et s'installent sur le site.

### Le frein : la terre à portée
Rien n'interdit de défricher, mais le terroir à portée (rayon de 10 tiles autour des hameaux) se remplit, et les terres lointaines sont plus pauvres : quand une pop de plus ne produit plus de quoi se nourrir (et se chauffer), la population plafonne ; seule la fondation d'un hameau plus loin repousse ce plafond. La taille d'équilibre **émerge** de la fertilité, de la distance et du temps de travail ; elle oscille en « dents de scie » autour de ce plafond. Le confort (viande, vêtements) **ne freine pas** la croissance (une pop insatisfaite croît à +0,5 %/an).

### Journal
Chaque lancement et chaque fin de chantier est inscrit au journal d'événements, avec son **motif**.

### Limites connues
Pas d'abandon : un hameau vidé par un déclin reste debout, une terre sans bras reste inactive. Pas de réaffectation de l'équipe en cas de besoin urgent ailleurs. Les coûts de défrichement du verger, de la forêt et de l'élevage ne viennent pas de la référence 1750.

## 8. Évolutions de design décidées (2026-11-02, implémentées dans `sim/`)

Décisions prises après l'étude des ratios de 1750 (référence chiffrée dans `04`), implémentées dans `sim/` et `app/` : tile de 250 m, terres de N tiles, budget de jours, textile domestique.

### 8.1 Tile fixe de 250 m
Voir §1. Conséquence : le POC actuel (60–90 pops dans ~7 tiles de rayon) suppose des tiles de ~0,7–1 km ; à 250 m, le même village s'étale sur ~25 tiles de rayon et la limite de `distance_max` (10 tiles) devient le moteur d'extension.

### 8.2 Une terre exploitée = un bâtiment de N tiles
- Un champ, un pré, une forêt… est **un seul bâtiment actif** dont l'emprise est de N tiles contiguës. La production et le besoin en travail sont proportionnels à N (et à la fertilité de chaque tile).
- Le défrichement **ajoute des tiles contiguës** à la terre (ou en crée une nouvelle).
- Intérêt : réalisme spatial sans multiplier les nœuds (~1 nœud pour 5–6 tiles). Les parcelles contiguës ouvrent plus tard la voie aux enclosures et à la propriété (cadastre = ensemble de tiles).

### 8.3 Le travail se compte en jours
> ⚠️ **Partiellement remplacé par §10.4** (décision du 2026-11-03, non implémentée) : une pop travaille en un seul lieu par jour, avec un stock personnel. Le besoin des lieux en pop-jours reste valable.
- Chaque terre a un **besoin en jours-travail par an** (jours/ha × 6,25 ha par tile). Ordre de grandeur 1750 : céréales ~250 jours-personne par tile et par an (voir `04`).
- **Budget de jours par hameau** : capacité = pops × `jours_par_pop_an` (~12 500 = 50 travailleurs × 250 jours ; **exception à la règle de `00` §3**, à acter, voir `05`). Allocation par priorité : **vital** (nourriture, chauffage), puis confort, puis ce qui reste. Pas de « raisonnement » individuel des pops : un budget par hameau, déterministe.
- **Étape 1 (POC-A)** : budget **annuel lissé**, sans saisons. Les jours non pris par les champs vont au **travail domestique** (§8.4).
- **Étape 2 (plus tard)** : **profils mensuels** (moisson juillet-août ≈ ⅓ du travail des céréales en 4–6 semaines ; battage nov-fév ; labours et semis oct-nov et mars), c'est-à-dire les saisons (`05`).

### 8.4 Suppression du hameau textile
- En 1750, filage, tissage et couture sont des **travaux domestiques d'hiver** (proto-industrie). Le « hameau textile » et son poste d'atelier sont **supprimés** : il n'y a plus qu'un type de hameau (2 pops), qui transforme lin et laine avec ses jours libres.
- Les recettes textiles (`04`) sont conservées ; leur coût passe en jours-pop (à recalibrer).
- Intérêt historique : ces jours libres sont ce que le marchand-fabricant achètera en POC-B (putting-out), première filière où le capitalisme émerge.

### 8.5 Pourquoi un hameau, pourquoi un centre
- **Hameau** : on s'installe au plus près de la terre travaillée. Au-delà de ~2,5 km de marche (`distance_max`), la terre est trop loin : on bâtit un nouvel habitat (règle de §7, aucune règle supplémentaire).
- **Polyculture** : en 1750, le hameau est polyculteur (assolement triennal, élevage, jardin, chanvre) ; chaque parcelle est mono-culture et le hameau est le client direct de ses parcelles. La **spécialisation régionale** (vigne, lin, herbages, céréales) apparaît avec le marché et le coût du transport (anneaux de Von Thünen) : elle **émerge en POC-B**, elle n'est pas codée.
- **Centres (bourg, ville)** : ils naissent des besoins que le hameau ne produit pas (sel, fer et outils, meule, four, tuiles…) et du marché. Premier candidat POC-B : 1 ou 2 biens importés produits seulement au bourg.

## 9. Infrastructure en arêtes (décision du 2026-11-02, à implémenter)

Maquette de référence : `docs/maquettes/routes-250m.html` (à ouvrir dans un navigateur : avant/après du réseau, puis emprise d'une voie sur une tile).

### 9.1 Le réseau
- L'infrastructure est une **couche d'arêtes** reliant les centres de deux tiles voisines, dans **8 directions**. La tile garde son usage (champ, hameau…) : la comptabilité de la tile ne change plus (à l'écran, un chemin ne traverse pas un champ : §10.1), et une piste de 5 m ne consomme plus 6,25 ha.
- **Coûts entiers** (déterministes) : **10** pour un côté, **14** pour une diagonale. L'erreur reste ≤ 8 % ; sans diagonales, un trajet à 45° est surévalué de 41 % (maquette : +16 à +41 % sur les liaisons). `distance_max` = 100 unités.
- **Dijkstra à coûts entiers**, toujours borné, remplace la recherche en largeur à coût 1 de `sim/src/reseau.rs`. Le terrain module le coût (forêt ×3 dans la maquette ; à calibrer avec le relief).
- Un lieu est **desservi** si une arête passe par sa tile. Plus de règle « accolé à un chemin » ni de « bout d'antenne d'une tile ».
- Une tile porte au plus 4 demi-arêtes actives (`03` §6). Deux diagonales qui se croisent dans un carré 2×2 sans nœud commun : interdites ou jonction (à trancher à l'implémentation).

### 9.2 Emprise d'une voie dans une tile
`emprise = largeur × longueur de l'arête dans la tile` ; la tile fait 62 500 m². Longueur : 250 m (traversante droite), 354 m (diagonale), 125 m (demi-arête, cul-de-sac).

| Voie | Largeur | Traversante | Diagonale | Cul-de-sac |
|---|---|---|---|---|
| Chemin de terre | 5 m | 1 250 m² (2,0 %) | 1 768 m² (2,8 %) | 625 m² (1,0 %) |
| Route royale | 12 m | 3 000 m² (4,8 %) | 4 243 m² (6,8 %) | 1 500 m² (2,4 %) |
| Voie ferrée | 15 m | 3 750 m² (6,0 %) | 5 303 m² (8,5 %) | 1 875 m² (3,0 %) |
| Autoroute (1950) | 40 m | 10 000 m² (16 %) | 14 142 m² (22,6 %) | 5 000 m² (8 %) |

Largeurs d'ordre de grandeur, à vérifier.

### 9.3 Effet sur le bâti
- **Les rues locales sont déjà dans la densité** : un chemin de terre ne retire rien. Seules les voies majeures retirent de la place.
- **Le plafond ne dépend pas du type de voie par une règle** : il ressort du nombre de maisons qui tiennent dans la surface restante (maison de 9 m, parcelle ~300 m², écart minimal 14 m ; un ménage = 5 hab., une pop = 20 ménages). Maquette, route d'abord, traversante, quartier dense de 10 pops : chemin et route royale 0 %, rail −1 %, autoroute −11 %. Un hameau ne perd rien.
- **L'ordre d'arrivée décide qui s'adapte**, avec la même règle. *Route d'abord* : les maisons se placent autour et le plafond de la tile baisse. *Maisons d'abord* : poser la route a un prix, les maisons sur le tracé sont à reloger (autoroute en diagonale sur un quartier dense : 54 maisons, soit ~2,7 pops).
- **Le coût de pose inclut les pops à reloger** : les routes neuves **contournent** donc les tiles bâties (contournements de bourgs), sans règle dédiée.
- **Pops à reloger** *(défaut proposé, à confirmer)* : même mécanisme que les pops en attente de logement (§6 et §7) : elles se logent à portée dans une tile qui a de la place, sinon la jauge attend et fait bâtir un hameau. Elles ne disparaissent pas.
- **Traversée d'un village** *(défaut proposé)* : un simple coût, pas de veto. Un veto des habitants relève de la politique (reporté).
- **Reporté** : coupure (une autoroute sépare un village en deux), bruit, échangeurs. L'emprise suffit pour le POC.

### 9.4 Pistes à approfondir (non tranchées)
- **Une maison par pop** *(ligne directrice de représentation, décidée le 2026-11-02)* : une tile d'habitat montre autant de maisons que de pops (1 à 10 pour le bâti bas). Une maison représente **100 habitants**, pas un ménage ; son sprite est exagéré, comme la largeur des routes (`03` §6). Conséquences :
  1. la capacité d'une tile = le nombre de maisons possibles le long de ses arêtes, donc elle se crée **maison par maison** (chantier d'une maison de 1 pop) plutôt que par hameau de 2 pops ;
  2. la perte de capacité due à une voie majeure se calcule au niveau des pops : `plafond × (1 − part de la tile prise par l'emprise)`. Pour une autoroute traversante, cela donne −16 %, contre −11 % dans la maquette (calcul par ménages de 9 m) : à calibrer ;
  3. au-delà de 10 pops, c'est le **type de bâtiment** (immeuble) qui change, pas le nombre de sprites ;
  4. chaque maison est orientée vers l'arête qui la dessert ; une pop à reloger fait disparaître une maison, une pop qui naît en fait apparaître une.
  La maquette `docs/maquettes/routes-250m.html` dessine des ménages (9 m) pour montrer l'emprise physique : ce n'est pas le rendu du jeu.
- **Matériau de construction** : le **bois** seul pour l'instant (la maison consomme du bois au chantier, §7). Champ `materiau` nullable sur le bâtiment pour la suite (torchis, pierre, brique), qui conditionnera la densification (immeubles en pierre ou en brique). Historiquement, la France de 1750 construit surtout en pierre, en torchis et en pans de bois avec un toit de chaume : le sprite « bois » aura l'allure d'une maison à pans de bois et toit de chaume.
- **Tile d'habitat desservie par une route ou une demi-route** : toute tile d'habitat est traversée par une arête ou porte une demi-arête, et les maisons se placent le long (façade), au fur et à mesure. Une demi-arête jamais prolongée reste une **impasse**. Capacité = f(longueur de façade, niveau de densification). Un hameau de 1 pop se contente d'une demi-route ; au-delà, la tile devient traversante. Une ferme isolée au bord du chemin ne pose pas de difficulté jusqu'à une certaine quantité de pops, puis il faut prolonger l'arête.
- **Densification** : le bâti s'améliore (maison isolée, mitoyenne, petit immeuble, immeuble). Le plafond de 10 pops est celui du bâti bas ; des immeubles le relèvent (ordre de grandeur : ~400 hab/ha, soit ~25 pops par tile). Le plafond deviendrait une propriété du type de bâtiment, pas une constante.
- **Rues rectilignes en ville** : en ville les rues sont planifiées (lotissement) : droites, avec des immeubles d'angle. À la campagne, les chemins sont sinueux et contournent le bâti existant (parfois autour de petits bâtiments). Le **décalage organique du rendu décroît avec le niveau d'urbanisation** (0 pour une trame planifiée). Une trame urbaine est un ensemble de rues parallèles dans la tile (voirie native), pas des arêtes distinctes tant qu'elles sont locales.
- **Rendu des routes** : courbe lissée (Catmull-Rom vers Bézier) passant par les centres de tiles, avec décalage déterministe (hash de la position et de la graine, ±0,34 tile au plus) ; largeur exagérée à l'écran. L'atlas de 15 tiles de chemin est abandonné pour le réseau (`03` §6).

## 10. Décisions de design du 2026-11-03 (§10.1 implémenté, le reste à implémenter)

Issues d'une discussion sur le rendu des routes, la forme des terres et la vie des pops. Elles **précisent ou remplacent** : §7 « Où » (route qui traverse des tiles occupées), §9.1 (« un chemin ne coupe plus un champ de N tiles » : vrai pour la comptabilité, faux à l'écran), §8.3 (budget de jours réparti) et §6 (couverture par hameau).

### 10.1 Traversabilité et tile nœud
- **Tile nœud = entrée du bâtiment.** C'est la seule tile d'une emprise sur laquelle le réseau se raccorde (c'est déjà `Batiment.tuile`). Elle est placée **en bordure de l'emprise**, côté route, pour que la route n'entre jamais dans la terre. Agrandir l'emprise ne la déplace pas.
- **Traversabilité par type de tile**, en donnée (`poc.toml`) : `libre`, `couteuse` ou `interdite`.

| Type | Traversabilité (POC) | Raison |
|---|---|---|
| Habitat (maisons) | libre | les rues locales sont dans la densité (§1) |
| Terre exploitée (champ, pré, verger, forêt, lin, élevage) | **interdite** | une route couperait la parcelle : lisibilité du joueur et calculs |
| Chantier | interdite | |
| Usine, atelier (plus tard) | interdite | on s'y raccorde par la tile nœud (la porte), cour en cul-de-sac |

- **Conséquences.** Le pathfinding exclut les tiles interdites, sauf comme destination (tile nœud). Les routes **contournent** les terres. Une terre qu'aucune route ne peut desservir n'est pas défrichée. Le passage à `couteuse` reste possible en donnée si le contournement devient trop contraignant.
- **Écarté : la parcelle coupée en deux** (calculs et visuels multipliés, illisible pour le joueur).
- **Écarté pour le POC : routes sur les bords de tiles.** Une diagonale entre sommets coupe toujours une tile en deux triangles ; il faudrait des routes orthogonales lissées au rendu, des distances en norme Manhattan (jusqu'à +41 %), et décider à quelle tile appartient une route de frontière (§9.2). Le gain est déjà obtenu par la tile nœud et la traversabilité. À reprendre si besoin (voir `05`, question 20).
- **Implémenté (2026-11-03)** : champ `traversable` par type de bâtiment (`donnees.rs`, `interdite` par défaut), `Simulation::traversabilite` et exclusion dans `meilleur_site` (`couteuse` = surcoût de 30). Le nœud est en bordure par construction : il est la tile d'arrivée de la route neuve, dont les tiles ne sont jamais prises par une terre (une tile à arête n'est pas « libre »). Test : `les_routes_contournent_les_terres`. Le raccord à la tile nœud existait déjà ; reste le rendu côté `app/` à valider visuellement.

### 10.2 Extension compacte des terres
`Simulation::agrandir` choisit aujourd'hui, de proche en proche, la tile libre voisine **la plus fertile** : un filon de fertilité donne un champ en serpent. Remplacer par un **score de placement** par tile candidate, avec des poids en donnée :

| Terme | Sens physique |
|---|---|
| Fertilité de la tile | rendement (inchangé) |
| Voisins déjà dans l'emprise (4-voisinage) | minimise le **périmètre** : clôtures, haies, trajets sur la parcelle |
| Distance au hameau demandeur | coût du trajet de travail |

La compacité **émerge d'un coût réel**, elle n'est pas scriptée (principe 6). Garde-fous :
- **Terroir par hameau** : une zone de terres (distance sur le réseau, type Voronoi) par hameau ; deux hameaux ne se disputent pas les mêmes tiles et leurs champs ne se soudent pas par accident.
- **Terres voisines du même type** : des bâtiments **distincts**, séparés par une haie au rendu (base du cadastre, des enclosures, des propriétaires). Leur fusion viendra de la propriété (§10.3).
- Un critère de forme dur (rapport des côtés de la boîte englobante) reste en réserve si le score ne suffit pas.

### 10.3 Économies d'échelle de l'emprise *(à modéliser après §10.2, pas dans le POC actuel)*
Intention : **privilégier l'extension d'une terre** (puis le cumul de terres proches, issu des enclosures et de la propriété, et l'extension d'usines) plutôt que la multiplication de petites cellules autonomes aussi rentables. Aujourd'hui la production est proportionnelle à N : deux terres de 4 tiles valent une de 8, donc rien ne récompense l'extension. Les gains doivent avoir une **cause physique**, pas un bonus plat :

| Source | Effet |
|---|---|
| Coûts fixes par bâtiment (route et nœud neufs, équipe minimale, outillage) | économisés quand on étend au lieu de fonder |
| Compacité (périmètre ÷ aire) | moins de jours-travail par tile pour une terre compacte : une terre « moche » est moins rentable |
| Seuil d'équipe (une charrue et son attelage couvrent N tiles) | sous-utilisation en dessous, seconde équipe au-dessus : l'optimum émerge |

Garde-fous contre les champs géants : emprise maximale par type (`04`), distance, périmètre. C'est aussi le mécanisme historique de la **concentration** (enclosures), cœur du jeu.

### 10.4 Une pop travaille en un seul lieu par jour ; stock par pop
**Règle.** Une pop travaille **en un seul lieu chaque jour**, visible le jour J (cohérent avec `00` §3 : un seul lieu de travail). Cela **remplace** le budget de jours réparti entre les terres (§8.3 et « Main-d'œuvre » POC-A), qui rendait « son lieu de travail du jour » ambigu.

**Décision quotidienne d'une pop (raisonnement simple du POC, à enrichir : salaire, contrat, démission).**
1. Elle calcule, pour chaque ration, son **autonomie** : `stock_de_la_pop ÷ ration_par_jour` (en jours).
2. Elle classe ses besoins : **vital d'abord** (nourriture, chauffage), puis confort, puis le reste (travail domestique, chantier).
3. Candidats : lieux **à portée** (`distance_max`) qui ont du travail à donner aujourd'hui (postes du jour) et dont la production répond au besoin visé.
4. Elle choisit le lieu qui produit la ressource **la moins couverte**, puis le plus proche. Rien de pressant : travail domestique ou chantier.
5. Elle **enregistre le motif** : ressource visée, autonomie, candidats et raison d'écart (trop loin, pas de poste, stock plein).

**Production linéaire.** Un jour de travail donne un jour de production. Le besoin d'un lieu s'exprime en pop-jours (formule actuelle) : une terre de 4 tiles ne demande qu'environ 1/12 de pop par jour, donc les pops **tournent** d'un lieu à l'autre au fil des jours selon leurs besoins. Les **postes du jour** de chaque lieu évitent que toutes les pops aillent au même endroit.

**Stock par pop** *(et non par tile ni par hameau)* : chaque pop a son inventaire personnel (grain, viande, fruit, bois, vêtements) et **consomme sur son propre stock**. Deux pops d'une même maison ou d'un même hameau peuvent avoir des statuts, besoins et couvertures différents (strates, richesse, plus tard). Conséquences :
- la couverture des besoins (§6) se calcule **par pop**, plus par hameau ;
- les stocks de bâtiments (sortie) approvisionnent les pops par les liens (J+1, principe 5) ; la **règle de partage** du POC reste au prorata du manque (équivalente à l'actuelle) tant qu'il n'y a pas de monnaie ; elle deviendra salaire ou achat en POC-B ;
- le travail domestique (§8.4) transforme les intrants **de la pop** ;
- attributs de vie à prévoir en champs **nullables**, sans logique tant que le POC n'en a pas besoin (principe 7) : richesse, santé, culture, déménagement, émigration.

**Impacts à traiter à l'implémentation.** `sim/` (affectation quotidienne, consommation, partage), `03` (schéma : stock porté par la pop), `app/` (panneau de pop), recalibrage de `jours_personne_par_pop_an` qui ne sert plus qu'à convertir les besoins des lieux en pop-jours (`05`, question 9). Migration en deux temps conseillée : (a) un lieu par jour avec les stocks de hameau actuels, (b) puis stock par pop. **Risques** : oscillations (toutes les pops vers la même ressource le même jour), à corriger par les postes du jour ; coût (pops = entités individuelles, déjà le cas, `03` §5b).

### 10.5 Comprendre les flux et les stocks (priorité avant les animations)
L'objet du jeu est l'organisation du travail et le partage de ses fruits : le joueur doit voir **qui travaille où et pourquoi, ce qui est produit, où c'est stocké, qui consomme**.

| Vue | Contenu |
|---|---|
| **Pop** (clic sur son icône dans sa maison) | lieu de travail du jour et motif (§10.4), lieux possibles (surlignés sur la carte, avec la raison d'écart des autres), stock personnel et autonomie par ration, consommation du jour, état (satisfaite, en manque) ; plus tard salaire, contrat, richesse, santé, culture |
| **Lieu de production** | production du jour (cumul), facteur limitant, stock de sortie, pops présentes aujourd'hui, destinataires des sorties |
| **Par ressource** (ex. grain) | la carte met en évidence producteurs, stocks (bâtiments **et** pops), flux en transit et consommateurs ; bilan produit / stocké / consommé |

Données à conserver pour cela : affectation et motif du jour par pop, production et consommation du jour par bâtiment et par pop, flux J→J+1 par lien. Ce sont des agrégats déjà calculés par le moteur ; il manque leur exposition (`PanneauInfos`, instantanés du pont wasm).

### 10.6 Animation (non prioritaire)
Une fois §10.4 en place, les sprites de pops marchent sur le **chemin réel** de leur maison à leur lieu de travail du jour : ce sont de vraies pops, pas des figurants dérivés. Jour/nuit = rendu seul. Les saisons dans la simulation restent hors POC pour l'instant (production continue, `04`).
