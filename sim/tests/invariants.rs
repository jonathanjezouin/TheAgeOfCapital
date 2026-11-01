//! Invariants du POC-A (économie de subsistance) : déterminisme, conservation, cohérence du
//! village, distances, fertilité, et chacun des mécanismes expliqués dans docs/02.

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
        "{} {:?} {:?} {:?} {:?} {}",
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
            .collect::<Vec<_>>(),
        s.evenements.len()
    )
}

fn sans_extension(mut s: Simulation) -> Simulation {
    s.cat.extension.chantiers_max = 0;
    s
}

// ---------- Données et génération ----------

#[test]
fn donnees_par_defaut_valides() {
    let c = Catalogue::defaut();
    assert_eq!(c.ressources.len(), 8);
    // 6 terres exploitées, 1 atelier, 1 type chantier.
    assert_eq!(c.batiments.len(), 8);
    assert_eq!(c.batiments.iter().filter(|b| b.chantier).count(), 1);
    // Hameau ordinaire : 2 pops ; hameau textile : 3 pops dont 1 à l'atelier ; densité max 10.
    assert_eq!(
        c.habitats
            .iter()
            .map(|h| h.capacite_pops)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(c.habitat.densite_max_pops, 10);
    let atelier = c.habitats[1].atelier.expect("atelier du hameau textile");
    assert_eq!(c.batiments[atelier].pops, 1);
    // Terres exploitées : 19 pops pour une exploitation de chaque type.
    let postes: u32 = c
        .batiments
        .iter()
        .filter(|b| !b.chantier && b.cout_creation.is_some())
        .map(|b| b.pops)
        .sum();
    assert_eq!(postes, 19);
    // Économie de subsistance : ni filature ni atelier de couture.
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
    // Densité au-delà du plafond de 10 pops par tile.
    assert!(Catalogue::depuis_toml(&t.replace("capacite_pops = 2", "capacite_pops = 11")).is_err());
    assert!(Catalogue::depuis_toml(
        &t.replace("atelier = \"atelier_textile\"", "atelier = \"inconnu\"")
    )
    .is_err());
    assert!(Catalogue::depuis_toml(&t.replace("distance_max = 10", "distance_max = 0")).is_err());
}

#[test]
fn composition_equilibree() {
    let c = Catalogue::defaut();
    let nb = |n: &sim::village::Composition, nom: &str| n.batiments[c.bat_id(nom).unwrap()];
    // 10 hameaux : il faut 3 hameaux textiles (3 ateliers × 4 vêtements ≥ besoins de 23 pops).
    let n = sim::village::composition(&c, 10);
    assert_eq!(n.hameaux_textiles, 3);
    assert_eq!(nb(&n, "atelier_textile"), 3);
    assert_eq!(
        (
            nb(&n, "champ"),
            nb(&n, "paturage"),
            nb(&n, "verger"),
            nb(&n, "foret_exploitee")
        ),
        (1, 1, 1, 1)
    );
    assert_eq!((nb(&n, "champ_lin"), nb(&n, "elevage_laine")), (1, 1));
    // Un village deux fois plus grand a deux fois plus de champs et de hameaux textiles.
    let n = sim::village::composition(&c, 20);
    assert_eq!(n.hameaux_textiles, 6);
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
            .map(|t| format!("{:?}{}{}", t.contenu, t.variante, t.fertilite_ppm))
            .collect::<Vec<_>>()
    };
    assert_eq!(contenus(&a), contenus(&b));
    assert_ne!(contenus(&a), contenus(&c));
}

#[test]
fn villages_toujours_connexes_et_pourvus() {
    for graine in 0..100 {
        let s = generer(
            Catalogue::defaut(),
            &ParamsVillage {
                graine,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("graine {graine} : {e}"));
        let nq = s.monde.quartiers.len();
        let dmax = s.cat.infra.distance_max;
        for p in &s.pops {
            if let Some(b) = p.emploi {
                let ch = s
                    .reseau
                    .chemin(p.residence, nq + b)
                    .unwrap_or_else(|| panic!("graine {graine} : poste inaccessible"));
                assert!(ch.cout <= dmax);
            }
        }
        let occ = s.occupation();
        for b in 0..s.monde.batiments.len() {
            assert!(occ[b] <= s.postes_de(b));
        }
        for (i, q) in s.monde.quartiers.iter().enumerate() {
            assert_eq!(s.monde.tuiles[q.tuile].contenu, Contenu::Quartier(i));
            // Un hameau textile embarque son atelier sur la même tile.
            assert_eq!(q.atelier.is_some(), q.habitat_id == 1);
            if let Some(a) = q.atelier {
                assert_eq!(s.monde.batiments[a].tuile, q.tuile);
            }
        }
        let effectifs = s.effectifs();
        assert!(effectifs
            .iter()
            .zip(&s.monde.quartiers)
            .all(|(e, q)| *e <= q.capacite_pops));
    }
}

// ---------- Distances ----------

#[test]
fn la_distance_est_le_nombre_de_tiles_chemin_empruntees() {
    let s = village(1);
    let nq = s.monde.quartiers.len();
    let dmax = s.cat.infra.distance_max;
    for q in 0..nq {
        if let Some(a) = s.monde.quartiers[q].atelier {
            // Le travail sur place : même tile, distance nulle.
            assert_eq!(s.reseau.chemin(q, nq + a).unwrap().cout, 0);
        }
    }
    for de in 0..s.reseau.nb_lieux {
        for vers in 0..s.reseau.nb_lieux {
            let Some(ch) = s.reseau.chemin(de, vers) else {
                continue;
            };
            let tuile = |l: usize| {
                if l < nq {
                    s.monde.quartiers[l].tuile
                } else {
                    s.monde.batiments[l - nq].tuile
                }
            };
            assert!(ch.cout <= dmax, "jamais au-delà de la limite");
            if tuile(de) == tuile(vers) {
                assert_eq!(ch.cout, 0);
            } else {
                // Deux lieux distincts : au moins une tile empruntée ; autant de segments + 1.
                assert_eq!(ch.cout, ch.segments.len() as i64 + 1);
            }
        }
    }
}

#[test]
fn au_dela_de_la_limite_on_ne_va_ni_travailler_ni_s_approvisionner() {
    let court = sim::donnees::DONNEES_POC.replace("distance_max = 10", "distance_max = 3");
    let s = generer(
        Catalogue::depuis_toml(&court).unwrap(),
        &ParamsVillage {
            nb_hameaux: 4,
            ..Default::default()
        },
    );
    // Avec une limite de 3 tiles, le village de 4 hameaux ne peut pas se ravitailler partout :
    // soit la génération refuse (lieux hors de portée), soit aucun chemin ne dépasse 3.
    if let Ok(s) = s {
        for de in 0..s.reseau.nb_lieux {
            for vers in 0..s.reseau.nb_lieux {
                if let Some(ch) = s.reseau.chemin(de, vers) {
                    assert!(ch.cout <= 3);
                }
            }
        }
    }
}

// ---------- Moteur ----------

#[test]
fn run_deterministe() {
    let (mut a, mut b) = (village(3), village(3));
    a.avancer(365 * 80);
    b.avancer(365 * 80);
    assert_eq!(empreinte(&a), empreinte(&b));
}

#[test]
fn conservation_de_la_matiere_et_stocks_positifs() {
    for graine in [1, 2, 3] {
        let mut s = village(graine);
        for jour in 0..365 * 80 {
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
fn stable_sans_extension_sur_plusieurs_siecles() {
    let mut s = sans_extension(village(1));
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
fn la_production_suit_la_fertilite_de_la_tile() {
    let produit_grain = |fertilite: i64| {
        let mut s = sans_extension(village(1));
        let champ = s
            .monde
            .batiments
            .iter()
            .position(|b| s.cat.batiments[b.type_id].nom == "champ")
            .unwrap();
        let tuile = s.monde.batiments[champ].tuile;
        s.monde.tuiles[tuile].fertilite_ppm = fertilite;
        // Plein effectif et stock vide : toute la production du jour s'ajoute au stock.
        s.monde.batiments[champ]
            .stock_sortie
            .iter_mut()
            .for_each(|x| *x = 0);
        let g = s.cat.res_id("grain").unwrap();
        let avant = s.bilan.produit[g];
        s.jour_suivant();
        s.bilan.produit[g] - avant
    };
    let (plein, moitie) = (produit_grain(PPM), produit_grain(PPM / 2));
    assert!(plein > 0);
    // Les autres champs sont inchangés : seule la part du champ modifié varie.
    assert!(
        moitie < plein && moitie > plein / 3,
        "plein {plein} moitié {moitie}"
    );
}

#[test]
fn la_population_suit_ce_que_la_terre_nourrit() {
    // Oracle (sans extension) : un grain 4 fois moins abondant abaisse la population d'équilibre.
    let finale = |diviseur: i64| {
        let mut s = sans_extension(village(1));
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
    // Le plus proche fournisseur de vêtements d'un hameau textile est son propre atelier (distance 0).
    let mut vu = 0;
    for (q, qu) in s.monde.quartiers.iter().enumerate() {
        if let Some(a) = qu.atelier {
            assert_eq!(s.reseau.chemin(q, nq + a).unwrap().cout, 0);
            vu += 1;
        }
    }
    assert!(vu >= 1);
}

#[test]
fn le_travail_sur_place_demande_une_pop_du_hameau() {
    let mut s = village(1);
    let q = s
        .monde
        .quartiers
        .iter()
        .position(|q| q.atelier.is_some())
        .unwrap();
    let a = s.monde.quartiers[q].atelier.unwrap();
    for p in s.pops.iter_mut().filter(|p| p.emploi == Some(a)) {
        p.emploi = None;
    }
    s.jour_suivant();
    assert_eq!(s.monde.batiments[a].presents, 0);
    assert_eq!(s.monde.batiments[a].prod_ppm, 0);
}

#[test]
fn une_jauge_pleine_duplique_la_pop_dans_son_hameau() {
    let mut s = sans_extension(village(1));
    let seuil = s.seuil_croissance();
    let eff = s.effectifs();
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
    let mut s = sans_extension(village(1));
    s.avancer(365 * 150); // logement saturé
    let seuil = s.seuil_croissance();
    let capacite: u32 = s.monde.quartiers.iter().map(|q| q.capacite_pops).sum();
    assert_eq!(s.pops.len() as u32, capacite);
    assert!(s.pops.iter().all(|p| p.croissance <= seuil));
}

#[test]
fn les_sans_poste_prennent_les_postes_vacants() {
    let mut s = sans_extension(village(2));
    for p in s.pops.iter_mut().take(3) {
        p.emploi = None;
    }
    assert_eq!(s.sans_poste(), 3);
    s.avancer(s.cat.mobilite.periode_jours); // réaffectation mensuelle
    assert_eq!(s.sans_poste(), 0);
    let occ = s.occupation();
    for b in 0..s.monde.batiments.len() {
        assert!(occ[b] <= s.postes_de(b));
    }
}

#[test]
fn les_bras_vont_la_ou_le_besoin_se_fait_sentir() {
    let mut s = sans_extension(village(1));
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

// ---------- Extension ----------

/// Village dont tous les logements sont pleins, une pop attendant une place.
fn village_sature() -> Simulation {
    let mut s = generer(
        Catalogue::defaut(),
        &ParamsVillage {
            occupation_initiale_pct: 100,
            ..Default::default()
        },
    )
    .unwrap();
    let seuil = s.seuil_croissance();
    s.pops[0].croissance = seuil;
    s
}

#[test]
fn des_pops_qui_attendent_font_batir_un_hameau_qui_finit_par_ouvrir() {
    let mut s = village_sature();
    let (nq, cap_avant) = (
        s.monde.quartiers.len(),
        s.monde
            .quartiers
            .iter()
            .map(|q| q.capacite_pops)
            .sum::<u32>(),
    );
    s.etendre();
    assert_eq!(s.chantiers_en_cours(), 1);
    assert_eq!(s.monde.quartiers.len(), nq + 1);
    assert_eq!(
        s.monde.quartiers[nq].capacite_pops, 0,
        "pas de logement avant la fin du chantier"
    );
    // 300 pop-jours à 2 pops : au moins 150 jours, plus l'acheminement du bois ; la corvée comble l'équipe.
    s.avancer(365 * 2);
    assert_eq!(s.chantiers_en_cours(), 0, "chantier terminé");
    assert!(s.monde.quartiers[nq].capacite_pops > 0);
    assert!(
        s.monde
            .quartiers
            .iter()
            .map(|q| q.capacite_pops)
            .sum::<u32>()
            > cap_avant
    );
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouveau hameau construit")));
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn un_chantier_consomme_du_travail_et_du_bois() {
    let mut s = village_sature();
    s.cat.extension.chantiers_max = 1; // pas d'autre chantier pendant celui-ci
    s.etendre();
    assert_eq!(s.chantiers_en_cours(), 1);
    let bois = s.cat.res_id("bois").unwrap();
    let consomme_avant = s.bilan.consomme_prod[bois];
    let mut jours = 0;
    while s.chantiers_en_cours() > 0 {
        s.jour_suivant();
        jours += 1;
        assert!(jours < 365 * 3, "le chantier ne finit pas");
    }
    // 200 de bois pour un hameau, et au moins 300 pop-jours / 2 pops = 150 jours de travail.
    let consomme = s.bilan.consomme_prod[bois] - consomme_avant;
    assert!(
        (consomme - 200 * 1000).abs() <= 1000,
        "bois consommé : {consomme}"
    );
    assert!(jours >= 150, "terminé en {jours} jours seulement");
}

#[test]
fn le_village_s_etend_selon_le_besoin_sans_deborder_la_distance() {
    let mut s = village(1);
    let pops0 = s.pops.len();
    s.avancer(365 * 150);
    assert!(
        s.pops.len() > pops0,
        "croissance de {pops0} à {}",
        s.pops.len()
    );
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouveau hameau construit")));
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouvelle terre exploitée")));
    let nq = s.monde.quartiers.len();
    let dmax = s.cat.infra.distance_max;
    // Tout poste reste accessible dans la distance limite, aucun logement n'est dépassé.
    for p in &s.pops {
        if let Some(b) = p.emploi {
            let ch = s
                .reseau
                .chemin(p.residence, nq + b)
                .expect("poste accessible");
            assert!(ch.cout <= dmax);
        }
    }
    let eff = s.effectifs();
    assert!(eff
        .iter()
        .zip(&s.monde.quartiers)
        .all(|(e, q)| *e <= q.capacite_pops));
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn aucun_chantier_ne_reste_bloque() {
    let mut s = village(1);
    // Le travail restant de chaque chantier doit diminuer au moins tous les 3 ans.
    let mut vu: std::collections::BTreeMap<usize, (i64, u32)> = Default::default();
    for _ in 0..200 {
        s.avancer(365);
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            match &bat.chantier {
                Some(ch) => {
                    let e = vu.entry(b).or_insert((ch.travail_restant, 0));
                    if ch.travail_restant < e.0 {
                        *e = (ch.travail_restant, 0);
                    } else {
                        e.1 += 1;
                        assert!(
                            e.1 <= 3,
                            "chantier b{b} sans progrès depuis {} ans (an {})",
                            e.1,
                            s.jour / 365
                        );
                    }
                }
                None => {
                    vu.remove(&b);
                }
            }
        }
    }
}

// ---------- Transport ----------

#[test]
fn la_congestion_differe_les_livraisons_sans_perdre_de_matiere() {
    let texte = sim::donnees::DONNEES_POC
        .replace("capacite_nominale = 4000.0", "capacite_nominale = 25.0")
        .replace("capacite_max = 8000.0", "capacite_max = 50.0");
    let mut s = sans_extension(
        generer(
            Catalogue::depuis_toml(&texte).unwrap(),
            &ParamsVillage::default(),
        )
        .unwrap(),
    );
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
