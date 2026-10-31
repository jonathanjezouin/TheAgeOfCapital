# 04 — Périmètre du POC

## Objectif

Une **simulation économique autonome** qui tourne seule, sans intervention du joueur : des pops travaillent, produisent et accumulent des ressources, qui satisfont (ou non) leurs besoins ; la satisfaction pilote la natalité et la mortalité. Le maire est inactif.

**Deux étapes**

| Étape | Contenu |
|---|---|
| **POC-A — boucle physique** | Production, flux, congestion, consommation directe des pops (sans monnaie), satisfaction, démographie. Carte initiale fixe, personne ne construit. |
| **POC-B — monnaie** | Salaires, prix, profit des firmes, réinvestissement, rachat, faillite, construction (placement automatique). |

**Critères de succès**
- La simulation tourne indéfiniment sans blocage ni dérive absurde.
- Les quantités produites / consommées restent proportionnellement cohérentes.
- Les premières dynamiques émergent : pénuries localisées, goulots (chaîne textile), congestion, croissance puis saturation des quartiers selon la satisfaction des besoins.
- POC-B : firmes qui réinvestissent ou font faillite, croissance de nouveaux quartiers.

## Périmètre

| Élément | POC |
|---|---|
| Carte | 200×200 tiles, une seule ville, vue micro uniquement |
| Personnage | Un seul : **le maire**, **inactif** (aucune décision, ne construit pas). Ni leviers (pouvoir / capital / influence), ni postes, ni fortune dans le POC |
| Population | **Pops de 100 hab** : 10 pops « paysans » au départ par quartier rural, une seule culture / religion / orientation, attributs fixes. **Natalité / mortalité selon la satisfaction des besoins** (nouvelle pop à +100, disparition à −100, plafond = capacité de logement). Pas de mobilité ni de bascule |
| Monnaie | POC-A : aucune (consommation directe des pops). POC-B : salaires et prix |
| Construction | POC-A : aucune, carte initiale fixe. POC-B : réinvestissement des firmes |
| Firmes | Autonomes, règles simples (voir `02`) |
| Terrain | Un seul : **plaine** (herbe) |
| Infrastructure | Une seule : **chemin de terre** |
| Habitation | Une seule : **quartier rural** (capacité 1 000 hab = 10 pops) |
| Niveau technologique | 0 (production primitive) |
| Transport | Implicite |
| Intermédiaires | Aucun (relation directe fournisseur ↔ client) |
| Livraison différée sur congestion | **Oui** |

## Ressources

| Catégorie | Ressources POC | Reporté |
|---|---|---|
| Nourriture | grain, viande, fruit — consommés directement (pas de farine) | poisson ; nourriture industrielle |
| Textile | lin, laine → tissu → vêtements | coton |
| Bois | bois, consommé directement | transformation |
| Travail | `MAIN_OEUVRE` | qualifications |

## Catalogue des tiles (identifiants d'assets)

`terrain_plaine` · `infra_chemin_terre` (toutes combinaisons de demi-directions) ·
`bat_champ` · `bat_paturage` · `bat_verger` · `bat_foret_exploitee` · `bat_champ_lin` · `bat_elevage_laine` · `bat_filature` · `bat_atelier_couture` · `bat_quartier_rural`

## Calibration (effectifs 100–500, unités rondes)

Ce qui compte est la **proportion**, pas la valeur absolue.

### Production

| Bâtiment | Travailleurs | Production / jour | Productivité (u/travailleur) |
|---|---|---|---|
| Champ (grain) | 200 | 100 grain | 0,5 |
| Pâturage (viande) | 100 | 20 viande | 0,2 |
| Verger (fruit) | 150 | 30 fruit | 0,2 |
| Exploitation forestière | 200 | 50 bois | 0,25 |
| Champ de lin | 200 | 40 lin | 0,2 |
| Élevage laine | 100 | 20 laine | 0,2 |
| Filature / tissage | 300 | 30 tissu | 0,1 |
| Atelier couture | 200 | 20 vêtements | 0,1 |

Logique : agriculture céréalière extensive et productive par tête ; élevage peu productif ; transformation textile intensive en main-d'œuvre (beaucoup de bras, lent).

### Consommation d'un quartier rural (1 000 hab)

| Ressource | Par jour |
|---|---|
| Grain | 50 |
| Viande | 10 |
| Fruit | 10 |
| Bois | 20 |
| Vêtements | 5 |

### Besoins, satisfaction et démographie
- Consommation par pop de 100 hab : 5 grain, 1 viande, 1 fruit, 2 bois, 0,5 vêtement par jour.
- Satisfaction, natalité et mortalité : formules dans `02` §6.
- À calibrer : `poids_r` des besoins, fonctions `taux_naissance` / `taux_mortalite`, seuil de ±100 pour créer ou supprimer une pop.

### Proportions qui en découlent
- ~**7 tiles de production pour 2 quartiers ruraux**.
- La **chaîne textile** (fibre → filature → couture) est le goulot naturel : une chaîne complète (20 vêtements/jour) couvre ~4 quartiers. Première filière candidate à l'intégration verticale.
- Un quartier supplémentaire sans production supplémentaire crée une tension sur la main-d'œuvre → incitation à investir.

### ⚠️ Points de calibration non tranchés
- **Main-d'œuvre disponible par quartier** : l'ancienne valeur (250 actifs / 1 000 hab) date de la première calibration (effectifs 15–50). Elle est **incompatible** avec les effectifs actuels (la liste ci-dessus totalise 1 450 travailleurs). Recalibrer : soit le taux d'actifs, soit le nombre de quartiers nécessaires.
- **Affectation pop → lieu de travail** : **tranché**, la pop est un bloc discret affecté à un seul lieu (voir `00` §3). Reste à calibrer : le `taux_activité` (main-d'œuvre fournie par pop) et la granularité de l'affectation (effectifs des bâtiments vs actifs d'une pop). À mesurer : coût de simulation du facteur ×10 en objets.
- **Ratios d'intrants de transformation** non fixés (combien de fibre par tissu, de tissu par vêtement).
- POC-B : prix de départ, salaires de référence, seuils de réinvestissement / rachat / faillite : à définir.

## Prochaine étape de design identifiée

**Algorithme de placement automatique des bâtiments** (étape « construction », saisonnière ; POC-B, car personne ne construit en POC-A) — identifié comme le cœur de l'émergence spatiale, jamais encore traité. Il devra préciser qui décide (maire ? firmes ?), comment on évalue un emplacement (accès aux intrants, à la main-d'œuvre, aux clients, coût de l'infrastructure à créer), et comment se crée l'infrastructure associée.
