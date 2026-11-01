"""Chantiers : un pour bâtir un hameau, un pour défricher un champ (ou toute terre exploitée).

Dans la simulation, un chantier est un seul type de lieu ; le rendu choisit l'image selon sa cible
(`Cible::Hameau` ou `Cible::Terre`).
"""
import math

import numpy as np
from PIL import Image

from .dessin import (TUILE, Toile, assise, bruit, coord, empiler, image, lisse, masque_parcelle,
                     melange)
from .terrain import HERBE_SOMBRE, TERRE, chemin
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


def chantier_hameau(graine, n=TUILE):
    """Bâtir un hameau : terre foulée, fondations de pierre, charpente à moitié dressée, grumes et planches."""
    rng = np.random.default_rng(graine)
    c = n / 2
    X, Y = coord(n)
    # Terre piétinée autour du carrefour de rues.
    b = bruit(n, rng, (3, 6, 12), (0.5, 0.3, 0.2)) - 0.5
    masque = lisse((1.0 - np.hypot((X - c) / 118, (Y - c) / 118) + b * 0.5) * 5)
    sol = _terre_nue(rng, masque)

    t = Toile(n)
    # Fondations : rectangle de pierres dans un quadrant, murets inachevés.
    fx, fy = rng.uniform(16, 36), rng.uniform(16, 34)
    fw, fh = 78, 52
    t.rect(fx - 4, fy - 4, fx + fw + 4, fy + fh + 4, (104, 84, 60))      # tranchée
    t.rect(fx + 6, fy + 6, fx + fw - 6, fy + fh - 6, (120, 94, 66))       # intérieur battu
    for (ax, ay, bx, by) in ((fx, fy, fx + fw, fy), (fx + fw, fy, fx + fw, fy + fh), (fx, fy + fh, fx + fw, fy + fh), (fx, fy, fx, fy + fh)):
        longueur = math.hypot(bx - ax, by - ay)
        k = int(longueur // 9)
        for i in range(k):
            f0, f1 = i / k, (i + 0.86) / k
            if rng.random() < 0.18:  # pierres manquantes : le mur est inachevé
                continue
            px0, py0 = ax + (bx - ax) * f0, ay + (by - ay) * f0
            px1, py1 = ax + (bx - ax) * f1, ay + (by - ay) * f1
            t.ligne(px0, py0, px1, py1, PIERRE[rng.integers(3)], 8.0)
    # Charpente : deux fermes dressées sur les fondations (poutres vues de dessus).
    for i in range(3):
        x = fx + 10 + i * (fw - 20) / 2
        t.rect(x - 2.5, fy + 5, x + 2.5, fy + fh - 5, POUTRE)
        t.rect(x - 1.2, fy + 5, x + 1.2, fy + fh - 5, POUTRE_CLAIRE)
    t.rect(fx + 8, fy + fh / 2 - 2.5, fx + fw - 8, fy + fh / 2 + 2.5, POUTRE)
    t.rect(fx + 8, fy + fh / 2 - 1.2, fx + fw - 8, fy + fh / 2 + 1.2, POUTRE_CLAIRE)
    # Tas de grumes (quadrant opposé).
    gx, gy = n - 100 + rng.uniform(-6, 6), n - 92 + rng.uniform(-6, 6)
    for k in range(5):
        _grume(t, gx + (k % 2) * 3, gy + k * 11, 56 + rng.uniform(-6, 6), rng.uniform(-0.05, 0.05))
    # Planches empilées.
    px, py = rng.uniform(150, 174), rng.uniform(18, 40)
    for k in range(6):
        t.rect(px, py + k * 5.4, px + 64, py + k * 5.4 + 4.2, (190 - k * 2, 158 - k * 2, 110 - k))
        t.rect(px, py + k * 5.4, px + 64, py + k * 5.4 + 1.2, (214, 184, 138))
    # Chariot à bras, dans le dernier quadrant.
    cx0, cy0 = rng.uniform(22, 40), rng.uniform(172, 194)
    t.rect(cx0, cy0, cx0 + 46, cy0 + 26, (112, 82, 54))
    t.rect(cx0 + 3, cy0 + 3, cx0 + 43, cy0 + 23, (146, 112, 76))
    t.rect(cx0 + 4, cy0 + 8, cx0 + 40, cy0 + 20, (118, 94, 70))
    for dy in (-3, 29):
        t.rect(cx0 + 10, cy0 + dy, cx0 + 24, cy0 + dy + 4, (58, 46, 36))
    t.ligne(cx0 + 46, cy0 + 13, cx0 + 76, cy0 + 13, POUTRE, 3.0)
    ob = t.vers_image()
    return empiler(sol, assise(ob, 0.30, 3), ob)


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
