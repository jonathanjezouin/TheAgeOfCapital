# app — représentation visuelle du POC

Carte **PixiJS**, interface **Vue 3**, moteur **Rust en WebAssembly** (`../sim-wasm`, qui enveloppe `../sim`).
Tout tourne dans la page : aucun serveur. La même page marche dans un navigateur, un Artifact ou Electron.

```bash
npm install
npm run wasm      # compile le moteur (cargo, cible wasm32) -> public/sim_wasm.wasm
npm run assets    # (re)génère l'atlas de tiles -> public/assets/ (Python, voir ../graphGeneration)
npm run dev       # http://localhost:5173
npm run build     # dist/index.html (JS et CSS inclus) + sim_wasm.wasm + assets/
node tests/capture.mjs dist /tmp/captures   # validation visuelle : captures Chromium sans écran
```

Prérequis : Node 20+, Rust avec la cible `wasm32-unknown-unknown` (`rustup target add wasm32-unknown-unknown`).

## Ce que l'on voit

| Élément | Détail |
|---|---|
| Carte | Sol, chemins (selon les voisins raccordés), hameaux, terres, chantiers avec barre d'avancement, **une pop = un point** (vert satisfaite, orange insatisfaite, rouge en manque, gris sans poste) |
| Vitesse | Pause, +1 jour, **Jour/nuit** (un jour dure 24 s : la nuit tombe et les pops rentrent chez elles), ×1, ×10, ×100, ×1000 jours par seconde |
| Sélection | Clic sur une tile : hameau (pops, rations, stock, croissance), terre (production, **facteur limitant**, fertilité, pops affectées), chantier (avancement, équipe, **motif**), chemin, terre libre |
| Modes de carte | Normal, **Fertilité** (carte de chaleur), **Besoins** (hameaux et terres colorés par couverture) |
| Journal | Chantiers lancés et terminés avec leur motif ; un clic sélectionne la tile |
| Courbes | Population par année, produit / consommé du dernier jour |
| Caméra | Molette pour zoomer, glisser pour déplacer, espace pour la pause |

## Organisation

| Fichier | Rôle |
|---|---|
| `src/sim.js` | Chargeur du `.wasm` et appels (`creer`, `avancer`, `meta`, `carte`, `etat`, `fertilite`) |
| `src/carte.js` | Rendu PixiJS : couches, caméra, sélection, modes, nuit |
| `src/store.js` | État partagé (vitesse, mode, sélection, événements) |
| `src/libelles.js` | Libellés français, date de jeu (1ᵉʳ janvier 1750), traduction du journal |
| `src/components/` | Barre du haut, panneau d'information, journal, courbes |
| `tests/capture.mjs` | Scénario de captures (départ, sélections, 120 ans, modes, chantier, nuit) |

Le moteur n'est jamais modifié par l'interface : elle lit des **instantanés JSON** (`sim_etat_json`) et ne renvoie que deux ordres (créer, avancer).
