# GraphicGeneration

Usine à assets procédurale (Python pur : numpy + Pillow) : vue de dessus, style réaliste et simple,
**soleil au zénith** (aucune ombre portée ; la lumière en rotation viendra d'un shader).

    pip install -r requirements.txt
    python generate_assets.py --apercu      # -> ../app/public/assets/ (atlas, manifeste, relief)
    python -m unittest discover -s tests    # raccords, zénith, manifeste, maisons

| Module | Rôle |
|---|---|
| `graphicgen/terrain.py` | herbe raccordable ; `sol_chemin` : terre battue raccordable (texture du trait des routes) |
| `graphicgen/terres.py` | champ, champ de lin, verger, forêt exploitée, pâturage, élevage de laine (tiles de 256 px) |
| `graphicgen/maisons.py` | **maison en bois** (sprite de 128 px, une par pop, toit de bardeaux, cour et tas de bois) et **chantier de maison** (3 états) ; sortie cuite + albedo + normales |
| `graphicgen/chantiers.py` | chantier de défrichement d'une terre |
| `graphicgen/atlas.py` | atlas à deux tailles (tiles de 256 px, sprites de 128 px), manifeste JSON, texture `sol_chemin.png`, aperçu en contexte |
| `graphicgen/house.py` | toits par carte de hauteurs et éclairage ; sort aussi l'albedo et les normales (`ZENITH`) |
| `graphicgen/dessin.py` | outils communs : bruit périodique, toile sur-échantillonnée, assise de contact |

Le réseau routier n'est plus un jeu de tiles : le rendu le trace en courbes (docs/02 §9, docs/03 §6) et le
remplit avec `sol_chemin.png`. Les maisons sont dessinées « face au Sud » et pivotées par le rendu.

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
