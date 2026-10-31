//! CLI headless : simule N années et écrit des séries temporelles en CSV.
//!
//!   sim-cli --annees 100 --graine 7 --csv sortie.csv

use sim::monde::Limitant;
use sim::{generer, Catalogue, ParamsVillage, PPM, U};
use std::fmt::Write as _;

fn aide() -> ! {
    eprintln!(
        "sim-cli [--annees N] [--graine N] [--quartiers N] [--occupation PCT]\n\
         \x20       [--donnees fichier.toml] [--accel X] [--csv fichier.csv] [--pas JOURS]\n\
         \x20       [--json-final fichier.json]"
    );
    std::process::exit(2)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut annees, mut pas) = (50u64, 30u64);
    let mut pv = ParamsVillage::default();
    let (mut donnees, mut csv, mut json, mut accel): (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<f64>,
    ) = (None, None, None, None);
    while let Some(a) = args.next() {
        let mut val = || args.next().unwrap_or_else(|| aide());
        match a.as_str() {
            "--annees" => annees = val().parse().unwrap_or_else(|_| aide()),
            "--graine" => pv.graine = val().parse().unwrap_or_else(|_| aide()),
            "--quartiers" => pv.nb_quartiers = val().parse().unwrap_or_else(|_| aide()),
            "--occupation" => pv.occupation_initiale_pct = val().parse().unwrap_or_else(|_| aide()),
            "--pas" => pas = val().parse().unwrap_or_else(|_| aide()),
            "--accel" => accel = Some(val().parse().unwrap_or_else(|_| aide())),
            "--donnees" => donnees = Some(val()),
            "--csv" => csv = Some(val()),
            "--json-final" => json = Some(val()),
            _ => aide(),
        }
    }

    let mut cat = match &donnees {
        Some(f) => {
            Catalogue::depuis_toml(&std::fs::read_to_string(f).expect("lecture des données"))
                .unwrap_or_else(|e| panic!("{f} : {e}"))
        }
        None => Catalogue::defaut(),
    };
    if let Some(x) = accel {
        cat.demo.acceleration_ppm = (x * PPM as f64).round() as i64;
    }
    let mut sim = generer(cat, &pv).unwrap_or_else(|e| panic!("génération du village : {e}"));

    let mut sortie = String::new();
    let noms: Vec<String> = sim.cat.ressources.iter().map(|r| r.nom.clone()).collect();
    write!(
        sortie,
        "jour,annee,pops,chomeurs,sat_moy,sat_nourriture_moy,differes,retards,charge_max_pct"
    )
    .unwrap();
    for n in &noms {
        write!(sortie, ",stock_{n},prod_{n},conso_{n}").unwrap();
    }
    sortie.push('\n');

    let total = annees * sim.cat.sim.jours_par_an as u64;
    for _ in 0..total {
        sim.jour_suivant();
        if sim.jour % pas == 0 {
            let nq = sim.monde.quartiers.len() as i64;
            let sat: i64 = sim
                .monde
                .quartiers
                .iter()
                .map(|q| q.sat_jour_ppm)
                .sum::<i64>()
                / nq;
            let sat_n: i64 = sim
                .monde
                .quartiers
                .iter()
                .map(|q| q.sat_nourriture_jour_ppm)
                .sum::<i64>()
                / nq;
            write!(
                sortie,
                "{},{},{},{},{:.4},{:.4},{},{},{:.1}",
                sim.jour,
                sim.jour / sim.cat.sim.jours_par_an as u64,
                sim.pops.len(),
                sim.chomeurs(),
                sat as f64 / PPM as f64,
                sat_n as f64 / PPM as f64,
                sim.stats.nb_differes,
                sim.stats.retards_pops,
                sim.stats.charge_max_ppm as f64 / PPM as f64 * 100.0
            )
            .unwrap();
            let stock = sim.stock_total();
            for r in 0..noms.len() {
                write!(
                    sortie,
                    ",{:.1},{:.1},{:.1}",
                    stock[r] as f64 / U as f64,
                    sim.stats.produit[r] as f64 / U as f64,
                    sim.stats.consomme_pops[r] as f64 / U as f64
                )
                .unwrap();
            }
            sortie.push('\n');
        }
    }
    if let Some(f) = &csv {
        std::fs::write(f, &sortie).expect("écriture du CSV");
    }

    // Résumé lisible.
    println!(
        "Village graine {} : {} quartiers, {} bâtiments, {} segments",
        pv.graine,
        sim.monde.quartiers.len(),
        sim.monde.batiments.len(),
        sim.reseau.segments.len()
    );
    println!(
        "Après {} ans : {} pops, {} chômeuses",
        annees,
        sim.pops.len(),
        sim.chomeurs()
    );
    for (q, qu) in sim.monde.quartiers.iter().enumerate() {
        println!(
            "  quartier {q} : {} pops, sat an {:.2} (nourriture {:.2})",
            sim.pops_du_quartier(q),
            qu.sat_annuelle_ppm as f64 / PPM as f64,
            qu.sat_nourriture_annuelle_ppm as f64 / PPM as f64
        );
    }
    let occ = sim.occupation();
    for (b, bat) in sim.monde.batiments.iter().enumerate() {
        let t = &sim.cat.batiments[bat.type_id];
        let lim = match bat.limitant {
            Limitant::Aucun => "-".to_string(),
            Limitant::MainOeuvre => "main-d'œuvre".to_string(),
            Limitant::Intrant(r) => format!("intrant {}", sim.cat.ressources[r].nom),
            Limitant::StockPlein => "stock plein".to_string(),
        };
        println!(
            "  {:<16} {}/{} pops, production {:>3}%  limite : {lim}",
            t.nom,
            occ[b],
            t.pops,
            bat.prod_ppm * 100 / PPM
        );
    }
    let ecart = sim.ecart_conservation();
    println!(
        "Conservation de la matière : {}",
        if ecart.iter().all(|&e| e == 0) {
            "OK".into()
        } else {
            format!("ÉCART {ecart:?}")
        }
    );
    if let Some(f) = &json {
        let doc = serde_json::json!({ "jour": sim.jour, "monde": sim.monde, "pops": sim.pops, "historique": sim.historique });
        std::fs::write(f, serde_json::to_string(&doc).unwrap()).expect("écriture du JSON");
    }
}
