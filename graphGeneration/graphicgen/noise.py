import numpy as np
from PIL import Image


def value_noise(shape, cell, rng, octaves=1):
    """Bruit lissé dans [0, 1]. `cell` = taille (px) de la plus grande structure."""
    h, w = shape
    out = np.zeros(shape, np.float32)
    amp, total = 1.0, 0.0
    for o in range(octaves):
        c = max(cell / 2**o, 1.0)
        gh, gw = int(h / c) + 3, int(w / c) + 3
        grid = rng.random((gh, gw)).astype(np.float32)
        big = Image.fromarray(grid).resize((int(gw * c), int(gh * c)), Image.BICUBIC)
        out += np.asarray(big)[:h, :w] * amp
        total += amp
        amp *= 0.5
    return np.clip(out / total, 0.0, 1.0)


def hash2(a, b, seed=0):
    """Hasard déterministe par case (a, b) entières -> [0, 1]."""
    x = (a.astype(np.int64) * 374761393 + b.astype(np.int64) * 668265263
         + seed * 2147483647) & 0xFFFFFFFF
    x = ((x ^ (x >> 13)) * 1274126177) & 0xFFFFFFFF
    x = x ^ (x >> 16)
    return (x & 0xFFFF).astype(np.float32) / 65535.0
