"""Terres exploitées vues de dessus : champ, pâturage, verger, forêt, lin, élevage de laine.

Chaque sprite est un calque RGBA à fond transparent, à poser sur une tile d'herbe.
Soleil au zénith : pas d'ombre portée, une légère assise de contact sous les objets.
"""
import math

import numpy as np
from PIL import Image

from .dessin import (TUILE, Toile, assise, bruit, coord, empiler, image, lisse, masque_parcelle,
                     melange, tableaux)
from .terrain import HERBE, HERBE_SECHE, HERBE_SOMBRE

TERRE_LABOUR = np.array([112, 84, 58], np.float32)
TERRE_SILLON = np.array([84, 60, 42], np.float32)
BOIS = np.array([128, 96, 62], np.float32)
BOIS_CLAIR = np.array([164, 128, 86], np.float32)


# --------------------------------------------------------------------------- parcelles à sillons

def _sillons(rng, n, angle, pas, force=0.30):
    """Champ de crêtes/sillons orienté (valeur 0..1 : 1 = crête). Retourne aussi la coordonnée de rang."""
    X, Y = coord(n)
    u = (X - n / 2) * math.sin(angle) + (Y - n / 2) * math.cos(angle)
    gigue = (bruit(n, rng, (4, 8), (0.6, 0.4)) - 0.5) * pas * 0.45
    ph = (u + gigue) / pas
    return 0.5 + 0.5 * np.cos(2 * math.pi * ph), np.floor(ph), (ph - np.floor(ph))


def _parcelle(rng, couleur_a, couleur_b, angle, pas, marge=14, irregularite=5.0):
    """Terre labourée : crêtes claires, sillons sombres, mouchetures. Retourne (rgb, masque)."""
    n = TUILE
    crete, rang, frac = _sillons(rng, n, angle, pas)
    texture = bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25))
    rgb = melange(couleur_b, couleur_a, lisse(crete * 1.1 - 0.05)) * (0.9 + 0.2 * texture)[..., None]
    rgb *= (0.96 + 0.08 * rng.random((n, n)))[..., None]
    return rgb, masque_parcelle(rng, marge, irregularite), rang, frac


def _haie(rng, masque, n=TUILE):
    """Liseré de haie/talus le long du bord de la parcelle : touffes vertes sombres."""
    X, Y = coord(n)
    bord = (masque > 0.05) & (masque < 0.999)
    # bande de 4 px sous le bord intérieur
    intérieur = lisse(masque * 1.0)
    from PIL import ImageFilter
    m = Image.fromarray((masque > 0.5).astype(np.uint8) * 255).filter(ImageFilter.MinFilter(9))
    cœur = np.asarray(m, np.float32) / 255
    bande = np.clip((masque > 0.5) * 1.0 - cœur, 0, 1)
    t = bruit(n, rng, (24, 48), (0.5, 0.5))
    rgb = melange(HERBE_SOMBRE * 0.85, HERBE * 0.9, t)
    return rgb, bande * (0.55 + 0.45 * t)


def _plantes(rng, toile, rang, frac, angle, pas, couleurs, densite, rayon=(2.2, 3.6), n=TUILE):
    """Rangs de plants le long des crêtes : files continues de petits amas, légèrement ondulées."""
    sa, ca = math.sin(angle), math.cos(angle)
    u = -n
    while u < n:
        decal = rng.uniform(0, 2 * math.pi)
        s = -n
        while s < n:
            x = n / 2 + u * sa + s * ca + math.sin(s / 19.0 + decal) * 1.3
            y = n / 2 + u * ca - s * sa
            if -6 < x < n + 6 and -6 < y < n + 6:
                c = couleurs[rng.integers(len(couleurs))]
                c = tuple(np.clip(np.array(c) * rng.uniform(0.86, 1.14), 0, 255))
                toile.disque(x, y, rng.uniform(*rayon), c)
            s += 2.6 / densite
        u += pas


def champ(graine, stade="pousse", n=TUILE):
    """Champ de céréales : sillons, jeunes pousses vertes (ou chaume doré si `stade` = 'mur')."""
    rng = np.random.default_rng(graine)
    angle = rng.choice([0.0, math.pi / 2]) + rng.uniform(-0.07, 0.07)
    pas = rng.uniform(11, 14)
    rgb, masque, rang, frac = _parcelle(rng, TERRE_LABOUR, TERRE_SILLON, angle, pas)
    base = image(rgb, masque)
    t = Toile(n)
    if stade == "mur":
        cs = [(206, 176, 82), (190, 158, 70), (220, 190, 100)]
        _plantes(rng, t, rang, frac, angle, pas, cs, 1.7, rayon=(2.6, 4.0))
    else:
        cs = [(98, 142, 52), (86, 128, 46), (112, 156, 62), (74, 112, 42)]
        _plantes(rng, t, rang, frac, angle, pas, cs, 1.3)
    plants = t.vers_image()
    # on n'affiche les plants que dans la parcelle
    a = np.asarray(plants, np.float32)
    a[..., 3] *= masque
    plants = Image.fromarray(a.astype(np.uint8), "RGBA")
    hr, ha = _haie(rng, masque)
    haie = image(hr, ha)
    return empiler(assise(base, 0.22, 3), base, plants, haie)


def champ_lin(graine, n=TUILE):
    """Champ de lin : rangs serrés vert-bleu, semés de petites fleurs bleues."""
    rng = np.random.default_rng(graine)
    angle = rng.choice([0.0, math.pi / 2]) + rng.uniform(-0.07, 0.07)
    pas = rng.uniform(9, 11)
    rgb, masque, rang, frac = _parcelle(rng, TERRE_LABOUR * 1.05, TERRE_SILLON, angle, pas, 14, 6.0)
    base = image(rgb, masque)
    t = Toile(n)
    cs = [(110, 150, 102), (96, 138, 96), (124, 160, 108), (88, 128, 92)]
    _plantes(rng, t, rang, frac, angle, pas, cs, 2.1, rayon=(2.0, 3.0))
    for _ in range(520):  # fleurs
        x, y = rng.uniform(16, n - 16, 2)
        t.disque(x, y, rng.uniform(1.0, 1.9), (116 + rng.integers(-14, 14), 144 + rng.integers(-14, 14), 214 + rng.integers(-20, 20)))
    f = t.vers_image()
    a = np.asarray(f, np.float32)
    a[..., 3] *= masque
    f = Image.fromarray(a.astype(np.uint8), "RGBA")
    hr, ha = _haie(rng, masque)
    return empiler(assise(base, 0.22, 3), base, f, image(hr, ha))


# --------------------------------------------------------------------------- arbres

def _couronne(toile, x, y, r, rng, verts, fruits=None):
    """Couronne d'arbre vue de dessus : disque de feuillage fait de nombreux petits amas."""
    toile.disque(x, y, r * 1.02, tuple(np.array(verts[0]) * 0.72))
    for _ in range(int(r * 2.2)):
        a = rng.uniform(0, 2 * math.pi)
        d = r * math.sqrt(rng.uniform(0, 1)) * 0.86
        c = np.array(verts[rng.integers(len(verts))]) * rng.uniform(0.84, 1.14)
        # plus clair vers le centre (feuillage dense dessus), plus sombre vers le bord
        c = c * (1.12 - 0.34 * (d / r))
        toile.disque(x + d * math.cos(a), y + d * math.sin(a), r * rng.uniform(0.22, 0.38), tuple(np.clip(c, 0, 255)))
    if fruits:
        for _ in range(max(3, int(r * 0.45))):
            a = rng.uniform(0, 2 * math.pi)
            d = r * math.sqrt(rng.uniform(0.1, 1)) * 0.8
            toile.disque(x + d * math.cos(a), y + d * math.sin(a), rng.uniform(1.4, 2.2), fruits[rng.integers(len(fruits))])


def _fond_herbe(rng, masque, teinte=1.0):
    n = TUILE
    t = bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25))
    rgb = melange(HERBE * 0.96 * teinte, HERBE_SECHE * 0.95 * teinte, t * 0.6)
    rgb *= (0.95 + 0.1 * rng.random((n, n)))[..., None]
    return image(rgb, masque)


def verger(graine, n=TUILE):
    """Verger : grille décalée d'arbres fruitiers sur une prairie, avec quelques fruits rouges."""
    rng = np.random.default_rng(graine)
    masque = masque_parcelle(rng, 12, 6.0)
    sol = _fond_herbe(rng, masque, 1.02)
    arbres = Toile(n)
    verts = [(82, 124, 50), (96, 140, 58), (70, 112, 46), (106, 148, 62)]
    fruits = [(190, 52, 44), (214, 84, 56), (226, 170, 60)]
    pas = 56
    couronnes = []
    for j in range(4):
        for i in range(4):
            x = 36 + i * pas + (pas / 2 if j % 2 else 0) + rng.uniform(-4, 4)
            y = 36 + j * pas + rng.uniform(-4, 4)
            if x < n - 22:
                couronnes.append((x, y, rng.uniform(19, 23)))
    ombres = Toile(n)
    for x, y, r in couronnes:
        ombres.disque(x, y, r + 3, (16, 22, 8, 120))
    ombres.flou(3)
    for x, y, r in couronnes:
        _couronne(arbres, x, y, r, rng, verts, fruits)
    hr, ha = _haie(rng, masque)
    return empiler(sol, ombres.vers_image(), arbres.vers_image(), image(hr, ha))


def _foret_arbres(rng, n, zone, verts, nombre, rmin, rmax):
    """Couronnes réparties dans `zone` (masque 0..1), rejetant les chevauchements excessifs."""
    pts = []
    essais = 0
    while len(pts) < nombre and essais < nombre * 60:
        essais += 1
        x, y = rng.uniform(4, n - 4, 2)
        if zone[int(y), int(x)] < 0.55:
            continue
        r = rng.uniform(rmin, rmax)
        if all(math.hypot(x - px, y - py) > (r + pr) * 0.62 for px, py, pr in pts):
            pts.append((x, y, r))
    pts.sort(key=lambda p: p[2])
    t = Toile(n)
    ombres = Toile(n)
    for x, y, r in pts:
        ombres.disque(x, y, r + 3, (10, 18, 6, 110))
    ombres.flou(3)
    for x, y, r in pts:
        _couronne(t, x, y, r, rng, verts)
    return ombres.vers_image(), t.vers_image(), pts


def foret_exploitee(graine, n=TUILE):
    """Forêt exploitée : couvert dense, avec une clairière où sont empilées des grumes et des souches."""
    rng = np.random.default_rng(graine)
    X, Y = coord(n)
    # Blob irrégulier couvrant presque toute la tile.
    b = bruit(n, rng, (3, 6, 12), (0.5, 0.3, 0.2))
    d = np.hypot((X - n / 2) / (n / 2), (Y - n / 2) / (n / 2))
    zone = lisse((1.04 - d + (b - 0.5) * 0.55) * 6)
    # Clairière décentrée.
    cx, cy = n * rng.uniform(0.36, 0.64), n * rng.uniform(0.42, 0.62)
    rc = rng.uniform(46, 58)
    clair = lisse((rc - np.hypot((X - cx) * 0.9, (Y - cy) * 1.1)) / 8.0)
    sol = melange(np.array([104, 84, 56], np.float32), np.array([126, 104, 70], np.float32), bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25)))
    sol = sol * (0.93 + 0.14 * rng.random((n, n)))[..., None]
    fond = image(np.where(zone[..., None] > 0, sol, 0), zone)
    verts = [(52, 92, 44), (62, 104, 48), (44, 80, 40), (72, 112, 52)]
    ombres, couvert, pts = _foret_arbres(rng, n, zone * (1 - clair), verts, 46, 14, 24)
    # Clairière : souches et grumes empilées.
    t = Toile(n)
    for _ in range(int(rng.integers(3, 6))):
        a, dd = rng.uniform(0, 2 * math.pi), rc * math.sqrt(rng.uniform(0.15, 0.85))
        sx, sy = cx + dd * math.cos(a), cy + dd * math.sin(a)
        t.disque(sx, sy, 6.5, (118, 88, 58))
        t.disque(sx, sy, 5.0, (176, 142, 98))
        t.disque(sx, sy, 3.0, (150, 118, 80))
        t.disque(sx, sy, 1.2, (132, 100, 68))
    ang = rng.uniform(-0.4, 0.4)
    for k in range(4):  # tas de grumes
        t.rect_oriente(cx + 4 + math.cos(ang) * 0, cy - 14 + k * 9, 50, 8.0, ang, (112, 80, 52))
        t.rect_oriente(cx + 4 + 25 * math.cos(ang), cy - 14 + k * 9 + 25 * math.sin(ang), 2.6, 7.6, ang, (190, 156, 108))
    bois = t.vers_image()
    return empiler(fond, assise(fond, 0.18, 3), ombres, couvert, bois)


# --------------------------------------------------------------------------- animaux, clôtures

def _cloture(toile, x0, y0, x1, y1, pas=22):
    """Clôture de piquets et de deux lisses le long d'un rectangle."""
    bord = (126, 96, 66)
    clair = (170, 136, 94)
    for (ax, ay, bx, by) in ((x0, y0, x1, y0), (x1, y0, x1, y1), (x1, y1, x0, y1), (x0, y1, x0, y0)):
        toile.ligne(ax, ay, bx, by, bord, 2.6)
        toile.ligne(ax, ay, bx, by, clair, 1.0)
        longueur = math.hypot(bx - ax, by - ay)
        k = max(2, int(longueur / pas))
        for i in range(k + 1):
            px, py = ax + (bx - ax) * i / k, ay + (by - ay) * i / k
            toile.rect(px - 2.4, py - 2.4, px + 2.4, py + 2.4, (96, 70, 46))
            toile.rect(px - 1.2, py - 1.2, px + 1.2, py + 1.2, (150, 118, 82))


def _vache(toile, x, y, ang, rng):
    corps = rng.choice([(122, 80, 52), (96, 62, 42), (186, 160, 128), (138, 98, 66)])
    toile.ellipse(x, y, 16, 9.5, tuple(corps), ang)
    if rng.random() < 0.6:  # taches claires
        for _ in range(2):
            dx, dy = rng.uniform(-7, 7), rng.uniform(-3, 3)
            toile.ellipse(x + dx * math.cos(ang) - dy * math.sin(ang), y + dx * math.sin(ang) + dy * math.cos(ang),
                          rng.uniform(3, 5), rng.uniform(2, 3.4), (226, 214, 196), ang + rng.uniform(-1, 1))
    hx, hy = x + 16 * math.cos(ang), y + 16 * math.sin(ang)
    toile.ellipse(hx, hy, 6.8, 5.2, tuple(np.array(corps) * 0.9), ang)
    toile.disque(hx + 4 * math.cos(ang), hy + 4 * math.sin(ang), 2.1, (60, 44, 36))
    toile.disque(x - 16 * math.cos(ang), y - 16 * math.sin(ang), 1.6, (80, 60, 44))


def _mouton(toile, x, y, ang, rng):
    laine = (232, 226, 208)
    for dx, dy in ((-5, 0), (0, -2.5), (0, 2.5), (5, 0), (0, 0)):
        toile.disque(x + dx * math.cos(ang) - dy * math.sin(ang), y + dx * math.sin(ang) + dy * math.cos(ang),
                     rng.uniform(5.2, 6.6), tuple(np.array(laine) * rng.uniform(0.92, 1.0)))
    hx, hy = x + 9.5 * math.cos(ang), y + 9.5 * math.sin(ang)
    toile.ellipse(hx, hy, 4.2, 3.2, (70, 56, 48), ang)


def _pre(rng, n, masque):
    sol = _fond_herbe(rng, masque, 1.06)
    return sol


def _place_animaux(rng, toile, n, nombre, dessin, rmin_dist=26, marge=36):
    pts = []
    essais = 0
    while len(pts) < nombre and essais < 400:
        essais += 1
        x, y = rng.uniform(marge, n - marge, 2)
        if all(math.hypot(x - px, y - py) > rmin_dist for px, py in pts):
            pts.append((x, y))
            dessin(toile, x, y, rng.uniform(0, 2 * math.pi), rng)
    return pts


def paturage(graine, n=TUILE):
    """Pâturage clos : prairie, clôture, une demi-douzaine de bovins."""
    rng = np.random.default_rng(graine)
    m = 14
    masque = masque_parcelle(rng, m, 4.0)
    sol = _pre(rng, n, masque)
    t = Toile(n)
    _cloture(t, m + 4, m + 4, n - m - 4, n - m - 4)
    ombre = Toile(n)
    pts = _place_animaux(rng, ombre, n, 5, _vache)
    ombres = Toile(n)
    for x, y in pts:
        ombres.ellipse(x, y, 17, 10, (14, 20, 8, 110))
    ombres.flou(2.5)
    vaches = Toile(n)
    rng2 = np.random.default_rng(graine + 1)
    for x, y in pts:
        _vache(vaches, x, y, rng2.uniform(0, 2 * math.pi), rng2)
    return empiler(sol, ombres.vers_image(), vaches.vers_image(), t.vers_image())


def elevage_laine(graine, n=TUILE):
    """Élevage de moutons : enclos, bergerie à toit de chaume dans un coin, moutons."""
    from .cultures import CULTURES
    from .house import ZENITH, generate_house
    rng = np.random.default_rng(graine)
    m = 14
    masque = masque_parcelle(rng, m, 4.0)
    sol = _pre(rng, n, masque)
    t = Toile(n)
    _cloture(t, m + 4, m + 4, n - m - 4, n - m - 4)
    # Bergerie : toit de chaume (générateur de maisons), coin haut-gauche.
    cu = CULTURES["chaume"]
    abri = generate_house(graine * 7 + 3, cu, 0.15, ppm=6, light=ZENITH, relief=1.2)
    bx, by = 34, 34
    toit = abri.roof.crop(abri.roof.getbbox())
    ombre_abri = assise(Image.new("RGBA", (n, n)), 0.0, 1)  # placeholder
    support = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    support.alpha_composite(toit, (bx, by))
    halo = assise(support, 0.30, 2.5)
    sheep = Toile(n)
    rng2 = np.random.default_rng(graine + 5)
    pts = []
    essais = 0
    while len(pts) < 8 and essais < 500:
        essais += 1
        x, y = rng2.uniform(40, n - 40, 2)
        if (bx - 14 < x < bx + toit.width + 14) and (by - 14 < y < by + toit.height + 14):
            continue
        if all(math.hypot(x - px, y - py) > 24 for px, py in pts):
            pts.append((x, y))
    ombres = Toile(n)
    for x, y in pts:
        ombres.ellipse(x, y, 14, 9, (14, 20, 8, 100))
    ombres.flou(2.2)
    for x, y in pts:
        _mouton(sheep, x, y, rng2.uniform(0, 2 * math.pi), rng2)
    return empiler(sol, ombres.vers_image(), halo, support, sheep.vers_image(), t.vers_image())
