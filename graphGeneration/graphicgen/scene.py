import numpy as np
from PIL import Image

from .noise import value_noise

GRASS = np.array([118, 128, 56], np.float32)
GRASS_DRY = np.array([138, 138, 64], np.float32)
GRASS_DARK = np.array([78, 98, 40], np.float32)


def make_grass(w, h, rng, scale=1.0):
    """Fond d'herbe de prévisualisation (bruit multi-échelle)."""
    big = value_noise((h, w), 140 * scale, rng, octaves=3)[..., None]
    mid = value_noise((h, w), 30 * scale, rng, octaves=2)[..., None]
    fine = rng.random((h, w, 1)).astype(np.float32)
    img = GRASS * (1 - big * 0.5) + GRASS_DRY * big * 0.5 + GRASS_DARK * (mid - 0.5).clip(0) * 0.8
    img = img * (0.92 + 0.16 * mid) * (0.96 + 0.08 * fine)
    return Image.fromarray(np.clip(img, 0, 255).astype(np.uint8), "RGB").convert("RGBA")


def contact_sheet(sprites, cols, pad=14, seed=0, zoom=2):
    """Place les sprites en grille sur de l'herbe. Retourne une image RGBA."""
    cw = max(s.roof.width for s in sprites) + pad * 2
    ch = max(s.roof.height for s in sprites) + pad * 2
    rows = -(-len(sprites) // cols)
    sheet = make_grass(cols * cw, rows * ch, np.random.default_rng(seed))
    for i, s in enumerate(sprites):
        x = (i % cols) * cw + (cw - s.roof.width) // 2
        y = (i // cols) * ch + (ch - s.roof.height) // 2
        sheet.alpha_composite(s.shadow, (x, y))
        sheet.alpha_composite(s.roof, (x, y))
    if zoom > 1:
        sheet = sheet.resize((sheet.width * zoom, sheet.height * zoom), Image.NEAREST)
    return sheet
