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
- **Manufacture textile** (filature, atelier de couture) : le textile se fait à l'atelier d'un hameau en 1750 ; c'est la première filière où le capitalisme émergera (marchand-fabricant)
- Saisons (travail d'hiver : filage, couture), stocks annuels (greniers), **aléa de récolte** (outil de debug en attendant)
- Rendements décroissants des terres, fertilité variable par tile, droits d'usage communaux du bois, prélèvements (dîme, rente, impôts) : le surplus qui quitte le village
- Promotion de classe par la satisfaction du confort
- Abandon des lieux en déclin (hameaux vides, terres sans bras), délai de production d'un verger
- **Lumière en rotation** : shader de ré-éclairage à partir de l'albedo et des normales des hameaux (déjà générés, voir `03` §7) ; relief des autres sprites (arbres, bâtiments futurs)
- Avancement visible d'un chantier (plusieurs états)
- Congestion et livraisons différées : code présent mais **en sommeil** (capacités très au-dessus des flux dans le POC)
- Poisson, coton, transformation du bois, nourriture industrielle
- Autres terrains, infrastructures (route pavée, canal, rail, port), quartiers (artisan, bourgeois, urbain)
- Niveaux technologiques > 0
- Niveaux de zoom méso et macro

## Fonctionnalités écartées du POC-A (à reprendre)

| Sujet | État actuel | Pourquoi on y reviendra |
|---|---|---|
| **Déclin et abandon** des lieux | Un hameau vidé reste debout, une terre sans bras reste inactive, un chantier sans équipe attend | Réalisme du recul démographique ; réutilisation des lieux ; lisibilité de la carte |
| **Aléa de récolte** / famines ponctuelles | Seul le test « grain ÷ 4 » ; pas de choc extérieur | Source naturelle des dents de scie historiques ; outil de debug prévu |
| **Saisons**, stocks annuels (greniers) | Production continue | Travail d'hiver (filage, couture), soudure, famines de fin d'hiver |
| **Délai de production d'un verger** | Immédiat à la fin du chantier | Investissement lent réaliste |
| **Réaffectation d'une équipe de chantier** vers un besoin urgent | L'équipe finit son chantier | Réactivité de la communauté |
| **Rendements décroissants par pop** | Effectif fixe par terre, production proportionnelle à la fertilité | Sous-emploi agraire réaliste |
| **Le confort freine la croissance** | Non : une pop insatisfaite croît à +0,5 %/an | À reconsidérer avec la promotion de classe |
| **Prélèvements** (dîme, rente, impôts) | Aucun | Le surplus qui quitte le village : amorce du marché et du capital |

## Questions ouvertes

1. **Déclin et abandon** — *écarté pour l'instant, à reprendre plus tard*. L'extension communautaire est faite (`02` §7) ; reste à traiter ce qui se passe quand la population baisse durablement : hameaux vides, terres sans bras, équipes de chantier orphelines, mémoire des lieux abandonnés (et leur réutilisation).
1b. **Placement automatique des bâtiments par les firmes** (POC-B) : généralisation de l'extension communautaire, avec des critères de rentabilité à la place des besoins de subsistance.
2. **Formation des prix** : simple offre/demande hebdomadaire ? négociation des contrats cadre ? prix plancher ?
3. **Qui construit** : personne en POC-A ; en POC-B, les firmes (réinvestissement) et/ou le maire ? Articulation des deux logiques.
4. **Équilibre habitat / production** : émerge de l'extension communautaire et de la fertilité (voir `02` §7) ; à observer sur plusieurs graines et plusieurs terroirs.
4b. **Le confort ne freine pas la croissance** : une pop insatisfaite croît à +0,5 %/an. Le village peut donc plafonner en laissant son confort s'effondrer (viande, vêtements). Voulu pour le POC ; à reconsidérer avec la promotion de classe.
5. **Personnages** : principe retenu (postes, dévolution, voir `00`) ; restent à préciser la succession du joueur à sa mort, l'équilibrage des rôles et les règles de bascule des pops.
7. **Calibration démographique et des besoins** : seuils vitaux, capacité du hameau (1 ou 2 pops), `mortalite_famine`, taille de village de départ (voir `02` §6 et `04`).
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
| Emprise des bâtiments | Usage unique par tile | « Bâtiments sur plusieurs tiles » (vision) ; hameau textile = habitat + atelier | Une tile = un bâtiment dans le POC, sauf le hameau (exception documentée, `02` §1) |
| Taille de tile | 50–200 m², puis ~1 km² | Unité comptable à surface implicite variable | Unité comptable (voir `02`) |
