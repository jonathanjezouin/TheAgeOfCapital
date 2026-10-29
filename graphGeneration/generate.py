"""Usage : python generate.py [--seed N] [--ppm 6] [--out out]

Produit une planche (lignes = cultures, colonnes = niveau de richesse) et les
sprites individuels (toit + ombre sur calques séparés).
"""
import argparse
from pathlib import Path

from graphicgen import CULTURES, contact_sheet, generate_house

WEALTH = (0.1, 0.4, 0.7, 1.0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--ppm", type=int, default=6, help="pixels par mètre")
    ap.add_argument("--variants", type=int, default=2, help="variantes par case")
    ap.add_argument("--out", default="out")
    args = ap.parse_args()

    out = Path(args.out)
    (out / "sprites").mkdir(parents=True, exist_ok=True)

    sprites = []
    for ci, cu in enumerate(CULTURES.values()):
        for v in range(args.variants):
            for wi, wealth in enumerate(WEALTH):
                seed = args.seed * 100000 + ci * 1000 + v * 100 + wi
                s = generate_house(seed, cu, wealth, ppm=args.ppm)
                sprites.append(s)
                name = f"{cu.name}_w{int(wealth * 100):03d}_{seed}"
                s.roof.save(out / "sprites" / f"{name}_roof.png")
                s.shadow.save(out / "sprites" / f"{name}_shadow.png")
    sheet = contact_sheet(sprites, cols=len(WEALTH), seed=args.seed)
    sheet.save(out / "sheet.png")
    print(f"{len(sprites)} maisons -> {out / 'sheet.png'}")


if __name__ == "__main__":
    main()
