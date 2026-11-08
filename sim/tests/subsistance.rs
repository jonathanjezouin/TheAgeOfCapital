//! Tests de la subsistance d'une pop (docs/08) : invariants ET trajectoires.
use sim::calendrier::JOURS_PAR_AN;
use sim::*;

fn annees(p: Params, n: u32) -> Simulation {
    let mut s = Simulation::avec(p);
    for _ in 0..n * JOURS_PAR_AN {
        s.tick();
    }
    s
}

fn moyenne(p: Params) -> Params {
    Params { meteo_fixe_pm: Some(1000), ..p }
}

#[test]
fn etat_initial_conforme_au_doc() {
    let s = Simulation::new();
    let champs = s.monde.tiles.iter().filter(|t| matches!(t, Tile::Champ(_))).count();
    let forets = s.monde.tiles.iter().filter(|t| matches!(t, Tile::Foret(_))).count();
    assert_eq!((champs, forets), (5, 3), "5 champs et 3 forêts par pop");
    assert!(matches!(s.monde.tiles[s.monde.ferme], Tile::Ferme));
}

#[test]
fn deterministe() {
    let a = annees(Params { graine: 7, ..Default::default() }, 30);
    let b = annees(Params { graine: 7, ..Default::default() }, 30);
    assert_eq!(a.etat_json(), b.etat_json());
    let c = annees(Params { graine: 8, ..Default::default() }, 30);
    assert_ne!(a.etat_json(), c.etat_json());
}

#[test]
fn un_an_fait_365_jours_dont_52_dimanches() {
    let s = annees(Default::default(), 10);
    for b in &s.historique {
        assert_eq!(b.jours.total(), 365, "année {}", b.annee);
        assert!((52..=53).contains(&b.jours.repos), "dimanches {} en {}", b.jours.repos, b.annee);
        assert!(b.jours.travailles() <= 313);
    }
}

#[test]
fn conservation_du_grain_et_du_bois() {
    let s = annees(Default::default(), 60);
    let (g0, b0, _) = s.params.stock_initial;
    let mut grain_champs = 0;
    let mut bois_foret = 0;
    for t in &s.monde.tiles {
        match t {
            Tile::Champ(c) => grain_champs += c.grain,
            Tile::Foret(f) => bois_foret += f.coupe,
            _ => {}
        }
    }
    let h = &s.historique;
    let net_grain: i64 = h.iter().chain([&s.bilan]).map(|b| b.grain_brut - b.grain_preleve - b.grain_mange).sum();
    assert_eq!(g0 + net_grain, s.stocks.grain + grain_champs, "grain");
    let net_bois: i64 = h.iter().chain([&s.bilan]).map(|b| b.bois_brut - b.bois_preleve - b.bois_brule).sum();
    assert_eq!(b0 + net_bois, s.stocks.bois + bois_foret, "bois");
}

#[test]
fn annee_moyenne_production_du_doc() {
    // Doc §3-§4 : 5 champs × 3 grains = 15 ; prélèvement de 15 % arrondi à l'unité (2,25 → 2) ; 12 consommés.
    let s = annees(moyenne(Default::default()), 10);
    for b in &s.historique {
        assert_eq!(b.grain_brut, 15);
        assert_eq!(b.grain_preleve, 2);
        assert_eq!(b.grain_mange, 12);
        assert_eq!(b.manque_grain, 0);
        assert_eq!(b.champs_recoltes, 5);
        // Bois : le petit bois repousse d'un coup, 9 unités au plus par an, 5 brûlées.
        assert!(b.bois_brut <= 9);
        assert_eq!(b.bois_brule, 5);
    }
}

#[test]
fn le_prelevement_a_lieu_a_date_fixe_dans_le_stock_de_la_ferme() {
    // 11 novembre = jour 314 de l'année (0-based) ; la part est un entier, prise sur le stock de la ferme.
    let mut s = Simulation::avec(moyenne(Default::default()));
    for _ in 0..315 {
        s.tick();
    }
    let ev: Vec<_> = s.journal.iter().filter(|e| e.texte.starts_with("Prélèvement")).collect();
    assert_eq!(ev.len(), 1);
    assert_eq!(ev[0].jour, 314);
    assert_eq!(s.bilan.grain_preleve, 2);
    assert!(s.bilan.bois_preleve >= 1);
    assert_eq!(s.grain_a_taxer, 0, "assiette remise à zéro");
}

#[test]
fn le_prelevement_est_partiel_si_le_stock_manque() {
    // Le seigneur ne prend pas plus que ce que la ferme a en stock : 10 grains dus (15 % de 70), 3 en stock.
    let mut s = Simulation::new();
    s.stocks.grain = 3;
    s.grain_a_taxer = 70;
    s.prelevement();
    assert_eq!(s.stocks.grain, 0);
    assert_eq!(s.bilan.grain_preleve, 3);
    assert!(s.journal.last().unwrap().texte.contains("prélèvement partiel"));
}

#[test]
fn le_prelevement_est_arrondi_a_l_unite_la_plus_proche() {
    for (brut, attendu) in [(15, 2), (13, 2), (12, 2), (10, 2), (9, 1), (3, 0), (4, 1)] {
        let mut s = Simulation::new();
        s.stocks.grain = 100;
        s.grain_a_taxer = brut;
        s.prelevement();
        assert_eq!(100 - s.stocks.grain, attendu, "15 % de {brut}");
    }
}

#[test]
fn le_temps_de_travail_tient_dans_l_annee() {
    // Doc §8 : 100 j de culture, 30 à 45 j de transport du grain (15 grains × 2,4 tiles ≈ 36 j).
    let s = annees(moyenne(Default::default()), 30);
    for b in &s.historique {
        let j = b.jours;
        assert_eq!(j.labour + j.semis + j.recolte, 100, "culture, année {}", b.annee);
        assert!((30..=45).contains(&j.transport_grain), "transport grain {} en {}", j.transport_grain, b.annee);
        assert!(j.travailles() <= 313, "plus de jours travaillés que d'ouvrables en {}", b.annee);
    }
    // Le vêtement (usure aléatoire, point ouvert n°6 du doc) fait varier le total : on juge la moyenne.
    let moy = s.historique.iter().map(|b| b.jours.travailles()).sum::<u32>() as f64 / s.historique.len() as f64;
    println!("jours travaillés par an en moyenne : {moy:.0}");
    assert!((190.0..=250.0).contains(&moy), "moyenne {moy}");
}

#[test]
fn la_moisson_n_est_jamais_ratee_faute_de_temps() {
    for graine in 1..=5 {
        let s = annees(Params { graine, ..Default::default() }, 100);
        for b in &s.historique {
            assert_eq!(b.champs_perdus, 0, "graine {graine}, année {}", b.annee);
            assert_eq!(b.champs_recoltes, 5, "graine {graine}, année {}", b.annee);
        }
    }
}

#[test]
fn trajectoire_100_ans_avec_meteo_aleatoire() {
    // Doc §8 : très tendu (+0,75/an) ; environ un tiers des années sont déficitaires au sens de la production,
    // mais le stock reporté amortit. On vérifie la forme, pas une valeur exacte.
    let mut annees_deficit = 0;
    let mut total = 0;
    for graine in 1..=20 {
        let s = annees(Params { graine, ..Default::default() }, 100);
        for b in &s.historique {
            total += 1;
            if b.manque_grain > 0 {
                annees_deficit += 1;
            }
        }
        assert!(s.stocks.grain >= 0 && s.stocks.bois >= 0 && s.stocks.vetement >= 0);
    }
    let part = annees_deficit as f64 / total as f64;
    println!("part d'années avec manque de grain : {:.1} %", part * 100.0);
    assert!(part < 0.5, "trop de manques : {part}");
}

#[test]
fn le_vetement_manque_rarement() {
    // Constat : avec l'usure à 20 %/mois tirée au hasard, un stock de 2 et 20 j par vêtement,
    // deux usures rapprochées peuvent vider le stock avant la fin de la couture (point ouvert n°6).
    let s = annees(moyenne(Default::default()), 50);
    let manques: i64 = s.historique.iter().map(|b| b.manque_vetement).sum();
    println!("vêtements manquants en 50 ans : {}", manques);
    assert!(manques < 20);
}
