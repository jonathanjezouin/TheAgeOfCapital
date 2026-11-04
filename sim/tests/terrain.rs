//! Invariants du générateur de terrain (docs/07 §3) : déterminisme, parts, cours d'eau, site du village.

use sim::monde::Contenu;
use sim::terrain::{generer_terrain, Eau, ParamsTerrain, Terrain};
use sim::{generer, Catalogue, ParamsVillage};

fn carte(graine: u64) -> sim::terrain::CarteTerrain {
    generer_terrain(&ParamsTerrain::defaut(graine, 200, 200))
}

#[test]
fn le_terrain_est_deterministe_et_varie_selon_la_graine() {
    let (a, b, c) = (carte(5), carte(5), carte(6));
    assert_eq!(a.terrain, b.terrain);
    assert_eq!(a.eau, b.eau);
    assert_ne!(a.terrain, c.terrain);
}

#[test]
fn les_parts_de_terrain_respectent_les_parametres() {
    for graine in 1..=5 {
        let c = carte(graine);
        let p = ParamsTerrain::defaut(graine, 200, 200);
        let n = 200 * 200;
        let compte = |t: Terrain| c.terrain.iter().filter(|&&x| x == t).count();
        // Parts demandées (en % de la carte), sur les tiles hors cours d'eau : écart d'au plus 1 point
        // (les cours d'eau retirent ~2 % de tiles aux terrains qu'on retire en dernier).
        assert_eq!(compte(Terrain::Marecageuse), n * p.pct_marais / 100);
        assert_eq!(compte(Terrain::Foret), n * p.pct_foret / 100);
        assert_eq!(compte(Terrain::Rocailleuse), n * p.pct_rocailleuse / 100);
        assert_eq!(compte(Terrain::PlaineRiche), n * p.pct_riche / 100);
        let eau = c.eau.iter().filter(|&&e| e != Eau::Aucune).count();
        assert!((200..=1500).contains(&eau), "eau {eau} tiles (graine {graine})");
    }
}

#[test]
fn chaque_cours_d_eau_va_au_bord_de_la_carte_et_s_agrandit_vers_l_aval() {
    for graine in 1..=5 {
        let c = carte(graine);
        let (w, h) = (c.largeur, c.hauteur);
        assert!(c.eau.contains(&Eau::Fleuve) || c.eau.contains(&Eau::Riviere), "graine {graine} : ni rivière ni fleuve");
        for i in 0..w * h {
            if c.eau[i] == Eau::Aucune {
                assert_eq!(c.aval[i], 255);
                continue;
            }
            // On suit l'aval : toujours de l'eau, par un côté, de gabarit croissant, jusqu'au bord.
            let (mut cur, mut gabarit, mut pas) = (i, c.eau[i] as u8, 0);
            loop {
                let (x, y) = (cur % w, cur / w);
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    break;
                }
                let d = c.aval[cur] as usize;
                assert!(d < 8 && d % 2 == 0, "graine {graine} : l'eau en ({x}, {y}) n'a pas d'aval par un côté");
                let (dx, dy) = [(1i32, 0i32), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)][d];
                cur = ((y as i32 + dy) as usize) * w + (x as i32 + dx) as usize;
                assert!(c.eau[cur] != Eau::Aucune, "graine {graine} : l'aval n'est pas de l'eau");
                assert!(c.eau[cur] as u8 >= gabarit, "le gabarit ne décroît pas vers l'aval");
                gabarit = c.eau[cur] as u8;
                pas += 1;
                assert!(pas < 5000, "boucle");
            }
        }
    }
}

#[test]
fn le_site_du_village_est_degage_et_a_de_l_eau_a_portee() {
    for graine in 1..=8 {
        let c = carte(graine);
        let (r, x, y) = sim::terrain::site_village(&c).expect("un site dégagé");
        let r = r as i64;
        for dy in -r..=r {
            for dx in -r..=r {
                let i = (y as i64 + dy) as usize * 200 + (x as i64 + dx) as usize;
                assert!(c.constructible(i), "graine {graine} : tile non constructible dans la fenêtre du village");
            }
        }
        let d = sim::terrain::distance_eau(200, 200, &c.eau)[y * 200 + x];
        assert!(d <= 17, "graine {graine} : eau à {d} tiles");
    }
}

#[test]
fn rien_n_est_bati_ni_trace_sur_l_eau_la_foret_ou_le_marais() {
    for graine in [1u64, 2, 3] {
        let mut sim = generer(
            Catalogue::defaut(),
            &ParamsVillage { graine, ..Default::default() },
        )
        .unwrap();
        for _ in 0..120 {
            sim.avancer(365);
        }
        let m = &sim.monde;
        let mut bati = 0;
        for t in &m.tuiles {
            if t.contenu != Contenu::Vide || t.aretes != 0 {
                bati += 1;
                assert!(t.constructible(), "graine {graine} : bâti ou chemin sur un terrain fermé ({:?}, {:?})", t.terrain, t.eau);
            }
        }
        assert!(bati > 60, "le village doit exister");
    }
}
