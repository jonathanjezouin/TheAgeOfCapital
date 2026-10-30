# CLAUDE.md — Contexte du projet

> Lu par Claude Code au début de chaque session. Garder ce fichier **court** : il pointe vers `docs/` pour le détail.
> Titre du jeu : **The Age Of Capital**.

## Le projet en 5 lignes

Jeu de stratégie / simulation historique **1750–1950** sur la naissance et le développement du capitalisme : économie spatiale sur grille de tiles, firmes autonomes, concentration capitaliste émergente, puis (plus tard) couche politique avec des personnages autonomes dont le joueur n'est qu'un parmi d'autres.
Inspirations : Victoria 3 (économie/politique), Songs of Syx (échelle, tiles), RimWorld (émergence narrative).
Distribution visée : Steam. Licence : Apache 2.0.

**Phase actuelle : POC** = une simulation économique autonome, sans intervention du joueur, sur une carte 200×200.

## Stack

| Couche | Techno |
|---|---|
| Shell desktop | Electron |
| UI | Vue 3 |
| Rendu | PixiJS (WebGL) |
| Moteur de simulation | **Rust** via napi-rs (addon `.node`) — Rust dès le départ, pas de migration ultérieure |
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

## Principes non négociables

1. **Les tiles sont des unités comptables, pas des surfaces géographiques fixes.** La surface implicite varie selon le type (un champ ≈ 5 km², un quartier rural ≈ 0,5 km²).
2. **Les tiles sont passives** (données en SQLite, hors boucle de tick). **Les bâtiments sont actifs** (nœuds d'un graphe en mémoire).
3. **La Firme est l'acteur économique**, pas le bâtiment. Un groupe possède des firmes mono-secteur.
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
