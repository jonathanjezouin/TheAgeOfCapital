# 05 — Roadmap, reports et questions ouvertes

## Roadmap par étapes

| Étape | Contenu | Valide |
|---|---|---|
| **1 — POC** | 200×200, une ville ; POC-A boucle physique (pops, besoins, démographie), puis POC-B monnaie et firmes ; maire inactif | Mécanique économique de base |
| **2 — Méso** | Plusieurs villes, échanges inter-cités abstraits | L'échelle régionale |
| **3 — Monde** | Carte monde en tiles macro, personnages, politique | Le jeu complet |

Chaque étape doit pouvoir fonctionner comme un jeu en soi.

## Prochaine étape du POC : jour/nuit, ombres et saisons

Décidée le 2026-11-03 (détail dans `04`). Elle reprend les trois premiers sujets des listes ci-dessous : **saisons** (profils mensuels du travail, stocks annuels, aléa de récolte), **lumière en rotation** et ombres portées (shader de ré-éclairage), **cycle jour/nuit** lisible à l'écran.

## Explicitement reporté après le POC

- **Passage à l'échelle** (centaines de milliers de bâtiments / pops, rendu sur la seule zone caméra) : principes retenus dans `03` §5b ; **banc d'essai reporté** (moteur : 500 k bâtiments + 200 k pops synthétiques, structures denses vs existant, mono-thread vs Rayon ; rendu : carte 2 000×2 000 avec chunks, culling et niveaux de détail, captures Chromium + FPS). À lancer quand le design sera prêt
- Migration du moteur vers tableaux denses, pont binaire `viewport`, chunks de rendu, sauvegarde binaire

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
- **Manufacture textile** (filature, atelier de couture) : en 1750 le textile se fait à la maison (travail domestique d'hiver, voir `02` §8.4) ; c'est la première filière où le capitalisme émergera (marchand-fabricant)
- Saisons (profils mensuels du travail ; l'étape 1 est un budget annuel de jours, voir `02` §8.3), stocks annuels (greniers), **aléa de récolte** (outil de debug en attendant)
- Rendements décroissants des terres, fertilité variable par tile, droits d'usage communaux du bois, prélèvements (dîme, rente, impôts) : le surplus qui quitte le village
- Promotion de classe par la satisfaction du confort
- Abandon des lieux en déclin (hameaux vides, terres sans bras), délai de production d'un verger
- **Lumière en rotation** : shader de ré-éclairage à partir de l'albedo et des normales des hameaux (déjà générés, voir `03` §7) ; relief des autres sprites (arbres, bâtiments futurs)
- Avancement visible d'un chantier (plusieurs états)
- Congestion et livraisons différées : code présent mais **en sommeil** (capacités très au-dessus des flux dans le POC)
- Poisson, coton, transformation du bois, nourriture industrielle
- Autres terrains, infrastructures (route pavée, canal, rail, port), quartiers (artisan, bourgeois, urbain)
- Niveaux technologiques > 0
- **Matériaux de construction autres que le bois** (torchis, pierre, brique) : champ `materiau` nullable prévu (`02` §9.4)
- **Densification du bâti** (maison, mitoyenne, petit immeuble, immeuble) et plafond de pops par type de bâtiment (`02` §9.4)
- **Trame urbaine planifiée** (rues rectilignes, immeubles d'angle) et rendu des routes selon le niveau d'urbanisation
- Coupure, bruit et échangeurs des voies majeures ; veto des habitants sur une traversée
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

## Décisions techniques prises

| Sujet | Décision |
|---|---|
| Hôte Steam | **Electron + napi-rs** (Tauri écarté) ; wasm conservé pour démonstrations et tests visuels (`03` §1) |
| Échelle visée | ~4 M tiles, 300–500 k bâtiments, 100–300 k pops, 10–50 k personnages (`03` §5), à valider par banc d'essai ; **réviser** avec la tile fixe de 250 m (question 0) |
| Pont | Tableaux typés par viewport + drapeaux « sale » par chunk, pas de JSON du monde (`03` §5b) |
| Représentation du bâti | Une maison par pop (100 hab.), sprite exagéré, orienté vers l'arête ; bois seul comme matériau (`02` §9.4, `03` §6) |
| Réseau | Arêtes à 8 directions, coûts entiers 10/14, Dijkstra borné ; rendu en courbes, atlas de chemins abandonné (`02` §9, `03` §6) |
| Persistance | SQLite hors tick ; sauvegarde = instantané binaire compressé |

## Questions ouvertes

0. **Échelle exacte** : 2 000×2 000 et 500 k bâtiments sont des ordres de grandeur provisoires ; à confirmer avec le banc d'essai avant de figer les structures de données. **Avec la tile fixe de 250 m (16 tiles/km²)**, 2 000×2 000 = 500 × 500 km (~½ France) et le monde habité de 1750 (~20 M km²) = quelques centaines de millions de tiles. Cela ne tient que si : une terre = 1 nœud pour N tiles (~1 nœud pour 5–6 tiles), le terrain inactif est un raster passif, et les nœuds stables sont « dormants » (mêmes règles, cadence variable). Relancer le banc d'essai avec ces hypothèses.

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
9. **Jours de travail par pop** : `[travail] jours_personne_par_pop_an = 12 500` (50 travailleurs × 250 jours) est dans `poc.toml` et dans le moteur, ce qui **déroge à la règle de `00` §3** (« le 50 % de travailleurs n'apparaît pas dans les formules »). Dérogation assumée, à acter dans `00`.
10. **Moteur de travail** : étape 1 faite (budget lissé, priorité aux terres en tension, temps libre au textile) ; reste les profils mensuels (étape 2), le coût en jours des recettes textiles, les jours par tile du verger, de la forêt et de l'élevage (à calibrer), et des priorités plus fines (vital > confort).
11. **Spécialisation régionale** (anneaux de Von Thünen, vigne, lin, herbages) : émergence en POC-B avec prix et transport ; premier bien importé (sel, fer, outils) à choisir pour faire naître le bourg.
12. **Dispersion de l'habitat** : fermes isolées (bocage) vs villages groupés à champs ouverts ; paramètre du générateur.
13. **Maillage** : critère de succès ~1:4:15:45 (ville : bourgs : villages : hameaux) et espacements (`04`), à mesurer sur plusieurs graines, jamais codé.
14. **Une seule simulation ou multi-échelle ?** `03` §5 prévoit un modèle agrégé pour les zones non observées ; la volonté actuelle est de **ne pas** avoir deux simulations (locale et régionale). Piste : mêmes règles partout, nœuds dormants à cadence réduite ; à trancher avec le banc d'essai.
15. ~~Calibration du POC à la tile de 250 m~~ : **faite** (rayons de fertilité ×4, `distance_max` = 100, terres par tile, coûts de chantier par tile).
23. **Taille du village** : la croix de départ (bras de 8 tiles) porte ~4 hameaux ; le terroir à portée plafonne à 8–15 pops. Plusieurs groupes de hameaux (une zone de marché de ~21 000 hab., `04`) demandent un générateur multi-croix et des liens entre groupes.
24. **Fondation de hameau** : les pops sans poste s'installent sur le site ; comment les familles se répartissent (migration, héritage) est à préciser.
16. **Tile d'habitat desservie par une (demi-)route** : façade, impasse, capacité = f(longueur de façade, densification) ; à quel seuil de pops un hameau passe de la demi-arête à la traversante (`02` §9.4).
17. **Densification et plafond** : 10 pops pour le bâti bas ; ~25 pops pour des immeubles ; le plafond devient une propriété du type de bâtiment.
18. **Pops à reloger** quand une route est posée sur une tile pleine : défaut proposé = mécanisme des pops en attente de logement, sans disparition ; à confirmer (`02` §9.3).
19. **Traversée d'un village** : coût seul par défaut ; veto des habitants plus tard (politique).
20. **Croisement de diagonales** dans un carré 2×2 : interdit ou jonction.
21. **Rues rectilignes en ville** : décalage organique du rendu décroissant avec l'urbanisation, trame planifiée et immeubles d'angle (`02` §9.4).
22. **Chantier d'une maison** (1 pop) au lieu d'un chantier de hameau (2 pops) : valeur de départ dérivée de l'existant (300 pop-jours et 200 bois pour 2 pops, soit ~150 pop-jours et ~100 bois par maison) ; à calibrer à l'alignement du POC. Les pops en attente de logement font bâtir une maison, plus un hameau.

## Incohérences relevées dans les échanges de design (à trancher)

| Sujet | Version ancienne | Version la plus récente | Proposition |
|---|---|---|---|
| Granularité temporelle | Flux de main-d'œuvre **horaire** | Tick économique **quotidien**, journaliers | Garder le quotidien ; l'horloge quart d'heure ne sert qu'au rythme d'affichage |
| Tick de la boucle | 1 mois / 1 saison | Horloge 15 min, multi-fréquences | Multi-fréquences (voir `02`) |
| Taille de carte POC | 150×150 | 200×200 | 200×200 |
| Ressources | bois transformé, charbon, minerai… | nourriture / textile / bois | Périmètre de `04` |
| Décision économique | Firmes autonomes | Personnages qui dirigent les firmes | POC : firmes à règles simples (POC-B) + maire inactif ; personnages plus tard |
| Emprise des bâtiments | Usage unique par tile | « Bâtiments sur plusieurs tiles » (vision) ; hameau textile = habitat + atelier | **Tranché le 2026-11-02** : une terre exploitée = un bâtiment de N tiles contiguës ; hameau = 1 tile ; hameau textile supprimé (`02` §8) |
| Infrastructure | Tiles de chemin (usage unique par tile), orthogonal seulement | Arêtes sur les tiles, 8 directions | **Tranché le 2026-11-02** : couche d'arêtes, coûts 10/14, emprise = largeur × longueur (`02` §9) |
| Taille de tile | 50–200 m², puis ~1 km², puis unité comptable à surface variable | Surface réelle fixe | **Tranché le 2026-11-02** : 250 m × 250 m = 6,25 ha (`02` §1) |
