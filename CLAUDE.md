# CLAUDE.md — Contexte du projet

> Lu par Claude Code au début de chaque session. Garder ce fichier **court** : il pointe vers `docs/` pour le détail.
> Titre du jeu : **The Age Of Capital**.

## Le projet en 5 lignes

Jeu de stratégie / simulation historique **1750–1950** sur la naissance et le développement du capitalisme : économie spatiale sur grille de tiles, firmes autonomes, concentration capitaliste émergente, puis (plus tard) couche politique avec des personnages autonomes dont le joueur n'est qu'un parmi d'autres.
Inspirations : Victoria 3 (économie/politique), Songs of Syx (échelle, tiles), RimWorld (émergence narrative).
Distribution visée : Steam. Licence : Apache 2.0.

**Phase actuelle : POC v2** (`docs/09-poc-v2.md`) : repartir d'une base presque vide, un concept à la fois ; étape 1 réalisée : subsistance d'une pop (`docs/08-concept-subsistance.md`). Le POC v1 est archivé dans `_archive/poc-v1/`.

## Stack

| Couche | Techno |
|---|---|
| Shell desktop | Electron (Tauri écarté) |
| UI | Vue 3 |
| Rendu | PixiJS (WebGL) |
| Moteur de simulation | **Rust** via napi-rs (addon `.node`) pour Steam ; WebAssembly pour démos/tests — Rust dès le départ. Cible d'échelle : centaines de milliers d'objets (`docs/03` §5b) |
| Persistance | SQLite, mode WAL, via `rusqlite` |
| Pont | Node.js (main process Electron) ↔ renderer par IPC (deltas uniquement) |
| IA embarquée (plus tard) | node-llama-cpp + Qwen 2.5 3B Q4_K_M, sortie JSON via GBNF — **hors POC** |

## Documentation de design

| Fichier | Contenu |
|---|---|
| `docs/00-concept-jeu.md` | Concept : ce qu'est le jeu, ce que fait le joueur, pops (100 hab) et personnages (postes) |
| `docs/01-vision.md` | Vision long terme : acteurs, politique, niveaux de jeu, régimes économiques |
| `docs/02-modele-economique.md` | Modèle de simulation : tiles, firmes, flux unifié, contrats, congestion, séquence du tick |
| `docs/03-architecture.md` | Architecture technique, schéma de données, pipeline de rendu (technique) |
| `docs/04-poc.md` | Périmètre exact du POC, calibration des quantités, critères de succès |
| `docs/05-roadmap-et-questions.md` | Ce qui est reporté, questions ouvertes, points à trancher |
| `docs/06-fertilite-cible.md` | Réflexions sur la fertilité cible (aptitude sol/culture) ; hors POC |
| `docs/09-poc-v2.md` | **POC v2 en cours** : pourquoi on repart de presque rien, règle de méthode, étapes |
| `docs/07-peuplement-emergent.md` | Peuplement émergent, carte de plaine à champs ouverts, terrains, rivières, forêts, règles de site, abandon et destruction ; étapes 1 à 6 réalisées (carte, aptitude, défrichement, peuplement, abandon, seigneur et serfs, réglage) + accès par marche et routes décidées par le seigneur (§18), banalités à venir |
| `docs/08-concept-subsistance.md` | Concept de subsistance pour une pop : grain, bois, vêtement, prélèvement 15 %, transport, décision quotidienne, consommation mensuelle (décisions du 2026-11-07, hors code actuel) |

## Code

| Dossier | Contenu |
|---|---|
| `sim/` | **POC v2** : moteur Rust de la subsistance d'une pop (docs/08) + CLI (`cargo run --bin sim-cli -- --annees 20 --meteo 1000`, `cargo test`) |
| `sim-wasm/` | Pont WebAssembly du moteur v2 (interface C, instantanés JSON) |
| `app/` | Interface v2 : carte PixiJS (sélection de tiles) + panneaux Vue 3 + moteur WASM (`npm install && npm run wasm && npm run dev`) ; atlas copié du v1 |
| `_archive/poc-v1/` | POC v1 archivé (`sim/`, `sim-wasm/`, `app/`) ; réserve d'idées, pas une base. Voir `_archive/README.md` |
| `graphGeneration/` | Générateur d'assets : atlas de 49 tiles dans `app/public/assets/` (référence graphique) |

## Principes non négociables

1. **Une tile = une surface réelle fixe : 250 m × 250 m (6,25 ha)** (décision du 2026-11-02, remplace l'ancienne « unité comptable à surface variable »). Les distances en tiles ont un sens réel (10 tiles = 2,5 km ≈ 30–50 min à pied). C'est l'**emprise** et la **densité** qui varient selon le type : une terre exploitée = bâtiment de N tiles contiguës, accessible par n'importe laquelle de ses tiles (plus de tile « nœud », `docs/07` §18), un hameau = 1 tile (2 pops), plafond 10 pops/tile (voir `docs/02` §1 et §8).
2. **Les tiles sont passives** (données en SQLite, hors boucle de tick). **Les bâtiments sont actifs** (nœuds d'un graphe en mémoire).
3. **Le personnage est le décideur politico-économique** ; la **Firme** est l'entité économique qu'il possède ou gère (pas le bâtiment ; un groupe possède des firmes mono-secteur) ; la **pop** est l'unité de travail et de consommation, autonome dans ce qui lui est ouvert (où elle travaille, ce qu'elle consomme, son stock) mais qui ne décide ni des règles ni des constructions. *(Reformulé le 2026-11-04, voir `docs/07` §15.)*
4. **Flux unifié** : la main-d'œuvre est une ressource comme les autres (produite par les quartiers, consommée par la production, soumise à la même congestion).
5. **Transport comptable** (style Victoria 3) : les marchandises « téléportent » de stock à stock au tick suivant ; l'infrastructure est sollicitée au tick du **transfert** (J+1), pas de la production (J).
6. **Émergence, pas de scripts** : le ratio habitat/production, la concentration capitaliste, l'urbanisation doivent *émerger* des règles, jamais être codés en dur.
7. **POC-first** : tout ce qui n'est pas nécessaire pour valider les mécaniques de base est reporté (voir `docs/05`). Prévoir les extensions par des champs nullables plutôt que par de la logique.

## Façon de travailler avec Jonathan

- Langue : **français**. Réponses **concises et structurées** (tableaux, raisonnement explicite).
- Jonathan challenge et simplifie souvent : proposer des options courtes, accepter le rétrécissement itératif plutôt que des premiers jets exhaustifs.
- Respecter la séparation des sujets (modèle de simulation / rendu / architecture).
- Code graphique : le code généré « à l'aveugle » est mécaniquement juste mais visuellement pauvre → **toujours valider visuellement** (capture, rendu live) avant de considérer une tâche de rendu terminée.
- Avant toute décision qui contredit un principe ci-dessus : le signaler et demander.

## Conventions

- Noms du domaine en français dans le code et le schéma (`firme`, `lien_economique`, `segment_infra`, `main_oeuvre`…), cohérents avec les docs.
- Identifiants d'assets : préfixe par catégorie (`terrain_*`, `infra_*`, `bat_*`).
