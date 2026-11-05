use sim::*;

#[test]
fn etat_initial() {
    let s = Simulation::new();
    assert_eq!(s.monde.pops(), 1);
    assert_eq!(s.monde.tiles[1], Tile::Champ);
    assert_eq!(s.annee(), 1750);
}

#[test]
fn le_temps_passe() {
    let mut s = Simulation::new();
    for _ in 0..JOURS_PAR_AN * 3 + 10 {
        s.tick();
    }
    assert_eq!(s.annee(), 1753);
    assert_eq!(s.jour_de_l_annee(), 10);
}
