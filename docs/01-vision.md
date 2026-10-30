# 01 — Vision long terme

> Ce document décrit **la cible**, pas le POC. Le POC (voir `04-poc.md`) n'en implémente qu'une infime partie.

## Intention

Simuler **l'économie politique spatiale** de 1750 à 1950 : comment des sociétés majoritairement agraires deviennent industrielles, comment le capital se concentre, comment l'économique et le politique se façonnent mutuellement. Le cœur stratégique du jeu, à terme, est l'**interaction politico-économique** et l'**organisation du travail**.

La « géographie du capitalisme » est centrale : rente de localisation, spécialisation spatiale, ségrégation par classe, étalement urbain, congestion comme signal d'investissement.

## Le monde

- Carte en tiles du **monde entier** à terme (pas seulement une ville).
- Hiérarchie spatiale : **Tile → Ville → Région → Pays**.
- Tiles terrain en couche basse (herbe, sable, mer, rivière, montagne…), bâtiments et infrastructures par-dessus.
- Le monde vit **sans le joueur**.

### Trois niveaux de jeu

| Niveau | Ce qu'on voit | Pour qui |
|---|---|---|
| **Micro** (ville) | Tiles, bâtiments, infrastructures, quartiers | Maire, capitaliste local, syndicaliste |
| **Méso** (région) | Villes, liens entre villes, flux | Dirigeant régional / national |
| **Macro** (international) | Pays, frontières | Chef d'État, général — géostratégie |

Seul le niveau micro est en scope pour le POC.

## Les acteurs

### Personnages
Des **personnages autonomes** dirigent tout : États, régions, villes, entreprises, partis, syndicats, mouvements contestataires, armées (ou parties d'armée).

- Ils agissent selon leur **idéologie**, leur **classe sociale**, leurs **intérêts matériels** (ex. : un propriétaire terrien français de 1750 soutient l'Ancien Régime pour conserver ses privilèges).
- Ils agissent **dans le cadre de leurs possibilités** : lois du pays, contestation, coup d'État militaire, révolution, lobbying des entreprises…
- Les firmes et institutions sont des **instruments** des personnages ; les personnages ne décident pas tous à chaque tick.
- Les personnages sont créés par des **postes** (institution, firme, organisation, porte-parole d'une base), pas par un ratio de population : voir `00-concept-jeu.md`.

### Le joueur
Le joueur **est l'un de ces personnages**, ni plus ni moins. Il peut être un puissant dirigeant conquérant, un grand capitaliste, un militant libéral… mais le monde tourne sans lui. Son levier dépend de son rôle (ex. : contrôle de l'investissement selon les lois en vigueur : interventionnisme, marché libre, planification).

### La population
- Gérée **par pops de 100 habitants**, jamais individuellement (« cette pop habite ici, travaille là ; insatisfaction 12 % car elle mange correctement et les taxes sont moyennes »). Voir `00-concept-jeu.md`.
- De ces masses peuvent **émerger des personnages remarquables** (leader syndical, révolutionnaire, entrepreneur) qui rejoignent la couche des acteurs, via la règle de pourvoi des postes.

## Le capitalisme comme mécanique

### Montée en échelle
Début du XIXᵉ : nombreuses petites entreprises. Au fil du temps : concentration de quelques acteurs majeurs par secteur (trajectoire Rockefeller). Mais aussi nationalisations, planification communiste, dirigisme, libéralisme.

La concentration doit **émerger** de règles simples :
- profit → réinvestissement → **expansion horizontale** (nouveau bâtiment du même type) ;
- **intégration verticale** (rachat d'un fournisseur / distributeur) ;
- **concentration horizontale** (rachat d'un concurrent vulnérable) ;
- **économies d'échelle** : coût unitaire décroissant avec le volume → éviction des petits.

### Types de firmes (fonctions objectif différentes)

| Type | Logique | Époque typique |
|---|---|---|
| Artisan / petite entreprise | Survie, peu de réinvestissement | 1750–1820 |
| Firme capitaliste privée | Maximisation du profit, réinvestissement | 1800+ |
| Société par actions | Mutualisation du capital, croissance rapide | 1850+ |
| Monopole / trust | Fixation des prix, barrières à l'entrée | 1880+ |
| Coopérative | Redistribution aux travailleurs | 1850+ |
| Firme d'État | Objectifs non lucratifs (emploi, stratégie) | 1900+ |
| Planification centrale | Allocation par plan, pas par prix | 1917+ |

### Chaîne de capture de valeur
À terme, négoce, entrepôts, transport, banques sont **des firmes comme les autres** qui prélèvent leur part (marge, péage, intérêts). La firme intégrée verticalement garde tout — c'est l'avantage compétitif type Standard Oil.

### Évolution historique des échanges (cible)

| Époque | Forme dominante |
|---|---|
| Pré-capitalisme | Liens directs fournisseur → client, courte distance |
| Capitalisme marchand | Marché / entrepôt local comme nœud de distribution (négociants) |
| Capitalisme industriel | Rail, marchés régionaux, grossistes, compagnies ferroviaires |
| Capitalisme intégré | Firmes qui internalisent la chaîne et court-circuitent le marché |

Autres thèmes historiques à porter : **standardisation** (qualité différenciée avant, homogène après), **travail journalier** puis contrats de travail, licenciements, syndicats, **cités ouvrières / tramways** comme réponses émergentes à la contrainte de main-d'œuvre, **Haussmannisation** (coût de percer une infrastructure dans du bâti dense).

## Évolution technologique
Les bâtiments **mutent** plutôt qu'être remplacés : un champ `niveau_tech` (0 = proto-industriel, 1 = vapeur, 2 = électrique…) change les paramètres d'un même type de bâtiment. Une filature vapeur produit beaucoup plus par tile → attire plus de travailleurs → les quartiers s'agglomèrent → le ratio habitat/production s'inverse (voir `02`).

## Couche narrative (plus tard)
SLM local (Qwen 2.5 3B) en chaîne Narrateur → Détecteur → appels par personnage, sorties JSON contraintes par grammaire. Hors POC.
