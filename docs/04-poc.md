# 04 — Périmètre du POC

## Objectif

Une **économie de subsistance** : un hameau isolé de **1750**, simulé de façon autonome (sans intervention du joueur). Des pops vivent à la campagne, travaillent des terres exploitées et fabriquent chez elles ce qu'il faut pour se vêtir. La couverture de leurs besoins pilote la croissance ou le déclin de chaque pop. Pas de monnaie, pas d'atelier, pas de firme : le maire est inactif.

**Deux étapes**

| Étape | Contenu |
|---|---|
| **POC-A — subsistance** | Production continue (pas de saisons), circulation des marchandises (production, stock, distribution, consommation), besoins vitaux et de confort, croissance par jauge de pop, réorientation des bras selon le besoin. Carte initiale générée, personne ne construit. |
| **POC-B — monnaie et marché** | Extension du village (défrichement, nouveaux hameaux, voir `05`), puis salaires, prix, firmes, surplus qui quitte le village. |

**Critères de succès**
- La simulation tourne indéfiniment sans blocage ni dérive absurde.
- Les quantités produites / consommées restent proportionnellement cohérentes (conservation de la matière vérifiée).
- Les dynamiques de subsistance émergent : croissance lente (+0,5 %/an quand le confort manque, +1,5 %/an quand tout est couvert), déclin en cas de manque vital, pénuries localisées, production qui s'ajuste au besoin (les bras se déplacent vers ce qui manque).
- **Chaque mécanisme est lisible** par le joueur (voir « Ce que voit le joueur »).

## Périmètre

| Élément | POC |
|---|---|
| Carte | 200×200 tiles, **un village généré** (graine, aléatoire mais cohérent, voir ci-dessous), vue micro uniquement |
| Personnage | Un seul : **le maire**, **inactif** (aucune décision, ne construit pas). Ni leviers (pouvoir / capital / influence), ni postes, ni fortune dans le POC |
| Population | **Pops de 100 hab**, une seule strate (paysans), une seule culture / religion / orientation, attributs fixes. **Une jauge de croissance par pop** : à +100 hab elle se duplique, à −100 elle disparaît (voir `02` §6). La satisfaction du confort servira plus tard à la promotion vers d'autres classes |
| Habitat | Une seule : le **hameau** (tile de vie à faible densité, **2 pops = 200 hab**), qui embarque le travail textile domestique |
| Production | Des **terres exploitées** sans logement (champ, pâturage, verger, forêt, lin, laine), travaillées par les pops des hameaux. Ni filature ni atelier (voir `05`) |
| Monnaie | Aucune (autoconsommation, échanges en nature entre voisins) |
| Construction | Aucune en POC-A : hameaux et terres fixes. Le défrichement et la fondation de hameaux sont à définir (voir `05`) |
| Firmes | Aucune en POC-A |
| Terrain | Un seul : **plaine** (herbe) |
| Infrastructure | Une seule : **chemin de terre** (orthogonal) |
| Temps | Production **continue**, pas de saisons |
| Niveau technologique | 0 (production primitive) |
| Transport | Le lien **pop → hameau de résidence → lieu de travail** est matérialisé et le coût de distance détermine le fournisseur le plus proche. Congestion et livraisons différées : **en sommeil** (capacités très au-dessus des flux) |
| Intermédiaires | Aucun (relation directe fournisseur ↔ client) |

## Ressources

| Catégorie | Ressources POC | Reporté |
|---|---|---|
| Nourriture | grain, viande, fruit — **interchangeables** pour le besoin vital (unités nutritives) | poisson ; nourriture industrielle |
| Textile | lin, laine → tissu → vêtements, **à domicile** | coton ; manufacture |
| Bois | bois (chauffage), consommé directement | transformation |
| Travail | les pops | qualifications |

## Catalogue des tiles (identifiants d'assets)

`terrain_plaine` (8 à 12 variantes d'herbe raccordables) · `infra_chemin_terre` (POC : **orthogonal seulement**, 15 combinaisons de demi-directions ; diagonales plus tard) ·
`bat_champ` · `bat_paturage` · `bat_verger` · `bat_foret_exploitee` · `bat_champ_lin` · `bat_elevage_laine` · `bat_hameau`

Reportés : `bat_filature`, `bat_atelier_couture` (manufacture), `bat_quartier_rural` (bassin de peuplement de 1 000 hab).

## Calibration (effectifs en pops, unités rondes)

Ce qui compte est la **proportion**, pas la valeur absolue. Effectifs en **pops** (1 pop ≈ 50 travailleurs réels, équivalence de calibration, voir `00` §3). Toutes les valeurs vivent dans `sim/donnees/poc.toml`.

### Terres exploitées

| Lieu | Pops | Production / jour | Par pop |
|---|---|---|---|
| Champ (grain) | 4 | 100 grain | 25 |
| Pâturage (viande) | 2 | 20 viande | 10 |
| Verger (fruit) | 3 | 30 fruit | 10 |
| Exploitation forestière | 4 | 50 bois | 12,5 |
| Champ de lin | 4 | 40 lin | 10 |
| Élevage laine | 2 | 20 laine | 10 |

### Travail textile à domicile (dans chaque hameau)

Fait par les **résidents** du hameau, sans poste ni trajet (travail d'appoint, tout au long de l'année dans ce POC sans saisons). Capacité proportionnelle au nombre de résidents (référence : 2) :

| Étape | Intrants | Extrants (à 2 résidents) |
|---|---|---|
| Tissage | 3 lin + 1,5 laine | 2,25 tissu |
| Couture | 2,25 tissu | 1,5 vêtement |

(Recettes : 4 lin + 2 laine → 3 tissu ; 3 tissu → 2 vêtements.) Un hameau plein produit 1,5 vêtement/jour pour 1 consommé : le surplus peut partir chez les voisins.

### Besoins d'une pop (par jour)

| Ressource | Ration normale | Rôle |
|---|---|---|
| Grain | 5 | Nourriture (1 unité nutritive par unité) |
| Viande | 1 | Nourriture **et** part de viande attendue (confort) |
| Fruit | 1 | Nourriture |
| Bois | 2 | Chauffage (vital) |
| Vêtements | 0,5 | Minimum de confort |

| Besoin | Seuil | Effet |
|---|---|---|
| **Vital** : nourriture | 5 unités nutritives (n'importe quelle combinaison de grain, viande, fruit) | Sous le seuil : déclin |
| **Vital** : chauffage | 2 bois | Sous le seuil : déclin |
| **Confort** : viande | 1 | Sous le minimum : insatisfaite |
| **Confort** : vêtements | 0,5 | Sous le minimum : insatisfaite |

| État de la pop | Croissance annuelle |
|---|---|
| Vital et confort couverts (**satisfaite**) | **+1,5 %** |
| Vital couvert, confort insuffisant (**insatisfaite**) | **+0,5 %** (interpolé selon le confort) |
| Vital insuffisant (**manque**) | de +0,5 % à **−8 %** quand le vital tombe à zéro (linéaire) |

### Proportions qui en découlent
- Un village de **10 hameaux** (20 pops, 2 000 hab) demande 1 champ, 1 pâturage, 1 verger, 1 forêt, 1 champ de lin et 1 élevage de laine, soit **19 pops de travail** pour 20 pops. À ce stade, les bras ne sont pas en excès : tout le monde travaille, c'est l'image d'une société agraire.
- Le hameau étant plein à 2 pops, la croissance est d'abord limitée par le **logement**, pas par la nourriture : l'extension du village est la prochaine mécanique à définir (`05`).

### ⚠️ Points de calibration
- **Tranché** : bloc discret de 100 hab (`00` §3), effectifs en pops, recettes textiles, nourriture interchangeable, taux de croissance 1,5 % / 0,5 % / −8 %.
- **À calibrer** avec le CLI headless : seuils vitaux, capacité du hameau (1 ou 2 pops), taille de village de départ, `mortalite_famine`.
- **Plus tard** : POC-B (prix, salaires, seuils de réinvestissement) ; aléa de récolte ; rendements décroissants des terres (voir `05`).

## Générateur de village

- **Déterministe** (même graine, même village), progressif : un hameau de 1750 à évolution lente et production quasi stable, puis un deuxième village avec liens économiques, puis un troisième, puis une ville. Simple en POC, il gagnera en finesse à chaque étape.
- Dimensionné pour être **proche de l'équilibre** : le nombre de terres de chaque type est déduit des besoins des hameaux (jamais un ratio codé), pops initiales à 80 % de la capacité de logement (donc quelques postes vacants), stocks initiaux non nuls, jauges de croissance décalées pour éviter les duplications synchrones.
- Réseau de chemins connexe reliant hameaux et terres (chemin principal et antennes).

## Outillage de test et d'observation

- Crate Rust `sim` **pure** (sans dépendance napi), **CLI headless** : N années en quelques secondes, séries temporelles en CSV.
- **Invariants automatisés** : pas de stock négatif, conservation de la matière (produit = consommé + Δstock + pertes), équilibre malthusien comme oracle (une terre moins productive abaisse la population d'équilibre), pas de dérive sur de longues durées, déterminisme.
- **Déterminisme** : quantités en entiers, itérations ordonnées.
- Paramètres (bâtiments, recettes, besoins) en **fichiers de données**, pas en dur.

## Ce que voit le joueur (POC)

Chaque indicateur est affiché avec son **facteur limitant** (le moteur enregistre quel `min()` a mordu : main-d'œuvre, intrant, stock plein).

| Niveau | Contenu |
|---|---|
| Hameau | Ses pops et leur lieu de travail, état de chaque pop (satisfaite / insatisfaite / en manque), couverture de chaque ration (reçu/requis, et auprès de quel fournisseur), jauge de croissance, travail textile à domicile |
| Terre exploitée | Production effective %, facteur limitant, stocks, pops affectées et leurs hameaux, clients |
| Pop sans poste | Pastille distincte dans la liste du hameau |
| Modes de carte | Couverture des besoins, pénuries, utilisation des terres |
| Global | Date, vitesse, courbes (pop, production vs consommation), journal d'événements (naissance, disparition, bras réorientés vers…) |
| Temps | Jour/nuit par teinte globale ; une pop = un sprite, au travail le jour, au logement la nuit |
| Debug | Choc de récolte (« mauvaise récolte »), modifier un paramètre en direct |

## Prochaine étape de design identifiée

**Extension du village par ses habitants** (défrichement de nouvelles terres, fondation de hameaux) : mécanisme communautaire, saisonnier, sans monnaie. Proposition à valider dans `05`. Elle précède le placement automatique par des firmes (POC-B), qui en sera la généralisation.
