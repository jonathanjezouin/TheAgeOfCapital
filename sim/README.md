# sim — moteur de simulation (POC-A)

Crate Rust **pure** (aucune dépendance à Electron / napi) : économie spatiale de The Age Of Capital,
boucle quotidienne, démographie annuelle, générateur de village déterministe. Spécification : `../docs/` (02, 03, 04).

```bash
cargo run --release --bin sim-cli -- --annees 100 --graine 7 --csv sortie.csv
cargo test --release
```

| Option | Rôle |
|---|---|
| `--annees N` | durée simulée (1 an ≈ 3 ms en release) |
| `--graine N` | graine du village (même graine, même résultat) |
| `--quartiers N` / `--occupation PCT` | taille du village / pops initiales en % de la capacité |
| `--accel X` | facteur sur natalité et mortalité (observer l'équilibre plus vite) |
| `--donnees f.toml` | remplace `donnees/poc.toml` |
| `--csv f.csv` / `--pas J` | série temporelle tous les J jours |
| `--json-final f.json` | état final (tiles, quartiers, bâtiments, pops) pour le futur renderer |

## Organisation

| Fichier | Contenu |
|---|---|
| `donnees/poc.toml` | ressources, besoins, bâtiments, infrastructure, démographie (aucune valeur de gameplay en dur) |
| `src/donnees.rs` | chargement et validation ; quantités en milli-unités, proportions en ppm (entiers) |
| `src/monde.rs` | tiles passives, quartiers, bâtiments, demi-directions (orthogonales) |
| `src/reseau.rs` | segments, Dijkstra, chemins précalculés entre lieux |
| `src/village.rs` | génération déterministe, dimensionnée près de l'équilibre |
| `src/simulation.rs` | jour : différés → contrats (pro rata) → main-d'œuvre → production → consommation → péremption ; an : démographie |
| `tests/invariants.rs` | déterminisme, conservation de la matière, stocks ≥ 0, stabilité sur 500 ans, oracle malthusien, congestion |

## Règles de conception
- **Déterminisme** : entiers uniquement dans la simulation, itérations ordonnées (`BTreeMap`, `Vec`), RNG maison.
- **Conservation** : `stock = initial + produit − consommé − intrants − périmé`, vérifiée par `ecart_conservation()`.
- Chaque bâtiment expose son **facteur limitant** (`Limitant`) et son taux de production, pour l'UI.
