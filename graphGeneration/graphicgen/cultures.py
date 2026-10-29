from dataclasses import dataclass


@dataclass(frozen=True)
class Culture:
    """Tout ce qui distingue visuellement une culture de toit : données, pas code."""
    name: str
    colors: tuple          # palette de tuiles (RGB), interpolée au hasard par tuile
    slope: float           # pente du toit (hauteur / distance horizontale)
    tile_row: float        # hauteur d'une rangée (m)
    tile_col: float        # largeur d'une tuile (m)
    seam: float            # force des joints entre rangées
    col_seam: float        # force des joints verticaux
    jitter: float          # variation de luminosité par tuile
    main_w: tuple          # longueur du corps principal (m)
    main_d: tuple          # profondeur du corps principal (m)
    wing_prob: float       # probabilité d'une aile perpendiculaire
    hip_prob: float        # probabilité d'un toit à quatre pans
    chimney_max: int
    metal_patches: bool    # rustines de tôle quand le bâtiment est usé


CULTURES = {
    "tuile_rouge": Culture(
        "tuile_rouge",
        ((150, 72, 52), (128, 60, 45), (165, 90, 62), (112, 56, 46)),
        slope=0.50, tile_row=0.42, tile_col=0.32, seam=0.30, col_seam=0.15,
        jitter=0.14, main_w=(7.5, 11.0), main_d=(5.0, 7.0),
        wing_prob=0.55, hip_prob=0.15, chimney_max=2, metal_patches=True),
    "ardoise": Culture(
        "ardoise",
        ((82, 88, 98), (70, 76, 86), (96, 100, 106), (60, 64, 72)),
        slope=0.62, tile_row=0.34, tile_col=0.28, seam=0.38, col_seam=0.22,
        jitter=0.12, main_w=(8.0, 12.0), main_d=(5.5, 7.5),
        wing_prob=0.45, hip_prob=0.35, chimney_max=3, metal_patches=True),
    "chaume": Culture(
        "chaume",
        ((156, 132, 82), (138, 114, 70), (170, 148, 100), (120, 98, 62)),
        slope=0.75, tile_row=0.10, tile_col=3.0, seam=0.03, col_seam=0.0,
        jitter=0.12, main_w=(6.0, 9.0), main_d=(4.5, 6.0),
        wing_prob=0.20, hip_prob=0.55, chimney_max=1, metal_patches=False),
}
