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

from graphicgen import atlas, maisons, terrain, terres_pleines  # noqa: E402
from graphicgen.cultures import CULTURES  # noqa: E402
from graphicgen.house import ZENITH, generate_house  # noqa: E402

def rgba(img):
    return np.asarray(img.convert("RGBA"), np.float32)


class Raccords(unittest.TestCase):
    def test_sol_de_chemin_raccordable(self):
        # La texture de route est répétée par le rendu : bords gauche/droite et haut/bas se raccordent.
        t = rgba(terrain.sol_chemin())
        voisins = np.abs(t[:, 100] - t[:, 101]).mean()
        self.assertLess(np.abs(t[:, -1] - t[:, 0]).mean(), voisins * 1.6 + 2)
        self.assertLess(np.abs(t[-1, :] - t[0, :]).mean(), voisins * 1.6 + 2)
        self.assertTrue((t[..., 3] == 255).all(), "texture opaque")

    def test_terres_pleines_raccordables(self):
        # Une emprise de N tiles répète la texture : aucune couture visible entre deux tiles voisines.
        for f in (terres_pleines.champ_plein, terres_pleines.champ_lin_plein, terres_pleines.verger_plein,
                  terres_pleines.foret_pleine, terres_pleines.paturage_plein, terres_pleines.elevage_laine_plein):
            t = rgba(f(2))
            self.assertTrue((t[..., 3] == 255).all(), f.__name__ + " opaque")
            interne = max(np.abs(t[:, 100] - t[:, 101]).mean(), np.abs(t[100, :] - t[101, :]).mean())
            for couture in (np.abs(t[:, -1] - t[:, 0]).mean(), np.abs(t[-1, :] - t[0, :]).mean()):
                self.assertLess(couture, interne * 1.7 + 3, f"{f.__name__} : couture {couture:.1f} vs {interne:.1f}")

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

    def test_plus_de_tiles_de_chemin_ni_de_hameau(self):
        for g in ["infra_chemin_terre", "bat_hameau", "bat_hameau_textile", "bat_chantier_hameau"]:
            self.assertNotIn(g, self.m["groupes"], g)

    def test_groupes_attendus(self):
        for g in ["terrain_plaine", "bat_champ", "bat_paturage", "bat_verger", "bat_foret_exploitee",
                  "bat_champ_lin", "bat_elevage_laine", "bat_chantier_champ", "bat_maison_bois",
                  "bat_chantier_maison", "bat_champ_plein", "bat_champ_lin_plein", "bat_verger_plein",
                  "bat_foret_exploitee_plein", "bat_paturage_plein", "bat_elevage_laine_plein"]:
            self.assertIn(g, self.m["groupes"], g)
        self.assertGreaterEqual(len(self.m["groupes"]["terrain_plaine"]), 8)
        self.assertEqual(len(self.m["groupes"]["bat_maison_bois"]), 6)
        self.assertEqual(len(self.m["groupes"]["bat_chantier_maison"]), 3, "trois états de chantier")

    def test_deux_tailles_de_frames(self):
        for nom in self.m["groupes"]["bat_maison_bois"]:
            self.assertEqual((self.m["frames"][nom]["w"], self.m["frames"][nom]["h"]), (maisons.PETIT, maisons.PETIT))
        for nom in self.m["groupes"]["terrain_plaine"]:
            self.assertEqual(self.m["frames"][nom]["w"], self.m["tuile"])

    def test_texture_de_route_hors_atlas(self):
        self.assertTrue((self.dossier / self.m["textures"]["sol_chemin"]).exists())

    def test_frames_dans_l_atlas(self):
        w, h = self.m["taille"]
        for nom, r in self.m["frames"].items():
            self.assertLessEqual(r["x"] + r["w"], w, nom)
            self.assertLessEqual(r["y"] + r["h"], h, nom)

    def test_relief_des_maisons(self):
        for nom, r in self.m["relief"].items():
            self.assertTrue((self.dossier / r["albedo"]).exists(), nom)
            self.assertTrue((self.dossier / r["normal"]).exists(), nom)
        self.assertEqual(len(self.m["relief"]), 6)  # une par variante de maison


class Maisons(unittest.TestCase):
    def test_maison_au_zenith_sans_ombre_portee(self):
        base, albedo, normales = maisons.maison_bois(3)
        self.assertEqual(base.size, (maisons.PETIT, maisons.PETIT))
        # Assise de contact seulement : aucune ombre franche hors du toit et de la cour.
        self.assertFalse(np.isnan(rgba(base)).any())
        self.assertEqual(albedo.size, base.size)
        self.assertEqual(normales.size, base.size)

    def test_chantiers_de_maison_distincts(self):
        etats = [rgba(maisons.chantier_maison(e, 5)) for e in range(3)]
        self.assertGreater(np.abs(etats[0] - etats[1]).mean(), 0.5)
        self.assertGreater(np.abs(etats[1] - etats[2]).mean(), 0.5)


if __name__ == "__main__":
    unittest.main()
