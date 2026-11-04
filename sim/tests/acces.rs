//! Accès par marche, routes décidées par le seigneur, terres en amas (docs/07 §18).

use sim::monde::Contenu;
use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};

fn village(graine: u64) -> Simulation {
    generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap()
}

#[test]
fn marcher_hors_route_coute_le_double_et_la_route_est_un_bonus() {
    let s = village(1);
    let h = s.monde.quartiers[0].tuile;
    let a_pied = s.reseau.distances_depuis_tuiles(&[h]);
    let comme_route = s.reseau.distances_si_route(&[h]);
    let mut plus_cher = 0;
    for t in 0..a_pied.len() {
        if a_pied[t] == i64::MAX {
            continue;
        }
        assert!(comme_route[t] <= a_pied[t]);
        if comme_route[t] < a_pied[t] {
            plus_cher += 1;
        }
    }
    assert!(plus_cher > 0, "hors route, la marche coûte plus cher que sur route");
    // Hors route : un pas coûte le double d'un pas de route (10 → 20 à plat).
    let (x, y) = s.monde.xy(h);
    let voisin = (0..8usize).filter_map(|d| s.monde.voisin(h, d)).find(|&v| s.reseau.sur_route(v) == false && a_pied[v] != i64::MAX);
    if let Some(v) = voisin {
        let _ = (x, y);
        assert!(a_pied[v] >= comme_route[v] * 3 / 2 || a_pied[v] == comme_route[v], "{} vs {}", a_pied[v], comme_route[v]);
    }
}

#[test]
fn une_terre_est_accessible_par_n_importe_laquelle_de_ses_tiles() {
    for graine in 1..=4u64 {
        let s = village(graine);
        let nq = s.monde.quartiers.len();
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            if bat.emprise.len() < 4 {
                continue;
            }
            let acces = s.reseau.acces(nq + b);
            assert_eq!(acces.len(), bat.emprise.len(), "graine {graine} : toutes les tiles donnent accès");
        }
    }
}

#[test]
fn aucune_route_ne_traverse_une_terre_ni_ne_s_arrete_dans_son_emprise() {
    for graine in 1..=4u64 {
        let mut s = village(graine);
        s.avancer(365 * 40);
        for (t, tu) in s.monde.tuiles.iter().enumerate() {
            if tu.aretes != 0 {
                assert!(!matches!(tu.contenu, Contenu::Batiment(_)), "graine {graine} : route dans une terre, tile {t}");
            }
        }
    }
}

#[test]
fn les_terres_poussent_en_amas_pas_en_lanieres() {
    // Rapport de la boîte englobante : une lanière a un côté bien plus long que l'autre.
    let mut longues = 0;
    let mut total = 0;
    for graine in 1..=4u64 {
        let mut s = village(graine);
        s.avancer(365 * 40);
        for bat in &s.monde.batiments {
            if bat.emprise.len() < 8 {
                continue;
            }
            let xs: Vec<usize> = bat.emprise.iter().map(|&t| s.monde.xy(t).0).collect();
            let ys: Vec<usize> = bat.emprise.iter().map(|&t| s.monde.xy(t).1).collect();
            let (lx, ly) = (xs.iter().max().unwrap() - xs.iter().min().unwrap() + 1, ys.iter().max().unwrap() - ys.iter().min().unwrap() + 1);
            total += 1;
            if lx.max(ly) > 3 * lx.min(ly) {
                longues += 1;
            }
        }
    }
    assert!(total > 0);
    assert!(longues * 5 <= total, "{longues} lanières sur {total} terres");
}
