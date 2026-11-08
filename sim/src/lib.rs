//! POC v2 : la subsistance d'une pop (docs/08-concept-subsistance.md).
//!
//! Une ferme isolée, 5 champs, 3 forêts, une pop. Un tick = un jour. Grain, bois et vêtement
//! sont produits, transportés (quantité × distance × k jours), consommés ; le seigneur prélève
//! 15 % de la production dans le stock de la ferme, à date fixe.
//! Toutes les quantités sont des entiers : l'unité de matériau est indivisible (décision du 2026-11-08).

pub mod calendrier;
pub mod rng;

use calendrier::*;
use rng::Rng;
use serde::Serialize;

// ───────────────────────── Paramètres ─────────────────────────

#[derive(Clone, Debug)]
pub struct Params {
    pub graine: u64,
    /// Si présent, météo fixe en ‰ (1000 = année moyenne) au lieu d'un tirage entre 800 et 1200.
    pub meteo_fixe_pm: Option<i64>,
    pub prelevement_pct: i64,
    /// Date fixe du prélèvement (mois, jour du mois) : Saint-Martin, hypothèse à valider.
    pub prelevement_date: (u32, u32),
    pub grains_par_champ: i64,
    pub bois_par_foret: i64,
    pub labour_j: u32,
    pub semis_j: u32,
    pub recolte_j: u32,
    pub coupe_j_par_unite: u32,
    pub vetement_j: u32,
    pub k_grain: i64,
    pub k_bois: i64,
    pub conso_grain_mois: i64,
    pub conso_bois_mois_froid: i64,
    pub usure_vetement_pct_mois: u64,
    /// Stock au 1er janvier 1750, en unités (point ouvert n°5 du doc : hypothèse de départ).
    pub stock_initial: (i64, i64, i64),
}

impl Default for Params {
    fn default() -> Self {
        Params {
            graine: 1,
            meteo_fixe_pm: None,
            prelevement_pct: 15,
            prelevement_date: (11, 11),
            grains_par_champ: 3,
            bois_par_foret: 3,
            labour_j: 5,
            semis_j: 5,
            recolte_j: 10,
            coupe_j_par_unite: 2,
            vetement_j: 20,
            k_grain: 1,
            k_bois: 2,
            conso_grain_mois: 1,
            conso_bois_mois_froid: 1,
            usure_vetement_pct_mois: 20,
            stock_initial: (12, 7, 2),
        }
    }
}

// ───────────────────────── Monde ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Rien de fait : à labourer en octobre-novembre.
    Nu,
    /// Labouré : à semer en mars-avril.
    Laboure,
    /// Semé : à récolter en juillet-août.
    Seme,
}

#[derive(Clone, Debug)]
pub struct Champ {
    pub phase: Phase,
    /// Jours de travail déjà faits dans la phase en cours.
    pub travail: u32,
    /// Grain récolté qui attend d'être transporté à la ferme (unités entières).
    pub grain: i64,
    /// Jours déjà passés à porter l'unité en cours.
    pub portage: u32,
}

#[derive(Clone, Debug)]
pub struct Foret {
    /// Petit bois à ramasser cette année : la forêt est permanente, le petit bois repousse au 1er janvier.
    pub disponible: i64,
    /// Bois coupé qui attend d'être transporté.
    pub coupe: i64,
    /// Jours de travail déjà faits sur l'unité en cours de coupe.
    pub travail: u32,
    /// Jours déjà passés à porter l'unité en cours.
    pub portage: u32,
}

#[derive(Clone, Debug)]
pub enum Tile {
    Plaine,
    Ferme,
    Champ(Champ),
    Foret(Foret),
}

#[derive(Clone, Debug)]
pub struct Monde {
    pub largeur: usize,
    pub hauteur: usize,
    pub tiles: Vec<Tile>,
    pub ferme: usize,
}

impl Monde {
    /// Ferme en (1,2) ; 5 champs à l'est (distances 1 à 3) ; 3 forêts au nord (distances 2 et 3).
    pub fn initial(p: &Params) -> Self {
        let (largeur, hauteur) = (6, 5);
        let mut tiles = vec![Tile::Plaine; largeur * hauteur];
        let ferme = 2 * largeur + 1;
        tiles[ferme] = Tile::Ferme;
        for (x, y) in [(2, 2), (3, 1), (3, 2), (3, 3), (4, 2)] {
            tiles[y * largeur + x] = Tile::Champ(Champ { phase: Phase::Laboure, travail: 0, grain: 0, portage: 0 });
        }
        for (x, y) in [(1, 0), (0, 0), (2, 0)] {
            tiles[y * largeur + x] = Tile::Foret(Foret { disponible: p.bois_par_foret, coupe: 0, travail: 0, portage: 0 });
        }
        Monde { largeur, hauteur, tiles, ferme }
    }

    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.largeur, i / self.largeur)
    }

    /// Distance à pied en tiles, par les axes (Manhattan).
    pub fn distance(&self, i: usize) -> i64 {
        let (x, y) = self.xy(i);
        let (fx, fy) = self.xy(self.ferme);
        ((x as i64 - fx as i64).abs() + (y as i64 - fy as i64).abs()).max(0)
    }

    fn indices<F: Fn(&Tile) -> bool>(&self, f: F) -> Vec<usize> {
        (0..self.tiles.len()).filter(|&i| f(&self.tiles[i])).collect()
    }
}

// ───────────────────────── Pop et comptes ─────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Default)]
pub struct Stocks {
    pub grain: i64,
    pub bois: i64,
    pub vetement: i64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Activite {
    Aucune,
    Repos,
    Labour(usize),
    Semis(usize),
    Recolte(usize),
    TransportGrain(usize),
    Coupe(usize),
    TransportBois(usize),
    Vetement,
    Chomage,
}

impl Activite {
    pub fn code(&self) -> &'static str {
        match self {
            Activite::Aucune => "aucune",
            Activite::Repos => "repos",
            Activite::Labour(_) => "labour",
            Activite::Semis(_) => "semis",
            Activite::Recolte(_) => "recolte",
            Activite::TransportGrain(_) => "transport_grain",
            Activite::Coupe(_) => "coupe",
            Activite::TransportBois(_) => "transport_bois",
            Activite::Vetement => "vetement",
            Activite::Chomage => "chomage",
        }
    }

    pub fn tile(&self) -> Option<usize> {
        match self {
            Activite::Labour(i)
            | Activite::Semis(i)
            | Activite::Recolte(i)
            | Activite::TransportGrain(i)
            | Activite::Coupe(i)
            | Activite::TransportBois(i) => Some(*i),
            _ => None,
        }
    }
}

/// Jours de l'année par poste (somme = 365).
#[derive(Clone, Copy, Debug, Serialize, Default, PartialEq, Eq)]
pub struct Jours {
    pub repos: u32,
    pub labour: u32,
    pub semis: u32,
    pub recolte: u32,
    pub transport_grain: u32,
    pub coupe: u32,
    pub transport_bois: u32,
    pub vetement: u32,
    pub chomage: u32,
}

impl Jours {
    fn ajouter(&mut self, a: &Activite) {
        match a {
            Activite::Repos => self.repos += 1,
            Activite::Labour(_) => self.labour += 1,
            Activite::Semis(_) => self.semis += 1,
            Activite::Recolte(_) => self.recolte += 1,
            Activite::TransportGrain(_) => self.transport_grain += 1,
            Activite::Coupe(_) => self.coupe += 1,
            Activite::TransportBois(_) => self.transport_bois += 1,
            Activite::Vetement => self.vetement += 1,
            Activite::Chomage | Activite::Aucune => self.chomage += 1,
        }
    }

    pub fn total(&self) -> u32 {
        self.repos + self.labour + self.semis + self.recolte + self.transport_grain + self.coupe + self.transport_bois + self.vetement + self.chomage
    }

    /// Jours réellement travaillés (hors dimanche et chômage).
    pub fn travailles(&self) -> u32 {
        self.total() - self.repos - self.chomage
    }
}

/// Bilan d'une année (quantités en parts).
#[derive(Clone, Debug, Serialize, Default)]
pub struct Bilan {
    pub annee: u32,
    /// Facteur météo en ‰.
    pub meteo_pm: i64,
    pub stock_debut: Stocks,
    pub stock_fin: Stocks,
    pub grain_brut: i64,
    pub grain_preleve: i64,
    pub bois_brut: i64,
    pub bois_preleve: i64,
    pub grain_mange: i64,
    pub bois_brule: i64,
    pub vetement_use: i64,
    pub vetements_faits: u32,
    pub manque_grain: i64,
    pub manque_bois: i64,
    pub manque_vetement: i64,
    pub champs_recoltes: u32,
    pub champs_perdus: u32,
    pub jours: Jours,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Point {
    pub jour: u32,
    pub grain: i64,
    pub bois: i64,
    pub vetement: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Evenement {
    pub jour: u32,
    pub texte: String,
}

// ───────────────────────── Simulation ─────────────────────────

#[derive(Clone, Debug)]
pub struct Simulation {
    pub params: Params,
    pub monde: Monde,
    /// Nombre de jours déjà simulés.
    pub jour: u32,
    rng: Rng,
    pub meteo_pm: i64,
    pub stocks: Stocks,
    pub travail_vetement: u32,
    /// Production brute depuis le dernier prélèvement (assiette du prochain).
    pub grain_a_taxer: i64,
    pub bois_a_taxer: i64,
    pub activite: Activite,
    pub raison: String,
    pub bilan: Bilan,
    pub historique: Vec<Bilan>,
    pub serie: Vec<Point>,
    pub journal: Vec<Evenement>,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    pub fn new() -> Self {
        Self::avec(Params::default())
    }

    pub fn avec(params: Params) -> Self {
        let monde = Monde::initial(&params);
        let (g, b, v) = params.stock_initial;
        let stocks = Stocks { grain: g, bois: b, vetement: v };
        let mut s = Simulation {
            rng: Rng::new(params.graine),
            monde,
            jour: 0,
            meteo_pm: 1000,
            stocks,
            travail_vetement: 0,
            grain_a_taxer: 0,
            bois_a_taxer: 0,
            activite: Activite::Aucune,
            raison: String::from("Pas encore commencé"),
            bilan: Bilan::default(),
            historique: Vec::new(),
            serie: Vec::new(),
            journal: Vec::new(),
            params,
        };
        s.ouvrir_annee();
        s
    }

    pub fn annee(&self) -> u32 {
        ANNEE_DEPART + self.jour / JOURS_PAR_AN
    }

    // ── Tick ──

    /// Simule un jour.
    pub fn tick(&mut self) {
        let jda = self.jour % JOURS_PAR_AN;
        let (mois, jdm) = mois_jour(jda);
        if jda == 0 && self.jour > 0 {
            self.fermer_annee();
            self.ouvrir_annee();
        }
        if jdm == 1 {
            self.debut_de_mois(mois);
        }
        if (mois, jdm) == self.params.prelevement_date {
            self.prelevement();
        }
        let (act, raison) = if self.jour % 7 == 6 {
            (Activite::Repos, String::from("Dimanche : jour chômé"))
        } else {
            self.travailler(mois)
        };
        self.bilan.jours.ajouter(&act);
        self.activite = act;
        self.raison = raison;
        self.jour += 1;
    }

    fn noter(&mut self, texte: String) {
        self.journal.push(Evenement { jour: self.jour, texte });
        if self.journal.len() > 200 {
            self.journal.remove(0);
        }
    }

    fn fermer_annee(&mut self) {
        let mut b = std::mem::take(&mut self.bilan);
        b.stock_fin = self.stocks;
        self.historique.push(b);
    }

    /// Nouvelle année : météo tirée, forêts qui repoussent, nouveau bilan.
    fn ouvrir_annee(&mut self) {
        self.meteo_pm = match self.params.meteo_fixe_pm {
            Some(m) => m,
            None => 800 + self.rng.sous(401) as i64,
        };
        let repousse = self.params.bois_par_foret;
        for t in &mut self.monde.tiles {
            if let Tile::Foret(f) = t {
                f.disponible = repousse;
            }
        }
        self.bilan = Bilan { annee: self.annee(), meteo_pm: self.meteo_pm, stock_debut: self.stocks, ..Default::default() };
        let m = self.meteo_pm;
        let a = self.annee();
        self.noter(format!("{a} : météo ×{:.2}", m as f64 / 1000.0));
    }

    fn debut_de_mois(&mut self, mois: u32) {
        // Fin des fenêtres de travail : ce qui n'a pas été fait retombe en friche.
        let a_faire: Vec<usize> = self.monde.indices(|t| matches!(t, Tile::Champ(_)));
        for i in a_faire {
            let (xy, retombe) = {
                let xy = self.monde.xy(i);
                let Tile::Champ(c) = &mut self.monde.tiles[i] else { continue };
                let retombe = match (mois, c.phase) {
                    (5, Phase::Laboure) => Some("semis non fait"),
                    (9, Phase::Seme) => Some("récolte non faite : grain perdu"),
                    (12, Phase::Nu) if c.travail > 0 => Some("labour inachevé"),
                    _ => None,
                };
                if retombe.is_some() {
                    c.phase = Phase::Nu;
                    c.travail = 0;
                }
                (xy, retombe)
            };
            if let Some(r) = retombe {
                if r.starts_with("récolte") {
                    self.bilan.champs_perdus += 1;
                }
                self.noter(format!("Champ ({},{}) retombe en friche : {r}", xy.0, xy.1));
            }
        }
        // Consommation mensuelle.
        let p = self.params.clone();
        let faim = p.conso_grain_mois;
        let mange = faim.min(self.stocks.grain);
        self.stocks.grain -= mange;
        self.bilan.grain_mange += mange;
        if mange < faim {
            self.bilan.manque_grain += faim - mange;
            self.noter(String::from("Manque de grain (trace, sans conséquence pour l'instant)"));
        }
        if mois_froid(mois) {
            let besoin = p.conso_bois_mois_froid;
            let brule = besoin.min(self.stocks.bois);
            self.stocks.bois -= brule;
            self.bilan.bois_brule += brule;
            if brule < besoin {
                self.bilan.manque_bois += besoin - brule;
                self.noter(String::from("Manque de bois (trace)"));
            }
        }
        if self.rng.sous(100) < p.usure_vetement_pct_mois {
            let use_ = 1.min(self.stocks.vetement);
            self.stocks.vetement -= use_;
            self.bilan.vetement_use += use_;
            if use_ < 1 {
                self.bilan.manque_vetement += 1 - use_;
                self.noter(String::from("Un vêtement s'use mais il n'y en a plus (trace)"));
            } else {
                self.noter(String::from("Un vêtement est usé"));
            }
        }
        self.serie.push(Point { jour: self.jour, grain: self.stocks.grain, bois: self.stocks.bois, vetement: self.stocks.vetement });
    }

    /// Part du seigneur, arrondie à l'unité la plus proche (unité indivisible).
    fn part_du_seigneur(&self, brut: i64) -> i64 {
        (brut * self.params.prelevement_pct + 50) / 100
    }

    /// Prélèvement à date fixe : 15 % de la production brute depuis le précédent, pris dans le stock de la ferme.
    pub fn prelevement(&mut self) {
        let (du_grain, du_bois) = (self.part_du_seigneur(self.grain_a_taxer), self.part_du_seigneur(self.bois_a_taxer));
        let pris_grain = du_grain.min(self.stocks.grain);
        let pris_bois = du_bois.min(self.stocks.bois);
        self.stocks.grain -= pris_grain;
        self.stocks.bois -= pris_bois;
        self.bilan.grain_preleve += pris_grain;
        self.bilan.bois_preleve += pris_bois;
        let mut texte = format!(
            "Prélèvement du seigneur : {pris_grain} grain (15 % de {}), {pris_bois} bois (15 % de {})",
            self.grain_a_taxer, self.bois_a_taxer
        );
        if pris_grain < du_grain || pris_bois < du_bois {
            texte.push_str(" — stock insuffisant, prélèvement partiel (trace)");
        }
        self.grain_a_taxer = 0;
        self.bois_a_taxer = 0;
        self.noter(texte);
    }

    // ── Décision quotidienne (docs/08 §6) ──

    /// Seuil en unités : le bois est ramassé tant que le stock est strictement inférieur
    /// à (5 + prélèvement attendu) + 10 %, arrondi à l'unité supérieure.
    pub fn seuil_bois(&self) -> i64 {
        let p = &self.params;
        let attendu = self.part_du_seigneur(p.bois_par_foret * 3);
        ((5 + attendu) * 11 + 9) / 10
    }

    pub fn seuil_vetement(&self) -> i64 {
        2
    }

    fn travailler(&mut self, mois: u32) -> (Activite, String) {
        // 2. Travail disponible sur la terre : culture d'abord, transport du grain ensuite.
        if let Some(r) = self.travail_terre(mois) {
            return r;
        }
        // 3. Bois si le stock est inférieur au seuil.
        if self.stocks.bois < self.seuil_bois() {
            if let Some(r) = self.travail_bois() {
                return r;
            }
        }
        // 4. Vêtement si le stock est inférieur à 2.
        if self.stocks.vetement < self.seuil_vetement() {
            self.travail_vetement += 1;
            if self.travail_vetement >= self.params.vetement_j {
                self.travail_vetement = 0;
                self.stocks.vetement += 1;
                self.bilan.vetements_faits += 1;
                self.noter(String::from("Un vêtement est terminé"));
            }
            return (Activite::Vetement, format!("Vêtements : stock {} < 2 → couture à la maison", self.stocks.vetement));
        }
        // 5. Sinon, chômage supplémentaire.
        (Activite::Chomage, String::from("Rien d'urgent : chômage supplémentaire"))
    }

    /// Champ où travailler : d'abord celui dont le travail est entamé, sinon le plus proche.
    fn choisir_champ(&self, phase: Phase) -> Option<usize> {
        let mut c = self.monde.indices(|t| matches!(t, Tile::Champ(c) if c.phase == phase));
        c.sort_by_key(|&i| {
            let entame = matches!(&self.monde.tiles[i], Tile::Champ(c) if c.travail > 0);
            (!entame, self.monde.distance(i), i)
        });
        c.first().copied()
    }

    fn travail_terre(&mut self, mois: u32) -> Option<(Activite, String)> {
        let p = self.params.clone();
        let (phase, requis, fenetre, nom) = match mois {
            7 | 8 => (Phase::Seme, p.recolte_j, "juillet-août", "Récolte"),
            3 | 4 => (Phase::Laboure, p.semis_j, "mars-avril", "Semis"),
            10 | 11 => (Phase::Nu, p.labour_j, "octobre-novembre", "Labour"),
            _ => (Phase::Nu, 0, "", ""),
        };
        if requis > 0 {
            if let Some(i) = self.choisir_champ(phase) {
                let (x, y) = self.monde.xy(i);
                let mut fini = false;
                let mut jour_n = 0;
                if let Tile::Champ(c) = &mut self.monde.tiles[i] {
                    c.travail += 1;
                    jour_n = c.travail;
                    if c.travail >= requis {
                        fini = true;
                    }
                }
                if fini {
                    match phase {
                        Phase::Nu => self.passer_phase(i, Phase::Laboure),
                        Phase::Laboure => self.passer_phase(i, Phase::Seme),
                        Phase::Seme => self.recolter(i),
                    }
                }
                let act = match phase {
                    Phase::Nu => Activite::Labour(i),
                    Phase::Laboure => Activite::Semis(i),
                    Phase::Seme => Activite::Recolte(i),
                };
                return Some((act, format!("{nom} du champ ({x},{y}), jour {jour_n}/{requis} ({fenetre})")));
            }
        }
        // Transport du grain : le champ le plus proche qui en a.
        let mut c = self.monde.indices(|t| matches!(t, Tile::Champ(c) if c.grain > 0));
        c.sort_by_key(|&i| {
            let entame = matches!(&self.monde.tiles[i], Tile::Champ(c) if c.portage > 0);
            (!entame, self.monde.distance(i), i)
        });
        let i = *c.first()?;
        let d = self.monde.distance(i);
        let cout = (d * p.k_grain) as u32;
        let (x, y) = self.monde.xy(i);
        let (mut reste, mut jour_n) = (0, 0);
        if let Tile::Champ(ch) = &mut self.monde.tiles[i] {
            ch.portage += 1;
            jour_n = ch.portage;
            if ch.portage >= cout {
                ch.portage = 0;
                ch.grain -= 1;
                self.stocks.grain += 1;
            }
            reste = ch.grain;
        }
        Some((
            Activite::TransportGrain(i),
            format!("Transport du grain du champ ({x},{y}) : 1 grain = {d} tiles × {} j, jour {jour_n}/{cout}, reste {reste} à porter", p.k_grain),
        ))
    }

    fn passer_phase(&mut self, i: usize, nouvelle: Phase) {
        let (x, y) = self.monde.xy(i);
        if let Tile::Champ(c) = &mut self.monde.tiles[i] {
            c.phase = nouvelle;
            c.travail = 0;
        }
        let nom = if nouvelle == Phase::Laboure { "labouré" } else { "semé" };
        self.noter(format!("Champ ({x},{y}) {nom}"));
    }

    /// Fin de la récolte d'un champ : grains tirés selon la météo, puis prélèvement du seigneur.
    fn recolter(&mut self, i: usize) {
        let pm = self.meteo_pm;
        let mut brut = 0;
        for _ in 0..self.params.grains_par_champ {
            let mut g = 1;
            if pm > 1000 && (self.rng.sous(1000) as i64) < pm - 1000 {
                g = 2;
            } else if pm < 1000 && (self.rng.sous(1000) as i64) < 1000 - pm {
                g = 0;
            }
            brut += g;
        }
        if let Tile::Champ(c) = &mut self.monde.tiles[i] {
            c.grain += brut;
            c.phase = Phase::Nu;
            c.travail = 0;
        }
        self.bilan.grain_brut += brut;
        self.grain_a_taxer += brut;
        self.bilan.champs_recoltes += 1;
        let (x, y) = self.monde.xy(i);
        self.noter(format!("Champ ({x},{y}) récolté : {brut} grains"));
    }

    fn travail_bois(&mut self) -> Option<(Activite, String)> {
        let p = self.params.clone();
        // Bois déjà coupé à rapporter.
        let mut c = self.monde.indices(|t| matches!(t, Tile::Foret(f) if f.coupe > 0));
        c.sort_by_key(|&i| {
            let entame = matches!(&self.monde.tiles[i], Tile::Foret(f) if f.portage > 0);
            (!entame, self.monde.distance(i), i)
        });
        if let Some(&i) = c.first() {
            let d = self.monde.distance(i);
            let cout = (d * p.k_bois) as u32;
            let (x, y) = self.monde.xy(i);
            let mut reste = 0;
            if let Tile::Foret(f) = &mut self.monde.tiles[i] {
                f.portage += 1;
                if f.portage >= cout {
                    f.portage = 0;
                    f.coupe -= 1;
                    self.stocks.bois += 1;
                }
                reste = f.coupe;
            }
            return Some((
                Activite::TransportBois(i),
                format!(
                    "Bois : stock {} < seuil {} → transport depuis la forêt ({x},{y}), 1 unité = {d} tiles × {} j, reste {}",
                    self.stocks.bois,
                    self.seuil_bois(),
                    p.k_bois,
                    reste
                ),
            ));
        }
        // Sinon couper : d'abord la forêt dont la coupe est entamée, sinon la plus proche.
        let mut c = self.monde.indices(|t| matches!(t, Tile::Foret(f) if f.disponible >= 1));
        c.sort_by_key(|&i| {
            let entame = matches!(&self.monde.tiles[i], Tile::Foret(f) if f.travail > 0);
            (!entame, self.monde.distance(i), i)
        });
        let i = *c.first()?;
        let (x, y) = self.monde.xy(i);
        let mut jour_n = 0;
        let mut finie = false;
        if let Tile::Foret(f) = &mut self.monde.tiles[i] {
            f.travail += 1;
            jour_n = f.travail;
            if f.travail >= p.coupe_j_par_unite {
                f.travail = 0;
                f.disponible -= 1;
                f.coupe += 1;
                self.bilan.bois_brut += 1;
                finie = true;
            }
        }
        if finie {
            self.bois_a_taxer += 1;
            self.noter(format!("Forêt ({x},{y}) : 1 unité de bois ramassée"));
        }
        Some((
            Activite::Coupe(i),
            format!(
                "Bois : stock {} < seuil {} → petit bois en forêt ({x},{y}), jour {jour_n}/{}",
                self.stocks.bois,
                self.seuil_bois(),
                p.coupe_j_par_unite
            ),
        ))
    }
}

// ───────────────────────── Vue JSON pour l'interface ─────────────────────────

#[derive(Serialize)]
struct VueTile {
    x: usize,
    y: usize,
    #[serde(rename = "type")]
    genre: &'static str,
    distance: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<Phase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    travail: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    travail_requis: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grain: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disponible: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coupe: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    portage: Option<u32>,
}

#[derive(Serialize)]
struct VueDate {
    annee: u32,
    mois: u32,
    jour_du_mois: u32,
    jour_de_l_annee: u32,
    jour_semaine: u32,
    nom_mois: &'static str,
    nom_jour: &'static str,
}

#[derive(Serialize)]
struct VuePop {
    activite: &'static str,
    tile: usize,
    raison: String,
    stocks: Stocks,
    travail_vetement: u32,
    grain_a_taxer: i64,
    bois_a_taxer: i64,
    seuil_bois: i64,
    seuil_vetement: i64,
}

#[derive(Serialize)]
struct VueRegles {
    prelevement_pct: i64,
    prelevement_mois: u32,
    prelevement_jour: u32,
    labour_j: u32,
    semis_j: u32,
    recolte_j: u32,
    coupe_j_par_unite: u32,
    vetement_j: u32,
    k_grain: i64,
    k_bois: i64,
}

#[derive(Serialize)]
struct Vue<'a> {
    jours_ecoules: u32,
    date: VueDate,
    meteo_pm: i64,
    largeur: usize,
    hauteur: usize,
    tiles: Vec<VueTile>,
    pop: VuePop,
    bilan: &'a Bilan,
    historique: &'a [Bilan],
    serie: &'a [Point],
    journal: &'a [Evenement],
    regles: VueRegles,
}

impl Simulation {
    /// Date du dernier jour simulé (ou du jour 0 avant le premier tick).
    fn date(&self) -> VueDate {
        let j = self.jour.saturating_sub(1);
        let jda = j % JOURS_PAR_AN;
        let (mois, jdm) = mois_jour(jda);
        VueDate {
            annee: ANNEE_DEPART + j / JOURS_PAR_AN,
            mois,
            jour_du_mois: jdm,
            jour_de_l_annee: jda,
            jour_semaine: j % 7,
            nom_mois: NOMS_MOIS[mois as usize - 1],
            nom_jour: NOMS_JOURS[(j % 7) as usize],
        }
    }

    /// Instantané complet de l'état, pour l'interface (quantités en parts, voir `q`).
    pub fn etat_json(&self) -> String {
        let p = &self.params;
        let tiles = (0..self.monde.tiles.len())
            .map(|i| {
                let (x, y) = self.monde.xy(i);
                let mut v = VueTile {
                    x,
                    y,
                    genre: "plaine",
                    distance: self.monde.distance(i),
                    phase: None,
                    travail: None,
                    travail_requis: None,
                    grain: None,
                    disponible: None,
                    coupe: None,
                    portage: None,
                };
                match &self.monde.tiles[i] {
                    Tile::Plaine => {}
                    Tile::Ferme => v.genre = "ferme",
                    Tile::Champ(c) => {
                        v.genre = "champ";
                        v.phase = Some(c.phase);
                        v.travail = Some(c.travail);
                        v.travail_requis = Some(match c.phase {
                            Phase::Nu => p.labour_j,
                            Phase::Laboure => p.semis_j,
                            Phase::Seme => p.recolte_j,
                        });
                        v.grain = Some(c.grain);
                        v.portage = Some(c.portage);
                    }
                    Tile::Foret(f) => {
                        v.genre = "foret";
                        v.disponible = Some(f.disponible);
                        v.coupe = Some(f.coupe);
                        v.portage = Some(f.portage);
                        v.travail = Some(f.travail);
                        v.travail_requis = Some(p.coupe_j_par_unite);
                    }
                }
                v
            })
            .collect();
        let vue = Vue {
            jours_ecoules: self.jour,
            date: self.date(),
            meteo_pm: self.meteo_pm,
            largeur: self.monde.largeur,
            hauteur: self.monde.hauteur,
            tiles,
            pop: VuePop {
                activite: self.activite.code(),
                tile: self.activite.tile().unwrap_or(self.monde.ferme),
                raison: self.raison.clone(),
                stocks: self.stocks,
                travail_vetement: self.travail_vetement,
                grain_a_taxer: self.grain_a_taxer,
                bois_a_taxer: self.bois_a_taxer,
                seuil_bois: self.seuil_bois(),
                seuil_vetement: self.seuil_vetement(),
            },
            bilan: &self.bilan,
            historique: &self.historique,
            serie: &self.serie,
            journal: &self.journal[self.journal.len().saturating_sub(40)..],
            regles: VueRegles {
                prelevement_pct: p.prelevement_pct,
                prelevement_mois: p.prelevement_date.0,
                prelevement_jour: p.prelevement_date.1,
                labour_j: p.labour_j,
                semis_j: p.semis_j,
                recolte_j: p.recolte_j,
                coupe_j_par_unite: p.coupe_j_par_unite,
                vetement_j: p.vetement_j,
                k_grain: p.k_grain,
                k_bois: p.k_bois,
            },
        };
        serde_json::to_string(&vue).unwrap_or_default()
    }
}
