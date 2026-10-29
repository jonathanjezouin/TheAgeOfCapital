# GraphicGeneration

Usine à assets procédurale (Python pur : numpy + Pillow) pour produire des bâtiments
vus de dessus, dans un style réaliste et simple.

    pip install -r requirements.txt
    python generate.py --seed 1 --ppm 6      # -> out/sheet.png + out/sprites/*.png

- `graphicgen/cultures.py` : les cultures sont des **données** (palette, pente, tuiles, proportions).
- `graphicgen/house.py` : carte de hauteurs -> normales -> éclairage (Lambert), tuiles, usure, cheminées, ombre.
- `graphicgen/scene.py` : herbe de prévisualisation et planche-contact.

Chaque maison sort en deux calques (toit, ombre) pour pouvoir changer la lumière en jeu.
La direction de la lumière est un simple paramètre (`light=`).
