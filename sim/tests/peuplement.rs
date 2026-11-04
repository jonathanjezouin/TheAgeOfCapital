//! Peuplement : fondations sans désertion, dispersion et noyaux, pas d'extinction (docs/07 §17).

use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};

fn village(graine: u64) -> Simulation {
    generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap()
}

#[test]
fn fonder_un_hameau_ne_vide_jamais_celui_d_ou_l_on_part() {
    // Chaque hameau n'a qu'une pop : une fondation n'a personne à prendre sans déserter un hameau.
    let mut s = village(1);
    s.cat.peuplement.seuil_fondation_ppm = 0; // fonder est toujours préféré
    s.cat.extension.chantiers_max = 4;
    let nq = s.monde.quartiers.len();
    let mut gardees = 0;
    let mut gardee_par_hameau = vec![false; nq];
    s.pops.retain(|p| {
        let q = p.residence;
        if gardee_par_hameau[q] {
            false
        } else {
            gardee_par_hameau[q] = true;
            gardees += 1;
            true
        }
    });
    let seuil = s.seuil_croissance();
    s.pops[0].croissance = seuil;
    s.etendre();
    let fondation = s.monde.batiments.iter().any(|b| matches!(b.chantier.as_ref().map(|c| c.cible), Some(sim::monde::Cible::Hameau(_))));
    assert!(!fondation, "aucune fondation possible sans vider un hameau ({gardees} hameaux d'une pop)");
    assert!(s.evenements.iter().all(|e| !e.texte.contains("bâtir un hameau")));
    // Toutes les pops sont encore dans leur hameau d'origine.
    let eff = s.effectifs();
    assert!(eff.iter().all(|&e| e <= 1));
    let _ = PPM;
}

#[test]
fn les_villages_se_peuplent_sans_s_eteindre_ni_deserter_en_chaine() {
    // Sur 6 graines et 150 ans : aucune extinction, et peu de hameaux détruits (avant le réglage : ~26 par village).
    let (mut detruits, mut fondes) = (0u32, 0usize);
    for graine in 1..=6u64 {
        let mut s = village(graine);
        s.avancer(365 * 150);
        let st = s.stats_peuplement();
        assert!(st.pops >= 3, "graine {graine} : {} pops après 150 ans", st.pops);
        detruits += st.detruits;
        fondes += s.evenements.iter().filter(|e| e.texte.starts_with("Nouveau hameau construit")).count();
        // Les hameaux restent petits mais jamais au-delà de la densité maximale.
        assert!(st.tailles.iter().all(|&t| t <= s.cat.habitat.densite_max_pops));
    }
    assert!(detruits <= 30, "{detruits} hameaux détruits sur 6 villages");
    assert!(fondes > 0, "aucune fondation en 6 villages et 150 ans");
}

#[test]
fn des_noyaux_de_plusieurs_pops_emergent_et_la_plupart_des_hameaux_restent_petits() {
    let mut tailles: Vec<u32> = Vec::new();
    for graine in 1..=7u64 {
        let mut s = village(graine);
        s.avancer(365 * 200);
        tailles.extend(s.stats_peuplement().tailles);
    }
    let petits = tailles.iter().filter(|&&t| t <= 2).count();
    let noyaux = tailles.iter().filter(|&&t| t >= 4).count();
    println!("{} hameaux : {petits} de 1 à 2 pops, {noyaux} de 4 pops ou plus (max {:?})", tailles.len(), tailles.iter().max());
    assert!(petits * 100 >= tailles.len() * 70, "au moins 70 % de hameaux de 1 à 2 pops");
    assert!(noyaux >= 2, "au moins quelques noyaux de 4 pops et plus");
}
