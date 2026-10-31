# 05 — Roadmap, reports et questions ouvertes

## Roadmap par étapes

| Étape | Contenu | Valide |
|---|---|---|
| **1 — POC** | 200×200, une ville ; POC-A boucle physique (pops, besoins, démographie), puis POC-B monnaie et firmes ; maire inactif | Mécanique économique de base |
| **2 — Méso** | Plusieurs villes, échanges inter-cités abstraits | L'échelle régionale |
| **3 — Monde** | Carte monde en tiles macro, personnages, politique | Le jeu complet |

Chaque étape doit pouvoir fonctionner comme un jeu en soi.

## Explicitement reporté après le POC

- Maintenance des infrastructures par des firmes municipales (frein à la prolifération des routes)
- Propriété des infrastructures (routes souvent publiques ; péages)
- Firmes de transport ; prestataire de transport inclus dans le contrat
- Marchés, entrepôts, négociants (nœuds de distribution)
- Qualité différenciée des marchandises avant la standardisation
- Capital financier, crédit, banques, sociétés par actions
- Économie politique : lois, régimes, rôles du joueur, partis, syndicats, révolutions
- Personnages complets : postes et dévolution, fortune, trois leviers (pouvoir / capital / influence), énergie / santé, émergence (voir `00`)
- Dramaturgie des personnages (mariages, traits, intrigues) : souhaitable, pas dans les premières versions
- Pops : natalité, mobilité (résidence, emploi, strate), bascule culturelle / religieuse / d'orientation / alphabétisation par jauge de pression et seuil (voir `00` §3)
- Contrats de travail, licenciements, négociation collective
- Logique propre des groupes (stratégies : expansion, prédation…)
- Valeur foncière et ségrégation spatiale
- Couche narrative SLM
- Poisson, coton, transformation du bois, nourriture industrielle
- Autres terrains, infrastructures (route pavée, canal, rail, port), quartiers (artisan, bourgeois, urbain)
- Niveaux technologiques > 0
- Niveaux de zoom méso et macro

## Questions ouvertes

1. **Placement automatique des bâtiments** — prochain gros sujet (voir `04`).
2. **Formation des prix** : simple offre/demande hebdomadaire ? négociation des contrats cadre ? prix plancher ?
3. **Qui construit** : personne en POC-A ; en POC-B, les firmes (réinvestissement) et/ou le maire ? Articulation des deux logiques.
4. **Équilibre habitat / production** (le village généré le résout en POC-A) avec des effectifs en pops (29 pops pour la chaîne complète vs 10 pops par quartier, voir `04`).
5. **Personnages** : principe retenu (postes, dévolution, voir `00`) ; restent à préciser la succession du joueur à sa mort, l'équilibrage des rôles et les règles de bascule des pops.
7. **Calibration démographique** : fonctions `taux_naissance` / `taux_mortalite`, poids des besoins, seuil de ±100 (voir `02` §6 et `04`).
8. **Monnaie** (POC-B) : salaires, budget des pops, formation des prix ; articulation avec la consommation directe du POC-A.
6. **Monde extérieur** : abstraction type marché mondial vs simulation multi-niveaux.

## Incohérences relevées dans les échanges de design (à trancher)

| Sujet | Version ancienne | Version la plus récente | Proposition |
|---|---|---|---|
| Granularité temporelle | Flux de main-d'œuvre **horaire** | Tick économique **quotidien**, journaliers | Garder le quotidien ; l'horloge quart d'heure ne sert qu'au rythme d'affichage |
| Tick de la boucle | 1 mois / 1 saison | Horloge 15 min, multi-fréquences | Multi-fréquences (voir `02`) |
| Taille de carte POC | 150×150 | 200×200 | 200×200 |
| Ressources | bois transformé, charbon, minerai… | nourriture / textile / bois | Périmètre de `04` |
| Décision économique | Firmes autonomes | Personnages qui dirigent les firmes | POC : firmes à règles simples (POC-B) + maire inactif ; personnages plus tard |
| Emprise des bâtiments | Usage unique par tile | « Bâtiments sur plusieurs tiles » (vision) | Une tile = un bâtiment dans le POC |
| Taille de tile | 50–200 m², puis ~1 km² | Unité comptable à surface implicite variable | Unité comptable (voir `02`) |
