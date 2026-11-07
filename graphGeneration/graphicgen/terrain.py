"""Sol : herbe raccordable et texture de terre battue pour les routes."""
import math

import numpy as np

from .dessin import TUILE, bruit, bruit_periodique, coord, image, lisse, melange

HERBE = np.array([112, 126, 54], np.float32)
HERBE_SECHE = np.array([140, 140, 66], np.float32)
HERBE_SOMBRE = np.array([76, 98, 42], np.float32)

HERBE_MOYENNE = np.array([112, 126, 55], np.float32)

TERRE = (np.array([150, 122, 88], np.float32), np.array([126, 98, 68], np.float32),
         np.array([168, 140, 102], np.float32))


def herbe(graine, n=TUILE):
    """Tile d'herbe raccordable (le bord droit se raccorde au bord gauche, le bas au haut)."""
    rng = np.random.default_rng(graine)
    teinte = bruit(n, rng, (2, 3, 5), (0.5, 0.3, 0.2))[..., None]          # jaunissement doux, grande échelle
    touffes = bruit(n, rng, (10, 20, 40), (0.4, 0.35, 0.25))[..., None]     # touffes, échelle moyenne
    fin = bruit(n, rng, (64, 128), (0.6, 0.4))[..., None]                   # grain fin, corrélé sur 2-4 px
    pixel = rng.random((n, n, 1)).astype(np.float32)
    img = HERBE * (1 - teinte * 0.30) + HERBE_SECHE * teinte * 0.30
    img = img * (0.88 + 0.22 * touffes) * (0.92 + 0.16 * fin) * (0.97 + 0.06 * pixel)
    img = img + HERBE_SOMBRE * ((touffes - 0.62).clip(0) * 0.9)
    # Brins : petits traits clairs et sombres, répartis modulo la tile (donc raccordables).
    for _ in range(5200):
        x, y = rng.integers(0, n, 2)
        dx, dy = rng.integers(-1, 2), rng.integers(1, 4)
        clair = rng.random() < 0.55
        facteur = (1.0 + rng.uniform(0.04, 0.20)) if clair else (1.0 - rng.uniform(0.05, 0.24))
        for k in range(dy + 1):
            xx, yy = (x + dx * k) % n, (y - k) % n
            img[yy, xx] = img[yy, xx] * facteur
    # Même couleur moyenne pour toutes les variantes : pas de quadrillage visible entre tiles voisines.
    img = img * (HERBE_MOYENNE / img.reshape(-1, 3).mean(axis=0))
    return image(img)


def sol_chemin(graine=7, n=TUILE):
    """Texture de terre battue raccordable (RGB opaque), pour le trait des routes.

    Le rendu trace le réseau en courbes (docs/02 §9, docs/03 §6) et remplit le trait avec cette
    texture, répétée : le bord droit se raccorde au bord gauche, le haut au bas."""
    rng = np.random.default_rng(graine)
    textures = bruit(n, rng, (6, 12, 24, 48), (0.35, 0.3, 0.2, 0.15))
    grain = rng.random((n, n)).astype(np.float32)
    couleur = melange(TERRE[0], TERRE[1], lisse((textures - 0.35) * 2.0))
    couleur = melange(couleur, TERRE[2], lisse((textures - 0.60) * 3.0) * 0.6)
    couleur = couleur * (0.95 + 0.10 * grain)[..., None]
    # Cailloux : points clairs et petits creux sombres, répartis modulo la tile.
    for _ in range(160):
        x, y = rng.integers(0, n, 2)
        r = int(rng.integers(1, 3))
        clair = rng.random() < 0.6
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if dx * dx + dy * dy <= r * r:
                    f = 1.22 if clair else 0.82
                    couleur[(y + dy) % n, (x + dx) % n] *= f
    return image(couleur)
