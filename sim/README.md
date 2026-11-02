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
| `--rayon-fertile T` / `--rayon-sterile T` | terroir : rayon (tiles) de pleine fertilité / de terre la plus pauvre |
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
| `src/village.rs` | génération déterministe (croix de chemins, champ de fertilité), dimensionnée près de l'équilibre |
| `src/simulation.rs` | jour : différés → contrats (pro rata) → main-d'œuvre → production → consommation → jauges de croissance → péremption ; mois : réorientation des bras ; saison : extension (chantiers) |
| `tests/invariants.rs` | déterminisme, conservation de la matière, stocks ≥ 0, stabilité sur 500 ans, oracle malthusien, jauges de croissance, réorientation des bras, producteur servi d'abord, distance = coût des arêtes (diagonale à 14, croisement interdit), fertilité, chantiers (travail, bois, corvée, jamais bloqués), extension, congestion en sommeil |

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
