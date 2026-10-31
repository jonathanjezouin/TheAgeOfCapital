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
    demographie: BrutDemo,
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
    rayon_max_cout: i64,
    nb_rondes_appel_offres: usize,
    stock_initial_quartier_jours: f64,
    stock_initial_entree_jours: f64,
    stock_initial_sortie_jours: f64,
}

#[derive(Deserialize)]
struct BrutInfra {
    cout_tuile: i64,
    capacite_nominale: f64,
    capacite_max: f64,
    poids_trajet_pop: f64,
}

#[derive(Deserialize)]
struct BrutDemo {
    habitants_par_pop: i64,
    capacite_quartier_pops: u32,
    naissance_max: f64,
    seuil_satisfaction_naissance: f64,
    mortalite_base: f64,
    mortalite_max: f64,
    acceleration: f64,
}

#[derive(Deserialize)]
struct BrutRessource {
    nom: String,
    peremption_jour: f64,
}

#[derive(Deserialize)]
struct BrutBesoin {
    ressource: String,
    par_pop_jour: f64,
    poids: f64,
    categorie: String,
}

#[derive(Deserialize)]
struct BrutFlux {
    ressource: String,
    qte: f64,
}

#[derive(Deserialize)]
struct BrutBatiment {
    nom: String,
    asset: String,
    pops: u32,
    #[serde(default)]
    intrants: Vec<BrutFlux>,
    #[serde(default)]
    extrants: Vec<BrutFlux>,
}

// ---------- Catalogue résolu ----------

#[derive(Clone, Debug)]
pub struct ParamsSim {
    pub jours_par_an: i64,
    pub jours_stock_sortie: f64,
    pub jours_stock_entree: f64,
    pub jours_stock_quartier: f64,
    pub rayon_max_cout: i64,
    pub nb_rondes: usize,
    pub stock_initial_quartier_jours: f64,
    pub stock_initial_entree_jours: f64,
    pub stock_initial_sortie_jours: f64,
}

#[derive(Clone, Debug)]
pub struct ParamsInfra {
    pub cout_tuile: i64,
    pub capacite_nominale: Qte,
    pub capacite_max: Qte,
    pub poids_trajet_pop: Qte,
}

#[derive(Clone, Debug)]
pub struct ParamsDemo {
    pub habitants_par_pop: i64,
    pub capacite_quartier_pops: u32,
    pub naissance_max_ppm: i64,
    pub seuil_sat_naissance_ppm: i64,
    pub mortalite_base_ppm: i64,
    pub mortalite_max_ppm: i64,
    pub acceleration_ppm: i64,
}

#[derive(Clone, Debug)]
pub struct Ressource {
    pub nom: String,
    pub peremption_ppm: i64,
}

#[derive(Clone, Debug)]
pub struct Besoin {
    pub ressource: usize,
    pub par_pop: Qte,
    pub poids_ppm: i64,
    pub nourriture: bool,
}

#[derive(Clone, Debug)]
pub struct TypeBatiment {
    pub nom: String,
    pub asset: String,
    pub pops: u32,
    pub intrants: Vec<(usize, Qte)>,
    pub extrants: Vec<(usize, Qte)>,
}

#[derive(Clone, Debug)]
pub struct Catalogue {
    pub sim: ParamsSim,
    pub infra: ParamsInfra,
    pub demo: ParamsDemo,
    pub ressources: Vec<Ressource>,
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
            })
            .collect();
        let id = |nom: &str| -> Result<usize, String> {
            ressources
                .iter()
                .position(|r| r.nom == nom)
                .ok_or_else(|| format!("ressource inconnue : {nom}"))
        };

        let mut besoins = Vec::new();
        for x in &b.besoins {
            besoins.push(Besoin {
                ressource: id(&x.ressource)?,
                par_pop: en_milli(x.par_pop_jour),
                poids_ppm: en_ppm(x.poids),
                nourriture: match x.categorie.as_str() {
                    "nourriture" => true,
                    "autre" => false,
                    c => return Err(format!("catégorie de besoin inconnue : {c}")),
                },
            });
        }
        let somme: i64 = besoins.iter().map(|x| x.poids_ppm).sum();
        if (somme - PPM).abs() > 1_000 {
            return Err(format!(
                "la somme des poids de besoins vaut {} au lieu de 1",
                somme as f64 / PPM as f64
            ));
        }
        if !besoins.iter().any(|x| x.nourriture) {
            return Err("aucun besoin de catégorie nourriture".into());
        }

        let mut batiments = Vec::new();
        for x in &b.batiments {
            if x.pops == 0 {
                return Err(format!("bâtiment {} sans effectif", x.nom));
            }
            let conv = |v: &Vec<BrutFlux>| -> Result<Vec<(usize, Qte)>, String> {
                v.iter()
                    .map(|f| Ok((id(&f.ressource)?, en_milli(f.qte))))
                    .collect()
            };
            batiments.push(TypeBatiment {
                nom: x.nom.clone(),
                asset: x.asset.clone(),
                pops: x.pops,
                intrants: conv(&x.intrants)?,
                extrants: conv(&x.extrants)?,
            });
        }

        let cat = Catalogue {
            sim: ParamsSim {
                jours_par_an: b.simulation.jours_par_an,
                jours_stock_sortie: b.simulation.jours_stock_sortie,
                jours_stock_entree: b.simulation.jours_stock_entree,
                jours_stock_quartier: b.simulation.jours_stock_quartier,
                rayon_max_cout: b.simulation.rayon_max_cout,
                nb_rondes: b.simulation.nb_rondes_appel_offres,
                stock_initial_quartier_jours: b.simulation.stock_initial_quartier_jours,
                stock_initial_entree_jours: b.simulation.stock_initial_entree_jours,
                stock_initial_sortie_jours: b.simulation.stock_initial_sortie_jours,
            },
            infra: ParamsInfra {
                cout_tuile: b.infrastructure.cout_tuile,
                capacite_nominale: en_milli(b.infrastructure.capacite_nominale),
                capacite_max: en_milli(b.infrastructure.capacite_max),
                poids_trajet_pop: en_milli(b.infrastructure.poids_trajet_pop),
            },
            demo: ParamsDemo {
                habitants_par_pop: b.demographie.habitants_par_pop,
                capacite_quartier_pops: b.demographie.capacite_quartier_pops,
                naissance_max_ppm: en_ppm(b.demographie.naissance_max),
                seuil_sat_naissance_ppm: en_ppm(b.demographie.seuil_satisfaction_naissance),
                mortalite_base_ppm: en_ppm(b.demographie.mortalite_base),
                mortalite_max_ppm: en_ppm(b.demographie.mortalite_max),
                acceleration_ppm: en_ppm(b.demographie.acceleration),
            },
            ressources,
            besoins,
            batiments,
        };
        cat.valider()?;
        Ok(cat)
    }

    fn valider(&self) -> Result<(), String> {
        if self.demo.seuil_sat_naissance_ppm >= PPM {
            return Err("seuil_satisfaction_naissance doit être < 1".into());
        }
        if self.infra.capacite_max < self.infra.capacite_nominale {
            return Err("capacite_max < capacite_nominale".into());
        }
        for b in &self.besoins {
            if self.producteurs(b.ressource).is_empty() {
                return Err(format!(
                    "aucun bâtiment ne produit {}",
                    self.ressources[b.ressource].nom
                ));
            }
        }
        for t in &self.batiments {
            for (r, _) in &t.intrants {
                if self.producteurs(*r).is_empty() {
                    return Err(format!(
                        "{} : intrant {} sans producteur",
                        t.nom, self.ressources[*r].nom
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn res_id(&self, nom: &str) -> Option<usize> {
        self.ressources.iter().position(|r| r.nom == nom)
    }

    pub fn bat_id(&self, nom: &str) -> Option<usize> {
        self.batiments.iter().position(|b| b.nom == nom)
    }

    /// Types de bâtiments qui produisent la ressource.
    pub fn producteurs(&self, res: usize) -> Vec<usize> {
        (0..self.batiments.len())
            .filter(|&i| self.batiments[i].extrants.iter().any(|(r, _)| *r == res))
            .collect()
    }

    /// Consommation journalière d'une pop pour une ressource (0 si aucun besoin).
    pub fn besoin_pop(&self, res: usize) -> Qte {
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
