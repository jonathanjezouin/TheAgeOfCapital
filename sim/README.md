# sim — moteur de simulation (POC-A)

Crate Rust **pure** (aucune dépendance à Electron / napi) : économie de subsistance d'un hameau isolé de 1750,
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
| `donnees/poc.toml` | ressources, besoins vitaux et de confort, terres exploitées, travail domestique, habitat, démographie, mobilité (aucune valeur de gameplay en dur) |
| `src/donnees.rs` | chargement et validation ; quantités en milli-unités, proportions en ppm (entiers) |
| `src/monde.rs` | tiles passives, quartiers, bâtiments, demi-directions (orthogonales) |
| `src/reseau.rs` | segments, recherche en largeur bornée par la distance limite, chemins précalculés entre lieux |
| `src/village.rs` | génération déterministe (croix de chemins, champ de fertilité), dimensionnée près de l'équilibre |
| `src/simulation.rs` | jour : différés → contrats (pro rata) → main-d'œuvre → production → consommation → jauges de croissance → péremption ; mois : réorientation des bras ; saison : extension (chantiers) |
| `tests/invariants.rs` | déterminisme, conservation de la matière, stocks ≥ 0, stabilité sur 500 ans, oracle malthusien, jauges de croissance, réorientation des bras, producteur servi d'abord, distance en tiles chemin, fertilité, chantiers (travail, bois, corvée, jamais bloqués), extension, congestion en sommeil |

## Règles de conception
- **Déterminisme** : entiers uniquement dans la simulation, itérations ordonnées (`BTreeMap`, `Vec`), RNG maison.
- **Conservation** : `stock = initial + produit − consommé − intrants − périmé`, vérifiée par `ecart_conservation()`.
- Chaque bâtiment expose son **facteur limitant** (`Limitant`) et son taux de production, pour l'UI.
