# 03 — Architecture technique

## 1. Vue d'ensemble

```
┌──────────────────────────────────────────────┐
│ ELECTRON MAIN PROCESS (Node.js)              │
│                                              │
│  simulation.node  (Rust via napi-rs)         │
│  ├── Bâtiments (graphe actif en mémoire)     │
│  ├── Firmes / Groupes                        │
│  ├── Personnages (décident tous les N ticks) │
│  ├── Graphe d'infrastructure + Dijkstra      │
│  └── tick()                                  │
│                                              │
│  SQLite (rusqlite, WAL)                      │
│  ├── tiles (monde entier, passif)            │
│  ├── bâtiments, infras (état persisté)       │
│  ├── nodes / edges (graphe persisté)         │
│  └── historique économique                   │
└───────────────────┬──────────────────────────┘
                    │ IPC — deltas uniquement
┌───────────────────▼──────────────────────────┐
│ ELECTRON RENDERER                            │
│  PixiJS : tiles + bâtiments visibles         │
│  Vue 3  : UI (stats, menus, actions)         │
└──────────────────────────────────────────────┘
```

Rust : moteur, graphe, économie, pathfinding, décisions des personnages. Node : pont IPC. Vue + PixiJS : rendu et UI.

### Pourquoi Rust dès le départ
Pas de GC (pas de micro-freezes), compilé natif, parallélisme réel (Rayon). Coût accepté : un binaire `.node` par plateforme (Windows x64, macOS ARM, Linux x64) à gérer dans le build (electron-builder).

### Organisation du code et de l'exécution
- Crate Rust **`sim` pure** (aucune dépendance à napi) : moteur, déterminisme, tests. Un **CLI headless** l'exécute sans interface.
- Wrapper **napi-rs** mince pour Electron. En dev, le renderer est une page web ordinaire (Vite) reliée à la sim par un petit serveur local : pas d'Electron avant le packaging Steam (Electron ou Tauri à trancher à ce moment).
- Rendu validé visuellement en Chromium headless (captures).

### Hôte actuel : le navigateur (WebAssembly)
Pour le POC, le moteur tourne **dans la page** : `sim-wasm` compile la crate `sim` en WebAssembly (interface C, sans `wasm-bindgen`) et expose des instantanés JSON ; `app/` (Vue 3 + PixiJS) les affiche. Aucun serveur, aucun pont Node : la page marche dans un navigateur, un Artifact, et plus tard dans Electron. Le moteur natif (napi-rs) reste l'hôte prévu pour la version Steam ; seul l'hôte change, pas la crate `sim`.

### Décision d'hôte pour Steam (tranchée après le POC)

| Hôte | Rôle | Raison |
|---|---|---|
| **Electron + napi-rs** | Version Steam | Multi-thread réel (Rayon), mémoire non bornée, Chromium identique sur toutes les plateformes. Coût : un binaire `.node` par plateforme, paquet plus lourd |
| WebAssembly (`sim-wasm`) | Démonstrations, Artifact, tests visuels | Déjà en place ; limites : 4 Go (wasm32), threads soumis à `SharedArrayBuffer` + COOP/COEP, 10–30 % plus lent que le natif |
| ~~Tauri~~ | Écarté | WebViews variables selon l'OS (WebKitGTK médiocre en WebGL) : risque inacceptable pour un jeu |

Le moteur tourne **hors du thread de rendu** (`utilityProcess` Electron ou Worker en wasm). La crate `sim` reste unique et pure.

## 2. Tiles passives, bâtiments actifs

| | Tiles | Bâtiments |
|---|---|---|
| Rôle | Données géographiques | Agents |
| Stockage | **Mémoire** (`Vec`, 40 000 tiles en POC) ; SQLite pour la sauvegarde | Mémoire (graphe) + persistance SQLite |
| Dans la boucle de tick | **Jamais** | Oui |
| Sollicitées pour | Rendu, recherche d'emplacement | Production, échanges, stocks |

- Les tiles ne sont lues que pour : **le rendu** (chunks sous la caméra) et **le placement** (recherche spatiale ponctuelle).
- **Chunking** (16×16 ou 32×32) : mémoire active = chunks visibles + un anneau tampon ; le reste sur disque. Le joueur voit 200×200 tiles ; un déplacement de caméra charge les voisines.
- **POC** : 200×200 = 40 000 tiles tiennent en mémoire ; SQLite sert à la sauvegarde / chargement (schéma versionné). Le chunking et la base spatiale concernent l'échelle finale.
- Recherche d'emplacement : requête SQL indexée sur `(city_id, x, y)` au stade POC (rare : un personnage décide tous les ~30 jours). À l'échelle finale : index spatial en mémoire (voir §5b).

## 3. Graphe de connectivité (option B)

- Les bâtiments sont des **nœuds** d'un graphe de connectivité ; ils ne connaissent pas les tiles, seulement leurs voisins réseau.
- Arêtes pondérées (type d'infra, qualité, coût).
- **SQLite = sauvegarde, graphe mémoire = moteur.** Une modification d'infra → update des arêtes en base + recalcul **local** du sous-graphe affecté (Dijkstra en mémoire).
- La création d'un bâtiment ou d'une infrastructure met à jour les nœuds du voisinage : quels habitants sont à portée, quels fournisseurs accessibles (opportunité de meilleur contrat).
- Interactions **localisées** pour éviter le O(n²) ; les effets globaux passeront plus tard par des canaux abstraits.
- Pas de NoSQL, pas de base graphe externe (incompatible avec une distribution Steam embarquée).

```sql
CREATE TABLE nodes (id INTEGER PRIMARY KEY, x INT, y INT, type TEXT, city_id INT);
CREATE TABLE edges (node_from INT, node_to INT, infra_type TEXT, cost REAL,
                    PRIMARY KEY (node_from, node_to));
```

## 4. Schéma de données (POC)

```
Tile
  id, x, y
  type            : quartier | production | infra | naturel
  firme_id        (nullable)
  capacite_max
  stock           { ressource: qte }
  capacite_logement (si habitat ; population = Σ des pops de la tile)
  fertilite       (ppm de 1,0 ; multiplie la production des terres exploitées ; le terrain reste unique)
  valeur_fonciere (non utilisé POC)

Firme
  id, nom
  type            : privee | publique | coop
  groupe_id       (nullable)
  secteur         : nourriture | textile | bois …
  capital, tresorerie
  tiles_ids       [ ]
  seuil_reinvest

Groupe
  id, nom, firmes_ids [ ], capital_consolide
  strategie       (non utilisé POC)

LienEconomique
  id, fournisseur_tile_id, client_tile_id
  ressource, volume_jour
  cout_transport
  segments_ids    [ ]
  operateur_id    (null = transport implicite, POC)
  contrat_id      (null = achat spot)

SegmentInfra
  id, tile_a_id, tile_b_id
  type_infra      : chemin | route | rail …
  capacite_nominale, capacite_max
  charge_courante, facteur_congestion
  cout_unitaire   = f(type, congestion)
  file_attente    [ {volume, contrat_id, priorite} ]

Contrat
  id, type (spot | cadre), prix_unitaire, volume_garanti, duree_ticks
  qualite         (non utilisé POC)

Pop (unité de population, 100 hab fixe)
  id, tile_residence_id, tile_emploi_id (nullable)
  croissance      (jauge en micro-habitants : +100 hab = duplication, −100 hab = disparition)
  strate          : paysan | ouvrier | bourgeois | domestique | noble …
  culture, religion, orientation, alphabetise   (fixes en POC ; bascule hors POC)
  pression_conversion   (nullable, non utilisé POC : jauge continue, bascule au franchissement d'un seuil)
  richesse, droits_politiques      (nullable, non utilisé POC)

Hameau / Quartier (tile d'habitat)
  habitat_id (hameau | hameau_textile), capacite_pops (0 pendant la construction), atelier (nullable : bâtiment à 1 poste sur la même tile)
  stock, vital_jour, confort_jour, couverture par ressource, vital_annuel, confort_annuel
  salaire_reference (POC-B)

Batiment (type)
  secteur, etapes [{intrants [{ressource, qte}], extrants [{ressource, qte}]}]   (étapes enchaînées le même jour)
  emploi (en pops), fertile (production × fertilité de la tile), cout_creation (travail en pop-jours, équipe, matériau), chantier (type de site : défrichement ou construction), niveau_tech, contraintes_geo
```

Ressources : `MAIN_OEUVRE` est une ressource ordinaire (stock dans le quartier, circule par liens, contribue à la congestion). Liste POC dans `04-poc.md`.

## 5. Ordres de grandeur (cible)

> Révisé après le POC : l'ambition est de l'ordre de **centaines de milliers** d'objets (l'ancienne cible, 5–20 k bâtiments, est abandonnée). Chiffres à valider par un banc d'essai (reporté, voir `05`).

| Objet | Cible | Mémoire estimée (SoA, ~64–128 o/objet) | Remarque |
|---|---|---|---|
| Tiles monde | ~4 M (2 000×2 000) | ~20–40 Mo | Passives, quelques octets chacune |
| Bâtiments actifs | 300–500 k | ~50 Mo | En mémoire, tableaux denses |
| Pops (1 pop = 100 hab) | 100–300 k (10–30 M d'habitants) | ~30 Mo | Flux agrégés par (résidence, emploi, strate) |
| Personnages | 10–50 k | négligeable | Créés par des postes (voir `00`), règles légères, pas tous à chaque tick |
| Firmes | ~1 000 – 10 000 | négligeable | |

Principe : **on simule finement ce qui est observé** ; les zones inactives n'évoluent que par agrégats simples et « se déploient » quand on y zoome. Résolution variable selon le niveau de zoom. L'agrégat doit être *dérivé* des règles détaillées, jamais codé en dur (principe n°6).

Le risque à cette échelle est **algorithmique** (un tick qui touche tout, une recherche de chemin non bornée), pas le langage : Rust suffit.

## 5b. Passage à l'échelle — principes retenus

À appliquer **après** le POC (dette connue du code actuel : `BTreeMap` dans la boucle chaude, instantanés JSON, rendu sans chunks).

### Moteur (`sim`)
| Sujet | Règle |
|---|---|
| Structures | Tableaux denses indexés par `u32` (SoA, un tableau par champ). Plus de `BTreeMap` dans la boucle chaude |
| Déterminisme | Conservé par construction : entiers, ordre fixe des indices, RNG maison. Parallélisme (Rayon) **par région / chunk**, fusion dans un ordre fixe |
| Fréquences | Bâtiments : chaque jour. Pops : par lots (1/N par jour) ou à l'événement. Personnages : tous les ~30 jours |
| Calcul événementiel | Liste d'éléments « actifs / sales » : un stock inchangé n'est pas recalculé |
| Zones non observées | Modèle agrégé dérivé des règles, détaillé à la demande |
| Réseau | Chemins bornés et précalculés (déjà le cas), recalcul **local** à chaque modification d'infrastructure ; hiérarchie de chunks pour les longs trajets |

### Pont moteur → rendu
- **Plus d'instantané JSON du monde.** Le moteur expose `viewport(x0, y0, x1, y1)` et renvoie des **tableaux typés** (`u16`/`u32` : position, type, état) lus directement dans la mémoire wasm / un `SharedArrayBuffer` / un buffer napi.
- **Drapeaux « sale » par chunk** : le rendu ne relit que les chunks modifiés depuis le dernier frame.
- Statistiques globales et courbes : petits agrégats séparés.

### Rendu (PixiJS)
| Zoom | Technique |
|---|---|
| Proche | Sprites d'atlas par chunk ; pops en points ou sprites |
| Moyen | Terrain en maillage par chunk 32×32 (1 draw call par chunk) ; bâtiments en `ParticleContainer` |
| Lointain | Une texture agrégée par chunk (régénérée quand le chunk est sale) ; carte de densité à la place des pops |

- Chunks hors caméra libérés, anneau tampon d'un chunk.
- Seul le **visible** coûte : 500 k bâtiments en mémoire n'ont pas de coût de rendu.
- Tiles 256 px trop lourdes pour un atlas à centaines de types : prévoir des niveaux de textures (256 / 128 / 64) et un mipmapping maîtrisé.
- WebGL2 suffit ; WebGPU (PixiJS v8) reste une option, non nécessaire.

### Persistance
- SQLite **hors de la boucle de tick** : sauvegarde et séries historiques uniquement.
- Sauvegarde = **instantané binaire compressé** (`bincode` + `zstd`) stocké en blob, schéma versionné.
- Recherche d'emplacement : **index spatial en mémoire** (grille de chunks), plus de requête SQL dans le chemin de décision (remplace la requête indexée de §2 à l'échelle finale).

## 6. Rendu — architecture technique

> Le style graphique est volontairement hors de ce document (nouveau pipeline d'assets en cours).

- **Couches** : 0 = terrain ; 1 = infrastructure (fond transparent) ; 2 = bâtiments (fond transparent). Les bâtiments n'ont pas de fond propre : ils sont composités sur une tile terrain.
- Résolution source **256×256 px** (puissance de deux → mipmapping automatique PixiJS ; aux zooms éloignés, le GPU moyenne et ne conserve que la couleur dominante). *À reconfirmer avec le nouveau pipeline.*
- **Texture atlas** (spritesheet PNG + JSON) pour les performances.
- Vue strictement **top-down**.

### Modèle des demi-directions (infrastructure)
- Une tile d'infrastructure porte des **demi-connexions** du centre vers le bord, dans jusqu'à **8 directions** (orthogonales + diagonales), **4 actives max** par tile.
- Deux tiles sont connectées quand leurs demi-directions **en vis-à-vis** sont présentes des deux côtés.
- Supporte culs-de-sac, routes orthogonales et diagonales, tiles mixtes infra/habitat.
- **POC : orthogonal seulement** (4 directions, 15 combinaisons non vides au lieu de 162) ; les diagonales viendront après.
- Coût d'ajout d'une demi-direction dominé par le **déplacement d'habitations existantes** → capture la dynamique haussmannienne sans règle dédiée.
- Assets : une variante par combinaison de demi-directions (`infra_chemin_terre_*`). La lumière (jour/nuit) est une teinte appliquée en rendu, pas précalculée.

### Sprites d'ambiance
Sprites stochastiques (piétons, charrettes, plus tard locomotives) sur les tiles d'infrastructure, **densité proportionnelle à la charge** du segment. Aucun suivi d'entité individuelle : c'est une visualisation de `charge_courante`, pas une simulation.

## 7. Outillage d'assets
Le générateur `graphGeneration/` (Python/Pillow/numpy) **fait référence** pour les assets. Conventions : nommage par catégorie (`terrain_*`, `infra_*`, `bat_*`), variantes multiples par type pour éviter la répétition visuelle, manifeste JSON.

```bash
cd graphGeneration && python generate_assets.py [--seed N] [--apercu]   # écrit dans app/public/assets/
```

| Sortie | Contenu |
|---|---|
| `atlas.png` + `atlas.json` | 49 tiles de 256×256 : herbe (10 variantes raccordables), 15 chemins, hameaux, terres, chantiers |
| `relief/` | Pour les hameaux (les seuls à avoir du relief) : image **sans éclairage directionnel** (`*_albedo.png`) et **carte de normales** (`*_normal.png`) |

- **Lumière : soleil au zénith.** Aucune ombre portée ; seule une légère assise de contact sous les objets. Le relief des toits est cuit (pans symétriques). Pour une **lumière en rotation**, un shader ré-éclairera les hameaux à partir de l'albedo et des normales, sans régénérer. Le jour/nuit reste une teinte globale en rendu.
- **Raccords.** Les chemins sont indexés par **masque de demi-directions** (N=1, E=2, S=4, O=8, mêmes valeurs que le moteur) et leur profil au bord est identique d'une tile à l'autre (bruit périodique) : pas de couture visible. L'herbe est raccordable et toutes ses variantes ont la même couleur moyenne.
- **Couches.** 0 terrain, 1 infrastructure, 2 bâtiments. **Les hameaux n'embarquent pas leurs rues** : le rendu pose sous le hameau (et sous le chantier de hameau) le chemin du masque des voisins réellement raccordés. Le hameau garde seulement une placette de terre battue.
- **Un chantier, deux images.** Dans la simulation un chantier est un seul type de lieu ; le rendu choisit `bat_chantier_hameau` ou `bat_chantier_champ` selon sa cible.
- Tests : `cd graphGeneration && python -m unittest discover -s tests` (raccords, zénith, manifeste).
