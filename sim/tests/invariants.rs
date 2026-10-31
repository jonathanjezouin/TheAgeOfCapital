//! Invariants du POC-A : déterminisme, conservation, stabilité, cohérence du village.

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
            .map(|p| (p.id, p.residence, p.emploi))
            .collect::<Vec<_>>(),
        s.stock_total(),
        s.bilan,
        s.historique
            .iter()
            .map(|h| (h.pops, h.naissances, h.deces))
            .collect::<Vec<_>>()
    )
}

#[test]
fn donnees_par_defaut_valides() {
    let c = Catalogue::defaut();
    assert_eq!(c.ressources.len(), 8);
    assert_eq!(c.batiments.len(), 8);
    // Effectifs de docs/04 : 29 pops pour la chaîne complète.
    assert_eq!(c.batiments.iter().map(|b| b.pops).sum::<u32>(), 29);
}

#[test]
fn donnees_incoherentes_refusees() {
    let mauvais = sim::donnees::DONNEES_POC.replace("poids = 0.4", "poids = 0.9");
    assert!(Catalogue::depuis_toml(&mauvais).is_err());
    let inconnu = sim::donnees::DONNEES_POC.replace(
        "ressource = \"bois\"\npar_pop",
        "ressource = \"mithril\"\npar_pop",
    );
    assert!(Catalogue::depuis_toml(&inconnu).is_err());
}

#[test]
fn composition_equilibree() {
    let c = Catalogue::defaut();
    let n = sim::village::composition(&c, 4);
    let nb = |nom: &str| n[c.bat_id(nom).unwrap()];
    assert_eq!(
        (
            nb("champ"),
            nb("paturage"),
            nb("verger"),
            nb("foret_exploitee")
        ),
        (2, 2, 1, 2)
    );
    // Une chaîne textile complète pour 4 quartiers.
    assert_eq!(
        (
            nb("champ_lin"),
            nb("elevage_laine"),
            nb("filature"),
            nb("atelier_couture")
        ),
        (1, 1, 1, 1)
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
        // Chaque pop atteint son poste.
        let nq = s.monde.quartiers.len();
        for p in &s.pops {
            if let Some(b) = p.emploi {
                assert!(
                    s.reseau.chemin(p.residence, nq + b).is_some(),
                    "graine {graine}"
                );
            }
        }
        // Aucun bâtiment au-delà de son effectif.
        let occ = s.occupation();
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            assert!(occ[b] <= s.cat.batiments[bat.type_id].pops);
        }
        // Les tiles référencent bien leurs entités.
        for (i, q) in s.monde.quartiers.iter().enumerate() {
            assert_eq!(s.monde.tuiles[q.tuile].contenu, Contenu::Quartier(i));
        }
    }
}

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
            // Vérification quotidienne la première année, puis annuelle.
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
    let cap_totale = s.monde.quartiers.len() as u32 * s.cat.demo.capacite_quartier_pops;
    for _ in 0..500 {
        s.avancer(365);
        assert!(!s.pops.is_empty(), "extinction à l'an {}", s.jour / 365);
        assert!(s.pops.len() as u32 <= cap_totale);
        for q in 0..s.monde.quartiers.len() {
            assert!(s.pops_du_quartier(q) as u32 <= s.cat.demo.capacite_quartier_pops);
        }
    }
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
    // Pas de dérive absurde : la satisfaction reste dans un intervalle raisonnable.
    let h = s.historique.last().unwrap();
    assert!(
        h.sat_moy_ppm > PPM / 2,
        "satisfaction finale {}",
        h.sat_moy_ppm
    );
}

#[test]
fn equilibre_malthusien_borne_par_la_production() {
    // Oracle : sur le même village, une production de grain réduite abaisse la population d'équilibre.
    let finale = |diviseur: i64| {
        let mut s = village(1);
        let champ = s.cat.bat_id("champ").unwrap();
        s.cat.batiments[champ].extrants[0].1 /= diviseur;
        s.cat.demo.acceleration_ppm = 4 * PPM; // observer l'équilibre plus vite
        s.avancer(365 * 300);
        // Moyenne sur 20 ans pour lisser les oscillations.
        let n = 20;
        s.historique
            .iter()
            .rev()
            .take(n)
            .map(|h| h.pops)
            .sum::<usize>() as f64
            / n as f64
    };
    let normal = finale(1);
    let famine = finale(2);
    assert!(famine < normal - 3.0, "famine {famine} vs normal {normal}");
}

#[test]
fn congestion_differe_les_livraisons_sans_perdre_de_matiere() {
    let texte = sim::donnees::DONNEES_POC
        .replace("capacite_nominale = 400.0", "capacite_nominale = 60.0")
        .replace("capacite_max = 800.0", "capacite_max = 120.0");
    let mut s = generer(
        Catalogue::depuis_toml(&texte).unwrap(),
        &ParamsVillage::default(),
    )
    .unwrap();
    let mut vu_differe = false;
    for _ in 0..365 * 5 {
        s.jour_suivant();
        vu_differe |= s.stats.nb_differes > 0;
        assert!(s.ecart_conservation().iter().all(|&e| e == 0));
    }
    assert!(
        vu_differe,
        "aucune livraison différée malgré des routes saturées"
    );
}

#[test]
fn les_naissances_prennent_les_postes_vacants() {
    let mut s = village(2);
    // Libère trois postes : les pops concernées deviennent chômeuses.
    for p in s.pops.iter_mut().take(3) {
        p.emploi = None;
    }
    assert_eq!(s.chomeurs(), 3);
    s.avancer(365); // fin d'année : les chômeuses reprennent les postes vacants
    assert_eq!(s.chomeurs(), 0);
    let occ = s.occupation();
    for (b, bat) in s.monde.batiments.iter().enumerate() {
        assert!(occ[b] <= s.cat.batiments[bat.type_id].pops);
    }
}
