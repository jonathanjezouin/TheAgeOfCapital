# 06 — Fertilité : réflexions pour la cible (hors POC)

> Statut : **notes de réflexion du 2026-11-03**, rien n'est implémenté. Le POC garde **une seule valeur de fertilité par tile** (`Tuile.fertilite_ppm`, voir `04` « Fertilité, distance et extension »). Les éléments historiques ci-dessous viennent de connaissances générales d'histoire agraire (Europe de l'ouest, vers 1750) : **à vérifier sur sources avant toute calibration**. Aire géographique de la carte : **plaine à champs ouverts** (décision du 2026-11-04) ; types de sol, couches de terrain et règles de peuplement dans `07`.

## 1. Idée centrale : l'aptitude, pas un scalaire
La fertilité n'est pas une qualité unique : c'est une **aptitude par type de terre**, calculée à partir de quelques couches de terrain. Une même tile peut être excellente pour un pré et médiocre pour du blé.

| Couche de terrain | Effet |
|---|---|
| **Sol** (limon, argile lourde, sable, calcaire, tourbe) | limon bien drainé : céréales ; argile lourde : dure à travailler avec les outils de l'époque ; sable : pauvre, plutôt forêt, lande ou seigle ; calcaire : coteaux, vigne, élevage ovin |
| **Humidité** (distance à la rivière, nappe, drainage) | fond de vallée : prés et fourrage (souvent inondable, donc pas toujours le meilleur champ de céréales) ; sols secs : céréales si le sol est bon |
| **Pente et altitude** | plaine : céréales ; coteau bien exposé : verger, vigne ; hauteurs : forêt, élevage |
| **Climat** (hors POC, constant sur la carte) | fixe les cultures possibles |

## 2. Aptitude par type de terre (hypothèses de travail)
| Terre | Favorable | Défavorable |
|---|---|---|
| Champ (grain) | limon, plaine, drainé | argile lourde, sable, pente forte, marais |
| Pré / pâturage | fond de vallée humide, sol frais | sol sec et maigre |
| Verger | coteau, sol léger et bien drainé | fond de vallée gélif |
| Forêt exploitée | sols pauvres ou lourds, pentes, hauteurs | (concurrencée par l'agriculture sur les bons sols) |
| Lin | sol frais, riche, fond de vallée | sol sec |
| Élevage de laine | hauteurs, landes, calcaire pauvre | |

## 3. Le déboisement et la situation de 1750
- En 1750 les **meilleures terres sont déjà défrichées depuis longtemps** (grands défrichements médiévaux, à vérifier). Ce qui reste à gagner est la **frontière marginale** : landes, marais, sols lourds, pentes, forêts.
- Conséquence pour le générateur de village : l'état initial a déjà ses champs sur les bons sols ; le défrichement du joueur et de la communauté pousse vers des sols de moins en moins aptes (rendements décroissants **émergents**, sans règle codée).
- La forêt a plusieurs rôles : bois de chauffe et de construction (ressource), réserve foncière, et marqueur de sols médiocres.

## 4. Fertilité dynamique (bien plus tard)
- Rotation et jachère (assolement triennal), fumure : le **lien élevage → fumier → céréales** fait de la fertilité un **état** qui évolue avec le travail et le bétail.
- Amendements, drainage des marais, irrigation : investissements qui changent l'aptitude (cible de firmes et de propriétaires ; lien avec la concentration, `02` §10.3).
- Épuisement des sols, rendements décroissants, aléa climatique (`05`).

## 5. Piste d'implémentation
1. **POC (actuel)** : un scalaire par tile, bruit lissé et déclin autour du village.
2. **Étape suivante, peu coûteuse** : générer 2 ou 3 couches (altitude, humidité avec une rivière, type de sol) et en dériver une **aptitude par type de terre** (table de `poc.toml`). Le moteur lit `aptitude[type][tile]` au lieu de `fertilite_ppm` ; la fertilité d'une terre reste la moyenne de son emprise.
3. **Cible** : fertilité dynamique (§4), fertilité par tile modifiable par le jeu.

## 6. Questions ouvertes
- Aire géographique et climat de la carte de départ (Europe de l'ouest ? carte fictive ?).
- Les rivières sont-elles un élément du terrain avant le réseau de transport (navigation, moulins) ? Lien avec l'infrastructure (`02` §4, §9).
- Visibilité pour le joueur : un mode de carte « aptitude » par type de terre (extension du mode fertilité existant).
