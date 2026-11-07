"""Usage : python generate_assets.py [--seed N] [--out ../app/public/assets] [--apercu]

Produit l'atlas de tiles du POC (sol, chemins, hameaux, terres exploitées, chantiers) :
`atlas.png`, `atlas.json`, `relief/` (albedo sans éclairage et normales des hameaux, pour une
lumière en rotation), et, avec --apercu, `apercu.png` (un village assemblé pour juger en contexte).
"""
import argparse
from pathlib import Path

from graphicgen import atlas


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default="../app/public/assets")
    ap.add_argument("--apercu", action="store_true")
    args = ap.parse_args()
    out = Path(args.out)
    atlas.construire(out, args.seed)
    if args.apercu:
        atlas.apercu(out, args.seed)
        print(f"aperçu -> {out / 'apercu.png'}")


if __name__ == "__main__":
    main()
