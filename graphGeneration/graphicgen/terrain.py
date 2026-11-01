"""Sol : herbe raccordable et chemins de terre (15 combinaisons de demi-directions orthogonales)."""
import math

import numpy as np

from .dessin import TUILE, bruit, bruit_periodique, coord, image, lisse, melange

# Demi-directions : mêmes valeurs que sim/src/monde.rs (N=1, E=2, S=4, O=8).
N, E, S, O = 1, 2, 4, 8

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


def _dist_segment(X, Y, ax, ay, bx, by):
    vx, vy = bx - ax, by - ay
    t = np.clip(((X - ax) * vx + (Y - ay) * vy) / (vx * vx + vy * vy), 0, 1)
    px, py = ax + t * vx, ay + t * vy
    return np.hypot(X - px, Y - py), (X - px) * (-vy) / math.hypot(vx, vy) + (Y - py) * vx / math.hypot(vx, vy)


def chemin(masque, graine=7, n=TUILE, demi_largeur=27.0):
    """Tile de chemin de terre à fond transparent.

    Chaque demi-direction relie le centre au milieu du bord correspondant ; le profil du chemin
    au bord est identique d'une tile à l'autre (bruit périodique, même graine), donc deux tiles
    voisines se raccordent sans couture.
    """
    rng = np.random.default_rng(graine)
    X, Y = coord(n)
    c = n / 2
    extremites = {N: (c, 0), E: (n, c), S: (c, n), O: (0, c)}
    # Largeur irrégulière : bruit périodique (même graine pour toutes les tiles de chemin).
    hw = demi_largeur * (0.80 + 0.40 * bruit_periodique(n, 8, rng))
    bord_irr = bruit(n, rng, (16, 32, 64), (0.4, 0.35, 0.25))
    textures = bruit(n, rng, (6, 12, 24, 48), (0.35, 0.3, 0.2, 0.15))
    grain = rng.random((n, n)).astype(np.float32)
    cailloux = rng.random((n, n)).astype(np.float32)

    dists, perps, k = [], [], 0.18
    for bit, (ex, ey) in extremites.items():
        if masque & bit:
            d, p = _dist_segment(X, Y, c, c, ex, ey)
            dists.append(d)
            perps.append(p)
    if not dists:
        return image(np.zeros((n, n, 3)), np.zeros((n, n)))
    D = np.stack(dists)
    d = -np.log(np.exp(-k * D).sum(axis=0)) / k  # minimum doux : jonctions arrondies
    perp = np.take_along_axis(np.stack(perps), D.argmin(axis=0)[None], axis=0)[0]

    # Bord irrégulier : l'herbe mord sur la terre par endroits.
    marge = hw + (bord_irr - 0.5) * 9.0
    alpha = lisse((marge - d) / 5.0)
    couleur = melange(TERRE[0], TERRE[1], lisse((textures - 0.35) * 2.0))
    couleur = melange(couleur, TERRE[2], lisse((textures - 0.60) * 3.0) * 0.6)
    # Ornières : deux sillons à peine plus sombres, bande centrale un peu herbeuse.
    ornieres = np.exp(-(((np.abs(perp) - 0.42 * hw) / 4.0) ** 2)) * (0.5 + 0.5 * textures)
    couleur = couleur * (1 - 0.13 * ornieres)[..., None]
    centre = lisse(1 - np.abs(perp) / (0.22 * hw)) * lisse((textures - 0.50) * 3)
    couleur = couleur * (1 - 0.35 * centre)[..., None] + HERBE_SOMBRE * (0.35 * centre)[..., None]
    couleur = couleur * (0.95 + 0.10 * grain)[..., None]
    pierre = (cailloux > 0.992)
    couleur = np.where(pierre[..., None], np.array([190, 178, 154], np.float32), couleur)
    # Bords : la terre s'éclaircit et se mêle à l'herbe sèche (pas de liseré sombre).
    bord = lisse(1 - (marge - d) / 13.0)
    sec = HERBE_SECHE * 0.85 + TERRE[0] * 0.15
    couleur = couleur * (1 - 0.55 * bord)[..., None] + sec * (0.55 * bord)[..., None]
    return image(couleur, alpha)
