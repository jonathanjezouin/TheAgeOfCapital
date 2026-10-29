"""Génération d'un toit vu de dessus, par carte de hauteurs + éclairage algorithmique.

Principe : chaque bâtiment est une union de volumes (toit à deux ou quatre pans).
On calcule la hauteur du toit en chaque pixel, on en déduit les normales, puis la
lumière (Lambert) donne le volume : pans clairs/sombres, vallées, faîtage, tout
vient du calcul et reste cohérent quelle que soit la forme.
"""
import math
from dataclasses import dataclass

import numpy as np
from PIL import Image, ImageFilter

from .cultures import Culture
from .noise import hash2, value_noise

SS = 4  # sur-échantillonnage (anti-aliasing par réduction)

# Lumière : x vers la droite, y vers le bas, z vers le haut. Ici venue du haut-gauche.
DEFAULT_LIGHT = (-0.55, -0.60, 0.58)

BRICK = np.array([128, 92, 80], np.float32)
MOSS = np.array([86, 100, 58], np.float32)
UNDERLAY = np.array([58, 46, 40], np.float32)
METAL = np.array([150, 150, 144], np.float32)


@dataclass
class Part:
    x0: float
    y0: float
    x1: float
    y1: float
    ridge_axis: str  # 'x' ou 'y'
    hip: bool


@dataclass
class HouseSprite:
    roof: Image.Image
    shadow: Image.Image
    meta: dict

    def composite(self):
        out = Image.new("RGBA", self.roof.size, (0, 0, 0, 0))
        out.alpha_composite(self.shadow)
        out.alpha_composite(self.roof)
        return out


def _build_parts(rng, cu: Culture, wealth: float):
    w = rng.uniform(*cu.main_w) * (0.85 + 0.3 * wealth)
    d = rng.uniform(*cu.main_d) * (0.9 + 0.2 * wealth)
    hip = rng.random() < cu.hip_prob
    parts = [Part(0, 0, w, d, "x", hip)]
    if rng.random() < cu.wing_prob * (0.4 + wealth):
        ww = d * rng.uniform(0.6, 0.85)
        wl = rng.uniform(3.0, 5.0)
        xp = rng.uniform(0.5, max(0.6, w - ww - 0.5))
        if rng.random() < 0.5:
            parts.append(Part(xp, -wl, xp + ww, d * 0.5, "y", rng.random() < cu.hip_prob))
        else:
            parts.append(Part(xp, d * 0.5, xp + ww, d + wl, "y", rng.random() < cu.hip_prob))
    if rng.random() < 0.5:  # orientation : on transpose tout
        parts = [Part(p.y0, p.x0, p.y1, p.x1, "y" if p.ridge_axis == "x" else "x", p.hip)
                 for p in parts]
    return parts


def _lerp_palette(colors, t):
    pal = np.asarray(colors, np.float32)
    idx = t * (len(pal) - 1)
    i = np.floor(idx).astype(int)
    f = (idx - i)[..., None]
    return pal[i] * (1 - f) + pal[np.minimum(i + 1, len(pal) - 1)] * f


def _shift(a, dx, dy):
    return np.roll(np.roll(a, int(round(dy)), axis=0), int(round(dx)), axis=1)


def generate_house(seed, culture: Culture, wealth=0.5, ppm=6, light=DEFAULT_LIGHT):
    """Retourne un HouseSprite (toit RGBA + ombre RGBA), à `ppm` pixels par mètre."""
    rng = np.random.default_rng(seed)
    L = np.asarray(light, np.float32)
    L /= np.linalg.norm(L)
    wealth = float(np.clip(wealth, 0, 1))
    wear = float(np.clip((1 - wealth) * rng.uniform(0.6, 1.1), 0, 1))

    parts = _build_parts(rng, culture, wealth)
    minx = min(p.x0 for p in parts)
    miny = min(p.y0 for p in parts)
    maxx = max(p.x1 for p in parts)
    maxy = max(p.y1 for p in parts)
    margin = 3.0
    Wf = math.ceil((maxx - minx + 2 * margin) * ppm)
    Hf = math.ceil((maxy - miny + 2 * margin) * ppm)
    k = ppm * SS  # pixels internes par mètre
    W, H = Wf * SS, Hf * SS
    ox, oy = margin - minx, margin - miny

    Y, X = np.mgrid[0:H, 0:W].astype(np.float32) + 0.5

    # --- carte de hauteurs --------------------------------------------------
    h = np.full((H, W), -1.0, np.float32)
    r_best = np.zeros((H, W), np.float32)
    c_best = np.zeros((H, W), np.float32)
    pid = np.zeros((H, W), np.int32)
    rmax = []
    for i, p in enumerate(parts):
        x0, x1 = (p.x0 + ox) * k, (p.x1 + ox) * k
        y0, y1 = (p.y0 + oy) * k, (p.y1 + oy) * k
        ins = (X >= x0) & (X < x1) & (Y >= y0) & (Y < y1)
        if p.hip:
            dx = np.minimum(X - x0, x1 - X)
            dy = np.minimum(Y - y0, y1 - Y)
            r = np.minimum(dx, dy)
            c = np.where(dx < dy, Y, X)
            rmax.append(min(x1 - x0, y1 - y0) / 2)
        elif p.ridge_axis == "x":
            half = (y1 - y0) / 2
            r, c = half - np.abs(Y - (y0 + y1) / 2), X
            rmax.append(half)
        else:
            half = (x1 - x0) / 2
            r, c = half - np.abs(X - (x0 + x1) / 2), Y
            rmax.append(half)
        hh = np.where(ins, r * culture.slope, -1.0)
        better = hh > h
        h = np.where(better, hh, h)
        r_best = np.where(better, r, r_best)
        c_best = np.where(better, c, c_best)
        pid = np.where(better, i, pid)
    mask = h >= 0
    rmax = np.asarray(rmax, np.float32)

    # --- éclairage ----------------------------------------------------------
    hz = np.where(mask, h, 0.0)
    gy, gx = np.gradient(hz)
    nrm = np.sqrt(gx**2 + gy**2 + 1.0)
    lam = (-gx * L[0] - gy * L[1] + L[2]) / nrm
    shade = 1.0 + 0.36 * (lam / L[2] - 1.0)

    # --- tuiles -------------------------------------------------------------
    Pr = culture.tile_row * k
    Pc = culture.tile_col * k
    rowf = r_best / Pr
    row = np.floor(rowf)
    fr = rowf - row
    colf = (c_best + (row % 2) * Pc / 2) / Pc
    col = np.floor(colf)
    fc = colf - col
    t_color = hash2(row, col, 1 + pid)
    t_jit = hash2(row, col, 101 + pid)
    t_miss = hash2(row, col, 201 + pid)

    rgb = _lerp_palette(culture.colors, t_color)
    rgb *= (1 + (t_jit[..., None] - 0.5) * culture.jitter * 2)
    seams = culture.seam * np.clip(1 - fr / 0.22, 0, 1) \
        + culture.col_seam * np.clip(1 - fc / 0.10, 0, 1)
    rgb *= (1 - seams[..., None])

    # tuiles manquantes + mousse + salissure selon l'usure
    if wear > 0:
        miss = (t_miss < wear * 0.05)[..., None]
        rgb = np.where(miss, UNDERLAY, rgb)
        m = value_noise((H, W), 1.4 * k, rng, octaves=3)
        moss = np.clip((m - (1 - wear * 0.6)) * 7, 0, 1)[..., None] * 0.65
        rgb = rgb * (1 - moss) + MOSS * moss * (0.8 + 0.4 * shade[..., None])
    grime = value_noise((H, W), 2.5 * k, rng, octaves=2)
    rgb *= (0.90 + 0.2 * grime)[..., None]
    rgb *= (0.97 + 0.06 * rng.random((H, W)).astype(np.float32))[..., None]

    # rustines de tôle ondulée
    if culture.metal_patches and wear > 0.45 and rng.random() < 0.8:
        p0 = parts[0]
        pw, ph = rng.uniform(1.2, 2.2) * k, rng.uniform(1.0, 1.8) * k
        px = rng.uniform(p0.x0 + 0.8, max(p0.x0 + 0.9, p0.x1 - 2.4)) + ox
        py = rng.uniform(p0.y0 + 0.5, max(p0.y0 + 0.6, p0.y1 - 2.0)) + oy
        px, py = px * k, py * k
        patch = (X >= px) & (X < px + pw) & (Y >= py) & (Y < py + ph) & mask
        stripes = 0.88 + 0.12 * np.sin(2 * np.pi * X / (0.14 * k))
        edge = np.clip(np.minimum(np.minimum(X - px, px + pw - X),
                                  np.minimum(Y - py, py + ph - Y)) / (0.06 * k), 0, 1)
        metal = METAL * (stripes * (0.7 + 0.3 * edge))[..., None]
        rgb = np.where(patch[..., None], metal, rgb)

    rgb *= np.clip(shade, 0.6, 1.4)[..., None]

    # faîtage : bande plus claire
    ridge = mask & (r_best >= rmax[pid] - 0.16 * k) & (h > 0.6 * culture.slope * rmax[pid])
    rgb = np.where(ridge[..., None], rgb * 1.14 + 6, rgb)

    # --- cheminées ----------------------------------------------------------
    n_chim = int(rng.integers(0, 1 + max(0, round(culture.chimney_max * (0.4 + wealth)))))
    sdir = np.array([-L[0], -L[1]], np.float32)
    sdir /= np.linalg.norm(sdir)
    chim_shadow = np.zeros((H, W), bool)
    chim_body = np.zeros((H, W), bool)
    chim_hole = np.zeros((H, W), bool)
    p0 = parts[0]
    for _ in range(n_chim):
        s = rng.uniform(0.8, 1.1) * k
        if p0.ridge_axis == "x":
            cx = rng.uniform(p0.x0 + 0.2 * (p0.x1 - p0.x0), p0.x1 - 0.2 * (p0.x1 - p0.x0)) + ox
            cy = (p0.y0 + p0.y1) / 2 + rng.uniform(-0.5, 0.5) + oy
        else:
            cy = rng.uniform(p0.y0 + 0.2 * (p0.y1 - p0.y0), p0.y1 - 0.2 * (p0.y1 - p0.y0)) + oy
            cx = (p0.x0 + p0.x1) / 2 + rng.uniform(-0.5, 0.5) + ox
        cx, cy = cx * k, cy * k
        body = (np.abs(X - cx) < s / 2) & (np.abs(Y - cy) < s / 2)
        hole = (np.abs(X - cx) < s * 0.28) & (np.abs(Y - cy) < s * 0.28)
        for t in np.linspace(0, 1.1 * k, 6):
            chim_shadow |= _shift(body, sdir[0] * t, sdir[1] * t)
        chim_body |= body
        chim_hole |= hole
    chim_shadow &= mask & ~chim_body
    rgb = np.where(chim_shadow[..., None], rgb * 0.62, rgb)
    top = BRICK * (0.85 + 0.3 * rng.random())
    rgb = np.where(chim_body[..., None], top, rgb)
    rgb = np.where(chim_hole[..., None], np.array([40, 32, 32], np.float32), rgb)
    mask = mask | chim_body

    # --- contour de rive (gouttière) ---------------------------------------
    m_img = Image.fromarray((mask * 255).astype(np.uint8))
    e1 = np.asarray(m_img.filter(ImageFilter.MinFilter(2 * max(1, round(0.07 * k)) + 1))) > 127
    e2 = np.asarray(m_img.filter(ImageFilter.MinFilter(2 * max(2, round(0.28 * k)) + 1))) > 127
    rgb = np.where((mask & ~e2)[..., None], rgb * 0.88, rgb)
    rgb = np.where((mask & ~e1)[..., None], rgb * 0.55, rgb)

    # --- sortie toit --------------------------------------------------------
    alpha = (mask * 255).astype(np.uint8)
    roof = np.dstack([np.clip(rgb, 0, 255).astype(np.uint8), alpha])
    roof_img = Image.fromarray(roof, "RGBA").resize((Wf, Hf), Image.LANCZOS)

    # --- ombre portée au sol + occlusion de contact ------------------------
    shadow_len = 2.0 * k
    sweep = np.zeros((H, W), np.float32)
    for t in np.linspace(0, shadow_len, 10):
        sweep = np.maximum(sweep, _shift(mask.astype(np.float32), sdir[0] * t, sdir[1] * t))
    sw = Image.fromarray((sweep * 255).astype(np.uint8)).filter(
        ImageFilter.GaussianBlur(0.35 * k))
    ao = Image.fromarray((mask * 255).astype(np.uint8)).filter(
        ImageFilter.GaussianBlur(0.30 * k))
    a = np.maximum(np.asarray(sw, np.float32) * 0.42, np.asarray(ao, np.float32) * 0.30)
    a = np.where(mask, 0, a)  # sous le toit, rien à voir
    sh = np.dstack([np.full((H, W, 3), (18, 22, 10), np.uint8), a.astype(np.uint8)])
    shadow_img = Image.fromarray(sh, "RGBA").resize((Wf, Hf), Image.LANCZOS)

    meta = dict(seed=seed, culture=culture.name, wealth=wealth, wear=round(wear, 2),
                parts=len(parts), chimneys=n_chim, size=(Wf, Hf))
    return HouseSprite(roof_img, shadow_img, meta)
