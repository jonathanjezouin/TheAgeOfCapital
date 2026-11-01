# GraphicGeneration

Usine à assets procédurale (Python pur : numpy + Pillow) : vue de dessus, style réaliste et simple,
**soleil au zénith** (aucune ombre portée ; la lumière en rotation viendra d'un shader).

    pip install -r requirements.txt
    python generate_assets.py --apercu      # -> ../app/public/assets/ (atlas, manifeste, relief)
    python -m unittest discover -s tests    # raccords des chemins, zénith, manifeste

| Module | Rôle |
|---|---|
| `graphicgen/terrain.py` | herbe raccordable ; 15 chemins de terre (masque N=1, E=2, S=4, O=8) qui se raccordent d'une tile à l'autre |
| `graphicgen/terres.py` | champ, champ de lin, verger, forêt exploitée, pâturage, élevage de laine |
| `graphicgen/hameau.py` | hameau et hameau textile : maisons (générateur de toits), potagers, puits, atelier, séchoirs ; sortie cuite + albedo + normales |
| `graphicgen/chantiers.py` | chantier de hameau, chantier de champ |
| `graphicgen/atlas.py` | assemblage de l'atlas 256 px, manifeste JSON, aperçu en contexte |
| `graphicgen/house.py` | toits par carte de hauteurs et éclairage ; sort aussi l'albedo et les normales (`ZENITH`) |
| `graphicgen/dessin.py` | outils communs : bruit périodique, toile sur-échantillonnée, assise de contact |

Les cultures de toits restent des **données** (`cultures.py`). `generate.py` (ancienne planche de maisons seules) est conservé.

---

## Générateur de maisons seules

Usine à assets procédurale (Python pur : numpy + Pillow) pour produire des bâtiments
vus de dessus, dans un style réaliste et simple.

    pip install -r requirements.txt
    python generate.py --seed 1 --ppm 6      # -> out/sheet.png + out/sprites/*.png

- `graphicgen/cultures.py` : les cultures sont des **données** (palette, pente, tuiles, proportions).
- `graphicgen/house.py` : carte de hauteurs -> normales -> éclairage (Lambert), tuiles, usure, cheminées, ombre.
- `graphicgen/scene.py` : herbe de prévisualisation et planche-contact.

Chaque maison sort en deux calques (toit, ombre) pour pouvoir changer la lumière en jeu.
La direction de la lumière est un simple paramètre (`light=`).
