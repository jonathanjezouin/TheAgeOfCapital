# POC v2 — repartir de presque rien

*Décision du 2026-11-05. Les docs 00 à 07 restent en l'état comme vision et réserve d'idées ; ce document trace le POC v2, qui les remplace comme périmètre de travail.*

## Pourquoi

Le POC v1 (archivé dans `_archive/poc-v1/`) a introduit trop de concepts en même temps (besoins vitaux et de confort, natalité, peuplement émergent, seigneur…). Résultat : la population ne croissait jamais à long terme, et chaque réglage en cassait un autre. Les tests d'invariants passaient sans garantir la dynamique voulue.

Rappel de fond : un village de 1750 est en équilibre malthusien ; ce sont les transformations du 19e siècle (rendements, transports, industrie) qui font croître la population. La base doit donc être **simple, stable, compréhensible**, pas « prospère ».

## Règle de méthode

**Un concept à la fois.** Un concept n'est ajouté que lorsque le précédent a un comportement dynamique vérifié par un test de trajectoire (pas seulement des invariants), et que Jonathan l'a vu tourner.

## Étape 1 (réalisée) : la subsistance d'une pop, selon `docs/08-concept-subsistance.md`

Le concept de `docs/08` (décisions du 2026-11-07) remplace les étapes 1 à 3 envisagées au départ : une pop, 1 ferme, 5 champs, 3 forêts, grain / bois / vêtement, prélèvement de 15 %, transport quantité × distance × k, décision quotidienne, consommation mensuelle. **Pas de mortalité ni de natalité** : un stock vide n'est qu'une trace (docs/08 §7).

| Élément | Réalisation |
|---|---|
| Moteur | `sim/` : `lib.rs` (monde, pop, tick journalier), `calendrier.rs`, `rng.rs` ; quantités entières, 1 unité = 120 parts (transport et 15 % exacts) |
| Carte | grille 6×5 : ferme (1,2), champs à l'est (distances 1 à 3, moyenne 2,4), forêts au nord (2 et 3, moyenne 2,7), distance de Manhattan |
| Interface | `app/` : clic sur une tile (fiche détaillée), survol, état des champs (nu, labouré, semé, mûr), barres de travail, tas de grain et de bois à porter, trajet et position de la pop ; panneaux pop, année (jours par poste, flux), courbes de stocks, historique, journal |
| Tests | `cargo test` dans `sim/` : invariants (365 j, dimanches, conservation du grain et du bois) et trajectoires (année moyenne = chiffres du doc, jamais de moisson ratée sur 5 graines × 100 ans) |

### Hypothèses prises sur les points ouverts de `docs/08` §9 (à valider)

| # | Point ouvert | Hypothèse du code |
|---|---|---|
| 1 | Moisson et transport | culture d'abord, transport du grain seulement quand il n'y a plus de travail de culture disponible |
| 2 | Grain laissé au champ | attend sans perte |
| 3 | Prélèvement | pris sur le champ à la fin de la récolte (et sur le bois à la coupe) ; seul le net est transporté, par la pop |
| 4 | Régénération des forêts | d'un coup au 1ᵉʳ janvier, pas d'effet de la météo |
| 5 | Stock initial | 12 grains, 7 bois, 2 vêtements |
| 6 | Vêtement | usure aléatoire de 20 % par mois |
| — | Étapes ratées | un semis non fait avant mai, une récolte non faite avant septembre ou un labour inachevé en décembre font retomber le champ en friche |
| — | Choix du champ | celui dont le travail est entamé, sinon le plus proche |

### Constats

- Avec une météo moyenne, le moteur retrouve les chiffres du doc §8 : grain net 12,75 pour 12 mangés (+0,75 par an), 100 j de culture, 33 j de transport du grain, 41 j de bois.
- **Aucun manque de grain** sur 20 graines × 100 ans (0,3 % d'années) : le stock initial et le surplus moyen amortissent la météo. Le « très tendu » du doc ne se voit pas dans ces conditions ; à régler par le stock initial ou le prélèvement.
- **Vêtement** : avec l'usure aléatoire, un stock de 2 et 20 j de couture, deux usures rapprochées vident parfois le stock (7 vêtements manquants en 50 ans) ; certaines années demandent 100 j de couture. Le compteur d'usure déterministe (point 6, alternative) lisserait cela.

## Suite possible

| # | Concept | Test de trajectoire attendu |
|---|---|---|
| 2 | Conséquences d'un stock vide (famine, mortalité, état d'esprit) | population stationnaire autour d'un plafond fixé par la terre |
| 3 | Naissances | idem, avec une année de disette qui se voit sur la population |
| 4 | Plusieurs pops et plusieurs fermes | plafond proportionnel aux terres |

L'ordre est provisoire ; chaque étape doit être décidée dans un doc de concept avant d'être codée.
