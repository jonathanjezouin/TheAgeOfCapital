"""Maisons en bois et leurs chantiers : sprites de 128 px, vue de dessus, soleil au zénith.

Une maison représente une pop (100 habitants) : le sprite est volontairement plus grand que la
maison réelle (~1/6 de tile, voir docs/03 §6). Il est dessiné « face au Sud » (la cour est au bas
de l'image) et le rendu le fait pivoter pour qu'il fasse face à l'arête qui le dessert ; comme le
soleil est au zénith, aucune ombre portée ne s'en trouve faussée.
Le bois est le seul matériau de construction du POC : toit de bardeaux, tas de grumes, poutres.
"""
import math

import numpy as np
from PIL import Image

from .cultures import CULTURES
from .dessin import Toile, assise, bruit, coord, empiler, image, lisse, melange
from .house import ZENITH, generate_house
from .terrain import HERBE_SOMBRE, TERRE

PETIT = 128  # côté d'un sprite de maison, en pixels

POUTRE = (126, 94, 60)
POUTRE_CLAIRE = (176, 140, 96)
BARDEAU = (140, 112, 78)


def _cour(rng, rect, n=PETIT):
    """Cour de terre battue : tache ovale irrégulière autour (ou devant) la maison."""
    x0, y0, x1, y1 = rect
    X, Y = coord(n)
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    rx, ry = (x1 - x0) / 2, (y1 - y0) / 2
    b = bruit(n, rng, (4, 8, 16), (0.45, 0.35, 0.2)) - 0.5
    d = 1.0 - np.hypot((X - cx) / rx, (Y - cy) / ry) + b * 0.35
    t = bruit(n, rng, (6, 12, 24), (0.4, 0.35, 0.25))
    rgb = melange(TERRE[0] * 0.95, TERRE[2], t * 0.7) * (0.95 + 0.1 * rng.random((n, n)))[..., None]
    return image(rgb, lisse(d / 0.25) * 0.85)


def _tas_de_grumes(toile, x, y, ang, rng, k=3):
    """Petit tas de bois de chauffage : k grumes parallèles."""
    ca, sa = math.cos(ang + math.pi / 2), math.sin(ang + math.pi / 2)
    for i in range(k):
        px, py = x + ca * i * 4.6, y + sa * i * 4.6
        toile.rect_oriente(px, py, 19, 4.4, ang, (112, 80, 52))
        toile.rect_oriente(px, py, 19, 2.4, ang, (140, 104, 70))
        for sgn in (-1, 1):
            toile.disque(px + sgn * 9.5 * math.cos(ang), py + sgn * 9.5 * math.sin(ang), 2.2, (186, 150, 104))


def maison_bois(graine, n=PETIT):
    """Une maison en bois (toit de bardeaux), sa cour devant, un tas de bois. Retourne
    (image cuite zénith, albedo sans éclairage, normales), RGBA n×n."""
    rng = np.random.default_rng(graine)
    richesse = float(rng.uniform(0.35, 0.85))
    for ppm in (8, 7, 6, 5, 4):
        maison = generate_house(graine * 13 + 5, CULTURES["bardeau"], richesse, ppm=ppm, light=ZENITH, relief=1.2)
        boite = maison.roof.getbbox()
        w, h = boite[2] - boite[0], boite[3] - boite[1]
        if w <= 82 and h <= 70:
            break

    def rogner(im):
        return im.crop(boite)

    x = int((n - w) / 2)
    y = int(n * 0.40 - h / 2)
    cour = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    cour.alpha_composite(_cour(rng, (x - 12, y + h * 0.35, x + w + 12, y + h + 30), n))
    deco = Toile(n)
    _tas_de_grumes(deco, x + w + 8, y + h * 0.5, math.pi / 2, rng, k=int(rng.integers(2, 4)))
    if rng.random() < 0.5:  # petit potager derrière la maison
        deco.rect(x + 4, y - 14, x + w - 4, y - 5, (104, 78, 54))
        for i in range(int((w - 12) // 6)):
            deco.disque(x + 9 + i * 6, y - 9.5, 2.2, [(94, 140, 58), (120, 156, 64), (150, 160, 70)][int(rng.integers(3))])
    toits = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    toits.alpha_composite(rogner(maison.roof), (x, y))
    contact = assise(toits, 0.34, 2.2)
    deco_img = deco.vers_image()
    base = empiler(cour, assise(deco_img, 0.25, 1.8), deco_img, contact, toits)
    alb = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    alb.alpha_composite(rogner(maison.albedo), (x, y))
    albedo = empiler(cour, assise(deco_img, 0.25, 1.8), deco_img, contact, alb)
    normales = Image.new("RGBA", (n, n), (128, 128, 255, 255))
    nrm = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    nrm.alpha_composite(rogner(maison.normal), (x, y))
    normales.alpha_composite(nrm)
    return base, albedo, normales


def chantier_maison(etat, graine, n=PETIT):
    """Chantier d'une maison en bois. `etat` : 0 = sol dégagé et sablières, 1 = ossature dressée,
    2 = charpente, avec les premiers bardeaux."""
    rng = np.random.default_rng(graine)
    X, Y = coord(n)
    b = bruit(n, rng, (3, 6, 12), (0.5, 0.3, 0.2)) - 0.5
    masque = lisse((1.0 - np.hypot((X - n / 2) / 56, (Y - n * 0.52) / 46) + b * 0.5) * 4)
    t0 = bruit(n, rng, (4, 8, 16), (0.4, 0.35, 0.25))
    sol = image(melange((138, 108, 76), (112, 86, 60), lisse((t0 - 0.3) * 2.2)) * (0.94 + 0.12 * rng.random((n, n)))[..., None], masque)
    t = Toile(n)
    fx, fy, fw, fh = n / 2 - 34, n * 0.30, 68, 44
    # Sablières : quatre poutres posées au sol, toujours là.
    for (ax, ay, bx, by) in ((fx, fy, fx + fw, fy), (fx + fw, fy, fx + fw, fy + fh),
                             (fx, fy + fh, fx + fw, fy + fh), (fx, fy, fx, fy + fh)):
        t.ligne(ax, ay, bx, by, POUTRE, 5.0)
        t.ligne(ax, ay, bx, by, POUTRE_CLAIRE, 2.2)
    if etat >= 1:  # ossature : poteaux aux angles et au milieu, traverse centrale
        for px in (fx, fx + fw / 2, fx + fw):
            for py in (fy, fy + fh):
                t.rect(px - 3.2, py - 3.2, px + 3.2, py + 3.2, POUTRE)
                t.rect(px - 1.6, py - 1.6, px + 1.6, py + 1.6, POUTRE_CLAIRE)
        t.rect(fx, fy + fh / 2 - 2.2, fx + fw, fy + fh / 2 + 2.2, POUTRE)
        t.rect(fx, fy + fh / 2 - 1.0, fx + fw, fy + fh / 2 + 1.0, POUTRE_CLAIRE)
    if etat >= 2:  # chevrons et premiers bardeaux sur un pan
        k = 8
        for i in range(k + 1):
            x = fx + 3 + i * (fw - 6) / k
            t.rect(x - 1.8, fy - 2, x + 1.8, fy + fh + 2, (146, 112, 74))
        t.rect(fx - 2, fy + fh / 2 - 1.6, fx + fw + 2, fy + fh / 2 + 1.6, POUTRE_CLAIRE)
        for i in range(int(k * 0.55)):
            x0 = fx + 3 + i * (fw - 6) / k
            for j in range(4):
                c = [(140, 112, 78), (122, 98, 70), (152, 126, 90)][int(rng.integers(3))]
                t.rect(x0 - 1.8, fy + fh / 2 + 2 + j * 9, x0 + (fw - 6) / k - 1.0, fy + fh / 2 + 10 + j * 9, c)
    # Bois en attente : tas de grumes et planches.
    _tas_de_grumes(t, n - 26, n * 0.78, 0.0, rng, k=3)
    for k2 in range(4):
        t.rect(14, n * 0.72 + k2 * 4.6, 50, n * 0.72 + k2 * 4.6 + 3.6, (190 - k2 * 2, 158 - k2 * 2, 110 - k2))
        t.rect(14, n * 0.72 + k2 * 4.6, 50, n * 0.72 + k2 * 4.6 + 1.0, (214, 184, 138))
    ob = t.vers_image()
    return empiler(sol, assise(ob, 0.28, 2.2), ob)
