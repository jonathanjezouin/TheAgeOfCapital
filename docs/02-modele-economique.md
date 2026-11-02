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

**POC-A : postes stables, entraide du hameau, sans marché.** Une pop travaille à **un seul endroit** et n'en change que par nécessité. En 1750 aucune pop ne « choisit » sa production (pas de salaire, pas de marché du travail) : elle suit le métier hérité, et le hameau réoriente des bras vers ce qui manque.

| Événement | Règle |
|---|---|
| Départ | Le générateur de village affecte chaque pop à un poste ; les postes sont dimensionnés pour démarrer près de l'équilibre |
| Duplication d'une pop | La nouvelle pop prend le **poste vacant le plus proche** de son hameau (coût de transport) ; à égalité, le lieu le moins rempli ; sinon elle est **sans poste** (elle consomme, ne produit pas) |
| Disparition d'une pop | La pop dont la jauge a franchi −100 ; son poste devient vacant |
| **Chaque mois** | 1. Les pops sans poste prennent les postes vacants les plus proches. 2. **Réorientation selon le besoin** : pour chaque ressource en tension (ration couverte à moins de 97 %, ou intrant qui manque à un producteur), une pop quitte un lieu dont le stock est plein (excédent) pour un lieu qui produit la ressource manquante et a un poste libre (la pop la plus proche de la destination). Au plus un déplacement par ressource et par mois |
| **Travail sur place** | L'atelier d'un hameau textile est un poste ordinaire, à distance 0 : un résident y travaille sans trajet. Les autres pops du hameau vont aux terres alentour |
| **Chantier** | Un chantier (défrichement, construction) emploie une équipe comme n'importe quel lieu de travail (voir §7) |
| Accès au poste | Le trajet domicile-travail charge les segments. Si la pop n'atteint pas son poste un jour donné (congestion, en sommeil dans le POC), la production de ce jour est réduite ; la main-d'œuvre ne se reporte pas à J+1 |

Boucle qui se corrige seule : des disparitions créent des postes vacants, la production baisse, les duplications reprennent les postes ; un manque de grain attire des bras vers les champs depuis les lieux en excédent. Le **travail saisonnier** (hiver : filage et couture) est hors POC (production continue).

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

Aucune firme, aucune monnaie : la **communauté** étend le village, à rythme **saisonnier** (tous les 90 jours), **après** la réaffectation mensuelle. Les pops prennent d'abord tous les postes disponibles, et ne s'étendent que ce qui reste.

### Quand
| Situation | Réaction |
|---|---|
| **Des pops attendent un logement** (jauge pleine, plus de place à portée) | Bâtir un **hameau**. Textile si les ateliers manquent de capacité, sinon ordinaire |
| **Une ressource est en tension** et des pops restent **sans poste** | **Défricher** une terre du type qui produit la ressource |
| Les producteurs de la ressource sont **bloqués par un intrant manquant** (ex. ateliers sans lin) | On ne les multiplie pas : la tension est reportée sur l'intrant (champ de lin) |
| Un chantier est déjà en cours pour cette ressource | On attend |

Au plus `chantiers_max` chantiers en même temps.

### Où
Un site doit être **libre**, **desservi par une arête** (existante, ou **arête de raccordement à créer** dont le coût de pose en pop-jours dépend de sa longueur et des maisons à reloger, voir §9.3), à **moins de `distance_max`** du hameau demandeur, et **desservi** : dans la limite de distance d'un producteur de chaque besoin du lieu (matériau de construction ; pour un hameau aussi chauffage et nourriture). Parmi les sites possibles :
- **terre exploitée** : maximise `fertilité / (distance + 1)` ;
- **hameau** : le plus proche, sur la terre la plus pauvre (on ne bâtit pas sur la meilleure terre).

### Coût
Un chantier est un lieu de travail temporaire : une **équipe** de pops (venues du hameau demandeur) y fournit un nombre de **pop-jours**, et le **matériau** (bois) arrive par le réseau comme n'importe quelle marchandise, consommé au prorata du travail. Sans bois, le chantier n'avance pas. À la fin, l'équipe devient la première main-d'œuvre du lieu.

| Création | Travail (pop-jours) | Équipe | Bois | Durée minimale |
|---|---|---|---|---|
| Champ | 240 | 2 | 0 | 120 j |
| Pâturage | 90 | 1 | 20 | 90 j |
| Verger | 300 | 2 | 0 | 150 j |
| Exploitation forestière | 120 | 2 | 0 | 60 j |
| Champ de lin | 200 | 2 | 0 | 100 j |
| Élevage laine | 100 | 1 | 20 | 100 j |
| **Hameau** | 300 | 2 | 200 | 150 j |
| **Hameau textile** | 400 | 2 | 260 | 200 j |

(Valeurs de départ plausibles, à calibrer : un champ se défriche en quelques mois à deux pops ; un hameau demande une saison de travail et quelques jours de production de bois. Le verger tarde à produire dans la réalité, ce délai n'est pas modélisé.)

### Équipe
1. Les pops **sans poste** les plus proches ; 2. à défaut, des bras pris dans les lieux dont le stock couvre au moins 5 jours (excédent) ; 3. un chantier resté **sans équipe** réquisitionne des bras (corvée) à chaque saison, même sans excédent. Un chantier incomplet recrute à nouveau à chaque saison.

### Le frein : la qualité de la terre
Rien n'interdit de défricher une plaine entière, mais les terres lointaines sont plus pauvres : quand une pop de plus ne produit plus de quoi se nourrir (et se chauffer) sur la terre qu'elle défriche, la population plafonne. La taille d'équilibre **émerge** de la fertilité et de la distance ; elle oscille en « dents de scie » autour de ce plafond (déclin lent si le vital manque, reprise ensuite). Le confort (viande, vêtements) **ne freine pas** la croissance (une pop insatisfaite croît à +0,5 %/an) : c'est le premier luxe que l'on abandonne quand la terre est ingrate.

### Journal
Chaque lancement et chaque fin de chantier est inscrit au journal d'événements, avec son **motif** (« grain manque, 2 pops sans poste » ; « 1 pops attendent un logement »).

### Limites connues
Pas d'abandon : un hameau vidé par un déclin reste debout, une terre sans bras reste inactive. Pas de délai de production d'un verger. Pas de réaffectation de l'équipe en cas de besoin urgent ailleurs.

## 8. Évolutions de design décidées (2026-11-02, à implémenter)

Décisions prises après l'étude des ratios de 1750 (référence chiffrée dans `04`). Le code `sim/` et `app/` reflète encore la version précédente (1 tile = 1 terre, hameau textile).

### 8.1 Tile fixe de 250 m
Voir §1. Conséquence : le POC actuel (60–90 pops dans ~7 tiles de rayon) suppose des tiles de ~0,7–1 km ; à 250 m, le même village s'étale sur ~25 tiles de rayon et la limite de `distance_max` (10 tiles) devient le moteur d'extension.

### 8.2 Une terre exploitée = un bâtiment de N tiles
- Un champ, un pré, une forêt… est **un seul bâtiment actif** dont l'emprise est de N tiles contiguës. La production et le besoin en travail sont proportionnels à N (et à la fertilité de chaque tile).
- Le défrichement **ajoute des tiles contiguës** à la terre (ou en crée une nouvelle).
- Intérêt : réalisme spatial sans multiplier les nœuds (~1 nœud pour 5–6 tiles). Les parcelles contiguës ouvrent plus tard la voie aux enclosures et à la propriété (cadastre = ensemble de tiles).

### 8.3 Le travail se compte en jours
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
- L'infrastructure est une **couche d'arêtes** reliant les centres de deux tiles voisines, dans **8 directions**. La tile garde son usage (champ, hameau…) : un chemin ne coupe plus un champ de N tiles, et une piste de 5 m ne consomme plus 6,25 ha.
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
- **Tile d'habitat desservie par une route ou une demi-route** : toute tile d'habitat est traversée par une arête ou porte une demi-arête, et les maisons se placent le long (façade), au fur et à mesure. Une demi-arête jamais prolongée reste une **impasse**. Capacité = f(longueur de façade, niveau de densification). Un hameau de 1 pop se contente d'une demi-route ; au-delà, la tile devient traversante. Une ferme isolée au bord du chemin ne pose pas de difficulté jusqu'à une certaine quantité de pops, puis il faut prolonger l'arête.
- **Densification** : le bâti s'améliore (maison isolée, mitoyenne, petit immeuble, immeuble). Le plafond de 10 pops est celui du bâti bas ; des immeubles le relèvent (ordre de grandeur : ~400 hab/ha, soit ~25 pops par tile). Le plafond deviendrait une propriété du type de bâtiment, pas une constante.
- **Rues rectilignes en ville** : en ville les rues sont planifiées (lotissement) : droites, avec des immeubles d'angle. À la campagne, les chemins sont sinueux et contournent le bâti existant (parfois autour de petits bâtiments). Le **décalage organique du rendu décroît avec le niveau d'urbanisation** (0 pour une trame planifiée). Une trame urbaine est un ensemble de rues parallèles dans la tile (voirie native), pas des arêtes distinctes tant qu'elles sont locales.
- **Rendu des routes** : courbe lissée (Catmull-Rom vers Bézier) passant par les centres de tiles, avec décalage déterministe (hash de la position et de la graine, ±0,34 tile au plus) ; largeur exagérée à l'écran. L'atlas de 15 tiles de chemin est abandonné pour le réseau (`03` §6).
