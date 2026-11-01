"""Outils de dessin communs : bruit périodique (raccordable), couleurs, toile sur-échantillonnée.

Toutes les tiles font `TUILE` pixels de côté. Vue strictement de dessus, soleil au zénith :
aucune ombre portée, seulement une légère assise de contact sous les objets.
"""
import math

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

TUILE = 256
SS = 2  # sur-échantillonnage des formes dessinées


def bruit_periodique(n, cellules, rng):
    """Bruit de valeur lissé, périodique : le bord droit se raccorde au bord gauche (et haut/bas)."""
    g = rng.random((cellules, cellules)).astype(np.float32)
    t = np.arange(n, dtype=np.float32) * cellules / n
    i0 = np.floor(t).astype(np.int64)
    f = (t - i0).astype(np.float32)
    f = f * f * (3 - 2 * f)
    i0 %= cellules
    i1 = (i0 + 1) % cellules
    a = g[i0][:, i0] * (1 - f)[None, :] + g[i0][:, i1] * f[None, :]
    b = g[i1][:, i0] * (1 - f)[None, :] + g[i1][:, i1] * f[None, :]
    return a * (1 - f)[:, None] + b * f[:, None]


def bruit(n, rng, cellules=(3, 6, 12, 24), poids=(0.45, 0.27, 0.18, 0.10)):
    """Somme de bruits périodiques, normalisée dans [0, 1]."""
    out = sum(p * bruit_periodique(n, c, rng) for c, p in zip(cellules, poids))
    return out / sum(poids)


def melange(c1, c2, t):
    """Interpolation de couleurs ; `t` est un tableau (h, w) dans [0, 1]."""
    c1, c2 = np.asarray(c1, np.float32), np.asarray(c2, np.float32)
    return c1 * (1 - t)[..., None] + c2 * t[..., None]


def lisse(x):
    x = np.clip(x, 0, 1)
    return x * x * (3 - 2 * x)


def image(rgb, alpha=None):
    """Tableau flottant (h, w, 3) [+ alpha (h, w) en 0..1] -> image RGBA."""
    rgb = np.clip(rgb, 0, 255).astype(np.uint8)
    a = np.full(rgb.shape[:2], 255, np.uint8) if alpha is None else (np.clip(alpha, 0, 1) * 255).astype(np.uint8)
    return Image.fromarray(np.dstack([rgb, a]), "RGBA")


def tableaux(img):
    """Image RGBA -> (rgb flottant (h, w, 3), alpha flottant (h, w) en 0..1)."""
    a = np.asarray(img.convert("RGBA"), np.float32)
    return a[..., :3], a[..., 3] / 255.0


def coord(n=TUILE):
    Y, X = np.mgrid[0:n, 0:n].astype(np.float32) + 0.5
    return X, Y


class Toile:
    """Calque RGBA transparent sur lequel on dessine des formes en coordonnées de tile (pixels)."""

    def __init__(self, n=TUILE):
        self.n = n
        self.img = Image.new("RGBA", (n * SS, n * SS), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.img)

    def _p(self, *pts):
        return [v * SS for v in pts]

    def disque(self, x, y, r, c):
        self.d.ellipse(self._p(x - r, y - r, x + r, y + r), fill=_rgba(c))

    def ellipse(self, x, y, rx, ry, c, angle=0.0):
        """Ellipse orientée de `angle` radians."""
        pts = []
        for i in range(28):
            t = 2 * math.pi * i / 28
            ex, ey = rx * math.cos(t), ry * math.sin(t)
            pts.append((x + ex * math.cos(angle) - ey * math.sin(angle),
                        y + ex * math.sin(angle) + ey * math.cos(angle)))
        self.polygone(pts, c)

    def rect(self, x0, y0, x1, y1, c):
        self.d.rectangle(self._p(x0, y0, x1, y1), fill=_rgba(c))

    def rect_oriente(self, x, y, long, large, angle, c):
        """Rectangle centré en (x, y), `long` le long de `angle`."""
        ca, sa = math.cos(angle), math.sin(angle)
        pts = []
        for dx, dy in ((-long / 2, -large / 2), (long / 2, -large / 2), (long / 2, large / 2), (-long / 2, large / 2)):
            pts.append((x + dx * ca - dy * sa, y + dx * sa + dy * ca))
        self.polygone(pts, c)

    def ligne(self, x0, y0, x1, y1, c, largeur=1.0):
        self.d.line(self._p(x0, y0, x1, y1), fill=_rgba(c), width=max(1, round(largeur * SS)))

    def polygone(self, pts, c):
        self.d.polygon([v * SS for p in pts for v in p], fill=_rgba(c))

    def contour_rect(self, x0, y0, x1, y1, c, largeur=2.0):
        self.d.rectangle(self._p(x0, y0, x1, y1), outline=_rgba(c), width=max(1, round(largeur * SS)))

    def flou(self, rayon):
        self.img = self.img.filter(ImageFilter.GaussianBlur(rayon * SS))
        self.d = ImageDraw.Draw(self.img)

    def vers_image(self):
        return self.img.resize((self.n, self.n), Image.LANCZOS)


def _rgba(c):
    c = tuple(int(round(v)) for v in c)
    return c if len(c) == 4 else c + (255,)


def assise(toile_formes, force=0.28, rayon=2.5):
    """Légère assise de contact : un halo sombre et flou sous les formes opaques d'une image RGBA."""
    a = np.asarray(toile_formes.convert("RGBA"))[..., 3]
    flou = Image.fromarray(a).filter(ImageFilter.GaussianBlur(rayon))
    halo = np.asarray(flou, np.float32) / 255.0 * force
    halo = np.where(a > 8, 0, halo)
    ombre = np.dstack([np.full(a.shape + (3,), (20, 22, 10), np.uint8), (halo * 255).astype(np.uint8)])
    return Image.fromarray(ombre, "RGBA")


def empiler(*calques):
    """Superpose des images RGBA de même taille, dans l'ordre donné (le premier est dessous)."""
    out = Image.new("RGBA", calques[0].size, (0, 0, 0, 0))
    for c in calques:
        out.alpha_composite(c)
    return out


def masque_parcelle(rng, marge=12, irregularite=5.0, n=TUILE, x0=None, y0=None, x1=None, y1=None):
    """Masque doux d'une parcelle rectangulaire à bords irréguliers (valeurs 0..1)."""
    X, Y = coord(n)
    x0 = marge if x0 is None else x0
    y0 = marge if y0 is None else y0
    x1 = n - marge if x1 is None else x1
    y1 = n - marge if y1 is None else y1
    b = bruit(n, rng, (6, 12, 24), (0.5, 0.3, 0.2)) - 0.5
    dist = np.minimum(np.minimum(X - x0, x1 - X), np.minimum(Y - y0, y1 - Y)) + b * 2 * irregularite
    return lisse(dist / 3.0)
