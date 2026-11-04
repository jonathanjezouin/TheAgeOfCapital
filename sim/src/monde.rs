//! La carte (tiles passives) et les entités actives (quartiers, bâtiments).

use crate::donnees::Qte;
use crate::terrain::{Eau, Terrain};
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

/// Statut d'une pop (docs/07 §15) : aujourd'hui toutes les pops sont serves ; `None` = libre (futur).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Statut {
    /// Pas de liberté d'installation ni de construction ; liberté du lieu de travail ; prélèvement en nature.
    Serf,
}

/// Un personnage : le décideur politico-économique (docs/07 §15). Créé par un poste. Il possède un stock (grenier)
/// et un domaine ; ses décisions (construire, installer, abandonner) sont prises dans `Simulation::etendre`.
#[derive(Clone, Debug, Serialize)]
pub struct Personnage {
    pub id: usize,
    pub nom: String,
    /// Poste qui l'a fait naître (`seigneur_local`) ; la règle de dévolution est une donnée, non implémentée.
    pub poste: String,
    /// Grenier : stock du personnage par ressource (milli-unités), alimenté par le champart.
    pub grenier: Vec<Qte>,
    /// Domaine : identifiant (aujourd'hui toute la carte) ; `None` = sans domaine.
    pub domaine: Option<usize>,
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
    /// Pour l'instant indépendante du terrain (champ radial autour du village, docs/07 étape 2).
    pub fertilite_ppm: i64,
    /// Terrain de la tile (docs/07 §3) : plaine, plaine riche, rocailleuse, marécageuse, forêt.
    pub terrain: Terrain,
    /// Cours d'eau qui occupe la tile (rien, ruisseau, rivière, fleuve) : non constructible tant qu'il n'y a pas de pont.
    pub eau: Eau,
    /// Direction (index de `DIRS`) vers la tile aval d'un cours d'eau, 255 sinon : pur rendu.
    pub aval: u8,
    /// Mois écoulés depuis que la tile a été abandonnée ou libérée (0 : jamais utilisée, ou occupée). Quand elle atteint
    /// la durée de repousse, la tile redevient forêt (docs/07 §14).
    pub vacant_mois: u16,
}

impl Tuile {
    /// Peut-on y poser un lieu (après conversion éventuelle du terrain) ? Tout sauf l'eau : forêt et marais
    /// se convertissent par un chantier (docs/07 §12) ; une route peut franchir l'eau par un pont.
    pub fn constructible(&self) -> bool {
        self.eau == Eau::Aucune
    }

    /// Terrain ouvert, sans conversion à faire : ni eau, ni forêt, ni marais.
    pub fn ouvert(&self) -> bool {
        self.eau == Eau::Aucune && self.terrain.ouvert()
    }
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
    /// Mois consécutifs sans aucune pop (le hameau est abandonné après `hameau_vide_ans`).
    pub vide_mois: u32,
    /// Hameau abandonné : mois depuis l'abandon. Il n'offre plus de logement, s'use, puis est détruit.
    pub abandon_mois: Option<u32>,
    /// Hameau détruit : sa tile est libre, l'index reste (les lieux sont indexés par position).
    pub detruit: bool,
    /// Capacité de logement avant l'abandon : celle que la remise en état rend.
    pub capacite_memoire: u32,
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
    /// Tile « nœud » du bâtiment : celle par laquelle il est desservi par le réseau.
    pub tuile: usize,
    /// Tiles occupées (au moins `tuile`) : N tiles contiguës pour une terre exploitée, 1 sinon.
    pub emprise: Vec<usize>,
    /// Hameau auquel appartient un atelier domestique embarqué (même tile).
    pub quartier: Option<usize>,
    /// Effectif de l'équipe, pour un site de chantier.
    pub equipe: u32,
    /// Travail restant avant mise en service, pour un site de chantier.
    pub chantier: Option<Chantier>,
    pub stock_entree: Vec<Qte>,
    pub stock_sortie: Vec<Qte>,
    /// Pops arrivées au poste aujourd'hui (équipe d'un chantier).
    pub presents: u32,
    /// Travail reçu aujourd'hui par une terre ou un atelier domestique, en milli pop-jours.
    pub mo_milli: i64,
    /// Production effective, en ppm de la production maximale : moyenne mobile sur ~30 jours pour une
    /// terre (le travail arrive par visites), valeur du jour sinon.
    pub prod_ppm: i64,
    /// Travail encore réclamé en fin de journée par une terre (milli pop-jours) : ce que personne n'est venu faire.
    pub demande_milli: i64,
    /// Récolte sur pied d'une terre, en ppm de journée de production : elle croît d'une journée par jour
    /// (plafonnée au stock de sortie) et le travail la convertit en stock. La terre ne peut donc pas
    /// produire plus vite que son rythme nominal, même si l'on y vient souvent (docs/02 §10.4).
    pub sur_pied_ppm: i64,
    /// Travail reçu, moyenne mobile sur ~30 jours (milli pop-jours par jour).
    pub mo_lisse_milli: i64,
    pub limitant: Limitant,
    /// Mois consécutifs sans travail reçu (une terre est abandonnée après `terre_inactive_ans`).
    pub inactif_mois: u32,
    /// Terre abandonnée : mois depuis l'abandon. Elle ne produit plus, puis redevient friche (docs/07 §14).
    pub abandon_mois: Option<u32>,
    /// Chantier : travail restant au dernier contrôle mensuel, pour détecter l'absence de progrès.
    pub progres_ref: Qte,
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
    /// Remise en état d'un hameau abandonné (moins chère qu'une fondation, de plus en plus chère avec l'usure).
    RemiseHameau(usize),
    /// Remise en service d'une terre abandonnée.
    RemiseTerre(usize),
    /// Route ordonnée par le seigneur entre un hameau et une terre dont le trajet à pied se répète (hameau, terre).
    Route(usize, usize),
    /// Défrichement de tiles de plus pour la terre exploitée d'index donné.
    Extension(usize),
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
    /// Maître d'ouvrage : le personnage qui décide et paie (son grenier fournit le matériau). `None` : sans maître.
    pub maitre: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Monde {
    pub largeur: usize,
    pub hauteur: usize,
    pub tuiles: Vec<Tuile>,
    pub quartiers: Vec<Quartier>,
    pub batiments: Vec<Batiment>,
    /// Les personnages (aujourd'hui un seul : le seigneur local).
    pub personnages: Vec<Personnage>,
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
