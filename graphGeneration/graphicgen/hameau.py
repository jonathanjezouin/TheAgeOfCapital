"""Hameaux : groupes de maisons réutilisant le générateur de toits, jardins, puits, arbres.

Le hameau embarque nativement des « rues » (voir docs/02 §1) : un chemin en croix traverse la tile
et se raccorde aux chemins voisins. Le hameau textile ajoute un grand atelier, des séchoirs à toile
et des ballots de fibres.
"""
import math

import numpy as np
from PIL import Image

from .cultures import CULTURES
from .dessin import TUILE, Toile, assise, bruit, coord, empiler, image, lisse, melange
from .house import ZENITH, generate_house
from .terrain import HERBE, HERBE_SECHE, TERRE, chemin
from .terres import _couronne

# Couleurs de toile et de fibres pour le séchoir et les ballots.
TOILES = [(226, 222, 206), (96, 120, 168), (196, 168, 112), (176, 82, 70)]


def _maison(graine, culture, richesse, ppm=6):
    return generate_house(graine, CULTURES[culture], richesse, ppm=ppm, light=ZENITH, relief=1.2)


def _cour(rng, rect, n=TUILE):
    """Cour de terre battue : tache ovale irrégulière autour d'une maison."""
    x0, y0, x1, y1 = rect
    X, Y = coord(n)
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    rx, ry = (x1 - x0) / 2, (y1 - y0) / 2
    b = bruit(n, rng, (6, 12, 24), (0.45, 0.35, 0.2)) - 0.5
    d = 1.0 - np.hypot((X - cx) / rx, (Y - cy) / ry) + b * 0.35
    t = bruit(n, rng, (10, 20, 40), (0.4, 0.35, 0.25))
    rgb = melange(TERRE[0] * 0.95, TERRE[2], t * 0.7) * (0.95 + 0.1 * rng.random((n, n)))[..., None]
    return image(rgb, lisse(d / 0.22) * 0.8)


def _placette(rng, n=TUILE):
    """Placette de terre battue au croisement des rues. Les rues elles-mêmes viennent de la couche
    infrastructure (selon les voisins réellement raccordés) ; le hameau n'en embarque pas."""
    X, Y = coord(n)
    b = bruit(n, rng, (6, 12, 24), (0.45, 0.35, 0.2)) - 0.5
    d = 1.0 - np.hypot(X - n / 2, Y - n / 2) / 40.0 + b * 0.3
    t = bruit(n, rng, (10, 20, 40), (0.4, 0.35, 0.25))
    rgb = melange(TERRE[0] * 0.95, TERRE[2], t * 0.7) * (0.95 + 0.1 * rng.random((n, n)))[..., None]
    return image(rgb, lisse(d / 0.25))


def _jardin(toile, rng, x, y, w, h):
    """Potager : planches de terre et rangs de légumes."""
    toile.rect(x, y, x + w, y + h, (104, 78, 54))
    rangs = int(h // 8)
    for i in range(rangs):
        yy = y + 4 + i * 8
        toile.rect(x + 2, yy - 1.5, x + w - 2, yy + 1.5, (86, 62, 44))
        px = x + 4
        while px < x + w - 4:
            c = [(94, 140, 58), (120, 156, 64), (150, 160, 70), (176, 70, 62)][rng.integers(4)]
            toile.disque(px, yy, rng.uniform(2.0, 3.2), c)
            px += 5
    toile.contour_rect(x, y, x + w, y + h, (130, 98, 66), 1.6)


def _puits(toile, x, y):
    toile.disque(x, y, 9.5, (150, 146, 136))
    toile.disque(x, y, 7.4, (112, 108, 100))
    toile.disque(x, y, 5.2, (30, 36, 40))
    toile.disque(x, y, 3.4, (46, 60, 70))


def _arbre(toile, ombres, rng, x, y, r=17):
    verts = [(78, 120, 50), (92, 136, 56), (68, 108, 46)]
    ombres.disque(x, y, r + 3, (16, 22, 8, 110))
    _couronne(toile, x, y, r, rng, verts)


def _poser_maisons(rng, graine, specs, quadrants):
    """Une maison par quadrant libre autour du croisement de rues.

    Une maison trop grande pour son quadrant est régénérée à plus petite échelle (même forme, car la
    forme ne dépend que de la graine), jusqu'à ce qu'elle tienne ; on la décale au hasard dans son quadrant.
    """
    ordre = list(rng.permutation(len(quadrants)))
    placees = []
    for k, (culture, richesse, ppm) in enumerate(specs):
        if k >= len(ordre):
            break
        qx0, qy0, qx1, qy1 = quadrants[ordre[k]]
        for echelle in (1.0, 0.84, 0.72, 0.62, 0.54):
            maison = _maison(graine * 31 + k * 7, culture, richesse, max(5, round(ppm * echelle)))
            toit = maison.roof.crop(maison.roof.getbbox())
            w, h = toit.size
            if w > qx1 - qx0 or h > qy1 - qy0:
                continue
            x = int(qx0 + rng.integers(0, qx1 - qx0 - w + 1))
            y = int(qy0 + rng.integers(0, qy1 - qy0 - h + 1))
            placees.append((toit, x, y, (x - 4, y - 4, x + w + 4, y + h + 4), maison, ordre[k]))
            break
    return placees


def hameau(graine, textile=False, n=TUILE):
    """Hameau ordinaire (3 maisons) ou textile (grand atelier + 2 maisons, séchoir, ballots).

    Retourne (image cuite zénith, albedo sans éclairage, normales), toutes RGBA 256x256.
    """
    rng = np.random.default_rng(graine)
    c = n / 2
    bande = 31  # demi-largeur des rues (27 px) plus un peu d'air : on ne bâtit pas dessus
    croix = [(c - bande, 0, c + bande, n), (0, c - bande, n, c + bande)]
    m = 6
    quadrants = [(m, m, c - bande - 2, c - bande - 2), (c + bande + 2, m, n - m, c - bande - 2),
                 (m, c + bande + 2, c - bande - 2, n - m), (c + bande + 2, c + bande + 2, n - m, n - m)]
    quadrants = [tuple(int(v) for v in q) for q in quadrants]
    # Chaume d'abord (1750), un peu de tuile ; maisons entretenues (peu de mousse).
    cultures = ["chaume", "chaume", "tuile_rouge", "chaume", "ardoise"]
    specs = [(cultures[int(rng.integers(len(cultures)))], float(rng.uniform(0.45, 0.85)), 11) for _ in range(3)]
    if textile:
        specs = [("tuile_rouge", 0.8, 12)] + specs[:2]  # l'atelier : un grand toit
    placees = _poser_maisons(rng, graine, specs, quadrants)
    occupes = {p[5] for p in placees}

    couche_cours = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    couche_cours.alpha_composite(_placette(rng))
    for toit, x, y, boite, _, _ in placees:
        couche_cours.alpha_composite(_cour(rng, (x - 13, y - 13, x + toit.width + 13, y + toit.height + 13)))

    deco = Toile(n)
    ombres = Toile(n)
    arbres = Toile(n)
    libres = [p[3] for p in placees] + croix
    def libre(x0, y0, x1, y1):
        return all(x1 < z[0] or x0 > z[2] or y1 < z[1] or y0 > z[3] for z in libres)
    # Potagers, puits, arbres, séchoir : dans les quadrants restants et les interstices.
    nb_jardins = 1 if textile else 2
    for _ in range(nb_jardins):
        for essai in range(80):
            w, h = rng.uniform(36, 48), rng.uniform(24, 32)
            x, y = rng.uniform(m, n - w - m), rng.uniform(m, n - h - m)
            if libre(x - 3, y - 3, x + w + 3, y + h + 3):
                _jardin(deco, rng, x, y, w, h)
                libres.append((x - 3, y - 3, x + w + 3, y + h + 3))
                break
    for essai in range(80):
        x, y = rng.uniform(20, n - 20, 2)
        if libre(x - 12, y - 12, x + 12, y + 12):
            _puits(deco, x, y)
            libres.append((x - 12, y - 12, x + 12, y + 12))
            break
    if textile:
        for essai in range(120):  # séchoir : cadre de bois avec toiles étendues
            horizontal = rng.random() < 0.5
            w, h = (76, 34) if horizontal else (34, 76)
            x, y = rng.uniform(m, n - w - m), rng.uniform(m, n - h - m)
            if libre(x - 3, y - 3, x + w + 3, y + h + 3):
                deco.rect(x, y, x + w, y + h, (140, 112, 76))
                k = 6
                for i in range(k):
                    tc = TOILES[int(rng.integers(len(TOILES)))]
                    if horizontal:
                        px = x + 3 + i * (w - 6) / k
                        deco.rect(px, y + 3, px + (w - 6) / k - 2, y + h - 3, tc)
                    else:
                        py = y + 3 + i * (h - 6) / k
                        deco.rect(x + 3, py, x + w - 3, py + (h - 6) / k - 2, tc)
                deco.contour_rect(x, y, x + w, y + h, (104, 78, 52), 2.4)
                libres.append((x - 3, y - 3, x + w + 3, y + h + 3))
                break
        for _ in range(4):  # ballots de fibres (lin pâle, laine crème)
            for essai in range(80):
                x, y = rng.uniform(14, n - 14, 2)
                if libre(x - 11, y - 11, x + 11, y + 11):
                    cb = [(196, 184, 138), (226, 220, 200), (150, 160, 130)][int(rng.integers(3))]
                    deco.disque(x, y, 10.4, tuple(np.array(cb) * 0.80))
                    deco.disque(x, y, 8.8, cb)
                    deco.disque(x - 1.8, y - 1.8, 4.0, tuple(np.clip(np.array(cb) * 1.08, 0, 255)))
                    libres.append((x - 11, y - 11, x + 11, y + 11))
                    break
    for _ in range(int(rng.integers(2, 5))):
        for essai in range(80):
            x, y, r = rng.uniform(16, n - 16), rng.uniform(16, n - 16), rng.uniform(13, 19)
            if libre(x - r, y - r, x + r, y + r):
                _arbre(arbres, ombres, rng, x, y, r)
                libres.append((x - r, y - r, x + r, y + r))
                break

    def poser(attribut):
        calque = Image.new("RGBA", (n, n), (0, 0, 0, 0))
        for toit, x, y, boite, maison, _ in placees:
            calque.alpha_composite(getattr(maison, attribut).crop(maison.roof.getbbox()), (x, y))
        return calque

    toits = poser("roof")
    contact = assise(toits, 0.34, 3)
    ombres.flou(3)
    deco_img = deco.vers_image()
    base = empiler(couche_cours, deco_img, ombres.vers_image(), contact, toits, arbres.vers_image())
    # Albedo et normales : seuls les toits ont du relief ; le reste est plat (normale vers le haut).
    normales = Image.new("RGBA", (n, n), (128, 128, 255, 255))
    normales.alpha_composite(poser("normal"))
    albedo = empiler(couche_cours, deco_img, ombres.vers_image(), contact, poser("albedo"), arbres.vers_image())
    return base, albedo, normales
