"""Tests du générateur d'assets : raccords, soleil au zénith, manifeste.

    cd graphGeneration && python -m unittest discover -s tests -v
"""
import json
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from graphicgen import atlas, terrain  # noqa: E402
from graphicgen.cultures import CULTURES  # noqa: E402
from graphicgen.house import ZENITH, generate_house  # noqa: E402

N, E, S, O = terrain.N, terrain.E, terrain.S, terrain.O


def rgba(img):
    return np.asarray(img.convert("RGBA"), np.float32)


class Raccords(unittest.TestCase):
    def test_chemins_horizontaux_se_raccordent(self):
        a, b = rgba(terrain.chemin(E)), rgba(terrain.chemin(O))
        # Bord droit de la tile à demi-direction Est, bord gauche de la tile voisine à demi-direction Ouest.
        ecart = np.abs(a[:, -1] - b[:, 0]).mean()
        self.assertLess(ecart, 14, f"couture visible entre deux chemins horizontaux : {ecart:.1f}")
        # Et le chemin touche bien le bord : au moins 30 px de terre opaque sur la colonne.
        self.assertGreater((a[:, -1, 3] > 200).sum(), 30)

    def test_chemins_verticaux_se_raccordent(self):
        a, b = rgba(terrain.chemin(S)), rgba(terrain.chemin(N))
        ecart = np.abs(a[-1, :] - b[0, :]).mean()
        self.assertLess(ecart, 14, f"couture visible entre deux chemins verticaux : {ecart:.1f}")

    def test_un_chemin_n_atteint_que_ses_bords(self):
        t = rgba(terrain.chemin(E))  # du centre vers l'Est : touche le bord Est, aucun autre
        self.assertGreater((t[:, -1, 3] > 200).sum(), 30)
        for bord, nom in ((t[:, 0, 3], "Ouest"), (t[0, :, 3], "Nord"), (t[-1, :, 3], "Sud")):
            self.assertEqual((bord > 10).sum(), 0, f"le chemin déborde sur le bord {nom}")

    def test_herbe_raccordable(self):
        g = rgba(terrain.herbe(3))
        couture = np.abs(g[:, -1] - g[:, 0]).mean()
        voisins = np.abs(g[:, 100] - g[:, 101]).mean()
        self.assertLess(couture, voisins * 1.6 + 2, f"couture {couture:.1f} vs voisins {voisins:.1f}")

    def test_variantes_d_herbe_de_meme_couleur_moyenne(self):
        moyennes = [rgba(terrain.herbe(s))[..., :3].reshape(-1, 3).mean(axis=0) for s in range(1, 6)]
        for m in moyennes:
            self.assertLess(np.abs(m - moyennes[0]).max(), 2.0)


class Zenith(unittest.TestCase):
    def setUp(self):
        self.s = generate_house(5, CULTURES["chaume"], 0.6, ppm=6, light=ZENITH, relief=1.2)

    def test_pas_d_ombre_portee(self):
        # Il ne reste que l'assise de contact : faible et sans prolongement directionnel.
        self.assertLess(rgba(self.s.shadow)[..., 3].max(), 0.35 * 255 + 2)

    def test_pans_symetriques(self):
        # Soleil au zénith : la normale moyenne est verticale (pas de côté privilégié).
        n = rgba(self.s.normal)
        m = n[..., 3] > 250
        self.assertAlmostEqual(float(n[..., 0][m].mean()), 127.5, delta=6)
        self.assertAlmostEqual(float(n[..., 1][m].mean()), 127.5, delta=6)

    def test_albedo_et_normales_alignes(self):
        self.assertEqual(self.s.albedo.size, self.s.roof.size)
        self.assertEqual(self.s.normal.size, self.s.roof.size)
        self.assertFalse(np.isnan(rgba(self.s.roof)).any())


class Manifeste(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.dossier = Path(cls.tmp.name)
        atlas.construire(cls.dossier, 1, verbeux=False)
        cls.m = json.loads((cls.dossier / "atlas.json").read_text(encoding="utf-8"))

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_quinze_chemins_indexes_par_masque(self):
        self.assertEqual(sorted(int(k) for k in self.m["groupes"]["infra_chemin_terre"]), list(range(1, 16)))

    def test_groupes_attendus(self):
        for g in ["terrain_plaine", "bat_hameau", "bat_hameau_textile", "bat_champ", "bat_paturage", "bat_verger",
                  "bat_foret_exploitee", "bat_champ_lin", "bat_elevage_laine", "bat_chantier_hameau", "bat_chantier_champ"]:
            self.assertIn(g, self.m["groupes"], g)
        self.assertGreaterEqual(len(self.m["groupes"]["terrain_plaine"]), 8)

    def test_frames_dans_l_atlas(self):
        w, h = self.m["taille"]
        for nom, r in self.m["frames"].items():
            self.assertLessEqual(r["x"] + r["w"], w, nom)
            self.assertLessEqual(r["y"] + r["h"], h, nom)

    def test_relief_des_hameaux(self):
        for nom, r in self.m["relief"].items():
            self.assertTrue((self.dossier / r["albedo"]).exists(), nom)
            self.assertTrue((self.dossier / r["normal"]).exists(), nom)
        self.assertEqual(len(self.m["relief"]), 5)  # 3 hameaux + 2 hameaux textiles


if __name__ == "__main__":
    unittest.main()
