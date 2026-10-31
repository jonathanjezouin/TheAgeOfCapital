//! La carte (tiles passives) et les entités actives (quartiers, bâtiments).

use crate::donnees::Qte;
use serde::Serialize;

/// Demi-connexions d'une tile d'infrastructure (POC : orthogonales).
pub const N: u8 = 1;
pub const E: u8 = 2;
pub const S: u8 = 4;
pub const O: u8 = 8;
pub const TOUTES: u8 = N | E | S | O;

/// (bit, dx, dy, bit opposé)
pub const DIRS: [(u8, i32, i32, u8); 4] =
    [(N, 0, -1, S), (E, 1, 0, O), (S, 0, 1, N), (O, -1, 0, E)];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Contenu {
    Vide,
    /// Chemin de terre avec ses demi-directions actives.
    Chemin {
        masque: u8,
    },
    /// Index dans `Monde::quartiers`. Embarque nativement des rues (toutes directions).
    Quartier(usize),
    /// Index dans `Monde::batiments`.
    Batiment(usize),
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Tuile {
    pub contenu: Contenu,
    /// Variante visuelle (herbe, sprite de bâtiment) : pur rendu, aucune incidence économique.
    pub variante: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct Quartier {
    pub tuile: usize,
    pub stock: Vec<Qte>,
    /// Compteur de natalité / mortalité, en milli-habitants.
    pub croissance_milli: i64,
    pub sat_jour_ppm: i64,
    pub sat_nourriture_jour_ppm: i64,
    /// Satisfaction du dernier jour par ressource (reçu / requis), indexée comme `Catalogue::ressources`.
    pub sat_par_ressource_ppm: Vec<i64>,
    pub cumul_sat_ppm: i64,
    pub cumul_sat_nourriture_ppm: i64,
    pub jours_an: i64,
    pub sat_annuelle_ppm: i64,
    pub sat_nourriture_annuelle_ppm: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Limitant {
    Aucun,
    MainOeuvre,
    /// Index de la ressource manquante.
    Intrant(usize),
    StockPlein,
}

#[derive(Clone, Debug, Serialize)]
pub struct Batiment {
    pub type_id: usize,
    pub tuile: usize,
    pub stock_entree: Vec<Qte>,
    pub stock_sortie: Vec<Qte>,
    /// Pops arrivées au poste aujourd'hui.
    pub presents: u32,
    /// Production effective du dernier jour, en ppm de la production maximale.
    pub prod_ppm: i64,
    pub limitant: Limitant,
}

#[derive(Clone, Debug, Serialize)]
pub struct Monde {
    pub largeur: usize,
    pub hauteur: usize,
    pub tuiles: Vec<Tuile>,
    pub quartiers: Vec<Quartier>,
    pub batiments: Vec<Batiment>,
}

impl Monde {
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.largeur + x
    }

    pub fn xy(&self, idx: usize) -> (usize, usize) {
        (idx % self.largeur, idx / self.largeur)
    }

    /// Voisin orthogonal dans la direction `d` (index de `DIRS`), s'il est dans la carte.
    pub fn voisin(&self, idx: usize, d: usize) -> Option<usize> {
        let (x, y) = self.xy(idx);
        let nx = x as i32 + DIRS[d].1;
        let ny = y as i32 + DIRS[d].2;
        if nx < 0 || ny < 0 || nx >= self.largeur as i32 || ny >= self.hauteur as i32 {
            None
        } else {
            Some(self.idx(nx as usize, ny as usize))
        }
    }

    /// Demi-connexions d'une tile traversable (chemin ou quartier).
    pub fn masque(&self, idx: usize) -> Option<u8> {
        match self.tuiles[idx].contenu {
            Contenu::Chemin { masque } => Some(masque),
            Contenu::Quartier(_) => Some(TOUTES),
            _ => None,
        }
    }
}
