# 02 — Modèle de simulation économique

## 1. La tile

- **Usage unique** par tile : terrain naturel, infrastructure, bâtiment de production, quartier d'habitation.
- Exception : la **tile quartier embarque nativement une infrastructure de circulation** (rues). Les marchandises peuvent y circuler, mais cette capacité sera insuffisante pour des stades de production avancés (rail, port… plus tard).
- **Unité comptable, pas géographique** : la surface implicite dépend du type.

| Type de tile | Surface implicite | Population |
|---|---|---|
| Champ | ~5 km² | — |
| Pâturage | ~10 km² | — |
| Exploitation forestière | ~20 km² | — |
| Quartier rural | ~0,5 km² | capacité 1 000 hab |
| Quartier urbain (futur) | ~0,1 km² | capacité ~5 000 hab |

- **Une tile quartier est une capacité de logement** (1 000 hab pour le quartier rural du POC), occupée par des **pops de 100 habitants** (voir `00`). Ex. : Rouen ≈ 50 000 hab ≈ 500 pops. Une tile quartier rurale représente un **bassin de peuplement** (villages et hameaux agrégés), pas un hameau isolé.
- La capacité varie avec le type de bâtiment et la densité ; elle n'est pas un nombre fixe d'habitants.
- `valeur_fonciere` existe dès le départ mais n'est pas utilisée dans le POC (futur moteur de la ségrégation spatiale).

### Le ratio habitat / production : indicateur émergent

| Contexte | Ratio tiles habitat : production |
|---|---|
| Campagne 1750 | 1 : 5–7 |
| Bourg proto-industriel 1800 | ~1 : 1 |
| Ville industrielle 1850 | 3–5 : 1 |
| Métropole 1900 | 8–10 : 1 |

**Ne jamais le coder.** Il doit émerger de la calibration (production extensive à la campagne, productivité par tile qui explose avec `niveau_tech`).

## 2. Les acteurs économiques

### Firme
- Possède une ou plusieurs tiles, emploie, achète, vend, dégage profit ou perte, réinvestit.
- **Mono-secteur** : une firme ne gère qu'un type de production.
- Statut : privée, publique, coopérative…
- Règles POC (Option A — firmes autonomes à règles simples) :

```
profit  > seuil        → réinvestir (construire un nouveau bâtiment)
capital > seuil        → racheter une firme vulnérable (peu rentable)
perte prolongée        → faillite, tile abandonnée
```

### Groupe
- Possède plusieurs firmes (c'est ainsi qu'un grand groupe internalise transport, approvisionnement, etc.).
- Dans le POC : structure présente, **pas de logique propre**.

### Pop
- Unité de population : **100 habitants, fixe**. Attributs discrets : tile de résidence, tile d'emploi (nullable), strate, culture, religion, orientation (voir `00`).
- Dans le POC : une seule strate (paysan), une seule culture et religion, attributs fixes.

### Personnages (lien avec la vision)
Les firmes et institutions sont dirigées par des personnages, créés par des **postes** (voir `00`). Dans le POC : **un seul personnage, le maire**, coquille presque vide dont l'objectif est d'augmenter la population en créant des bâtiments répondant aux besoins.

## 3. Flux unifié

Tout circule dans le même modèle : marchandises **et main-d'œuvre**.

- Un quartier **produit** de la main-d'œuvre et **consomme** des biens.
- Un bâtiment de production **consomme** main-d'œuvre + intrants, **produit** des extrants.
- **Périmètre** d'une tile = ensemble des tiles accessibles via infrastructure continue, pondéré par le coût de transport (graphe, pas rayon fixe).
- Un quartier met sa population à disposition dans son périmètre ; une usine met ses ressources à disposition dans son périmètre.
- Les **pops** sont l'unité de ce flux : chaque pop est affectée à un seul lieu de travail (résolution par le marché journalier ci-dessous).

### Lien économique
Créé entre deux tiles quand :
1. l'une produit ce que l'autre consomme ;
2. elles sont reliées par une infrastructure continue ;
3. la distance (coût de transport) est acceptable.

Sélection du fournisseur : **appel d'offres local** — le client choisit, dans son périmètre, le fournisseur au **coût livré** (prix + transport) le plus bas. Les chemins sont **précalculés** à la création du lien et recalculés uniquement si le réseau change.

### Transport (POC)
- **Implicite** : personne ne « réalise » le transport ; son coût est calculé et absorbé dans le prix de transaction.
- Préparé pour l'extension : `operateur_id` nullable sur le lien (`null` = transport implicite ; plus tard une firme de transport, éventuellement incluse au contrat).
- Pas d'intermédiaires (marchés, négociants) dans le POC : relation **directe** fournisseur ↔ client.

### Contrats
- **Spot** : achat ponctuel, prix du moment, pas de garantie (`contrat_id = null` par défaut).
- **Cadre** : volume régulier, prix négocié, priorité d'accès au stock.
- Champ `qualite` prévu (différenciation avant standardisation) — non exploité dans le POC.
- Mécanisme de formation des prix : **à approfondir** (voir `05`).

### Main-d'œuvre : modèle « journalier »
- Historiquement dominant jusqu'au tournant du XXᵉ siècle ; aucun état persistant côté travailleur.
- Chaque jour, une firme publie une demande (volume, taux journalier offert) ; les quartiers accessibles répondent selon disponibilité et coût → **mini-marché spot quotidien**.
- La main-d'œuvre **ne se stocke pas** : ce qui n'est pas employé reste au quartier (chômage latent).
- Une firme en expansion doit **payer plus** (attirer des quartiers lointains) ou **rester limitée** (quartiers proches épuisés, segments saturés) → pression émergente vers cités ouvrières, tramways, délocalisation.
- Extensions futures en **surcouche** sans refactorisation : contrats hebdo/mensuels prioritaires, licenciement, syndicats (`salaire_plancher` sur une zone).

## 4. Infrastructure et congestion

- Chaque segment porte un **volume journalier agrégé** (marchandises + main-d'œuvre).
- La sollicitation est imputée au **tick du transfert**.
- Modèle à deux couches :

```
charge_segment        = Σ volumes transférés via ce segment ce jour
facteur_congestion    = f(charge_segment / capacite_nominale)
cout_transport_effectif = cout_base × (1 + α × facteur_congestion²)
```

| Bande de charge | Effet |
|---|---|
| `charge ≤ capacite_nominale` | Transfert immédiat, coût standard |
| `capacite_nominale < charge ≤ capacite_max` | Transfert immédiat, **surcoût** congestion |
| `charge > capacite_max` | **Livraison différée** à J+1 (`file_attente` du segment) + surcoût |

- Les contrats sont traités **par priorité** (valeur, ancienneté, type) : les premiers passent au coût normal, **le surcoût ne s'applique qu'aux éléments en excès / différés**.
- Courbe **quadratique** : faible pénalité sous le seuil, forte au-delà → effets de bascule.
- La livraison différée est **dans le POC** : elle crée des ruptures de stock en cascade.
- À éviter : routes distinctes par direction, files par entité individuelle, saturation binaire.
- La congestion est un **signal d'investissement** (amélioration de l'infrastructure).

## 5. Temps et séquence du tick

### Horloge
- Horloge de base au **quart d'heure** ; vitesse nominale **1 s réelle = 15 min de jeu**, accélérable jusqu'à une journée en quelques secondes.
- **Résolution économique : la journée.** Pas de transferts horaires, pas de pointe matin/soir : ce qui compte à l'échelle 1750–1950 est la capacité journalière d'une route.

### Fréquences de mise à jour

| Événement | Fréquence |
|---|---|
| Contrats, main-d'œuvre, production, stocks, congestion | Quotidienne |
| Prix (offre / demande) | Hebdomadaire |
| Décisions des firmes (investir, racheter, contrats) | Mensuelle |
| Construction de nouveaux bâtiments | Saisonnière |
| Démographie des quartiers | Annuelle |

### Séquence quotidienne (option B : recruter avant de produire)

```
J ouverture
  1. Livraison des différés de J-1 (file_attente des segments)
  2. Résolution des contrats d'approvisionnement (marchandises)
  3. Résolution des flux de main-d'œuvre (journaliers)
  4. Calcul de la production effective
  5. Mise à jour des stocks et bilans journaliers
  6. [fin de mois] Décisions stratégiques des firmes
J clôture
```

Les étapes 2 et 3 sont résolues sur le même tick ; la production est calculée après les deux, contraintes combinées **multiplicativement** :

```
production_effective = production_max
  × min(1, main_oeuvre_obtenue / main_oeuvre_requise)
  × min(1, stock_input / input_requis)
```

### Exemple de cinématique
- **Jour 1** : l'usine A a tous ses travailleurs → en fin de journée, +10 unités dans son stock.
- **Jour 2** : l'usine B achète 5 unités à A (transfert imputé au segment ce jour-là), consomme les 5, produit 15 unités de B.

## 6. Démographie (POC)
- **Pas de natalité dans le POC** : les pops sont fixes (100 hab) et ne naissent ni ne disparaissent. La population totale ne croît que par **création de quartiers** (le maire construit, de nouvelles pops s'installent).
- La main-d'œuvre disponible est une fraction de la population de la pop.
- Hors POC : natalité (hypothèse : nouvelle pop de 100 quand la croissance accumulée l'atteint), mobilité, bascule des attributs (voir `00`).

État d'un quartier (agrégat des pops de la tile) :
```
population, actifs_disponibles, taux_chomage, salaire_reference (plancher local, évolue lentement)
```
