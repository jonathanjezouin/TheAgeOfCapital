//! Le seigneur local, les pops serves, le champart et le grenier, la remise en état (docs/07 §14, §15).

use sim::monde::{Cible, Statut};
use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};

fn village(graine: u64) -> Simulation {
    generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap()
}

#[test]
fn un_seigneur_decide_et_toutes_les_pops_sont_serves() {
    let mut s = village(1);
    assert_eq!(s.monde.personnages.len(), 1);
    let p = &s.monde.personnages[0];
    assert_eq!(p.poste, "seigneur_local");
    assert_eq!(p.domaine, Some(0));
    assert!(s.pops.iter().all(|x| x.statut == Some(Statut::Serf)));
    // Les pops nées plus tard sont serves aussi.
    s.avancer(365 * 60);
    assert!(s.pops.iter().all(|x| x.statut == Some(Statut::Serf)));
    // Tout chantier a le seigneur pour maître d'ouvrage.
    assert!(s.monde.batiments.iter().filter_map(|b| b.chantier.as_ref()).all(|c| c.maitre == Some(0)));
}

#[test]
fn le_champart_alimente_le_grenier_et_la_matiere_est_conservee() {
    let mut s = village(1);
    let grain = s.cat.res_id("grain").unwrap();
    // Sans train de vie ni chantier : tout le champart s'accumule.
    s.cat.seigneurie.train_de_vie_milli = 0;
    s.cat.extension.chantiers_max = 0;
    let avant = s.monde.personnages[0].grenier[grain];
    s.avancer(365 * 5);
    let apres = s.monde.personnages[0].grenier[grain];
    assert!(apres > avant, "grenier de grain : {avant} -> {apres}");
    assert!(s.ecart_conservation().iter().all(|&e| e == 0), "conservation de la matière");
    // Le prélèvement est une part de ce que produisent les terres : on compare à la production cumulée du bilan.
    let produit = s.bilan.produit[grain];
    let part = (apres - avant) as f64 / produit as f64;
    assert!((0.05..0.2).contains(&part), "part prélevée {part:.3}");
}

#[test]
fn le_seigneur_vit_sur_son_grenier() {
    let mut s = village(1);
    let grain = s.cat.res_id("grain").unwrap();
    s.cat.seigneurie.champart_ppm = 0;
    s.cat.extension.chantiers_max = 0;
    let avant = s.monde.personnages[0].grenier[grain];
    s.avancer(100);
    assert!(s.monde.personnages[0].grenier[grain] < avant, "son train de vie entame le grenier");
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

/// Un village où une pop attend un logement et où la maison est financée par le grenier : renvoie (village, bois initial).
fn village_qui_construit() -> Simulation {
    let mut s = generer(Catalogue::defaut(), &ParamsVillage { occupation_initiale_pct: 100, ..Default::default() }).unwrap();
    s.cat.peuplement.seuil_fondation_ppm = 1_000 * PPM; // la famille reste : une maison
    s.cat.extension.periode_jours = 1_000_000;
    s.cat.seigneurie.champart_ppm = 0;
    s.cat.seigneurie.train_de_vie_milli = 0;
    s.cat.seigneurie.secours_milli_jours = 0;
    let seuil = s.seuil_croissance();
    s.pops[0].croissance = seuil;
    s
}

#[test]
fn le_materiau_d_un_chantier_vient_du_grenier_du_maitre_d_ouvrage() {
    let mut s = village_qui_construit();
    let bois = s.cat.res_id("bois").unwrap();
    let avant = s.monde.personnages[0].grenier[bois];
    s.etendre();
    let b = (0..s.monde.batiments.len())
        .find(|&b| matches!(s.monde.batiments[b].chantier.as_ref().map(|c| c.cible), Some(Cible::Maison(_))))
        .expect("une maison est lancée");
    assert_eq!(s.monde.batiments[b].chantier.as_ref().unwrap().maitre, Some(0));
    let mut jours = 0;
    while s.chantiers_en_cours() > 0 {
        s.jour_suivant();
        jours += 1;
        assert!(jours < 365 * 3, "le chantier ne finit pas");
    }
    // Le grenier a payé la maison (100 de bois) : rien de plus, aucun producteur n'a livré.
    let paye = avant - s.monde.personnages[0].grenier[bois];
    assert!((paye - 100_000).abs() <= 1_000, "bois payé par le grenier : {paye}");
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn le_seigneur_ne_lance_que_ce_qu_il_peut_payer() {
    let mut s = village_qui_construit();
    let bois = s.cat.res_id("bois").unwrap();
    s.monde.personnages[0].grenier[bois] = 0; // grenier de bois vide
    s.etendre();
    assert_eq!(s.chantiers_en_cours(), 0, "aucun chantier sans bois dans le grenier");
}

#[test]
fn un_hameau_vide_est_remis_en_etat_plutot_que_refonde_et_l_usure_renchérit_la_remise() {
    // Deux mesures de coût : juste abandonné, puis très usé.
    let cout = |mois_abandon: u32| -> u32 {
        let mut s = generer(Catalogue::defaut(), &ParamsVillage { occupation_initiale_pct: 100, ..Default::default() }).unwrap();
        s.cat.peuplement.seuil_fondation_ppm = 1_000 * PPM;
        s.cat.seigneurie.champart_ppm = 0;
        // Les hameaux sont reliés (marcher hors route ne coûte pas plus cher) : la remise est à portée de main-d'œuvre.
        s.cat.infra.cout_hors_route_ppm = PPM;
        s.reconstruire_reseau();
        // Le hameau 0 est abandonné depuis `mois_abandon` mois ; ses pops sont parties ailleurs.
        for p in &mut s.pops {
            if p.residence == 0 {
                p.residence = 1;
            }
        }
        s.abandonner_hameau(0);
        s.monde.quartiers[0].abandon_mois = Some(mois_abandon);
        // Une pop de l'autre hameau attend un logement, et n'y a plus de place.
        let capacite = s.monde.quartiers[1].capacite_pops;
        for p in s.pops.iter_mut().filter(|p| p.residence == 1) {
            p.croissance = 0;
        }
        s.monde.quartiers[1].capacite_pops = s.pops.iter().filter(|p| p.residence == 1).count() as u32;
        let seuil = s.seuil_croissance();
        let i = s.pops.iter().position(|p| p.residence == 1).unwrap();
        s.pops[i].croissance = seuil;
        s.etendre();
        let _ = capacite;
        let ev = s.evenements.iter().find(|e| e.texte.contains("remettre en état")).map(|e| e.texte.clone());
        let ev = ev.unwrap_or_else(|| panic!("aucune remise en état ordonnée ({mois_abandon} mois) : {:?}", s.evenements.iter().map(|e| &e.texte).collect::<Vec<_>>()));
        // « … (N % de sa construction) »
        let debut = ev.find('(').and_then(|a| ev[a..].find('(').map(|_| a)).unwrap();
        let reste = &ev[debut..];
        let pct = reste.trim_start_matches(|c: char| !c.is_ascii_digit()).split(|c: char| !c.is_ascii_digit()).next().unwrap();
        // Le premier groupe entre parenthèses est les coordonnées ; on veut celui qui suit.
        let apres = &ev[ev.rfind('(').unwrap()..];
        let n: String = apres.chars().skip(1).take_while(|c| c.is_ascii_digit()).collect();
        let _ = pct;
        n.parse().unwrap()
    };
    let (jeune, vieux) = (cout(6), cout(12 * 30));
    assert!(jeune < vieux, "la remise coûte {jeune} % tôt, {vieux} % tard");
    assert!(jeune >= 10 && vieux <= 60, "entre remise_min et remise_max : {jeune} % et {vieux} %");
}

#[test]
fn une_remise_en_etat_rend_le_logement() {
    let mut s = village_qui_construit();
    s.cat.seigneurie.champart_ppm = 0;
    s.cat.infra.cout_hors_route_ppm = PPM;
    s.reconstruire_reseau();
    for p in &mut s.pops {
        if p.residence == 0 {
            p.residence = 1;
        }
    }
    s.abandonner_hameau(0);
    assert_eq!(s.monde.quartiers[0].capacite_pops, 0);
    let memoire = s.monde.quartiers[0].capacite_memoire;
    assert!(memoire > 0);
    // Le hameau 1 est plein et une de ses pops attend : le seigneur remet le hameau 0 en état.
    let n1 = s.pops.iter().filter(|p| p.residence == 1).count() as u32;
    s.monde.quartiers[1].capacite_pops = n1;
    let seuil = s.seuil_croissance();
    let i = s.pops.iter().position(|p| p.residence == 1).unwrap();
    s.pops[i].croissance = seuil;
    s.etendre();
    let b = (0..s.monde.batiments.len()).find(|&b| matches!(s.monde.batiments[b].chantier.as_ref().map(|c| c.cible), Some(Cible::RemiseHameau(0))));
    assert!(b.is_some(), "remise en état ordonnée");
    let mut jours = 0;
    while s.chantiers_en_cours() > 0 {
        s.jour_suivant();
        jours += 1;
        assert!(jours < 365 * 4, "la remise ne finit pas");
    }
    assert!(s.monde.quartiers[0].abandon_mois.is_none(), "plus abandonné");
    assert_eq!(s.monde.quartiers[0].capacite_pops, memoire, "le logement est rendu");
}
