# 04 — Périmètre du POC

## Objectif

Une **économie de subsistance** : un village isolé de **1750**, simulé de façon autonome (sans intervention du joueur). Des pops vivent dans des hameaux, travaillent des terres exploitées alentour, et certains hameaux fabriquent sur place de quoi vêtir tout le monde. La couverture de leurs besoins pilote la croissance ou le déclin de chaque pop, et **les habitants étendent eux-mêmes le village** (défrichement, nouveaux hameaux) quand il le faut. Pas de monnaie, pas de manufacture, pas de firme : le maire est inactif.

**Deux étapes**

| Étape | Contenu |
|---|---|
| **POC-A — subsistance** | Production continue (pas de saisons), circulation des marchandises (production, stock, distribution, consommation), besoins vitaux et de confort, croissance par jauge de pop, réorientation des bras selon le besoin, **extension du village par ses habitants** (voir `02` §7). Village initial généré. |
| **POC-B — monnaie et marché** | Salaires, prix, firmes, surplus qui quitte le village (dîme, marché), placement des bâtiments par des firmes. |

**Critères de succès**
- La simulation tourne indéfiniment sans blocage ni dérive absurde.
- Les quantités produites / consommées restent proportionnellement cohérentes (conservation de la matière vérifiée).
- Les dynamiques de subsistance émergent : croissance lente (+0,5 %/an quand le confort manque, +1,5 %/an quand tout est couvert), déclin en cas de manque vital, pénuries localisées, production qui s'ajuste au besoin (les bras se déplacent vers ce qui manque), **extension du village qui plafonne d'elle-même** (la terre lointaine est plus pauvre), population en « dents de scie » autour de ce plafond.
- **Chaque mécanisme est lisible** par le joueur (voir « Ce que voit le joueur »).

## Périmètre

| Élément | POC |
|---|---|
| Carte | 200×200 tiles, **un village généré** (graine, aléatoire mais cohérent, voir ci-dessous), vue micro uniquement |
| Personnage | Un seul : **le maire**, **inactif** (aucune décision, ne construit pas). Ni leviers (pouvoir / capital / influence), ni postes, ni fortune dans le POC |
| Population | **Pops de 100 hab**, une seule strate (paysans), une seule culture / religion / orientation, attributs fixes. **Une jauge de croissance par pop** : à +100 hab elle se duplique, à −100 elle disparaît (voir `02` §6). La satisfaction du confort servira plus tard à la promotion vers d'autres classes |
| Habitat | Deux types : le **hameau** (**2 pops = 200 hab**, toutes travaillent aux terres alentour) et le **hameau textile** (**3 pops = 300 hab**, dont 1 travaille à l'atelier sur place). Plafond de densité d'une tile d'habitat : 10 pops |
| Production | Des **terres exploitées** sans logement (champ, pâturage, verger, forêt, lin, laine), travaillées par les pops des hameaux ; l'**atelier textile** des hameaux textiles. Ni filature ni atelier de couture indépendants (voir `05`) |
| Monnaie | Aucune (autoconsommation, échanges en nature entre voisins) |
| Construction | Par la **communauté** : défrichement de terres et bâtiment de hameaux, par des équipes de pops, avec du bois, à rythme saisonnier (voir `02` §7) |
| Firmes | Aucune en POC-A |
| Terrain | Un seul : **plaine** (herbe) |
| Infrastructure | Une seule : **chemin de terre** (orthogonal) |
| Temps | Production **continue**, pas de saisons |
| Niveau technologique | 0 (production primitive) |
| Transport | Le lien **pop → hameau de résidence → lieu de travail** est matérialisé. **Distance = nombre de tiles chemin à emprunter**, bâtiment accolé à un chemin, aucune pénalité mais **limite de 10 tiles** pour travailler ou s'approvisionner, et approvisionnement au plus proche. Congestion et livraisons différées : **en sommeil** (capacités très au-dessus des flux) |
| Intermédiaires | Aucun (relation directe fournisseur ↔ client) |

## Ressources

| Catégorie | Ressources POC | Reporté |
|---|---|---|
| Nourriture | grain, viande, fruit — **interchangeables** pour le besoin vital (unités nutritives) | poisson ; nourriture industrielle |
| Textile | lin, laine → tissu → vêtements, **à l'atelier du hameau textile** | coton ; manufacture |
| Bois | bois (chauffage), consommé directement | transformation |
| Travail | les pops | qualifications |

## Catalogue des tiles (identifiants d'assets)

`terrain_plaine` (10 variantes d'herbe raccordables) · `infra_chemin_terre` (POC : **orthogonal seulement**, 15 combinaisons de demi-directions indexées par masque ; diagonales plus tard) ·
`bat_champ` (3) · `bat_champ_mur` (réservé aux saisons) · `bat_paturage` (2) · `bat_verger` (2) · `bat_foret_exploitee` (3) · `bat_champ_lin` (2) · `bat_elevage_laine` (2) · `bat_hameau` (3) · `bat_hameau_textile` (2) · `bat_chantier_hameau` (2) · `bat_chantier_champ` (2)

49 tiles, générées par `graphGeneration/` (voir `03` §7) ; aperçu : `docs/img/atlas-assets.png` et `docs/img/apercu-village-assets.png`. Un état d'avancement du chantier (fondations, charpente…) reste à prévoir.

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

### Atelier textile (hameau textile)

**1 pop** du hameau y travaille, sur place (distance 0). Un hameau textile = 3 pops : 1 à l'atelier, 2 aux terres alentour. Une pop à l'atelier fait **4 vêtements par jour**, soit les besoins de 8 pops :

| Étape | Intrants | Extrants (1 pop) |
|---|---|---|
| Tissage | 8 lin + 4 laine | 6 tissu |
| Couture | 6 tissu | 4 vêtements |

(Recettes : 4 lin + 2 laine → 3 tissu ; 3 tissu → 2 vêtements.) Le nombre de hameaux textiles est **déduit** des besoins : pour 10 hameaux, 3 hameaux textiles (23 pops de capacité, ~11,5 vêtements/jour, 12 produits au maximum).

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

### Fertilité, distance et extension
- **Fertilité** d'une tile : bruit lissé entre 0,85 et 1,3, qui s'effondre autour du centre du village (pleine jusqu'à 3 tiles, puis déclin linéaire jusqu'à 0,15 à 7 tiles). Elle multiplie la production des terres exploitées.
- **Distance maximale** : 10 tiles chemin entre deux lieux (travail, approvisionnement). À calibrer.
- **Coûts de création** (travail en pop-jours, équipe, bois) : voir `02` §7.
- Avec ces valeurs, le village plafonne vers **60 à 90 pops** (6 000 à 9 000 hab.) en « dents de scie », après plusieurs siècles. Un terroir plus large plafonne plus haut (170 pops à 4/9, 240 pops à 5/11).

### Proportions qui en découlent
- Un village de **10 hameaux** (dont 3 textiles, 23 pops) demande 1 champ, 1 pâturage, 1 verger, 1 forêt, 1 champ de lin et 1 élevage de laine, soit **19 pops de travail + 3 à l'atelier**, pour 23 pops. Les bras ne sont pas en excès : c'est l'image d'une société agraire.
- Le générateur démarre à 80 % de la capacité de logement. La croissance est d'abord limitée par le **logement**, puis par la **nourriture** à mesure que l'on gagne sur des terres plus pauvres.

### ⚠️ Points de calibration
- **Tranché** : bloc discret de 100 hab (`00` §3), effectifs en pops, hameau 2 pops et hameau textile 3 pops (1 à l'atelier), plafond de densité 10, recettes textiles, nourriture interchangeable, taux de croissance 1,5 % / 0,5 % / −8 %, distance = tiles chemin avec limite, extension par la communauté.
- **À calibrer** avec le CLI headless : seuils vitaux, `mortalite_famine`, `distance_max`, coûts de création, fertilité (rayons du terroir), taille de village de départ.
- **Plus tard** : POC-B (prix, salaires, seuils) ; aléa de récolte ; saisons ; rendements décroissants par pop ; abandon des lieux en déclin.

## Générateur de village

- **Déterministe** (même graine, même village), progressif : un village de 1750 à évolution lente et production quasi stable, puis un deuxième village avec liens économiques, puis un troisième, puis une ville. Simple en POC, il gagnera en finesse à chaque étape.
- **Disposition en croix** de chemins de terre, compacte pour rester sous la distance limite ; hameaux et terres accolés aux bras, ou au bout d'une antenne d'une tile. Si une disposition viole les contraintes d'accès, une autre est tirée de façon déterministe.
- Dimensionné pour être **proche de l'équilibre** : le nombre de terres de chaque type et de hameaux textiles est déduit des besoins (jamais un ratio codé), pops initiales à 80 % de la capacité de logement, stocks initiaux non nuls, jauges de croissance décalées.
- Champ de **fertilité** tiré avec la graine, centré sur le village.

## Outillage de test et d'observation

- Crate Rust `sim` **pure** (sans dépendance napi), **CLI headless** : N années en quelques secondes, séries temporelles en CSV.
- **Invariants automatisés** : pas de stock négatif, conservation de la matière (produit = consommé + Δstock + pertes), équilibre malthusien comme oracle (une terre moins productive abaisse la population d'équilibre), pas de dérive sur de longues durées, déterminisme.
- **Déterminisme** : quantités en entiers, itérations ordonnées.
- Paramètres (bâtiments, recettes, besoins) en **fichiers de données**, pas en dur.

## Ce que voit le joueur (POC)

Chaque indicateur est affiché avec son **facteur limitant** (le moteur enregistre quel `min()` a mordu : main-d'œuvre, intrant, stock plein).

| Niveau | Contenu |
|---|---|
| Hameau | Ses pops et leur lieu de travail, état de chaque pop (satisfaite / insatisfaite / en manque), couverture de chaque ration (reçu/requis, et auprès de quel fournisseur), jauge de croissance, atelier textile sur place |
| Terre exploitée | Production effective %, **fertilité de la tile**, facteur limitant, stocks, pops affectées et leurs hameaux, clients |
| Chantier | Ce qui est construit, travail restant, équipe présente, bois reçu, **motif** (« grain manque, 2 pops sans poste ») |
| Pop sans poste | Pastille distincte dans la liste du hameau |
| Modes de carte | Couverture des besoins, pénuries, utilisation des terres, **fertilité**, portée (tiles chemin) autour d'un lieu |
| Global | Date, vitesse, courbes (pop, production vs consommation), journal d'événements (naissance, disparition, bras réorientés vers…, **chantier lancé / terminé**) |
| Temps | Jour/nuit par teinte globale ; une pop = un sprite, au travail le jour, au logement la nuit |
| Debug | Choc de récolte (« mauvaise récolte »), modifier un paramètre en direct |

## Prochaine étape de design identifiée

**Représentation visuelle** : voir le village vivre (carte, vitesse de simulation, informations par tile). Le **déclin et l'abandon** des lieux, les **saisons**, l'**aléa de récolte** et les autres fonctionnalités écartées sont consignés dans `05` (« Fonctionnalités écartées du POC-A ») et seront repris plus tard. Puis POC-B : **placement des bâtiments par des firmes**, généralisation de l'extension communautaire.
