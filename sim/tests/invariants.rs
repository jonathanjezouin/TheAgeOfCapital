//! Invariants du POC-A (économie de subsistance) : déterminisme, conservation, cohérence du
//! village, distances, fertilité, et chacun des mécanismes expliqués dans docs/02.

use sim::monde::{Contenu, Monde, Tuile};
use sim::simulation::{Classe, Motif};
use sim::reseau::Reseau;
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
    // Un seul type de hameau (2 pops) ; densité max 10. Le textile est un travail domestique :
    // l'atelier du hameau n'a pas de poste, il demande 1 pop-jour par jour à plein régime.
    assert_eq!(
        c.habitats
            .iter()
            .map(|h| h.capacite_pops)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(c.habitat.densite_max_pops, 10);
    let atelier = c.habitats[0].atelier.expect("atelier domestique du hameau");
    assert!(c.batiments[atelier].domestique);
    assert_eq!(c.batiments[atelier].pops, 1);
    // Budget de jours : une pop fournit 12 500 jours-personne par an.
    assert_eq!(c.travail.jours_personne_par_pop_an, 12_500);
    // Terres exploitées sur N tiles : extrants par tile et par jour, travail par tile et par an.
    let champ = &c.batiments[c.bat_id("champ").unwrap()];
    assert!(champ.par_tile && champ.fertile);
    assert_eq!((champ.jours_par_tile_an, champ.emprise_max), (250, 36));
    assert_eq!(c.extension.tuiles_par_defrichement, 4);
    let terres = c.batiments.iter().filter(|b| b.par_tile).count();
    assert_eq!(terres, 6);
    // 20 pops : le travail des terres n'occupe qu'environ un tiers de leur temps (docs/04, référence 1750).
    let n = sim::village::composition(&c, 10);
    let jours: i64 = c
        .batiments
        .iter()
        .enumerate()
        .filter(|(_, b)| b.par_tile)
        .map(|(t, b)| n.batiments[t] as i64 * b.jours_par_tile_an)
        .sum();
    let part = jours as f64 / (20.0 * c.travail.jours_personne_par_pop_an as f64);
    assert!((0.25..0.5).contains(&part), "part du temps aux terres : {part}");
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
        &t.replace("atelier = \"atelier_domestique\"", "atelier = \"inconnu\"")
    )
    .is_err());
    assert!(Catalogue::depuis_toml(&t.replace("distance_max = 100", "distance_max = 0")).is_err());
}

#[test]
fn composition_equilibree() {
    let c = Catalogue::defaut();
    let nb = |n: &sim::village::Composition, nom: &str| n.batiments[c.bat_id(nom).unwrap()];
    // 10 hameaux (20 pops) : chacun fait ses vêtements ; le reste est en tiles, déduites des rations
    // (ration par pop × pops ÷ production par tile).
    let n = sim::village::composition(&c, 10);
    assert_eq!(nb(&n, "atelier_domestique"), 10);
    assert_eq!(nb(&n, "champ"), 250); // 5 grain × 20 pops ÷ 0,40
    assert_eq!(nb(&n, "paturage"), 48); // 1 viande × 20 ÷ 0,42
    assert_eq!(nb(&n, "verger"), 20); // 1 fruit × 20 ÷ 1,0
    assert_eq!(nb(&n, "foret_exploitee"), 114); // 2 bois × 20 ÷ 0,35
    // Vêtements : 0,5 × 20 = 10 par jour ; 1 pop-jour en fait 4 avec 8 lin et 4 laine.
    assert_eq!(nb(&n, "champ_lin"), 17); // 20 lin ÷ 1,2
    assert_eq!(nb(&n, "elevage_laine"), 67); // 10 laine ÷ 0,15
    // Un village deux fois plus grand a deux fois plus de tiles (à l'arrondi près).
    let n2 = sim::village::composition(&c, 20);
    assert_eq!(nb(&n2, "atelier_domestique"), 20);
    assert_eq!(nb(&n2, "champ"), 500);
}

#[test]
fn village_deterministe_et_varie_selon_la_graine() {
    let (a, b, c) = (village(5), village(5), village(6));
    let contenus = |s: &Simulation| {
        s.monde
            .tuiles
            .iter()
            .map(|t| format!("{:?}{}{}{}", t.contenu, t.aretes, t.variante, t.fertilite_ppm))
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
        // Chaque terre occupe N tiles contiguës, dont sa tile « nœud » desservie par le réseau.
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            assert!(bat.emprise.contains(&bat.tuile));
            if s.cat.batiments[bat.type_id].par_tile {
                assert!(bat.emprise.len() >= 2, "graine {graine} : terre d'une seule tile");
                assert!(bat.emprise.len() <= s.cat.batiments[bat.type_id].emprise_max as usize);
                assert_ne!(s.monde.masque(bat.tuile), 0, "graine {graine} : terre non desservie");
            }
            if s.cat.batiments[bat.type_id].par_tile {
                for &t in &bat.emprise {
                    assert_eq!(s.monde.tuiles[t].contenu, Contenu::Batiment(b));
                }
            }
        }
        assert!(s.pops.iter().all(|p| p.emploi.is_none()), "le travail se répartit chaque jour");
        for (i, q) in s.monde.quartiers.iter().enumerate() {
            assert_eq!(s.monde.tuiles[q.tuile].contenu, Contenu::Quartier(i));
            // Tout hameau bâti embarque son atelier domestique sur la même tile.
            assert_eq!(q.atelier.is_some(), q.capacite_pops > 0);
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
fn la_distance_est_le_cout_des_aretes_empruntees() {
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
                // Deux lieux distincts : la distance est la somme des coûts des arêtes empruntées.
                let somme: i64 = ch.segments.iter().map(|&sg| s.reseau.segments[sg].cout).sum();
                assert_eq!(ch.cout, somme);
                assert!(ch.cout >= 10);
            }
        }
    }
}

/// Monde vide de 3×3 tiles, sans lieu : sert à tester le graphe d'arêtes seul.
fn monde_3x3() -> Monde {
    Monde {
        largeur: 3,
        hauteur: 3,
        tuiles: (0..9)
            .map(|_| Tuile {
                contenu: Contenu::Vide,
                aretes: 0,
                variante: 0,
                fertilite_ppm: 1_000_000,
            })
            .collect(),
        quartiers: Vec::new(),
        batiments: Vec::new(),
    }
}

#[test]
fn une_diagonale_coute_14_et_raccourcit_le_trajet() {
    let cat = Catalogue::depuis_toml(sim::donnees::DONNEES_POC).unwrap();
    let mut m = monde_3x3();
    // Chemin en L : (0,0) → (1,0) → (2,0) → (2,1) → (2,2) : 4 côtés.
    for (a, b) in [(0, 1), (1, 2), (2, 5), (5, 8)] {
        m.relier(a, b);
    }
    let r = Reseau::construire(&m, &cat);
    assert_eq!(r.distances_depuis(0)[8], 40);
    // Ajout de la diagonale (0,0) → (1,1) → (2,2) : 2 × 14.
    m.relier(0, 4);
    m.relier(4, 8);
    let r = Reseau::construire(&m, &cat);
    assert_eq!(r.distances_depuis(0)[8], 28);
    assert_eq!(r.distances_depuis(0)[4], 14);
}

#[test]
fn deux_diagonales_ne_se_croisent_pas() {
    let mut m = monde_3x3();
    m.relier(0, 4); // (0,0) → (1,1)
    // L'autre diagonale du carré (0,0)-(1,0)-(0,1)-(1,1) : (1,0) ↔ (0,1).
    assert!(!m.peut_relier(1, 3));
    // Une diagonale d'un autre carré, ou un côté, reste possible.
    assert!(m.peut_relier(4, 8));
    assert!(m.peut_relier(1, 4));
    // Tiles non voisines : refusé.
    assert!(!m.peut_relier(0, 2));
}

#[test]
fn au_dela_de_la_limite_on_ne_va_ni_travailler_ni_s_approvisionner() {
    let court = sim::donnees::DONNEES_POC.replace("distance_max = 100", "distance_max = 30");
    let s = generer(
        Catalogue::depuis_toml(&court).unwrap(),
        &ParamsVillage {
            nb_hameaux: 4,
            ..Default::default()
        },
    );
    // Avec une limite de 30 (3 côtés), le village de 4 hameaux ne peut pas se ravitailler partout :
    // soit la génération refuse (lieux hors de portée), soit aucun chemin ne dépasse 30.
    if let Ok(s) = s {
        for de in 0..s.reseau.nb_lieux {
            for vers in 0..s.reseau.nb_lieux {
                if let Some(ch) = s.reseau.chemin(de, vers) {
                    assert!(ch.cout <= 30);
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

/// Les routes ne traversent ni terre ni chantier : une tile à arêtes est vide, habitée, ou la tile
/// nœud (entrée) de son bâtiment (docs/02 §10.1).
fn verifier_routes_hors_des_terres(s: &Simulation) {
    for (t, tuile) in s.monde.tuiles.iter().enumerate() {
        if tuile.aretes == 0 {
            continue;
        }
        if let Contenu::Batiment(b) = tuile.contenu {
            assert_eq!(
                s.monde.batiments[b].tuile, t,
                "une route traverse la tile {t} de l'emprise du bâtiment {b}"
            );
        }
    }
}

/// Compacité moyenne des terres de 8 tiles ou plus : périmètre ÷ périmètre du carré de même aire
/// (1,0 = carré parfait ; un serpent est bien au-dessus).
fn irregularite_moyenne(s: &Simulation) -> Option<f64> {
    let mut rapports = Vec::new();
    for bat in &s.monde.batiments {
        let n = bat.emprise.len();
        if bat.chantier.is_some() || n < 8 {
            continue;
        }
        let perimetre: usize = bat
            .emprise
            .iter()
            .map(|&t| {
                [0usize, 2, 4, 6]
                    .iter()
                    .filter(|&&d| s.monde.voisin(t, d).map_or(true, |v| !bat.emprise.contains(&v)))
                    .count()
            })
            .sum();
        rapports.push(perimetre as f64 / (4.0 * (n as f64).sqrt()));
    }
    if rapports.is_empty() {
        None
    } else {
        Some(rapports.iter().sum::<f64>() / rapports.len() as f64)
    }
}

#[test]
fn le_score_d_extension_rend_les_terres_plus_compactes() {
    let mesure = |compact: bool| {
        let (mut somme, mut nb) = (0.0, 0);
        for graine in 1..=6u64 {
            let mut s = village(graine);
            if !compact {
                // Ancien comportement : la fertilité seule décide.
                s.cat.extension.poids_voisins_ppm = 0;
                s.cat.extension.poids_distance_ppm = 0;
                s.cat.extension.penalite_terroir_ppm = 0;
            }
            s.avancer(365 * 150);
            if let Some(r) = irregularite_moyenne(&s) {
                somme += r;
                nb += 1;
            }
        }
        assert!(nb > 0, "aucune terre de 8 tiles");
        somme / nb as f64
    };
    let (compact, libre) = (mesure(true), mesure(false));
    println!("irrégularité moyenne : score {compact:.3}, fertilité seule {libre:.3}");
    assert!(compact < libre, "{compact} >= {libre}");
    assert!(compact < 1.35, "terres trop irrégulières : {compact}");
}

#[test]
fn les_routes_contournent_les_terres() {
    for graine in [1u64, 2, 3] {
        let mut s = village(graine);
        verifier_routes_hors_des_terres(&s);
        for _ in 0..60 {
            s.avancer(365);
            verifier_routes_hors_des_terres(&s);
        }
        // Toute terre est desservie par sa tile nœud.
        let nq = s.monde.quartiers.len();
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            if bat.chantier.is_none() && bat.emprise.len() > 1 {
                assert!(
                    !s.reseau.acces(nq + b).is_empty(),
                    "terre {b} (graine {graine}) sans accès au réseau"
                );
            }
        }
    }
}

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
        for &t in &s.monde.batiments[champ].emprise.clone() {
            s.monde.tuiles[t].fertilite_ppm = fertilite;
        }
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
        moitie < plein && moitie > plein / 4,
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
    // Le plus proche fournisseur de vêtements d'un hameau est son propre atelier (distance 0).
    let mut vu = 0;
    for (q, qu) in s.monde.quartiers.iter().enumerate() {
        if let Some(a) = qu.atelier {
            assert_eq!(s.reseau.chemin(q, nq + a).unwrap().cout, 0);
            vu += 1;
        }
    }
    assert!(vu >= 1);
}

/// Hameau dont on ne garde que `n` pops ; son atelier reçoit des intrants confortables et un stock vide.
fn hameau_avec_pops(s: &mut Simulation, n: usize) -> (usize, usize) {
    let q = s
        .monde
        .quartiers
        .iter()
        .position(|q| q.atelier.is_some())
        .unwrap();
    let a = s.monde.quartiers[q].atelier.unwrap();
    let mut gardees = 0;
    s.pops.retain(|p| {
        if p.residence != q {
            return true;
        }
        gardees += 1;
        gardees <= n
    });
    for r in ["lin", "laine"] {
        let r = s.cat.res_id(r).unwrap();
        s.monde.batiments[a].stock_entree[r] = 100_000;
    }
    for r in s.monde.batiments[a].stock_sortie.iter_mut() {
        *r = 0;
    }
    (q, a)
}

#[test]
fn le_textile_domestique_prend_le_temps_libre_du_hameau() {
    // Le temps libre est ce que les terres n'ont pas pris ; la production suit (travail requis : 1 pop-jour).
    let mut s = sans_extension(village(1));
    let (q, a) = hameau_avec_pops(&mut s, 2);
    s.jour_suivant();
    let libre = s.libre_milli[q];
    assert!(libre > 0, "temps libre du hameau : {libre}");
    assert_eq!(s.monde.batiments[a].prod_ppm, (libre * PPM / 1000).min(PPM));
    assert_eq!(s.monde.batiments[a].presents, 0, "aucun poste : personne n'y est affecté");
    // Aucune pop : pas de temps libre, pas de production.
    let mut s = sans_extension(village(1));
    let (q, a) = hameau_avec_pops(&mut s, 0);
    s.jour_suivant();
    assert_eq!(s.libre_milli[q], 0);
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
fn le_budget_de_jours_est_conserve() {
    // Chaque jour, le temps des pops se répartit entre équipes de chantier, terres et temps libre,
    // sans jamais dépasser ce qu'elles fournissent (1 pop = 1 pop-jour par jour).
    let mut s = village(2);
    for _ in 0..200 {
        s.avancer(30);
        let total = s.pops.len() as i64 * 1000;
        let terres: i64 = s
            .monde
            .batiments
            .iter()
            .filter(|b| s.cat.batiments[b.type_id].par_tile)
            .map(|b| b.mo_milli)
            .sum();
        let equipes: i64 = s.monde.batiments.iter().map(|b| b.presents as i64 * 1000).sum();
        let libre: i64 = s.libre_milli.iter().sum();
        assert!(terres + libre <= total - equipes + 3, "jour {} : {terres}+{libre} > {total}-{equipes}", s.jour);
    }
}

#[test]
fn sans_bras_une_terre_ne_produit_pas() {
    let mut s = sans_extension(village(1));
    s.pops.clear();
    let produit_avant = s.bilan.produit.clone();
    s.jour_suivant();
    for b in &s.monde.batiments {
        if s.cat.batiments[b.type_id].par_tile {
            assert_eq!(b.mo_milli, 0);
        }
    }
    assert_eq!(s.bilan.produit, produit_avant, "sans travail, rien ne se récolte");
}

#[test]
fn une_pop_travaille_en_un_seul_lieu_et_le_temps_est_conserve() {
    // Chaque pop libre fournit au plus un pop-jour, sur le seul lieu qu'elle a choisi ; le reste est du temps libre.
    let mut s = sans_extension(village(3));
    for _ in 0..120 {
        s.jour_suivant();
        for p in &s.pops {
            assert!(p.travail_milli >= 0 && p.travail_milli <= 1000);
            if p.emploi.is_none() {
                assert_eq!(p.travail_milli, 0);
                assert_eq!(p.motif, Motif::Repos);
            }
        }
        for (b, bat) in s.monde.batiments.iter().enumerate() {
            let somme: i64 = s.pops.iter().filter(|p| p.emploi == Some(b)).map(|p| p.travail_milli).sum();
            if s.cat.batiments[bat.type_id].par_tile {
                assert_eq!(somme, bat.mo_milli, "lieu {b} : le travail reçu est la somme de celui des pops venues");
            }
        }
        // Le temps libre est ce que les pops libres n'ont pas donné à leur lieu (moins, si la route était saturée).
        let attendu: i64 = s
            .pops
            .iter()
            .filter(|p| !matches!(p.motif, Motif::Chantier))
            .map(|p| 1000 - p.travail_milli)
            .sum();
        let libre: i64 = s.libre_milli.iter().sum();
        assert!(libre <= attendu, "libre {libre} > attendu {attendu}");
    }
}

#[test]
fn la_pop_qui_manque_de_nourriture_va_vers_une_terre_nourriciere() {
    let mut s = sans_extension(village(1));
    // Tout le monde a de quoi manger, sauf la pop 0 qui n'a plus rien de nutritif.
    let nr = s.cat.ressources.len();
    for p in s.pops.iter_mut() {
        for r in 0..nr {
            if s.cat.ressources[r].nutritive_milli > 0 {
                p.stock[r] = 200_000;
            }
        }
    }
    let nutritives: Vec<usize> = (0..nr).filter(|&r| s.cat.ressources[r].nutritive_milli > 0).collect();
    for &r in &nutritives {
        s.pops[0].stock[r] = 0;
    }
    s.jour_suivant();
    match s.pops[0].motif {
        Motif::Besoin { ressource, classe, manque_milli_jours } => {
            assert_eq!(classe, Classe::Vital);
            assert!(nutritives.contains(&ressource), "elle va vers de la nourriture");
            assert!(manque_milli_jours > 0);
        }
        autre => panic!("motif inattendu : {autre:?}"),
    }
    let lieu = s.pops[0].emploi.expect("elle travaille quelque part");
    assert!(s.pops[0].travail_milli > 0);
    let nq = s.monde.quartiers.len();
    assert!(s.reseau.chemin(s.pops[0].residence, nq + lieu).is_some(), "le lieu est à sa portée");
    // Les candidats expliquent le choix : le premier réclamant du travail est celui qu'elle a pris.
    let cands = s.expliquer_pop(s.pops[0].id).unwrap();
    assert!(cands.iter().any(|c| c.lieu == lieu));
}

#[test]
fn le_stock_est_personnel_et_la_couverture_aussi() {
    // Deux pops d'un même hameau, l'une à sec : elles ne sont pas nourries pareil (plus de stock de hameau).
    let mut s = sans_extension(village(1));
    let q = s.pops[0].residence;
    let idx: Vec<usize> = (0..s.pops.len()).filter(|&i| s.pops[i].residence == q).collect();
    assert!(idx.len() >= 2, "hameau de deux pops au moins");
    // Plus rien n'arrive : on vide les producteurs.
    for b in s.monde.batiments.iter_mut() {
        b.stock_sortie.iter_mut().for_each(|x| *x = 0);
    }
    let nr = s.cat.ressources.len();
    for r in 0..nr {
        s.pops[idx[0]].stock[r] = 0;
        s.pops[idx[1]].stock[r] = 200_000;
    }
    s.jour_suivant();
    assert!(s.pops[idx[0]].vital_jour_ppm < PPM, "la pop à sec a faim");
    assert_eq!(s.pops[idx[1]].vital_jour_ppm, PPM, "sa voisine mange à sa faim");
}

#[test]
fn le_stock_d_une_pop_disparue_revient_a_son_hameau() {
    let mut s = village(1);
    let seuil = s.seuil_croissance();
    let q = s.pops[0].residence;
    let avant = s.stock_hameau(q, s.cat.res_id("grain").unwrap());
    s.pops[0].croissance = -seuil - 10_000;
    s.jour_suivant();
    assert!(s.ecart_conservation().iter().all(|&e| e == 0), "la matière se conserve : {:?}", s.ecart_conservation());
    // Le hameau garde (à la consommation du jour près) le grain du défunt.
    let apres = s.stock_hameau(q, s.cat.res_id("grain").unwrap());
    assert!(apres > avant / 2, "stock du hameau : {avant} -> {apres}");
}

#[test]
fn une_terre_ne_produit_pas_plus_vite_que_son_rythme_nominal() {
    // Venir souvent ne fait pas pousser plus : la récolte est bornée par ce qui a poussé (docs/02 §10.4).
    let mut s = sans_extension(village(1));
    let g = s.cat.res_id("grain").unwrap();
    let nominal: i64 = s
        .monde
        .batiments
        .iter()
        .filter(|b| s.cat.batiments[b.type_id].extrants_finaux().iter().any(|&(r, _)| r == g))
        .map(|b| {
            let t = &s.cat.batiments[b.type_id];
            let fert = b.emprise.iter().map(|&i| s.monde.tuiles[i].fertilite_ppm).sum::<i64>() / b.emprise.len() as i64;
            t.extrants_finaux()[0].1 * b.emprise.len() as i64 * fert / PPM
        })
        .sum();
    let avant = s.bilan.produit[g];
    s.avancer(365);
    let produit = s.bilan.produit[g] - avant;
    // 365 jours de croissance, plus les jours de récolte déjà sur pied au départ (la moitié du stock de sortie).
    let borne = nominal * (365 + 10) + nominal;
    assert!(produit <= borne, "produit {produit} > borne {borne}");
    assert!(produit > nominal * 365 / 4, "la récolte n'est pas nulle : {produit}");
}

#[test]
fn une_terre_au_stock_plein_ne_demande_presque_pas_de_travail() {
    // Stock de sortie plein : la production s'arrêterait de toute façon, le temps va ailleurs.
    let mut s = sans_extension(village(1));
    let champ = s.cat.bat_id("champ").unwrap();
    let b = s
        .monde
        .batiments
        .iter()
        .position(|b| b.type_id == champ)
        .unwrap();
    let g = s.cat.res_id("grain").unwrap();
    s.monde.batiments[b].stock_sortie[g] = 10_000_000;
    s.jour_suivant();
    assert!(s.monde.batiments[b].mo_milli <= 1, "travail reçu : {}", s.monde.batiments[b].mo_milli);
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

fn capacite_totale(s: &Simulation) -> u32 {
    s.monde.quartiers.iter().map(|q| q.capacite_pops).sum()
}

#[test]
fn des_pops_qui_attendent_font_batir_une_maison() {
    let mut s = village_sature();
    let (nq, cap_avant) = (s.monde.quartiers.len(), capacite_totale(&s));
    s.etendre();
    assert_eq!(s.chantiers_en_cours(), 1);
    assert_eq!(
        s.monde.quartiers.len(),
        nq,
        "une maison de plus dans le hameau, pas un nouveau hameau"
    );
    assert_eq!(
        capacite_totale(&s),
        cap_avant,
        "pas de logement avant la fin du chantier"
    );
    // 150 pop-jours pour une pop : au moins 150 jours, plus l'acheminement du bois.
    s.avancer(365 * 2);
    assert!(capacite_totale(&s) > cap_avant, "la maison ouvre un logement");
    assert_eq!(s.monde.quartiers.len(), nq, "aucun hameau fondé tant qu'il reste de la place");
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouvelle maison")));
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn un_hameau_au_plafond_fait_fonder_un_autre_hameau() {
    // Plafond du hameau ramené à sa capacité de départ : il n'y a plus de place pour une maison de plus.
    let donnees = sim::donnees::DONNEES_POC.replace("capacite_max_pops = 4", "capacite_max_pops = 2");
    let mut s = generer(
        Catalogue::depuis_toml(&donnees).unwrap(),
        &ParamsVillage {
            occupation_initiale_pct: 100,
            ..Default::default()
        },
    )
    .unwrap();
    let seuil = s.seuil_croissance();
    s.pops[0].croissance = seuil;
    let (nq, cap_avant) = (s.monde.quartiers.len(), capacite_totale(&s));
    s.etendre();
    assert_eq!(s.chantiers_en_cours(), 1);
    assert_eq!(s.monde.quartiers.len(), nq + 1);
    assert_eq!(
        s.monde.quartiers[nq].capacite_pops, 0,
        "pas de logement avant la fin du chantier"
    );
    s.avancer(365 * 2);
    assert_eq!(
        s.monde.quartiers[nq].capacite_pops, 1,
        "un hameau neuf n'a que sa première maison"
    );
    assert!(capacite_totale(&s) > cap_avant);
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouveau hameau construit")));
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn un_hameau_ne_depasse_jamais_son_plafond() {
    let mut s = village(2);
    s.avancer(365 * 200);
    let max = s.cat.habitats[0].capacite_max_pops;
    assert!(s.monde.quartiers.iter().all(|q| q.capacite_pops <= max));
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouvelle maison")));
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
    // 100 de bois pour une maison, et 150 pop-jours pour 1 pop = 150 jours de travail.
    let consomme = s.bilan.consomme_prod[bois] - consomme_avant;
    assert!(
        (consomme - 100 * 1000).abs() <= 1000,
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
    // Les terres s'agrandissent tile par tile, et les maisons s'ajoutent une à une.
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Terre agrandie")));
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Nouvelle maison")));
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

#[test]
fn les_terres_restent_d_un_seul_tenant_en_s_agrandissant() {
    let mut s = village(1);
    s.avancer(365 * 120);
    assert!(s
        .evenements
        .iter()
        .any(|e| e.texte.starts_with("Terre agrandie")));
    for (b, bat) in s.monde.batiments.iter().enumerate() {
        if !s.cat.batiments[bat.type_id].par_tile || bat.chantier.is_some() {
            continue;
        }
        // Parcours en largeur depuis la tile nœud, par les côtés, à l'intérieur de l'emprise.
        let mut vu = vec![bat.tuile];
        let mut i = 0;
        while i < vu.len() {
            for d in [0usize, 2, 4, 6] {
                if let Some(v) = s.monde.voisin(vu[i], d) {
                    if bat.emprise.contains(&v) && !vu.contains(&v) {
                        vu.push(v);
                    }
                }
            }
            i += 1;
        }
        assert_eq!(vu.len(), bat.emprise.len(), "terre {b} en plusieurs morceaux");
        // Chaque tile n'appartient qu'à une terre.
        for &t in &bat.emprise {
            assert_eq!(s.monde.tuiles[t].contenu, Contenu::Batiment(b));
        }
        assert!(bat.emprise.len() <= s.cat.batiments[bat.type_id].emprise_max as usize);
    }
}
