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

// ---------- Aptitude par terrain (docs/07 §3.3) ----------

use sim::PPM;

/// Grain produit en un jour par le premier champ du village de la graine 1, dont toute l'emprise est posée
/// sur `terrain` (fertilité locale fixée à 1,0), sans extension.
fn grain_du_jour(terrain: Terrain) -> i64 {
    let mut s = generer(Catalogue::defaut(), &ParamsVillage { graine: 1, ..Default::default() }).unwrap();
    s.cat.extension.chantiers_max = 0;
    let champ = s.monde.batiments.iter().position(|b| s.cat.batiments[b.type_id].nom == "champ").unwrap();
    for &t in &s.monde.batiments[champ].emprise.clone() {
        s.monde.tuiles[t].terrain = terrain;
        s.monde.tuiles[t].fertilite_ppm = PPM;
    }
    s.monde.batiments[champ].stock_sortie.iter_mut().for_each(|x| *x = 0);
    let g = s.cat.res_id("grain").unwrap();
    let avant = s.bilan.produit[g];
    s.jour_suivant();
    s.bilan.produit[g] - avant
}

#[test]
fn le_rendement_d_une_terre_suit_l_aptitude_du_terrain() {
    let (riche, plaine, rocaille) = (grain_du_jour(Terrain::PlaineRiche), grain_du_jour(Terrain::Plaine), grain_du_jour(Terrain::Rocailleuse));
    assert!(plaine > 0);
    assert!(riche > plaine && plaine > rocaille, "riche {riche} plaine {plaine} rocailleuse {rocaille}");
    // Les rapports suivent la table : riche 1,25 ; rocailleuse 0,6 (les autres champs sont inchangés).
    let c = Catalogue::defaut();
    let champ = c.bat_id("champ").unwrap();
    assert_eq!(c.batiments[champ].aptitude_ppm[Terrain::PlaineRiche as usize], 1_250_000);
    assert_eq!(c.batiments[champ].aptitude_ppm[Terrain::Rocailleuse as usize], 600_000);
}

#[test]
fn le_travail_requis_et_le_cout_de_construction_suivent_le_terrain() {
    let c = Catalogue::defaut();
    let champ = c.bat_id("champ").unwrap();
    let ty = &c.batiments[champ];
    // Un champ sur marais demande plus de travail à la tile que sur plaine ; la plaine est la référence.
    assert_eq!(ty.travail_terrain_ppm[Terrain::Plaine as usize], PPM);
    assert!(ty.travail_terrain_ppm[Terrain::Marecageuse as usize] > ty.travail_terrain_ppm[Terrain::Rocailleuse as usize]);
    assert!(ty.travail_terrain_ppm[Terrain::Rocailleuse as usize] > PPM);
    // Bâtir coûte plus cher sur terrain difficile.
    assert_eq!(c.cout_terrain_ppm[Terrain::Plaine as usize], PPM);
    assert!(c.cout_terrain_ppm[Terrain::Marecageuse as usize] > c.cout_terrain_ppm[Terrain::Rocailleuse as usize]);
    // Le besoin de travail d'une terre de 4 tiles croît avec la part de tiles rocailleuses.
    let mut s = generer(Catalogue::defaut(), &ParamsVillage { graine: 1, ..Default::default() }).unwrap();
    let emprise: Vec<usize> = s.monde.batiments.iter().find(|b| b.type_id == champ).unwrap().emprise.iter().copied().take(4).collect();
    for &t in &emprise {
        s.monde.tuiles[t].terrain = Terrain::Plaine;
    }
    let base = s.cat.besoin_mo_terre_milli(champ, &emprise, &s.monde.tuiles);
    for &t in &emprise[..2] {
        s.monde.tuiles[t].terrain = Terrain::Rocailleuse;
    }
    let moitie = s.cat.besoin_mo_terre_milli(champ, &emprise, &s.monde.tuiles);
    assert!(moitie > base, "base {base} moitié rocailleuse {moitie}");
}

#[test]
fn un_terrain_inconnu_dans_les_donnees_est_refuse() {
    let texte = sim::donnees::DONNEES_POC.replace("[terrains.foret]", "[terrains.volcan]");
    let e = Catalogue::depuis_toml(&texte).unwrap_err();
    assert!(e.contains("volcan"), "{e}");
    let texte = sim::donnees::DONNEES_POC.replacen("plaine_riche = 1.25", "prairie = 1.25", 1);
    assert!(Catalogue::depuis_toml(&texte).unwrap_err().contains("prairie"));
}

#[test]
fn les_nouveaux_champs_evitent_la_rocaille() {
    // Parmi les tiles défrichées en 100 ans pour des champs, la part de rocailleuse (rendement 0,6 pour le grain)
    // est inférieure à sa part parmi les tiles libres et constructibles à portée du village : le score d'extension
    // (docs/02 §10.2) tient compte de l'aptitude, sans interdire la rocaille (la compacité pèse aussi).
    let (mut roc_champs, mut n_champs, mut roc_libres, mut n_libres) = (0usize, 0usize, 0usize, 0usize);
    for graine in 1..=8u64 {
        let mut s = generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap();
        let champ = s.cat.bat_id("champ").unwrap();
        let libres0: Vec<usize> = (0..s.monde.tuiles.len())
            .filter(|&i| s.monde.tuiles[i].contenu == Contenu::Vide && s.monde.tuiles[i].aretes == 0 && s.monde.tuiles[i].constructible())
            .collect();
        let ham = s.monde.quartiers[0].tuile;
        let (hx, hy) = s.monde.xy(ham);
        let a_portee = |s: &sim::Simulation, i: usize| {
            let (x, y) = s.monde.xy(i);
            x.abs_diff(hx).max(y.abs_diff(hy)) <= 14
        };
        let libres0: Vec<usize> = libres0.into_iter().filter(|&i| a_portee(&s, i)).collect();
        for &i in &libres0 {
            roc_libres += usize::from(s.monde.tuiles[i].terrain == Terrain::Rocailleuse);
            n_libres += 1;
        }
        s.avancer(365 * 100);
        for b in s.monde.batiments.iter().filter(|b| b.type_id == champ) {
            for &t in b.emprise.iter().filter(|t| libres0.contains(t)) {
                roc_champs += usize::from(s.monde.tuiles[t].terrain == Terrain::Rocailleuse);
                n_champs += 1;
            }
        }
    }
    assert!(n_champs > 30, "trop peu de tiles défrichées ({n_champs})");
    let (rc, rl) = (roc_champs as f64 / n_champs as f64, roc_libres as f64 / n_libres as f64);
    println!("part de rocailleuse : champs défrichés {rc:.3}, tiles libres à portée {rl:.3}");
    assert!(rc < rl, "champs {rc:.3} >= libres {rl:.3}");
}
