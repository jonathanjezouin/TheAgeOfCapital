//! Chargement et validation des données de jeu (TOML). Tout est en entiers :
//! quantités en milli-unités, proportions en ppm.

use serde::Deserialize;

pub type Qte = i64;
/// Une unité de marchandise = 1000 milli-unités.
pub const U: Qte = 1_000;
/// 1,0 = 1 000 000 ppm.
pub const PPM: i64 = 1_000_000;

pub fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    ((a as i128 * b as i128) / c as i128) as i64
}

fn en_milli(x: f64) -> Qte {
    (x * U as f64).round() as Qte
}

fn en_ppm(x: f64) -> i64 {
    (x * PPM as f64).round() as i64
}

// ---------- Format brut (fichier TOML) ----------

#[derive(Deserialize)]
struct Brut {
    simulation: BrutSim,
    infrastructure: BrutInfra,
    habitat: BrutHabitat,
    habitats: Vec<BrutTypeHabitat>,
    extension: BrutExtension,
    demographie: BrutDemo,
    mobilite: BrutMobilite,
    vital: BrutVital,
    confort: Vec<BrutBesoin>,
    ressources: Vec<BrutRessource>,
    besoins: Vec<BrutBesoin>,
    batiments: Vec<BrutBatiment>,
}

#[derive(Deserialize)]
struct BrutSim {
    jours_par_an: i64,
    jours_stock_sortie: f64,
    jours_stock_entree: f64,
    jours_stock_quartier: f64,
    nb_rondes_appel_offres: usize,
    stock_initial_quartier_jours: f64,
    stock_initial_entree_jours: f64,
    stock_initial_sortie_jours: f64,
}

#[derive(Deserialize)]
struct BrutInfra {
    distance_max: i64,
    capacite_nominale: f64,
    capacite_max: f64,
    poids_trajet_pop: f64,
}

#[derive(Deserialize)]
struct BrutHabitat {
    densite_max_pops: u32,
}

#[derive(Deserialize)]
struct BrutCout {
    travail_pop_jours: f64,
    equipe: u32,
    bois: f64,
}

#[derive(Deserialize)]
struct BrutTypeHabitat {
    nom: String,
    asset: String,
    capacite_pops: u32,
    atelier: Option<String>,
    cout_creation: BrutCout,
}

#[derive(Deserialize)]
struct BrutExtension {
    periode_jours: u64,
    chantiers_max: usize,
    materiau: String,
    couverture_donneur_jours: f64,
}

#[derive(Deserialize)]
struct BrutDemo {
    habitants_par_pop: i64,
    croissance_satisfaite: f64,
    croissance_insatisfaite: f64,
    mortalite_famine: f64,
    acceleration: f64,
}

#[derive(Deserialize)]
struct BrutMobilite {
    periode_jours: u64,
    seuil_tension: f64,
    deplacements_max_par_ressource: usize,
}

#[derive(Deserialize)]
struct BrutVital {
    nourriture: f64,
    chauffage_ressource: String,
    chauffage: f64,
}

#[derive(Deserialize)]
struct BrutRessource {
    nom: String,
    peremption_jour: f64,
    #[serde(default)]
    valeur_nutritive: f64,
}

#[derive(Deserialize)]
struct BrutBesoin {
    ressource: String,
    par_pop_jour: f64,
}

#[derive(Deserialize)]
struct BrutFlux {
    ressource: String,
    qte: f64,
}

#[derive(Deserialize)]
struct BrutEtape {
    #[serde(default)]
    intrants: Vec<BrutFlux>,
    #[serde(default)]
    extrants: Vec<BrutFlux>,
}

#[derive(Deserialize)]
struct BrutBatiment {
    nom: String,
    asset: String,
    pops: u32,
    #[serde(default)]
    chantier: bool,
    #[serde(default)]
    fertile: bool,
    cout_creation: Option<BrutCout>,
    etapes: Vec<BrutEtape>,
}

// ---------- Catalogue résolu ----------

#[derive(Clone, Debug)]
pub struct ParamsSim {
    pub jours_par_an: i64,
    pub jours_stock_sortie: f64,
    pub jours_stock_entree: f64,
    pub jours_stock_quartier: f64,
    pub nb_rondes: usize,
    pub stock_initial_quartier_jours: f64,
    pub stock_initial_entree_jours: f64,
    pub stock_initial_sortie_jours: f64,
}

#[derive(Clone, Debug)]
pub struct ParamsInfra {
    /// Nombre maximal de tiles chemin entre deux lieux (travail, approvisionnement).
    pub distance_max: i64,
    pub capacite_nominale: Qte,
    pub capacite_max: Qte,
    pub poids_trajet_pop: Qte,
}

#[derive(Clone, Debug)]
pub struct ParamsHabitat {
    /// Plafond de densité de toute tile d'habitat.
    pub densite_max_pops: u32,
}

/// Coût de création : travail d'une équipe (pop-jours, milli) et matériau (milli-unités au total).
#[derive(Clone, Debug)]
pub struct CoutCreation {
    pub travail: Qte,
    pub equipe: u32,
    pub materiau: Qte,
}

impl CoutCreation {
    /// Matériau consommé par pop-jour de travail (milli-unités).
    pub fn materiau_par_pop_jour(&self) -> Qte {
        if self.travail > 0 {
            mul_div(self.materiau, U, self.travail)
        } else {
            0
        }
    }
}

/// Type de tile d'habitat (hameau, plus tard quartier ouvrier…).
#[derive(Clone, Debug)]
pub struct TypeHabitat {
    pub nom: String,
    pub asset: String,
    pub capacite_pops: u32,
    /// Type de bâtiment (1 poste) embarqué sur la même tile : travail sur place.
    pub atelier: Option<usize>,
    pub cout_creation: CoutCreation,
}

#[derive(Clone, Debug)]
pub struct ParamsExtension {
    pub periode_jours: u64,
    pub chantiers_max: usize,
    pub materiau: usize,
    pub couverture_donneur_jours: f64,
}

#[derive(Clone, Debug)]
pub struct ParamsDemo {
    pub habitants_par_pop: i64,
    pub croissance_satisfaite_ppm: i64,
    pub croissance_insatisfaite_ppm: i64,
    pub mortalite_famine_ppm: i64,
    pub acceleration_ppm: i64,
}

#[derive(Clone, Debug)]
pub struct ParamsMobilite {
    pub periode_jours: u64,
    pub seuil_tension_ppm: i64,
    pub max_par_ressource: usize,
}

/// Besoins vitaux par pop et par jour.
#[derive(Clone, Debug)]
pub struct Vital {
    /// Unités nutritives (milli) : grain, viande et fruit sont interchangeables.
    pub nourriture: Qte,
    pub chauffage_ressource: usize,
    pub chauffage: Qte,
}

#[derive(Clone, Debug)]
pub struct Ressource {
    pub nom: String,
    pub peremption_ppm: i64,
    /// Unités nutritives par unité de marchandise (×1000).
    pub nutritive_milli: i64,
}

/// Quantité par pop et par jour (ration normale ou minimum de confort).
#[derive(Clone, Debug)]
pub struct Besoin {
    pub ressource: usize,
    pub par_pop: Qte,
}

#[derive(Clone, Debug)]
pub struct Etape {
    pub intrants: Vec<(usize, Qte)>,
    pub extrants: Vec<(usize, Qte)>,
}

#[derive(Clone, Debug)]
pub struct TypeBatiment {
    pub nom: String,
    pub asset: String,
    /// Effectif plein (en pops).
    pub pops: u32,
    /// Site de chantier (défrichement ou construction) ; l'équipe est propre à chaque chantier.
    pub chantier: bool,
    /// La production est proportionnelle à la fertilité de la tile.
    pub fertile: bool,
    pub cout_creation: Option<CoutCreation>,
    pub etapes: Vec<Etape>,
}

impl TypeBatiment {
    /// Intrants à fournir de l'extérieur (hors ce qu'une étape précédente produit).
    pub fn intrants_externes(&self) -> Vec<(usize, Qte)> {
        let mut v: Vec<(usize, Qte)> = Vec::new();
        for (i, e) in self.etapes.iter().enumerate() {
            for &(r, q) in &e.intrants {
                if self.etapes[..i]
                    .iter()
                    .any(|p| p.extrants.iter().any(|(x, _)| *x == r))
                {
                    continue;
                }
                match v.iter_mut().find(|(x, _)| *x == r) {
                    Some(t) => t.1 += q,
                    None => v.push((r, q)),
                }
            }
        }
        v
    }

    /// Extrants livrés au stock de sortie (hors ce qu'une étape suivante consomme).
    pub fn extrants_finaux(&self) -> Vec<(usize, Qte)> {
        let mut v: Vec<(usize, Qte)> = Vec::new();
        for (i, e) in self.etapes.iter().enumerate() {
            for &(r, q) in &e.extrants {
                if self.consomme_ensuite(i, r) {
                    continue;
                }
                match v.iter_mut().find(|(x, _)| *x == r) {
                    Some(t) => t.1 += q,
                    None => v.push((r, q)),
                }
            }
        }
        v
    }

    /// Le bâtiment livre-t-il `res` en sortie finale ? (sans allocation)
    pub fn livre(&self, res: usize) -> bool {
        self.etapes.iter().enumerate().any(|(i, e)| {
            e.extrants.iter().any(|(r, _)| *r == res) && !self.consomme_ensuite(i, res)
        })
    }

    /// Une étape postérieure à `etape` consomme-t-elle `res` ?
    pub fn consomme_ensuite(&self, etape: usize, res: usize) -> bool {
        self.etapes[etape + 1..]
            .iter()
            .any(|e| e.intrants.iter().any(|(x, _)| *x == res))
    }
}

#[derive(Clone, Debug)]
pub struct Catalogue {
    pub sim: ParamsSim,
    pub infra: ParamsInfra,
    pub habitat: ParamsHabitat,
    pub habitats: Vec<TypeHabitat>,
    pub extension: ParamsExtension,
    pub demo: ParamsDemo,
    pub mobilite: ParamsMobilite,
    pub vital: Vital,
    pub confort: Vec<Besoin>,
    pub ressources: Vec<Ressource>,
    /// Ration normale tirée chaque jour sur le stock du hameau.
    pub besoins: Vec<Besoin>,
    pub batiments: Vec<TypeBatiment>,
}

pub const DONNEES_POC: &str = include_str!("../donnees/poc.toml");

impl Catalogue {
    pub fn defaut() -> Self {
        Self::depuis_toml(DONNEES_POC).expect("données du POC invalides")
    }

    pub fn depuis_toml(texte: &str) -> Result<Self, String> {
        let b: Brut = toml::from_str(texte).map_err(|e| format!("TOML invalide : {e}"))?;
        let ressources: Vec<Ressource> = b
            .ressources
            .iter()
            .map(|r| Ressource {
                nom: r.nom.clone(),
                peremption_ppm: en_ppm(r.peremption_jour),
                nutritive_milli: en_milli(r.valeur_nutritive),
            })
            .collect();
        let id = |nom: &str| -> Result<usize, String> {
            ressources
                .iter()
                .position(|r| r.nom == nom)
                .ok_or_else(|| format!("ressource inconnue : {nom}"))
        };
        let cout_de = |c: &BrutCout| CoutCreation {
            travail: en_milli(c.travail_pop_jours),
            equipe: c.equipe,
            materiau: en_milli(c.bois),
        };
        let besoins_de = |v: &Vec<BrutBesoin>| -> Result<Vec<Besoin>, String> {
            v.iter()
                .map(|x| {
                    Ok(Besoin {
                        ressource: id(&x.ressource)?,
                        par_pop: en_milli(x.par_pop_jour),
                    })
                })
                .collect()
        };

        let mut batiments = Vec::new();
        for x in &b.batiments {
            if x.pops == 0 {
                return Err(format!("bâtiment {} sans effectif", x.nom));
            }
            if x.etapes.is_empty() && !x.chantier {
                return Err(format!("bâtiment {} sans étape de production", x.nom));
            }
            let conv = |v: &Vec<BrutFlux>| -> Result<Vec<(usize, Qte)>, String> {
                v.iter()
                    .map(|f| Ok((id(&f.ressource)?, en_milli(f.qte))))
                    .collect()
            };
            let mut etapes = Vec::new();
            for e in &x.etapes {
                etapes.push(Etape {
                    intrants: conv(&e.intrants)?,
                    extrants: conv(&e.extrants)?,
                });
            }
            batiments.push(TypeBatiment {
                nom: x.nom.clone(),
                asset: x.asset.clone(),
                pops: x.pops,
                chantier: x.chantier,
                fertile: x.fertile,
                cout_creation: x.cout_creation.as_ref().map(cout_de),
                etapes,
            });
        }
        let mut habitats = Vec::new();
        for h in &b.habitats {
            let atelier = match &h.atelier {
                None => None,
                Some(nom) => Some(
                    batiments
                        .iter()
                        .position(|t| &t.nom == nom && !t.chantier)
                        .ok_or_else(|| format!("{} : atelier inconnu : {nom}", h.nom))?,
                ),
            };
            habitats.push(TypeHabitat {
                nom: h.nom.clone(),
                asset: h.asset.clone(),
                capacite_pops: h.capacite_pops,
                atelier,
                cout_creation: cout_de(&h.cout_creation),
            });
        }

        let cat = Catalogue {
            sim: ParamsSim {
                jours_par_an: b.simulation.jours_par_an,
                jours_stock_sortie: b.simulation.jours_stock_sortie,
                jours_stock_entree: b.simulation.jours_stock_entree,
                jours_stock_quartier: b.simulation.jours_stock_quartier,
                nb_rondes: b.simulation.nb_rondes_appel_offres,
                stock_initial_quartier_jours: b.simulation.stock_initial_quartier_jours,
                stock_initial_entree_jours: b.simulation.stock_initial_entree_jours,
                stock_initial_sortie_jours: b.simulation.stock_initial_sortie_jours,
            },
            infra: ParamsInfra {
                distance_max: b.infrastructure.distance_max,
                capacite_nominale: en_milli(b.infrastructure.capacite_nominale),
                capacite_max: en_milli(b.infrastructure.capacite_max),
                poids_trajet_pop: en_milli(b.infrastructure.poids_trajet_pop),
            },
            habitat: ParamsHabitat {
                densite_max_pops: b.habitat.densite_max_pops,
            },
            extension: ParamsExtension {
                periode_jours: b.extension.periode_jours,
                chantiers_max: b.extension.chantiers_max,
                materiau: id(&b.extension.materiau)?,
                couverture_donneur_jours: b.extension.couverture_donneur_jours,
            },
            habitats,
            demo: ParamsDemo {
                habitants_par_pop: b.demographie.habitants_par_pop,
                croissance_satisfaite_ppm: en_ppm(b.demographie.croissance_satisfaite),
                croissance_insatisfaite_ppm: en_ppm(b.demographie.croissance_insatisfaite),
                mortalite_famine_ppm: en_ppm(b.demographie.mortalite_famine),
                acceleration_ppm: en_ppm(b.demographie.acceleration),
            },
            mobilite: ParamsMobilite {
                periode_jours: b.mobilite.periode_jours,
                seuil_tension_ppm: en_ppm(b.mobilite.seuil_tension),
                max_par_ressource: b.mobilite.deplacements_max_par_ressource,
            },
            vital: Vital {
                nourriture: en_milli(b.vital.nourriture),
                chauffage_ressource: id(&b.vital.chauffage_ressource)?,
                chauffage: en_milli(b.vital.chauffage),
            },
            confort: besoins_de(&b.confort)?,
            besoins: besoins_de(&b.besoins)?,
            ressources,
            batiments,
        };
        cat.valider()?;
        Ok(cat)
    }

    fn valider(&self) -> Result<(), String> {
        if self.infra.capacite_max < self.infra.capacite_nominale {
            return Err("capacite_max < capacite_nominale".into());
        }
        if self.habitats.is_empty() {
            return Err("aucun type d'habitat".into());
        }
        for h in &self.habitats {
            if h.capacite_pops == 0 || h.capacite_pops > self.habitat.densite_max_pops {
                return Err(format!(
                    "{} : capacité hors de 1..={}",
                    h.nom, self.habitat.densite_max_pops
                ));
            }
        }
        if self.batiments.iter().filter(|t| t.chantier).count() != 1 {
            return Err("il faut exactement un type de bâtiment « chantier »".into());
        }
        if self.infra.distance_max < 1 {
            return Err("distance_max < 1".into());
        }
        if self.mobilite.periode_jours == 0 {
            return Err("periode_jours de mobilité nulle".into());
        }
        if !self.ressources.iter().any(|r| r.nutritive_milli > 0) {
            return Err("aucune ressource nutritive".into());
        }
        for b in self.besoins.iter().chain(&self.confort) {
            if self.producteurs(b.ressource).is_empty() {
                return Err(format!(
                    "aucun bâtiment ne produit {}",
                    self.ressources[b.ressource].nom
                ));
            }
        }
        for r in 0..self.ressources.len() {
            if self.ressources[r].nutritive_milli > 0 && self.producteurs(r).is_empty() {
                return Err(format!(
                    "aucun bâtiment ne produit {}",
                    self.ressources[r].nom
                ));
            }
        }
        if self.producteurs(self.vital.chauffage_ressource).is_empty() {
            return Err("aucun producteur de la ressource de chauffage".into());
        }
        for t in self.batiments.iter().filter(|t| !t.chantier) {
            for (r, _) in t.intrants_externes() {
                if self.producteurs(r).is_empty() {
                    return Err(format!(
                        "{} : intrant {} sans producteur",
                        t.nom, self.ressources[r].nom
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn res_id(&self, nom: &str) -> Option<usize> {
        self.ressources.iter().position(|r| r.nom == nom)
    }

    pub fn hab_id(&self, nom: &str) -> Option<usize> {
        self.habitats.iter().position(|h| h.nom == nom)
    }

    /// Type de bâtiment « chantier ».
    pub fn type_chantier(&self) -> usize {
        self.batiments
            .iter()
            .position(|t| t.chantier)
            .expect("type chantier")
    }

    pub fn bat_id(&self, nom: &str) -> Option<usize> {
        self.batiments.iter().position(|b| b.nom == nom)
    }

    /// Types de bâtiments qui livrent la ressource en sortie finale.
    pub fn producteurs(&self, res: usize) -> Vec<usize> {
        (0..self.batiments.len())
            .filter(|&i| self.batiments[i].livre(res))
            .collect()
    }

    /// Ration journalière d'une pop pour une ressource (0 si aucune).
    pub fn ration_pop(&self, res: usize) -> Qte {
        self.besoins
            .iter()
            .find(|b| b.ressource == res)
            .map_or(0, |b| b.par_pop)
    }

    /// Quantité × nombre de jours (arrondi).
    pub fn jours_x(qte: Qte, jours: f64) -> Qte {
        (qte as f64 * jours).round() as Qte
    }
}
