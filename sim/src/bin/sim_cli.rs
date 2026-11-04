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
         \x20       [--json-final fichier.json] [--carte] [--terrain] [--journal] [--mesure N]"
    );
    std::process::exit(2)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut annees, mut pas) = (50u64, 30u64);
    let (mut carte, mut journal, mut terrain) = (false, false, false);
    let mut mesure: Option<u64> = None;
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
            "--carte" => carte = true,
            "--terrain" => terrain = true,
            "--mesure" => mesure = Some(val().parse().unwrap_or_else(|_| aide())),
            "--journal" => journal = true,
            _ => aide(),
        }
    }

    if let Some(n) = mesure {
        mesurer_peuplement(&pv, n, annees, donnees.as_deref());
        return;
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
        "Village graine {} : {} hameaux, {} lieux de production, {} segments, {} chantiers",
        pv.graine,
        sim.monde.quartiers.len(),
        sim.monde
            .batiments
            .iter()
            .filter(|b| b.quartier.is_none())
            .count(),
        sim.reseau.segments.len(),
        sim.chantiers_en_cours()
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
        let effectif = occ[b];
        let postes = sim.postes_de(b);
        println!(
            "  {:<19} {}/{} pops, production {:>3}%  limite : {lim}",
            if bat.chantier.is_some() {
                "(chantier)"
            } else {
                t.nom.as_str()
            },
            effectif,
            postes,
            bat.prod_ppm * 100 / PPM
        );
    }
    if terrain {
        println!("\n{}", sim.terrain_ascii());
    }
    if carte {
        println!("\n{}", sim.carte_ascii());
    }
    if journal {
        for e in &sim.evenements {
            println!(
                "  an {:>4} jour {:>3} : {}",
                e.jour / 365,
                e.jour % 365,
                e.texte
            );
        }
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

/// Mesure le peuplement sur les graines 1 à `n` : tailles des hameaux, hameaux vides, abandons (docs/07 §6, §14).
fn mesurer_peuplement(pv: &ParamsVillage, n: u64, annees: u64, donnees: Option<&str>) {
    let mut toutes: Vec<u32> = Vec::new();
    let (mut vides, mut abandonnes, mut detruits, mut pops, mut terres_ab, mut friche) = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    println!("graine  pops  hameaux  vides  abandonnés  détruits  terres abandonnées  tiles en friche  tailles");
    for g in 1..=n {
        let mut p = pv.clone();
        p.graine = g;
        let cat = match donnees {
            Some(f) => Catalogue::depuis_toml(&std::fs::read_to_string(f).expect("lecture des données")).expect("données"),
            None => Catalogue::defaut(),
        };
        let mut sim = generer(cat, &p).unwrap_or_else(|e| panic!("graine {g} : {e}"));
        sim.avancer(annees * 365);
        let st = sim.stats_peuplement();
        println!(
            "{g:>6}  {:>4}  {:>7}  {:>5}  {:>10}  {:>8}  {:>18}  {:>15}  {:?}",
            st.pops, st.tailles.len(), st.vides, st.abandonnes, st.detruits, st.terres_abandonnees, st.tiles_en_friche, st.tailles
        );
        toutes.extend(st.tailles.iter().copied());
        vides += st.vides;
        abandonnes += st.abandonnes;
        detruits += st.detruits;
        pops += st.pops;
        terres_ab += st.terres_abandonnees;
        friche += st.tiles_en_friche;
    }
    toutes.sort_unstable();
    let med = toutes.get(toutes.len() / 2).copied().unwrap_or(0);
    let max = toutes.last().copied().unwrap_or(0);
    println!(
        "\n{n} graines, {annees} ans : {pops} pops, {} hameaux vivants (médiane {med}, max {max}), {vides} vides ({:.0} %), {abandonnes} abandonnés, {detruits} détruits, {terres_ab} terres abandonnées, {friche} tiles en friche",
        toutes.len(),
        vides as f64 * 100.0 / toutes.len().max(1) as f64
    );
    let mut histo = [0u32; 8];
    for &t in &toutes {
        histo[(t as usize).min(7)] += 1;
    }
    println!("histogramme des tailles (0, 1, 2, 3, 4, 5, 6, 7 et +) : {histo:?}");
}
