//! La carte (tiles passives) et les entités actives (quartiers, bâtiments).

use crate::donnees::Qte;
use serde::Serialize;

/// Les 8 directions d'arêtes, dans l'ordre E, SE, S, SO, O, NO, N, NE :
/// (bit, dx, dy, bit opposé). Les index pairs sont orthogonaux, les impairs diagonaux.
pub const DIRS: [(u8, i32, i32, u8); 8] = [
    (1 << 0, 1, 0, 1 << 4),
    (1 << 1, 1, 1, 1 << 5),
    (1 << 2, 0, 1, 1 << 6),
    (1 << 3, -1, 1, 1 << 7),
    (1 << 4, -1, 0, 1 << 0),
    (1 << 5, -1, -1, 1 << 1),
    (1 << 6, 0, -1, 1 << 2),
    (1 << 7, 1, -1, 1 << 3),
];

/// Coût d'une arête orthogonale / diagonale (entiers : 14 ≈ 10 × √2, erreur ≤ 8 %).
pub const COUT_ORTHO: i64 = 10;
pub const COUT_DIAG: i64 = 14;

/// Coût de l'arête dans la direction `d` (index de `DIRS`).
pub fn cout_dir(d: usize) -> i64 {
    if d % 2 == 1 {
        COUT_DIAG
    } else {
        COUT_ORTHO
    }
}

/// Index de la direction opposée.
pub fn oppose(d: usize) -> usize {
    (d + 4) % 8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Contenu {
    Vide,
    /// Index dans `Monde::quartiers`.
    Quartier(usize),
    /// Index dans `Monde::batiments`.
    Batiment(usize),
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Tuile {
    pub contenu: Contenu,
    /// Demi-arêtes actives du centre de la tile vers ses voisines (bits de `DIRS`).
    /// L'infrastructure est une couche d'arêtes posée sur les tiles : elle n'en change pas l'usage
    /// (docs/02 §9). Deux tiles sont reliées quand les deux demi-arêtes en vis-à-vis sont présentes.
    pub aretes: u8,
    /// Variante visuelle (herbe, sprite de bâtiment) : pur rendu, aucune incidence économique.
    pub variante: u8,
    /// Fertilité de la tile (ppm de 1,0) : proportionnelle à la production des terres exploitées.
    /// Le terrain reste unique (plaine) ; seule la qualité de la terre varie.
    pub fertilite_ppm: i64,
}

/// Un hameau : lieu de vie à faible densité (logement + travail domestique).
#[derive(Clone, Debug, Serialize)]
pub struct Quartier {
    pub tuile: usize,
    /// Index dans `Catalogue::habitats`.
    pub habitat_id: usize,
    /// Capacité de logement (0 tant que le hameau est en construction).
    pub capacite_pops: u32,
    /// Atelier domestique embarqué sur la même tile (textile fait à la maison), s'il y en a un.
    pub atelier: Option<usize>,
    pub stock: Vec<Qte>,
    /// Besoins vitaux couverts (0..1 en ppm) lors du dernier jour.
    pub vital_jour_ppm: i64,
    /// Besoins de confort couverts (0..1 en ppm) lors du dernier jour.
    pub confort_jour_ppm: i64,
    /// Couverture de la ration du dernier jour par ressource (reçu / requis), indexée comme `Catalogue::ressources`.
    pub sat_par_ressource_ppm: Vec<i64>,
    pub cumul_vital_ppm: i64,
    pub cumul_confort_ppm: i64,
    pub jours_an: i64,
    pub vital_annuel_ppm: i64,
    pub confort_annuel_ppm: i64,
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
    /// Hameau auquel appartient un atelier domestique embarqué (même tile).
    pub quartier: Option<usize>,
    /// Effectif de l'équipe, pour un site de chantier.
    pub equipe: u32,
    /// Travail restant avant mise en service, pour un site de chantier.
    pub chantier: Option<Chantier>,
    pub stock_entree: Vec<Qte>,
    pub stock_sortie: Vec<Qte>,
    /// Pops arrivées au poste aujourd'hui.
    pub presents: u32,
    /// Production effective du dernier jour, en ppm de la production maximale.
    pub prod_ppm: i64,
    pub limitant: Limitant,
}

/// Ce qu'un chantier produit une fois terminé.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Cible {
    /// Défrichement : la tile devient une terre exploitée de ce type de bâtiment.
    Terre(usize),
    /// Fondation : le hameau d'index donné ouvre sa première maison.
    Hameau(usize),
    /// Une maison de plus (une pop) dans le hameau d'index donné.
    Maison(usize),
}

#[derive(Clone, Debug, Serialize)]
pub struct Chantier {
    pub cible: Cible,
    /// En milli-pop-jours.
    pub travail_restant: Qte,
    /// Travail total du chantier (milli-pop-jours) et matériau total (milli-unités) :
    /// le matériau se consomme au prorata du travail accompli, sans arrondi cumulé.
    pub travail_total: Qte,
    pub materiau_total: Qte,
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

    /// Voisin dans la direction `d` (index de `DIRS`, 8 directions), s'il est dans la carte.
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

    /// Demi-arêtes d'une tile (0 : elle ne fait pas partie du réseau).
    pub fn masque(&self, idx: usize) -> u8 {
        self.tuiles[idx].aretes
    }

    /// Direction (index de `DIRS`) de `a` vers `b`, si les tiles sont voisines.
    pub fn direction(&self, a: usize, b: usize) -> Option<usize> {
        let (ax, ay) = self.xy(a);
        let (bx, by) = self.xy(b);
        let (dx, dy) = (bx as i32 - ax as i32, by as i32 - ay as i32);
        DIRS.iter()
            .position(|&(_, ddx, ddy, _)| ddx == dx && ddy == dy)
    }

    /// Peut-on poser l'arête `a → b` ? Deux tiles voisines ; une diagonale ne croise pas
    /// l'autre diagonale du même carré 2×2 (sans nœud commun, voir docs/02 §9.1).
    pub fn peut_relier(&self, a: usize, b: usize) -> bool {
        let Some(d) = self.direction(a, b) else {
            return false;
        };
        if d % 2 == 0 {
            return true;
        }
        // L'autre diagonale relie les deux autres coins du carré.
        let (ax, ay) = self.xy(a);
        let (dx, dy) = (DIRS[d].1, DIRS[d].2);
        let c1 = self.idx((ax as i32 + dx) as usize, ay);
        let c2 = self.idx(ax, (ay as i32 + dy) as usize);
        match self.direction(c1, c2) {
            Some(d2) => self.tuiles[c1].aretes & DIRS[d2].0 == 0,
            None => true,
        }
    }

    /// Pose l'arête `a ↔ b` (les deux demi-arêtes). Sans effet si les tiles ne sont pas voisines.
    pub fn relier(&mut self, a: usize, b: usize) {
        if let Some(d) = self.direction(a, b) {
            self.tuiles[a].aretes |= DIRS[d].0;
            self.tuiles[b].aretes |= DIRS[d].3;
        }
    }
}
