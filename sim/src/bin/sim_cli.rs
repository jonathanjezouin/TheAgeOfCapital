use sim::*;

fn main() {
    let mut p = Params::default();
    let mut annees: u32 = 20;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut v = || args.next().and_then(|v| v.parse::<i64>().ok()).expect("valeur entière attendue");
        match a.as_str() {
            "--annees" => annees = v() as u32,
            "--graine" => p.graine = v() as u64,
            "--meteo" => p.meteo_fixe_pm = Some(v()),
            _ => panic!("option inconnue : {a} (--annees N --graine N --meteo PM)"),
        }
    }
    let mut s = Simulation::avec(p);
    println!("année météo | grain: récolte(brut) prélevé mangé manque stock | bois: stock | jours: travaillés (terre/transp/bois/vêt) chômage");
    for _ in 0..annees {
        for _ in 0..calendrier::JOURS_PAR_AN {
            s.tick();
        }
    }
    let mut deficits = 0;
    for b in &s.historique {
        let j = b.jours;
        if b.manque_grain > 0 {
            deficits += 1;
        }
        println!(
            "{} ×{:.2} | {:>5} {:>5} {:>5} {:>5} {:>6} | {:>5} | {:>3} ({:>2}/{:>2}/{:>2}/{:>2}) {:>3}",
            b.annee, b.meteo_pm as f64 / 1000.0,
            b.grain_brut, b.grain_preleve, b.grain_mange, b.manque_grain, b.stock_fin.grain,
            b.stock_fin.bois,
            j.travailles(), j.labour + j.semis + j.recolte, j.transport_grain, j.coupe + j.transport_bois, j.vetement, j.chomage,
        );
    }
    println!("années avec manque de grain : {deficits}/{}", s.historique.len());
}
