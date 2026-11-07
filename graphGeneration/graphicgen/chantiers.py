"""Chantier de défrichement d'une terre exploitée (`Cible::Terre`). Les chantiers de maison sont dans
`maisons.py` (`Cible::Maison`, `Cible::Hameau`).
"""
import math

import numpy as np
from PIL import Image

from .dessin import (TUILE, Toile, assise, bruit, coord, empiler, image, lisse, masque_parcelle,
                     melange)
from .terrain import HERBE_SOMBRE, TERRE
from .terres import TERRE_LABOUR, TERRE_SILLON, _sillons, _couronne

TERRE_NUE = (np.array([138, 108, 76], np.float32), np.array([112, 86, 60], np.float32))
PIERRE = [(150, 146, 136), (132, 128, 120), (168, 162, 150)]
POUTRE = (126, 94, 60)
POUTRE_CLAIRE = (176, 140, 96)


def _terre_nue(rng, masque, n=TUILE):
    t = bruit(n, rng, (6, 12, 24, 48), (0.35, 0.3, 0.2, 0.15))
    rgb = melange(TERRE_NUE[0], TERRE_NUE[1], lisse((t - 0.3) * 2.2)) * (0.94 + 0.12 * rng.random((n, n)))[..., None]
    return image(rgb, masque)


def _grume(toile, x, y, long, ang, rayon=4.6):
    """Grume vue de dessus : cylindre allongé, bouts clairs."""
    toile.rect_oriente(x, y, long, rayon * 2, ang, (112, 80, 52))
    toile.rect_oriente(x, y, long, rayon * 1.1, ang, (134, 98, 64))
    for sgn in (-1, 1):
        toile.disque(x + sgn * long / 2 * math.cos(ang), y + sgn * long / 2 * math.sin(ang), rayon * 0.9, (186, 150, 104))


def _souche(toile, x, y, r=7.0):
    toile.disque(x, y, r, (92, 66, 44))
    toile.disque(x, y, r * 0.78, (174, 140, 96))
    toile.disque(x, y, r * 0.5, (150, 116, 80))
    toile.disque(x, y, r * 0.2, (124, 94, 64))


def chantier_champ(graine, n=TUILE):
    """Défricher un champ : parcelle de terre nue, souches arrachées, première moitié labourée, piquets et corde."""
    rng = np.random.default_rng(graine)
    X, Y = coord(n)
    masque = masque_parcelle(rng, 14, 6.0)
    sol = _terre_nue(rng, masque)
    # Moitié labourée (sillons) d'un côté, terre brute de l'autre.
    angle = rng.choice([0.0, math.pi / 2])
    crete, rang, frac = _sillons(rng, n, angle, 12.5)
    texture = bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25))
    labour = melange(TERRE_SILLON, TERRE_LABOUR, lisse(crete * 1.1 - 0.05)) * (0.9 + 0.2 * texture)[..., None]
    coupe = lisse(((X - n * rng.uniform(0.42, 0.58)) if angle == 0.0 else (Y - n * rng.uniform(0.42, 0.58))) / -10.0)
    zone = image(labour, masque * coupe)
    t = Toile(n)
    # Racines : petits traits sombres sur la terre brute.
    for _ in range(26):
        x, y = rng.uniform(30, n - 30, 2)
        ang = rng.uniform(0, 2 * math.pi)
        for k in range(3):
            t.ligne(x, y, x + 12 * math.cos(ang + k * 0.5), y + 12 * math.sin(ang + k * 0.5), (88, 64, 46), 1.8)
    # Souches arrachées / restantes.
    for _ in range(6):
        x, y = rng.uniform(32, n - 32, 2)
        _souche(t, x, y, rng.uniform(6.0, 9.0))
    # Tas de branchages.
    for _ in range(2):
        x, y = rng.uniform(50, n - 50, 2)
        for _ in range(30):
            a = rng.uniform(0, math.pi)
            t.ligne(x + rng.uniform(-12, 12), y + rng.uniform(-8, 8), x + 16 * math.cos(a), y + 16 * math.sin(a),
                    [(104, 80, 52), (92, 112, 52), (128, 100, 64)][rng.integers(3)], 2.0)
    # Piquets aux coins et corde de tracé.
    pts = [(24, 24), (n - 24, 24), (n - 24, n - 24), (24, n - 24)]
    for i in range(4):
        (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % 4]
        t.ligne(x0, y0, x1, y1, (222, 212, 184), 1.4)
    for x, y in pts:
        t.disque(x, y, 4.0, (96, 70, 46))
        t.disque(x, y, 2.4, (176, 140, 96))
    # Une charrue abandonnée sur le labour.
    px, py = n * 0.30, n * 0.50
    t.rect_oriente(px, py, 50, 4.5, 0.0, POUTRE)
    t.rect_oriente(px + 4, py + 8, 28, 3.5, 0.5, POUTRE)
    t.disque(px - 22, py, 3.0, (96, 96, 96))
    ob = t.vers_image()
    return empiler(sol, zone, assise(ob, 0.22, 2.5), ob)
