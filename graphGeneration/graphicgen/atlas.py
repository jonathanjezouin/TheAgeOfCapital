"""Assemblage de l'atlas de tiles et du manifeste JSON consommés par le rendu (PixiJS).

Conventions (voir docs/03) : préfixes `terrain_*`, `infra_*`, `bat_*` ; vue de dessus ; soleil au
zénith. Les chemins sont indexés par masque de demi-directions, avec les mêmes valeurs que le
moteur de simulation (N=1, E=2, S=4, O=8).
"""
import json
import math
from pathlib import Path

import numpy as np
from PIL import Image

from . import chantiers, hameau, terres, terrain
from .dessin import TUILE

COLONNES = 8


def _frames():
    """Génère toutes les images : liste de (nom, groupe, clef, image baked, albedo, normales)."""
    out = []

    def ajouter(nom, groupe, image, clef=None, albedo=None, normales=None):
        out.append(dict(nom=nom, groupe=groupe, clef=clef, image=image, albedo=albedo, normales=normales))

    for i in range(10):
        ajouter(f"terrain_plaine_{i:02d}", "terrain_plaine", terrain.herbe(1000 + i))
    for m in range(1, 16):
        ajouter(f"infra_chemin_terre_{m:02d}", "infra_chemin_terre", terrain.chemin(m), clef=m)
    for i in range(3):
        b, a, nm = hameau.hameau(2000 + i, textile=False)
        ajouter(f"bat_hameau_{i}", "bat_hameau", b, albedo=a, normales=nm)
    for i in range(2):
        b, a, nm = hameau.hameau(3000 + i, textile=True)
        ajouter(f"bat_hameau_textile_{i}", "bat_hameau_textile", b, albedo=a, normales=nm)
    for i in range(3):
        ajouter(f"bat_champ_{i}", "bat_champ", terres.champ(4000 + i))
    ajouter("bat_champ_mur_0", "bat_champ_mur", terres.champ(4100, "mur"))
    for i in range(2):
        ajouter(f"bat_paturage_{i}", "bat_paturage", terres.paturage(5000 + i))
    for i in range(2):
        ajouter(f"bat_verger_{i}", "bat_verger", terres.verger(6000 + i))
    for i in range(3):
        ajouter(f"bat_foret_exploitee_{i}", "bat_foret_exploitee", terres.foret_exploitee(7000 + i))
    for i in range(2):
        ajouter(f"bat_champ_lin_{i}", "bat_champ_lin", terres.champ_lin(8000 + i))
    for i in range(2):
        ajouter(f"bat_elevage_laine_{i}", "bat_elevage_laine", terres.elevage_laine(9000 + i))
    for i in range(2):
        ajouter(f"bat_chantier_hameau_{i}", "bat_chantier_hameau", chantiers.chantier_hameau(10000 + i))
    for i in range(2):
        ajouter(f"bat_chantier_champ_{i}", "bat_chantier_champ", chantiers.chantier_champ(11000 + i))
    return out


def construire(sortie, graine=1, verbeux=True):
    """Écrit `atlas.png`, `atlas.json` et le dossier `relief/` dans `sortie`. Retourne le manifeste."""
    sortie = Path(sortie)
    (sortie / "relief").mkdir(parents=True, exist_ok=True)
    frames = _frames()
    lignes = math.ceil(len(frames) / COLONNES)
    atlas = Image.new("RGBA", (COLONNES * TUILE, lignes * TUILE), (0, 0, 0, 0))
    manifeste = {
        "version": 1, "tuile": TUILE, "lumiere": "zenith", "atlas": "atlas.png",
        "taille": [COLONNES * TUILE, lignes * TUILE], "frames": {}, "groupes": {}, "relief": {},
    }
    for i, f in enumerate(frames):
        x, y = (i % COLONNES) * TUILE, (i // COLONNES) * TUILE
        atlas.paste(f["image"], (x, y))
        manifeste["frames"][f["nom"]] = {"x": x, "y": y, "w": TUILE, "h": TUILE}
        g = f["groupe"]
        if f["clef"] is not None:  # chemins : indexés par masque de demi-directions
            manifeste["groupes"].setdefault(g, {})[str(f["clef"])] = f["nom"]
        else:
            manifeste["groupes"].setdefault(g, []).append(f["nom"])
        if f["albedo"] is not None:
            a, n = f"relief/{f['nom']}_albedo.png", f"relief/{f['nom']}_normal.png"
            f["albedo"].save(sortie / a, optimize=True)
            f["normales"].save(sortie / n, optimize=True)
            manifeste["relief"][f["nom"]] = {"albedo": a, "normal": n}
    atlas.save(sortie / "atlas.png", optimize=True)
    (sortie / "atlas.json").write_text(json.dumps(manifeste, indent=1, ensure_ascii=False), encoding="utf-8")
    if verbeux:
        print(f"{len(frames)} tiles -> {sortie / 'atlas.png'} ({atlas.width}x{atlas.height})")
    return manifeste


def apercu(sortie, graine=1, colonnes=14, lignes=9):
    """Petit village assemblé depuis l'atlas, pour juger les assets EN CONTEXTE (pas seulement isolés)."""
    sortie = Path(sortie)
    m = json.loads((sortie / "atlas.json").read_text(encoding="utf-8"))
    atlas = Image.open(sortie / "atlas.png").convert("RGBA")
    rng = np.random.default_rng(graine)

    def frame(nom):
        r = m["frames"][nom]
        return atlas.crop((r["x"], r["y"], r["x"] + r["w"], r["y"] + r["h"]))

    def au_hasard(groupe):
        return frame(m["groupes"][groupe][int(rng.integers(len(m["groupes"][groupe])))])

    scene = Image.new("RGBA", (colonnes * TUILE, lignes * TUILE))
    for j in range(lignes):
        for i in range(colonnes):
            scene.paste(au_hasard("terrain_plaine"), (i * TUILE, j * TUILE))
    cx, cy = colonnes // 2, lignes // 2
    chemins = m["groupes"]["infra_chemin_terre"]
    # Croix de chemins comme dans le générateur de village : bras de 4-5 tiles, hameaux accolés.
    lieux = {}
    for k in range(1, 6):
        for dx, dy in ((k, 0), (-k, 0), (0, k), (0, -k)):
            lieux[(cx + dx, cy + dy)] = "chemin"
    lieux[(cx, cy)] = "chemin"
    # Hameaux sur l'axe horizontal (rues natives) ; terres accolées au-dessus et en dessous.
    posees = [((cx - 2, cy), "bat_hameau"), ((cx + 2, cy), "bat_hameau_textile"), ((cx + 4, cy), "bat_hameau"),
              ((cx - 1, cy - 1), "bat_champ"), ((cx + 1, cy - 1), "bat_champ_lin"), ((cx - 3, cy - 1), "bat_verger"),
              ((cx + 3, cy - 1), "bat_paturage"), ((cx - 4, cy - 1), "bat_elevage_laine"), ((cx - 1, cy + 1), "bat_foret_exploitee"),
              ((cx + 1, cy + 1), "bat_champ_mur"), ((cx + 3, cy + 1), "bat_chantier_champ"), ((cx - 3, cy + 1), "bat_chantier_hameau"),
              ((cx - 5, cy), "bat_hameau")]
    for pos, g in posees:
        lieux[pos] = g

    def est_chemin(p):
        return lieux.get(p) in ("chemin", "bat_hameau", "bat_hameau_textile", "bat_chantier_hameau")

    # Couche infrastructure : chemins, mais aussi sous les hameaux (les rues y sont natives).
    for (x, y), g in lieux.items():
        if g in ("chemin", "bat_hameau", "bat_hameau_textile", "bat_chantier_hameau"):
            masque = (1 if est_chemin((x, y - 1)) else 0) | (2 if est_chemin((x + 1, y)) else 0) | \
                     (4 if est_chemin((x, y + 1)) else 0) | (8 if est_chemin((x - 1, y)) else 0)
            if masque:
                scene.alpha_composite(frame(chemins[str(masque)]), (x * TUILE, y * TUILE))
    # Couche bâtiments.
    for (x, y), g in lieux.items():
        if g != "chemin":
            scene.alpha_composite(au_hasard(g), (x * TUILE, y * TUILE))
    scene.save(sortie / "apercu.png")
    return scene
