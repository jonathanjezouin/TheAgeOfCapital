//! POC v2 : un monde presque vide.
//! Une tile habitation (1 pop), un champ à côté, et le temps qui passe. Rien d'autre.

pub const JOURS_PAR_AN: u32 = 365;
pub const ANNEE_DEPART: u32 = 1750;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Habitation { pops: u32 },
    Champ,
}

#[derive(Clone, Debug)]
pub struct Monde {
    pub largeur: usize,
    pub tiles: Vec<Tile>,
}

impl Monde {
    /// Habitation en (0,0), champ en (1,0).
    pub fn initial() -> Self {
        Monde { largeur: 2, tiles: vec![Tile::Habitation { pops: 1 }, Tile::Champ] }
    }

    pub fn pops(&self) -> u32 {
        self.tiles
            .iter()
            .map(|t| if let Tile::Habitation { pops } = t { *pops } else { 0 })
            .sum()
    }
}

#[derive(Clone, Debug)]
pub struct Simulation {
    pub monde: Monde,
    pub jour: u32,
}

impl Simulation {
    pub fn new() -> Self {
        Simulation { monde: Monde::initial(), jour: 0 }
    }

    /// Un tick = un jour. Pour l'instant, seul le temps avance.
    pub fn tick(&mut self) {
        self.jour += 1;
    }

    pub fn annee(&self) -> u32 {
        ANNEE_DEPART + self.jour / JOURS_PAR_AN
    }

    pub fn jour_de_l_annee(&self) -> u32 {
        self.jour % JOURS_PAR_AN
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Tile {
    pub fn nom(&self) -> &'static str {
        match self {
            Tile::Habitation { .. } => "habitation",
            Tile::Champ => "champ",
        }
    }
}

impl Simulation {
    /// Instantané JSON de l'état (écrit à la main : le moteur reste sans dépendance).
    pub fn etat_json(&self) -> String {
        let tiles: Vec<String> = self
            .monde
            .tiles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let pops = if let Tile::Habitation { pops } = t { *pops } else { 0 };
                format!(
                    r#"{{"x":{},"y":{},"type":"{}","pops":{}}}"#,
                    i % self.monde.largeur,
                    i / self.monde.largeur,
                    t.nom(),
                    pops
                )
            })
            .collect();
        format!(
            r#"{{"jour":{},"annee":{},"jour_de_l_annee":{},"pops":{},"largeur":{},"tiles":[{}]}}"#,
            self.jour,
            self.annee(),
            self.jour_de_l_annee(),
            self.monde.pops(),
            self.monde.largeur,
            tiles.join(",")
        )
    }
}
