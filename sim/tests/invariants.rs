//! Invariants du POC-A (économie de subsistance) : déterminisme, conservation,
//! stabilité, cohérence du village, et chacun des mécanismes expliqués dans docs/02.

use sim::monde::Contenu;
use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};

fn village(graine: u64) -> Simulation {
    generer(
        Catalogue::defaut(),
        &ParamsVillage {
            graine,
            ..Default::default()
        },
    )
    .expect("village")
}

fn empreinte(s: &Simulation) -> String {
    format!(
        "{} {:?} {:?} {:?} {:?}",
        s.jour,
        s.pops
            .iter()
            .map(|p| (p.id, p.residence, p.emploi, p.croissance))
            .collect::<Vec<_>>(),
        s.stock_total(),
        s.bilan,
        s.historique
            .iter()
            .map(|h| (h.pops, h.naissances, h.deces))
            .collect::<Vec<_>>()
    )
}

// ---------- Données et génération ----------

#[test]
fn donnees_par_defaut_valides() {
    let c = Catalogue::defaut();
    assert_eq!(c.ressources.len(), 8);
    assert_eq!(c.batiments.len(), 7);
    // Terres exploitées : 19 pops pour une exploitation de chaque type (docs/04).
    let postes: u32 = c
        .batiments
        .iter()
        .filter(|b| !b.domestique)
        .map(|b| b.pops)
        .sum();
    assert_eq!(postes, 19);
    // Économie de subsistance : ni filature ni atelier.
    assert!(c.bat_id("filature").is_none() && c.bat_id("atelier_couture").is_none());
}

#[test]
fn donnees_incoherentes_refusees() {
    let t = sim::donnees::DONNEES_POC;
    assert!(Catalogue::depuis_toml(&t.replace(
        "chauffage_ressource = \"bois\"",
        "chauffage_ressource = \"mithril\""
    ))
    .is_err());
    assert!(
        Catalogue::depuis_toml(&t.replace("capacite_pops = 2 ", "capacite_pops = 0 ")).is_err()
    );
    assert!(Catalogue::depuis_toml(&t.replace(
        "batiment_domestique = \"textile_domestique\"",
        "batiment_domestique = \"champ\""
    ))
    .is_err());
}

#[test]
fn composition_equilibree() {
    let c = Catalogue::defaut();
    let nb = |n: &[usize], nom: &str| n[c.bat_id(nom).unwrap()];
    let n = sim::village::composition(&c, 10);
    assert_eq!(
        (
            nb(&n, "champ"),
            nb(&n, "paturage"),
            nb(&n, "verger"),
            nb(&n, "foret_exploitee")
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(
        (
            nb(&n, "champ_lin"),
            nb(&n, "elevage_laine"),
            nb(&n, "textile_domestique")
        ),
        (1, 1, 10)
    );
    // Un village deux fois plus grand a deux fois plus de champs.
    let n = sim::village::composition(&c, 20);
    assert_eq!(
        (
            nb(&n, "champ"),
            nb(&n, "paturage"),
            nb(&n, "foret_exploitee")
        ),
        (2, 2, 2)
    );
}

#[test]
fn village_deterministe_et_varie_selon_la_graine() {
    let (a, b, c) = (village(5), village(5), village(6));
    let contenus = |s: &Simulation| {
        s.monde
            .tuiles
            .iter()
            .map(|t| format!("{:?}{}", t.contenu, t.variante))
            .collect::<Vec<_>>()
    };
    assert_eq!(contenus(&a), contenus(&b));
    assert_ne!(contenus(&a), contenus(&c));
}

#[test]
fn villages_toujours_connexes_et_pourvus() {
    for graine in 0..200 {
        let s = generer(
            Catalogue::defaut(),
            &ParamsVillage {
                graine,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("graine {graine} : {e}"));
        let nq = s.monde.quartiers.len();
        for p in &s.pops {
            if let Some(b) = p.emploi {
                assert!(
                    s.reseau.chemin(p.residence, nq + b).is_some(),
                    "graine {graine}"
                );
            }
        }
        let occ = s.occupation();
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            assert!(occ[b] <= s.cat.batiments[bat.type_id].pops);
        }
        for (i, q) in s.monde.quartiers.iter().enumerate() {
            assert_eq!(s.monde.tuiles[q.tuile].contenu, Contenu::Quartier(i));
            // Chaque hameau embarque son travail textile domestique sur la même tile.
            let d = q.domestique.expect("bâtiment domestique");
            assert_eq!(s.monde.batiments[d].tuile, q.tuile);
        }
        let effectifs = s.effectifs();
        assert!(effectifs
            .iter()
            .zip(&s.monde.quartiers)
            .all(|(e, q)| *e <= q.capacite_pops));
    }
}

// ---------- Moteur ----------

#[test]
fn run_deterministe() {
    let (mut a, mut b) = (village(3), village(3));
    a.avancer(365 * 30);
    b.avancer(365 * 30);
    assert_eq!(empreinte(&a), empreinte(&b));
}

#[test]
fn conservation_de_la_matiere_et_stocks_positifs() {
    for graine in [1, 2, 3] {
        let mut s = village(graine);
        for jour in 0..365 * 40 {
            s.jour_suivant();
            if jour < 365 || jour % 365 == 0 {
                assert!(
                    s.ecart_conservation().iter().all(|&e| e == 0),
                    "graine {graine} jour {jour}"
                );
                assert!(s.stock_total().iter().all(|&x| x >= 0));
                assert!(s
                    .monde
                    .quartiers
                    .iter()
                    .all(|q| q.stock.iter().all(|&x| x >= 0)));
                assert!(s.monde.batiments.iter().all(|b| b
                    .stock_entree
                    .iter()
                    .chain(&b.stock_sortie)
                    .all(|&x| x >= 0)));
            }
        }
    }
}

#[test]
fn stable_sur_plusieurs_siecles() {
    let mut s = village(1);
    for _ in 0..500 {
        s.avancer(365);
        assert!(!s.pops.is_empty(), "extinction à l'an {}", s.jour / 365);
        let eff = s.effectifs();
        assert!(eff
            .iter()
            .zip(&s.monde.quartiers)
            .all(|(e, q)| *e <= q.capacite_pops));
    }
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
    let h = s.historique.last().unwrap();
    assert!(
        h.vital_moy_ppm > PPM * 9 / 10,
        "besoins vitaux couverts à {}",
        h.vital_moy_ppm
    );
}

// ---------- Mécanismes ----------

#[test]
fn la_population_suit_ce_que_la_terre_nourrit() {
    // Oracle : sur le même village, un grain 4 fois moins abondant abaisse la population d'équilibre.
    let finale = |diviseur: i64| {
        let mut s = village(1);
        let champ = s.cat.bat_id("champ").unwrap();
        s.cat.batiments[champ].etapes[0].extrants[0].1 /= diviseur;
        s.avancer(365 * 300);
        s.pops.len()
    };
    let normal = finale(1);
    let famine = finale(4);
    assert!(famine + 3 <= normal, "famine {famine} vs normal {normal}");
}

#[test]
fn le_producteur_mange_d_abord() {
    let s = village(1);
    let nq = s.monde.quartiers.len();
    // Le plus proche fournisseur de vêtements d'un hameau est son propre travail domestique (coût 0).
    for (q, qu) in s.monde.quartiers.iter().enumerate() {
        let d = qu.domestique.unwrap();
        assert_eq!(s.reseau.chemin(q, nq + d).unwrap().cout, 0);
    }
}

#[test]
fn le_travail_domestique_depend_des_residents() {
    let mut s = village(1);
    let (q, d) = (0, s.monde.quartiers[0].domestique.unwrap());
    s.pops.retain(|p| p.residence != q);
    s.avancer(2);
    assert_eq!(s.monde.batiments[d].presents, 0);
    assert_eq!(s.monde.batiments[d].prod_ppm, 0);
}

#[test]
fn une_jauge_pleine_duplique_la_pop_dans_son_hameau() {
    let mut s = village(1);
    let seuil = s.seuil_croissance();
    let eff = s.effectifs();
    // Une pop d'un hameau qui a encore de la place.
    let i = s
        .pops
        .iter()
        .position(|p| eff[p.residence] < s.monde.quartiers[p.residence].capacite_pops)
        .unwrap();
    let hameau = s.pops[i].residence;
    s.pops[i].croissance = seuil - 1;
    let avant = s.pops.len();
    s.jour_suivant();
    assert_eq!(s.pops.len(), avant + 1);
    assert_eq!(s.effectifs()[hameau], eff[hameau] + 1);
    assert!(
        s.pops[i].croissance < seuil / 10,
        "la jauge du parent est repartie"
    );
}

#[test]
fn une_jauge_vide_fait_disparaitre_la_pop() {
    let mut s = village(1);
    let seuil = s.seuil_croissance();
    let id = s.pops[0].id;
    s.pops[0].croissance = -seuil - 10_000;
    let avant = s.pops.len();
    s.jour_suivant();
    assert_eq!(s.pops.len(), avant - 1);
    assert!(s.pops.iter().all(|p| p.id != id));
}

#[test]
fn sans_logement_la_jauge_attend() {
    let mut s = village(1);
    s.avancer(365 * 120); // logement saturé
    let seuil = s.seuil_croissance();
    assert_eq!(s.pops.len(), s.monde.quartiers.len() * 2);
    assert!(s.pops.iter().all(|p| p.croissance <= seuil));
}

#[test]
fn les_sans_poste_prennent_les_postes_vacants() {
    let mut s = village(2);
    for p in s.pops.iter_mut().take(3) {
        p.emploi = None;
    }
    assert_eq!(s.sans_poste(), 3);
    s.avancer(s.cat.mobilite.periode_jours); // réaffectation mensuelle
    assert_eq!(s.sans_poste(), 0);
    let occ = s.occupation();
    for (b, bat) in s.monde.batiments.iter().enumerate() {
        assert!(occ[b] <= s.cat.batiments[bat.type_id].pops);
    }
}

#[test]
fn les_bras_vont_la_ou_le_besoin_se_fait_sentir() {
    let mut s = village(1);
    let champ = s
        .monde
        .batiments
        .iter()
        .position(|b| s.cat.batiments[b.type_id].nom == "champ")
        .unwrap();
    let foret = s
        .monde
        .batiments
        .iter()
        .position(|b| s.cat.batiments[b.type_id].nom == "foret_exploitee")
        .unwrap();
    // Tout le monde au champ part à la forêt : plus de grain, bois en excédent.
    for p in s.pops.iter_mut().filter(|p| p.emploi == Some(champ)) {
        p.emploi = Some(foret);
    }
    assert_eq!(s.occupation()[champ], 0);
    s.avancer(365 * 3);
    assert!(
        s.occupation()[champ] >= 2,
        "le champ a retrouvé des bras : {}",
        s.occupation()[champ]
    );
}

#[test]
fn la_congestion_differe_les_livraisons_sans_perdre_de_matiere() {
    let texte = sim::donnees::DONNEES_POC
        .replace("capacite_nominale = 4000.0", "capacite_nominale = 25.0")
        .replace("capacite_max = 8000.0", "capacite_max = 50.0");
    let mut s = generer(
        Catalogue::depuis_toml(&texte).unwrap(),
        &ParamsVillage::default(),
    )
    .unwrap();
    let mut vu = false;
    for _ in 0..365 * 5 {
        s.jour_suivant();
        vu |= s.stats.nb_differes > 0 || s.stats.retards_pops > 0;
        assert!(s.ecart_conservation().iter().all(|&e| e == 0));
    }
    assert!(
        vu,
        "aucun effet de la congestion malgré des routes saturées"
    );
}

#[test]
fn par_defaut_la_congestion_est_en_sommeil() {
    let mut s = village(1);
    for _ in 0..365 * 20 {
        s.jour_suivant();
        assert_eq!((s.stats.nb_differes, s.stats.retards_pops), (0, 0));
        assert!(s.stats.charge_max_ppm < PPM / 2);
    }
}
