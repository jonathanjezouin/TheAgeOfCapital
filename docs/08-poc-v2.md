# POC v2 — repartir de presque rien

*Décision du 2026-11-05. Les docs 00 à 07 restent en l'état comme vision et réserve d'idées ; ce document trace le POC v2, qui les remplace comme périmètre de travail.*

## Pourquoi

Le POC v1 (archivé dans `_archive/poc-v1/`) a introduit trop de concepts en même temps (besoins vitaux et de confort, natalité, peuplement émergent, seigneur…). Résultat : la population ne croissait jamais à long terme, et chaque réglage en cassait un autre. Les tests d'invariants passaient sans garantir la dynamique voulue.

Rappel de fond : un village de 1750 est en équilibre malthusien ; ce sont les transformations du 19e siècle (rendements, transports, industrie) qui font croître la population. La base doit donc être **simple, stable, compréhensible**, pas « prospère ».

## Règle de méthode

**Un concept à la fois.** Un concept n'est ajouté que lorsque le précédent a un comportement dynamique vérifié par un test de trajectoire (pas seulement des invariants), et que Jonathan l'a vu tourner.

## État de départ (étape 0)

- Une tile **habitation** avec **1 pop**, une tile **champ** à côté.
- Le **temps** : 1 tick = 1 jour, 365 jours par an, départ en 1750.
- Rien d'autre : pas de production, pas de consommation, pas de natalité.
- Code : `sim/` (crate Rust minimale, `cargo test`, `cargo run --bin sim-cli -- --annees 10`).

## Étapes envisagées (à valider une par une)

| # | Concept ajouté | Test de trajectoire attendu |
|---|---|---|
| 1 | Le champ produit du grain (saisons) | production annuelle régulière |
| 2 | La pop travaille et mange | la pop survit indéfiniment ; sans champ, elle meurt |
| 3 | Mortalité et natalité simples | population stationnaire autour d'un plafond fixé par le champ |
| 4 | Plusieurs champs / habitations | plafond proportionnel aux terres |
| … | Puis les transformations (rendements, transports, industrie) | à définir |

L'ordre et le contenu des étapes sont provisoires.
