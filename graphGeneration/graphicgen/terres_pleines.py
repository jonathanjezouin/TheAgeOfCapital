"""Terres exploitées « pleines » : textures de 256 px **raccordables** (le bord droit rejoint le bord
gauche, le haut le bas), pour couvrir une emprise de N tiles contiguës (docs/02 §8.2). Les limites de
parcelle (haie, clôture) sont tracées par le rendu, seulement le long du bord de l'emprise.
Soleil au zénith, aucune ombre portée : seulement une légère assise de contact sous les arbres.
"""
import math

import numpy as np
from PIL import Image

from .dessin import TUILE, Toile, assise, bruit, empiler, image, lisse, melange
from .terrain import HERBE, HERBE_SECHE
from .terres import (TERRE_LABOUR, TERRE_SILLON, _couronne, _fond_herbe, _mouton, _sillons, _vache)


def _sol_sillons(rng, angle, pas, couleur_a, couleur_b, n=TUILE):
    crete, rang, frac = _sillons(rng, n, angle, pas)
    texture = bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25))
    rgb = melange(couleur_b, couleur_a, lisse(crete * 1.1 - 0.05)) * (0.9 + 0.2 * texture)[..., None]
    rgb *= (0.96 + 0.08 * rng.random((n, n)))[..., None]
    return rgb


def _plants_pleins(rng, toile, angle, pas, couleurs, densite, rayon, n=TUILE):
    """Rangs de plants sur les crêtes (axes exacts, pas qui divise la tile : périodique)."""
    k = round(n / pas)
    m = max(8, round(n / (2.6 / densite)))
    horizontal = abs(math.sin(angle)) < 0.5  # rangs parallèles à l'axe x (crêtes en y = n/2 + r·pas)
    for r in range(k):
        u = (n / 2 + r * n / k) % n
        decal = rng.uniform(0, 2 * math.pi)
        for i in range(m):
            s = i * n / m
            w = math.sin(2 * math.pi * s / n * 13 + decal) * 1.3
            x, y = (s, u + w) if horizontal else (u + w, s)
            c = couleurs[rng.integers(len(couleurs))]
            c = tuple(np.clip(np.array(c) * rng.uniform(0.86, 1.14), 0, 255))
            toile.disque(x, y, rng.uniform(*rayon), c)


def champ_plein(graine, stade="pousse", n=TUILE):
    rng = np.random.default_rng(graine)
    angle = float(rng.choice([0.0, math.pi / 2]))
    pas = n / 20
    rgb = _sol_sillons(rng, angle, pas, TERRE_LABOUR, TERRE_SILLON, n)
    t = Toile(n, rep=True)
    cs = [(206, 176, 82), (190, 158, 70), (220, 190, 100)] if stade == "mur" else [(98, 142, 52), (86, 128, 46), (112, 156, 62), (74, 112, 42)]
    _plants_pleins(rng, t, angle, pas, cs, 1.7 if stade == "mur" else 1.3, (2.6, 4.0) if stade == "mur" else (2.2, 3.6), n)
    return empiler(image(rgb), t.vers_image())


def champ_lin_plein(graine, n=TUILE):
    rng = np.random.default_rng(graine)
    angle = float(rng.choice([0.0, math.pi / 2]))
    pas = n / 26
    rgb = _sol_sillons(rng, angle, pas, TERRE_LABOUR * 1.05, TERRE_SILLON, n)
    t = Toile(n, rep=True)
    _plants_pleins(rng, t, angle, pas, [(110, 150, 102), (96, 138, 96), (124, 160, 108), (88, 128, 92)], 2.1, (2.0, 3.0), n)
    for _ in range(520):  # fleurs
        x, y = rng.uniform(0, n, 2)
        t.disque(x, y, rng.uniform(1.0, 1.9), (116 + rng.integers(-14, 14), 144 + rng.integers(-14, 14), 214 + rng.integers(-20, 20)))
    return empiler(image(rgb), t.vers_image())


def verger_plein(graine, n=TUILE):
    rng = np.random.default_rng(graine)
    sol = _fond_herbe(rng, np.ones((n, n), np.float32), 1.02)
    verts = [(82, 124, 50), (96, 140, 58), (70, 112, 46), (106, 148, 62)]
    fruits = [(190, 52, 44), (214, 84, 56), (226, 170, 60)]
    pas = n / 4
    arbres, ombres = Toile(n, rep=True), Toile(n, rep=True)
    couronnes = []
    for j in range(4):
        for i in range(4):
            x = pas / 2 + i * pas + (pas / 2 if j % 2 else 0) + rng.uniform(-4, 4)
            y = pas / 2 + j * pas + rng.uniform(-4, 4)
            couronnes.append((x % n, y, rng.uniform(19, 23)))
    for x, y, r in couronnes:
        ombres.disque(x, y, r + 3, (16, 22, 8, 120))
    ombres.flou(3)
    for x, y, r in couronnes:
        _couronne(arbres, x, y, r, rng, verts, fruits)
    return empiler(sol, ombres.vers_image(), arbres.vers_image())


def foret_pleine(graine, n=TUILE):
    rng = np.random.default_rng(graine)
    sol = melange(np.array([104, 84, 56], np.float32), np.array([126, 104, 70], np.float32), bruit(n, rng, (8, 16, 32), (0.4, 0.35, 0.25)))
    sol = sol * (0.93 + 0.14 * rng.random((n, n)))[..., None]
    verts = [(52, 92, 44), (62, 104, 48), (44, 80, 40), (72, 112, 52)]
    pts = []
    essais = 0
    while len(pts) < 54 and essais < 4000:
        essais += 1
        x, y, r = rng.uniform(0, n), rng.uniform(0, n), rng.uniform(14, 24)
        ok = True
        for px, py, pr in pts:  # distance sur le tore : la texture se raccorde
            dx, dy = abs(x - px), abs(y - py)
            dx, dy = min(dx, n - dx), min(dy, n - dy)
            if math.hypot(dx, dy) <= (r + pr) * 0.62:
                ok = False
                break
        if ok:
            pts.append((x, y, r))
    pts.sort(key=lambda p: p[2])
    ombres, couvert = Toile(n, rep=True), Toile(n, rep=True)
    for x, y, r in pts:
        ombres.disque(x, y, r + 3, (10, 18, 6, 110))
    ombres.flou(3)
    for x, y, r in pts:
        _couronne(couvert, x, y, r, rng, verts)
    return empiler(image(sol), ombres.vers_image(), couvert.vers_image())


def _prairie_animaux(graine, dessin, nombre, n=TUILE, teinte=1.06):
    rng = np.random.default_rng(graine)
    sol = _fond_herbe(rng, np.ones((n, n), np.float32), teinte)
    pts = []
    essais = 0
    while len(pts) < nombre and essais < 400:
        essais += 1
        x, y = rng.uniform(0, n, 2)
        if all(math.hypot(min(abs(x - px), n - abs(x - px)), min(abs(y - py), n - abs(y - py))) > 40 for px, py in pts):
            pts.append((x, y))
    ombres = Toile(n, rep=True)
    for x, y in pts:
        ombres.ellipse(x, y, 17, 10, (14, 20, 8, 110))
    ombres.flou(2.5)
    bêtes = Toile(n, rep=True)
    rng2 = np.random.default_rng(graine + 1)
    for x, y in pts:
        dessin(bêtes, x, y, rng2.uniform(0, 2 * math.pi), rng2)
    return empiler(sol, ombres.vers_image(), bêtes.vers_image())


def paturage_plein(graine, n=TUILE):
    return _prairie_animaux(graine, _vache, 3, n)


def elevage_laine_plein(graine, n=TUILE):
    return _prairie_animaux(graine, _mouton, 5, n)
