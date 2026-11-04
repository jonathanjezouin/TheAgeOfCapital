# sim — moteur de simulation (POC-A)

Crate Rust **pure** (aucune dépendance à Electron / napi) : économie de subsistance d'un village isolé de 1750,
boucle quotidienne, jauge de croissance par pop, extension du village par ses habitants, générateur de village déterministe. Spécification : `../docs/` (02, 03, 04).

```bash
cargo run --release --bin sim-cli -- --annees 100 --graine 7 --csv sortie.csv
cargo test --release
```

| Option | Rôle |
|---|---|
| `--annees N` | durée simulée (1 an ≈ 3 ms en release) |
| `--graine N` | graine du village (même graine, même résultat) |
| `--hameaux N` / `--occupation PCT` | taille du village de départ / pops initiales en % de la capacité de logement |
| `--terrain` | parts de chaque terrain et vue d'ensemble ASCII de la carte (`T` forêt, `%` marais, `^` rocailleuse, `"` plaine riche, `~ = #` ruisseau, rivière, fleuve, `V` hameau) |
| `--carte` / `--journal` | plan ASCII du village / journal des événements (chantiers, avec leur motif) |
| `--accel X` | facteur sur natalité et mortalité (observer l'équilibre plus vite) |
| `--donnees f.toml` | remplace `donnees/poc.toml` |
| `--csv f.csv` / `--pas J` | série temporelle tous les J jours |
| `--json-final f.json` | état final (tiles, quartiers, bâtiments, pops) pour le futur renderer |

## Organisation

| Fichier | Contenu |
|---|---|
| `donnees/poc.toml` | ressources, besoins vitaux et de confort, terres exploitées, travail domestique (temps libre, atelier du hameau), habitat, démographie, mobilité (aucune valeur de gameplay en dur) |
| `src/donnees.rs` | chargement et validation ; quantités en milli-unités, proportions en ppm (entiers) |
| `src/monde.rs` | tiles passives, quartiers, bâtiments, **arêtes** (demi-arêtes dans 8 directions par tile, coûts 10 / 14, pas de croisement de diagonales) |
| `src/reseau.rs` | segments (arêtes), Dijkstra à coûts entiers borné par `distance_max`, chemins précalculés entre lieux |
| `src/terrain.rs` | générateur de terrain (docs/07 §3, §10) : altitude, rivières par écoulement, terrains par parts, site du village. Conversion (défrichement, drainage) et ponts : `simulation.rs`, `poc.toml` `[terrains]` et `[eau]` (docs/07 §12) |
| `src/village.rs` | génération déterministe (état initial : premier hameau sur un site dégagé, puis terres et hameaux posés par les règles de site du jeu, `Simulation::peupler_initial`, docs/07 §13), dimensionnée près de l'équilibre |
| `src/simulation.rs` | jour : différés → contrats (pro rata) → budget de jours de travail → production → consommation → jauges de croissance → péremption ; saison : extension (chantiers de maison, de défrichement, de hameau) |
| `tests/terrain.rs` | déterminisme du terrain, parts, cours d'eau jusqu'au bord, site du village, rien de bâti sur un terrain fermé |
| `tests/invariants.rs` | déterminisme, conservation de la matière, stocks ≥ 0, stabilité sur 500 ans, oracle malthusien, jauges de croissance, budget de jours, producteur servi d'abord, distance = coût des arêtes (diagonale à 14, croisement interdit), fertilité, chantiers (travail, bois, jamais bloqués), terres d'un seul tenant, extension, congestion en sommeil |

## Terres sur N tiles et budget de jours (docs/02 §8.2, §8.3)
La tile fait 250 m. Une **terre exploitée** est un bâtiment actif qui occupe N tiles contiguës (`emprise`) ;
extrants (par tile et par jour, à fertilité 1) et travail (jours-personne par tile et par an) sont par tile,
le rendement de chaque tile est sa qualité locale × l'**aptitude de son terrain** pour ce type de terre (`aptitude` dans `poc.toml`), et le travail requis suit `travail_terrain` ; le rendement est la moyenne de l'emprise. Elle naît à `tuiles_par_defrichement` tiles (4) et s'agrandit par
blocs de 4 tiles voisines (les plus fertiles d'abord), jusqu'à `emprise_max` ; au-delà on en fonde une autre.
Le **travail** n'est plus un poste : chaque jour, le temps des pops (1 pop = 1 pop-jour, hors équipes de
chantier) est réparti entre les terres à portée au prorata du temps disponible par hameau, les terres dont
la ressource est en tension d'abord ; une terre au stock de sortie presque plein demande peu. Ce qui reste est
le **temps libre** du hameau (textile domestique). L'emploi d'une pop n'est qu'un affichage : sa terre
principale (`None` si presque inoccupée). Les nouvelles routes peuvent traverser des tiles occupées ; un
hameau se fonde en périphérie du réseau quand il n'y a plus de terre à portée, et des pops sans poste s'y
installent.

## Une maison par pop (docs/02 §9.4)
La capacité d'un hameau se crée **maison par maison** : un chantier de maison (150 pop-jours, équipe de 1,
100 bois, valeurs de départ) ajoute une pop de capacité, jusqu'à `capacite_max_pops` (4 pour le hameau).
Fonder un hameau en bâtit la première maison (`capacite_fondation`). Les pops qui attendent un logement
font d'abord bâtir une maison dans leur hameau, puis, hameau plein, fonder un autre hameau.

## Textile domestique (docs/02 §8.4)
Il n'y a plus de hameau textile : tout hameau embarque un **atelier domestique** (type `domestique = true`),
sans poste. Sa main-d'œuvre est le **temps libre** des pops du hameau : une pop sans poste y consacre tout
son temps, une pop employée la part `temps_libre_actif` (50 %, `[domestique]`). Le travail requis à plein
régime est `pops` (en pop-jours par jour) ; la production est proportionnelle au temps libre disponible.
C'est la version la plus simple du budget de jours (docs/02 §8.3, étape 1 : lissé, sans saisons ni priorités).
L'atelier ne commande de fibre que pour vêtir les pops de son hameau.

## Règles de conception
- **Déterminisme** : entiers uniquement dans la simulation, itérations ordonnées (`BTreeMap`, `Vec`), RNG maison.
- **Conservation** : `stock = initial + produit − consommé − intrants − périmé`, vérifiée par `ecart_conservation()`.
- Chaque bâtiment expose son **facteur limitant** (`Limitant`) et son taux de production, pour l'UI.
