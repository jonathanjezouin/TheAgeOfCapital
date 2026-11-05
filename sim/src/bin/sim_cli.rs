use sim::{Simulation, JOURS_PAR_AN};

fn main() {
    let mut annees: u32 = 10;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--annees" {
            annees = args.next().and_then(|v| v.parse().ok()).expect("--annees N");
        }
    }
    let mut s = Simulation::new();
    for _ in 0..annees {
        for _ in 0..JOURS_PAR_AN {
            s.tick();
        }
        println!("{} : {} pop(s)", s.annee(), s.monde.pops());
    }
}
