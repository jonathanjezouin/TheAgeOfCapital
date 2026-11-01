//! CLI headless : simule N années et écrit des séries temporelles en CSV.
//!
//!   sim-cli --annees 100 --graine 7 --csv sortie.csv

use sim::monde::Limitant;
use sim::{generer, Catalogue, ParamsVillage, PPM, U};
use std::fmt::Write as _;

fn aide() -> ! {
    eprintln!(
        "sim-cli [--annees N] [--graine N] [--hameaux N] [--occupation PCT]\n\
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
            "--hameaux" => pv.nb_hameaux = val().parse().unwrap_or_else(|_| aide()),
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
        "jour,annee,pops,sans_poste,vital_moy,confort_moy,differes,retards,charge_max_pct"
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
            let vital: i64 = sim
                .monde
                .quartiers
                .iter()
                .map(|q| q.vital_jour_ppm)
                .sum::<i64>()
                / nq;
            let confort: i64 = sim
                .monde
                .quartiers
                .iter()
                .map(|q| q.confort_jour_ppm)
                .sum::<i64>()
                / nq;
            write!(
                sortie,
                "{},{},{},{},{:.4},{:.4},{},{},{:.1}",
                sim.jour,
                sim.jour / sim.cat.sim.jours_par_an as u64,
                sim.pops.len(),
                sim.sans_poste(),
                vital as f64 / PPM as f64,
                confort as f64 / PPM as f64,
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
        "Village graine {} : {} hameaux, {} lieux de production, {} segments",
        pv.graine,
        sim.monde.quartiers.len(),
        sim.monde
            .batiments
            .iter()
            .filter(|b| b.quartier.is_none())
            .count(),
        sim.reseau.segments.len()
    );
    println!(
        "Après {} ans : {} pops, {} sans poste",
        annees,
        sim.pops.len(),
        sim.sans_poste()
    );
    let eff = sim.effectifs();
    for (q, qu) in sim.monde.quartiers.iter().enumerate() {
        println!(
            "  hameau {q:>2} : {}/{} pops, vital {:.2}, confort {:.2} (moyenne annuelle)",
            eff[q],
            qu.capacite_pops,
            qu.vital_annuel_ppm as f64 / PPM as f64,
            qu.confort_annuel_ppm as f64 / PPM as f64
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
        let effectif = if t.domestique { bat.presents } else { occ[b] };
        println!(
            "  {:<19} {}/{} pops, production {:>3}%  limite : {lim}",
            t.nom,
            effectif,
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
