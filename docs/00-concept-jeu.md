# 00 — Concept du jeu

> Ce document fixe **ce qu'est le jeu et ce que fait le joueur**. Il décrit la cible ; le POC (voir `04`) n'en implémente presque rien. Les décisions de modèle (pops, personnages) y sont consignées ; les détails de simulation sont dans `02`, le schéma dans `03`.

## 1. Le jeu en une phrase

**Le joueur incarne un personnage dans un monde simulé qui tourne sans lui ; son levier dépend de sa position.**

- Le joueur choisit **n'importe quel personnage** existant :
  - dirigeants d'État (monarques, présidents…) ;
  - dirigeants locaux (maires, régions, généraux, ministres, propriétaires, noblesse locale…) ;
  - autres personnalités (syndicalistes, intellectuels, lobbyistes, ambassadeurs…).
- Un **dirigeant** a du pouvoir dans son périmètre d'action. Une **autre personnalité** influence la population, organise des contre-pouvoirs, voire prend le pouvoir.
- **Bac à sable** : pas de condition de victoire globale pour l'instant.
- Direction à terme (hors POC) : la capacité d'un personnage à **naviguer dans l'univers économico-politique** (inspiration : *Le Guépard*).

### Trois leviers (cible, pas dans le POC)

| Levier | Ce que c'est | Rôle typique |
|---|---|---|
| **Pouvoir légal** | Ce que les institutions permettent de décider dans son périmètre | Maire, ministre, monarque |
| **Capital** | Fortune personnelle, parts de firmes | Capitaliste, propriétaire |
| **Influence** | Capacité de faire bouger des populations | Syndicaliste, intellectuel, lobbyiste |

Chaque personnage a une **fortune personnelle** (difficile à gagner pour un travailleur, cumulée par un capitaliste) et une jauge d'**énergie / santé** (maladie, fatigue) : coût de l'action, qui doit être **calculée** à partir du logement, du revenu et des heures travaillées, jamais fixée par rôle. Plus les revenus montent, plus le logement est qualitatif.

### Sujets ouverts par ce concept
- Pouvoir territorial : qui décide entre mairie, région, État ?
- Organisation politique : types de vote (universel, censitaire…).
- Organisation du travail : heures, salaire, protection sociale.

## 2. Deux niveaux : population et personnages

La difficulté centrale : pour l'**économie**, gérer de vraies populations positionnées sur la carte sans gérer d'individus ; pour la **politique**, gérer des individus ancrés dans des populations. Réponse : deux objets distincts, **découplés**.

| Objet | Rôle | Granularité |
|---|---|---|
| **Pop** | Unité de population de l'économie | 100 habitants, fixe |
| **Personnage** | Acteur politique, économique, joueur | Individu, créé par un **poste** |

## 3. La pop

- **Unité fixe : 100 habitants.**
- Une pop est **homogène** sur ses attributs discrets : une seule tile de résidence, un seul lieu de travail, une seule strate (paysan, ouvrier, bourgeois, domestique, noble…), une seule culture, une seule religion, une seule orientation.
- Plusieurs pops cohabitent sur une même tile : la tile quartier est une **capacité de logement** (1 000 hab pour le quartier rural du POC), qui varie avec le type de bâtiment et la densité.
- Un quartier riche mélange des pops de strates différentes sur la même tile (ex. : bourgeois + domestiques) ; les domestiques y habitent et y travaillent (lieu de vie pour les uns, lieu de travail pour les autres), sans mécanique supplémentaire grâce au flux unifié.
- Chaque pop porte une **richesse** et des **droits politiques**.

### Décision de modélisation : bloc discret, pas pop fluide
Une pop est un **bloc discret et homogène** (comme dans Imperator Rome), pas une quantité fluide (0–1 000 hab, pourcentages de culture / religion / alphabétisation, contribution partielle à plusieurs lieux). Raisons :
- l'affectation reste un **simple lien** (1 pop → 1 résidence, 1 emploi nullable) au lieu d'une matrice many-to-many ;
- les pourcentages (part de lettrés, de catholiques…) **se déduisent en comptant les pops** : rien à stocker ni à renormaliser ;
- une pop est quasi un individu : elle peut fournir un porte-parole (leader), se révolter, être citée dans un événement ;
- coût faible (1 M d'habitants = 10 000 pops).

Précisions :
- **Résidence ≠ travail** : la pop consomme à sa résidence et produit à son lieu de travail ; le trajet est un flux de main-d'œuvre qui sollicite l'infrastructure (flux unifié). La main-d'œuvre d'un bâtiment = somme des pops qui lui sont affectées. Une pop sans emploi compte comme chômeuse.
- **Âge et condition individuelle ignorés** : la main-d'œuvre se compte **en pops**, jamais en individus. Une pop employée = une unité de main-d'œuvre ; une pop sans emploi est une chômeuse.
- **Équivalence de calibration** (hypothèse historique, hors moteur) : ~50 % des habitants travaillent, donc 1 pop ≈ 50 travailleurs réels. Un atelier textile de 1850 qui emploie 100 personnes s'écrit **2 pops** dans le jeu. Ce taux sert uniquement à convertir des chiffres historiques en pops ; il n'apparaît ni dans le schéma, ni dans les formules, ni dans les effectifs des bâtiments (exprimés directement en pops).
- **100 est un paramètre, pas une structure** : si la granularité gêne, on peut passer à 50 ou 20 hab/pop sans changer les règles.

### Démographie (dans le POC)
- La **natalité et la mortalité** dépendent de la **satisfaction des besoins** de la pop (voir `02` §6). La croissance nette s'accumule par tile quartier : à +100, une **nouvelle pop** de 100 est créée (dans la limite de la capacité de logement) ; à −100, une pop disparaît.
- Population bornée par la capacité de logement : dans le POC, personne ne crée de quartier, la **saturation** est un résultat observé.

### Évolutions (hors POC)
- **Mobilité** : une pop peut déménager (résidence), changer de travail, changer de strate (paysan → ouvrier).
- **Bascule culturelle, religieuse, d'orientation, alphabétisation** : ces attributs sont fixes pour une pop, mais une pop peut *basculer* en bloc sous certaines conditions (assimilation, école, urbanisation, État…). Mécanisme retenu : une **jauge de pression de conversion** continue, portée par la pop et alimentée par ces conditions ; la pop bascule quand elle franchit un seuil (pas de tirage de dé à chaque tick : déterministe, rejouable, explicable). Une part d'aléa reste possible sur le seuil. Exemple : une pop bretonne dans l'État français peut devenir française.
- **Poids politique** d'une pop = effectif × droits × richesse ; la forme du calcul dépend du régime (universel : les têtes ; censitaire : la richesse ; Ancien Régime : le statut). Les types de vote sont donc des paramètres, pas un système à part.

## 4. Les personnages

- **Découplés de la population** : pas de ratio « un personnage par X habitants ».
- Un personnage n'apparaît jamais seul : il apparaît pour **occuper ou contester un poste**.

| Source du poste | Exemples |
|---|---|
| Institution | Maire, curé, monarque, ministre, général |
| Firme | Propriétaire, dirigeants de branche (grosses entreprises) |
| Organisation | Syndicat national, section locale, parti |
| Pop(s) | Porte-parole d'une base locale (influent local) |

- **Pourvoi d'un poste vacant** (mort, démission, création) : une seule règle, avec une **règle de dévolution** en donnée (primogéniture, élection, cooptation, nomination). Elle sert aussi à la succession des titres nobiliaires ou monarchiques.
- Les personnages **meurent** et transmettent **fortune, propriétés, titre et capital politique** selon cette règle.
- **Émergence** (génériques, pas de scripts) :
  - *pression non représentée* : des pops en colère sans porte-parole font apparaître un leader ;
  - *dérive de lignée* : un jeune hérite de la classe et de la fortune de sa famille, son orientation vient aussi de son entourage.
- Le personnage est une **interface** sur l'économie : son pouvoir est ce qu'il tient (postes, firmes, base sociale), pas une unité de simulation autonome.
- **Personnages légers par défaut** : sans enjeu, seuls leurs attributs évoluent, à rythme lent ; ils sont « promus » quand le joueur les incarne ou qu'un événement les met en jeu.

### Dramaturgie
Mariages, traits, intrigues familiales (à la *Crusader Kings*) : souhaitable à terme, **pas dans les premières versions**. Une lignée minimale (parents / enfants, pour l'héritage) suffit au départ.

### Questions ouvertes
- À la mort du joueur : choix du successeur par le joueur ou règle de dévolution ? Que faire si pas d'héritier ?
- Équilibrage des rôles : un syndicaliste pauvre et fatigué doit disposer d'un levier de mobilisation fort pour rester jouable.

## 5. Séquençage

| Phase | Contenu |
|---|---|
| **POC** | Pops de 100 (résidence, emploi, attributs fixes), natalité / mortalité selon la satisfaction des besoins ; un seul personnage, le maire, **inactif** |
| **Phase 2** | Postes, dévolution, mort et héritage, fortune et firmes ; mobilité des pops |
| **Phase 3** | Émergence des personnages, régimes politiques et suffrage, bascule culturelle des pops ; dramaturgie |
