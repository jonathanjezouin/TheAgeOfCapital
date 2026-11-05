//! Saisons du grain : labour, semis, récolte ; tiles perdues faute de travail ; semences (docs/07 §19).

use sim::{generer, Catalogue, ParamsVillage, Simulation};

fn village(graine: u64) -> Simulation {
    generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap()
}

fn champs(s: &Simulation) -> Vec<usize> {
    (0..s.monde.batiments.len()).filter(|&b| s.cat.batiments[s.monde.batiments[b].type_id].nom == "champ" && s.monde.batiments[b].chantier.is_none()).collect()
}

#[test]
fn le_champ_ne_reclame_du_travail_que_dans_ses_fenetres() {
    let mut s = village(1);
    s.cat.extension.chantiers_max = 0;
    let (mut en_saison, mut hors_saison) = (0, 0);
    for jour in 0..365u32 {
        s.avancer(1);
        let demande: i64 = champs(&s).iter().map(|&b| s.monde.batiments[b].demande_milli).sum();
        let fenetres = [(45, 125), (125, 160), (190, 270)];
        if fenetres.iter().any(|&(d, f)| jour >= d && jour < f) {
            en_saison += (demande > 0) as u32;
        } else {
            hors_saison += (demande > 0) as u32;
        }
    }
    assert!(en_saison > 20, "le champ réclame du travail en saison ({en_saison} jours)");
    assert_eq!(hors_saison, 0, "aucun travail réclamé hors saison");
}

#[test]
fn la_recolte_tombe_dans_sa_fenetre_et_les_semences_sont_gardees() {
    let mut s = village(1);
    s.cat.extension.chantiers_max = 0;
    let g = s.cat.res_id("grain").unwrap();
    let mut produit = vec![0i64; 365];
    for j in 0..365usize {
        s.avancer(1);
        produit[j] = s.stats.produit[g];
    }
    let hors: i64 = produit.iter().enumerate().filter(|(j, _)| *j < 190 || *j >= 270).map(|(_, p)| p).sum();
    let dans: i64 = produit[190..270].iter().sum();
    assert_eq!(hors, 0, "pas de grain produit hors récolte");
    assert!(dans > 0, "la récolte produit du grain");
    // Les semences sont gardées dans la grange de la terre pour l'an suivant.
    let graines: i64 = champs(&s).iter().map(|&b| s.monde.batiments[b].stock_entree[g]).sum();
    assert!(graines > 0, "semences en réserve");
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn un_champ_non_travaille_a_temps_retombe_en_friche() {
    let mut s = village(1);
    s.cat.extension.chantiers_max = 0;
    let tiles = |s: &Simulation| champs(s).iter().map(|&b| s.monde.batiments[b].emprise.len()).sum::<usize>();
    let avant = tiles(&s);
    assert!(avant > 0);
    // Personne pour labourer : toutes les pops disparaissent avant la saison.
    s.pops.clear();
    s.avancer(130);
    assert!(tiles(&s) < avant / 2, "{} tiles sur {avant} après un labour manqué", tiles(&s));
    assert!(s.evenements.iter().any(|e| e.texte.contains("non labouré à temps")));
    // Les tiles libérées sont des tiles libres (friche), rien ne reste dans une emprise vide.
    assert!(s.monde.tuiles.iter().any(|t| t.contenu == sim::monde::Contenu::Vide && t.vacant_mois > 0));
}

#[test]
fn sans_semences_le_semis_est_limite_et_le_seigneur_avance_la_graine() {
    let mut s = village(1);
    s.cat.extension.chantiers_max = 0;
    let g = s.cat.res_id("grain").unwrap();
    s.avancer(365 + 125); // une récolte au moins, puis le labour suivant fait
    // Le grenier du seigneur couvre la graine manquante : l'avance se voit sur le stock du champ.
    for b in champs(&s) {
        s.monde.batiments[b].stock_entree[g] = 0;
    }
    let grenier_avant = s.monde.personnages[0].grenier[g];
    s.avancer(40);
    assert!(s.monde.personnages[0].grenier[g] < grenier_avant, "le seigneur avance la semence");
}
