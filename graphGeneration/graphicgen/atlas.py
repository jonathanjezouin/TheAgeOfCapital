"""Assemblage de l'atlas de tiles et du manifeste JSON consommés par le rendu (PixiJS).

Conventions (voir docs/03) : préfixes `terrain_*`, `infra_*`, `bat_*` ; vue de dessus ; soleil au
zénith. Deux tailles de frames : des **tiles** de 256 px (sol, terres, chantier de défrichement) et
des **sprites** de 128 px (maisons en bois et leurs chantiers : une maison par pop, orientée par
rotation). Le réseau routier n'est plus un jeu de tiles : le rendu le trace en courbes et le
remplit avec la texture `sol_chemin.png` (raccordable, hors atlas).
"""
import json
import math
from pathlib import Path

import numpy as np
from PIL import Image

from . import chantiers, maisons, terres, terrain
from .dessin import TUILE
from .maisons import PETIT

COLONNES = 8
COLONNES_PETITES = (COLONNES * TUILE) // PETIT


def _frames():
    """Génère toutes les images : liste de dicts (nom, groupe, taille, image cuite, albedo, normales)."""
    out = []

    def ajouter(nom, groupe, image, taille=TUILE, albedo=None, normales=None):
        out.append(dict(nom=nom, groupe=groupe, taille=taille, image=image, albedo=albedo, normales=normales))

    for i in range(10):
        ajouter(f"terrain_plaine_{i:02d}", "terrain_plaine", terrain.herbe(1000 + i))
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
        ajouter(f"bat_chantier_champ_{i}", "bat_chantier_champ", chantiers.chantier_champ(11000 + i))
    # Sprites de 128 px : une maison en bois par pop, et ses chantiers.
    for i in range(6):
        b, a, nm = maisons.maison_bois(12000 + i)
        ajouter(f"bat_maison_bois_{i}", "bat_maison_bois", b, PETIT, a, nm)
    for etat in range(3):
        ajouter(f"bat_chantier_maison_{etat}", "bat_chantier_maison", maisons.chantier_maison(etat, 13000 + etat), PETIT)
    return out


def construire(sortie, graine=1, verbeux=True):
    """Écrit `atlas.png`, `atlas.json`, `sol_chemin.png` et le dossier `relief/` dans `sortie`.
    Retourne le manifeste."""
    sortie = Path(sortie)
    (sortie / "relief").mkdir(parents=True, exist_ok=True)
    for ancien in (sortie / "relief").glob("*.png"):
        ancien.unlink()  # les relevés des anciens hameaux ne servent plus
    frames = _frames()
    grandes = [f for f in frames if f["taille"] == TUILE]
    petites = [f for f in frames if f["taille"] == PETIT]
    lignes = math.ceil(len(grandes) / COLONNES)
    lignes_petites = math.ceil(len(petites) / COLONNES_PETITES)
    largeur, hauteur = COLONNES * TUILE, lignes * TUILE + lignes_petites * PETIT
    atlas = Image.new("RGBA", (largeur, hauteur), (0, 0, 0, 0))
    manifeste = {
        "version": 2, "tuile": TUILE, "petit": PETIT, "lumiere": "zenith", "atlas": "atlas.png",
        "taille": [largeur, hauteur], "frames": {}, "groupes": {}, "relief": {},
        "textures": {"sol_chemin": "sol_chemin.png"},
    }
    for i, f in enumerate(grandes):
        x, y = (i % COLONNES) * TUILE, (i // COLONNES) * TUILE
        atlas.paste(f["image"], (x, y))
        manifeste["frames"][f["nom"]] = {"x": x, "y": y, "w": TUILE, "h": TUILE}
    for i, f in enumerate(petites):
        x, y = (i % COLONNES_PETITES) * PETIT, lignes * TUILE + (i // COLONNES_PETITES) * PETIT
        atlas.paste(f["image"], (x, y))
        manifeste["frames"][f["nom"]] = {"x": x, "y": y, "w": PETIT, "h": PETIT}
    for f in frames:
        manifeste["groupes"].setdefault(f["groupe"], []).append(f["nom"])
        if f["albedo"] is not None:
            a, n = f"relief/{f['nom']}_albedo.png", f"relief/{f['nom']}_normal.png"
            f["albedo"].save(sortie / a, optimize=True)
            f["normales"].save(sortie / n, optimize=True)
            manifeste["relief"][f["nom"]] = {"albedo": a, "normal": n}
    terrain.sol_chemin().convert("RGB").save(sortie / "sol_chemin.png", optimize=True)
    atlas.save(sortie / "atlas.png", optimize=True)
    (sortie / "atlas.json").write_text(json.dumps(manifeste, indent=1, ensure_ascii=False), encoding="utf-8")
    if verbeux:
        print(f"{len(frames)} frames -> {sortie / 'atlas.png'} ({atlas.width}x{atlas.height})")
    return manifeste


def apercu(sortie, graine=1, colonnes=14, lignes=9):
    """Petit village assemblé depuis l'atlas, pour juger les assets EN CONTEXTE (pas seulement isolés) :
    herbe, terres, un chemin en courbe, des maisons orientées vers lui, un chantier."""
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
    for (x, y), g in {(cx - 1, cy - 1): "bat_champ", (cx + 1, cy - 1): "bat_champ_lin", (cx - 3, cy - 1): "bat_verger",
                      (cx + 3, cy - 1): "bat_paturage", (cx - 1, cy + 1): "bat_foret_exploitee",
                      (cx + 1, cy + 1): "bat_chantier_champ", (cx + 3, cy + 1): "bat_elevage_laine"}.items():
        scene.alpha_composite(au_hasard(g), (x * TUILE, y * TUILE))
    # Route : un trait de terre battue ondulant d'ouest en est, et des maisons orientées vers lui.
    tex = Image.open(sortie / m["textures"]["sol_chemin"]).convert("RGB")
    from PIL import ImageDraw
    masque = Image.new("L", scene.size, 0)
    d = ImageDraw.Draw(masque)
    pts = [(x, cy * TUILE + TUILE / 2 + 34 * math.sin(x / 190.0)) for x in range(0, scene.width, 8)]
    d.line(pts, fill=255, width=30, joint="curve")
    fond = Image.new("RGB", scene.size)
    for j in range(0, scene.height, TUILE):
        for i in range(0, scene.width, TUILE):
            fond.paste(tex, (i, j))
    scene.paste(fond, (0, 0), masque)
    for k in range(8):
        x = (2 + k * 1.5) * TUILE
        y0 = cy * TUILE + TUILE / 2 + 34 * math.sin(x / 190.0)
        haut = k % 2 == 0
        spr = au_hasard("bat_maison_bois")
        if haut:  # maison au nord de la route : la cour (bas du sprite) fait face au sud, tel quel
            scene.alpha_composite(spr, (int(x - PETIT / 2), int(y0 - PETIT - 6)))
        else:     # au sud : retournée de 180°, la cour fait face au nord
            scene.alpha_composite(spr.rotate(180), (int(x - PETIT / 2), int(y0 + 6)))
    scene.alpha_composite(frame("bat_chantier_maison_1"), (int((cx - 5) * TUILE), int((cy + 1) * TUILE + 40)))
    scene.save(sortie / "apercu.png")
    return scene
