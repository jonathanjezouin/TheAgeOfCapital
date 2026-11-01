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
- Recherche d'emplacement : requête SQL indexée sur `(city_id, x, y)` — quelques ms même sur des millions de tiles, acceptable car rare (un personnage décide tous les ~30 jours, jamais à chaque tick).

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
  capacite_logement (si quartier ; population = Σ des pops de la tile)
  croissance_accumulee (si quartier ; natalité / mortalité, voir `02` §6)
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
  capacite_pops, batiment_domestique (nullable : travail textile à domicile, même tile)
  stock, vital_jour, confort_jour, couverture par ressource, vital_annuel, confort_annuel
  salaire_reference (POC-B)

Batiment (type)
  secteur, etapes [{intrants [{ressource, qte}], extrants [{ressource, qte}]}]   (étapes enchaînées le même jour)
  emploi (en pops), domestique (travail des résidents du hameau : ni poste ni trajet), niveau_tech, contraintes_geo
```

Ressources : `MAIN_OEUVRE` est une ressource ordinaire (stock dans le quartier, circule par liens, contribue à la congestion). Liste POC dans `04-poc.md`.

## 5. Ordres de grandeur (cible)

| Objet | Quantité | Remarque |
|---|---|---|
| Tiles monde | millions | Passives, en base |
| Bâtiments actifs | 5 000 – 20 000 | En mémoire |
| Pops | à estimer (1 pop = 100 hab) | Objet de base de la population ; flux agrégés par (résidence, emploi, strate) |
| Personnages | ~10 000 | Créés par des postes (voir `00`), règles légères, pas tous à chaque tick |
| Firmes | ~1 000 | |

Principe : **on simule finement ce qui est observé** ; les zones inactives n'évoluent que par agrégats simples et « se déploient » quand on y zoome. Résolution variable selon le niveau de zoom.

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
Le générateur `graphGeneration/` (Python/Pillow/numpy, maisons par cultures, calques toit/ombre) **fait référence** pour les assets. Conventions conservées : nommage par catégorie (`terrain_*`, `infra_*`, `bat_*`), variantes multiples par type pour éviter la répétition visuelle, manifest JSON.
