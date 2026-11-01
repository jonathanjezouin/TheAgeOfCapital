//! Pont WebAssembly du moteur `sim`.
//!
//! Interface C minimale (pas de `wasm-bindgen`) : une simulation globale, des fonctions qui écrivent
//! un instantané JSON dans un tampon interne que le JavaScript lit (`sim_sortie_ptr` + longueur).
//! Le moteur n'est pas modifié : seules des *vues* de son état sont sérialisées ici.

use serde::Serialize;
use sim::monde::{Cible, Contenu, Limitant};
use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};
use std::cell::RefCell;

thread_local! {
    static SIM: RefCell<Option<Simulation>> = const { RefCell::new(None) };
    static SORTIE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static FERTILITE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn f(ppm: i64) -> f64 {
    ppm as f64 / PPM as f64
}

fn unites(milli: i64) -> f64 {
    milli as f64 / 1000.0
}

fn ecrire<T: Serialize>(v: &T) -> u32 {
    let octets = serde_json::to_vec(v).unwrap_or_default();
    let n = octets.len() as u32;
    SORTIE.with(|s| *s.borrow_mut() = octets);
    n
}

fn avec<R>(defaut: R, g: impl FnOnce(&Simulation) -> R) -> R {
    SIM.with(|s| s.borrow().as_ref().map_or(defaut, g))
}

// ---------- Vues sérialisées ----------

#[derive(Serialize)]
struct Meta<'a> {
    largeur: usize,
    hauteur: usize,
    jours_par_an: i64,
    distance_max: i64,
    ressources: Vec<&'a str>,
    /// Ration normale par pop et par jour (unités), par ressource (0 si aucune).
    rations: Vec<f64>,
    habitats: Vec<HabitatMeta<'a>>,
    batiments: Vec<BatimentMeta<'a>>,
    habitants_par_pop: i64,
    seuil_croissance_microhab: i64,
}

#[derive(Serialize)]
struct HabitatMeta<'a> {
    nom: &'a str,
    capacite: u32,
    atelier: bool,
}

#[derive(Serialize)]
struct BatimentMeta<'a> {
    nom: &'a str,
    pops: u32,
    chantier: bool,
    fertile: bool,
    /// Ressources livrées en sortie finale.
    extrants: Vec<&'a str>,
    intrants: Vec<&'a str>,
}

#[derive(Serialize)]
struct TuileCarte {
    x: usize,
    y: usize,
    /// "chemin" | "hameau" | "bat"
    c: &'static str,
    /// Masque de demi-directions (chemin) ; indice du hameau ou du bâtiment sinon.
    m: u32,
    v: u8,
}

#[derive(Serialize)]
struct Carte {
    version: u64,
    tuiles: Vec<TuileCarte>,
}

#[derive(Serialize)]
struct ChantierVue {
    cible: &'static str,
    /// Type de terre visée (cible "terre") ou habitat (cible "hameau").
    type_nom: String,
    avancement: f64,
    equipe: u32,
    presents: u32,
    restant_pop_jours: f64,
}

#[derive(Serialize)]
struct LieuVue {
    type_nom: String,
    x: usize,
    y: usize,
    quartier: Option<usize>,
    occ: u32,
    postes: u32,
    prod: f64,
    limitant: String,
    fertilite: f64,
    entree: Vec<f64>,
    sortie: Vec<f64>,
    chantier: Option<ChantierVue>,
}

#[derive(Serialize)]
struct HameauVue {
    x: usize,
    y: usize,
    habitat: String,
    capacite: u32,
    pops: u32,
    vital: f64,
    confort: f64,
    couverture: Vec<f64>,
    stock: Vec<f64>,
    atelier: Option<usize>,
    vital_annuel: f64,
    confort_annuel: f64,
}

#[derive(Serialize)]
struct Etat<'a> {
    jour: u64,
    version: u64,
    nb_pops: usize,
    sans_poste: usize,
    chantiers: usize,
    hameaux: Vec<HameauVue>,
    lieux: Vec<LieuVue>,
    /// [id, hameau de résidence, lieu de travail (-1 si aucun), jauge de croissance en % du seuil (−100..100)]
    pops: Vec<(u32, usize, i64, f64)>,
    evenements: Vec<(u64, &'a str)>,
    nb_evenements: usize,
    produit: Vec<f64>,
    consomme: Vec<f64>,
    /// [année, pops, sans poste, naissances, décès, vital moyen, confort moyen]
    annuel: Vec<(u64, usize, usize, u32, u32, f64, f64)>,
}

fn version_carte(s: &Simulation) -> u64 {
    let mut v = s.monde.quartiers.len() as u64 * 1_000_003 + s.monde.batiments.len() as u64 * 10_007;
    for b in &s.monde.batiments {
        v = v.wrapping_mul(31).wrapping_add(b.type_id as u64 + b.chantier.is_some() as u64 * 977);
    }
    for t in &s.monde.tuiles {
        if let Contenu::Chemin { masque } = t.contenu {
            v = v.wrapping_mul(131).wrapping_add(masque as u64 + 1);
        }
    }
    for q in &s.monde.quartiers {
        v = v.wrapping_mul(37).wrapping_add(q.capacite_pops as u64);
    }
    v
}

fn limitant_texte(s: &Simulation, l: Limitant) -> String {
    match l {
        Limitant::Aucun => String::new(),
        Limitant::MainOeuvre => "main_oeuvre".into(),
        Limitant::StockPlein => "stock_plein".into(),
        Limitant::Intrant(r) => format!("intrant:{}", s.cat.ressources[r].nom),
    }
}

// ---------- Interface C ----------

/// Crée un village. Retourne 0 si tout va bien, 1 sinon (le message est écrit dans le tampon).
#[no_mangle]
pub extern "C" fn sim_creer(graine: u32, hameaux: u32, rayon_fertile: i32, rayon_sterile: i32, occupation_pct: u32) -> i32 {
    let p = ParamsVillage {
        graine: graine as u64,
        nb_hameaux: hameaux as usize,
        occupation_initiale_pct: occupation_pct,
        rayon_fertile: rayon_fertile as i64,
        rayon_sterile: rayon_sterile as i64,
        ..Default::default()
    };
    match generer(Catalogue::defaut(), &p) {
        Ok(s) => {
            let fert: Vec<u8> = s.monde.tuiles.iter().map(|t| (t.fertilite_ppm / 10_000).clamp(0, 255) as u8).collect();
            FERTILITE.with(|f| *f.borrow_mut() = fert);
            SIM.with(|x| *x.borrow_mut() = Some(s));
            0
        }
        Err(e) => {
            SORTIE.with(|s| *s.borrow_mut() = e.into_bytes());
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn sim_avancer(jours: u32) {
    SIM.with(|s| {
        if let Some(sim) = s.borrow_mut().as_mut() {
            sim.avancer(jours as u64);
        }
    });
}

/// Pointeur vers le tampon de sortie (valable jusqu'au prochain appel qui écrit).
#[no_mangle]
pub extern "C" fn sim_sortie_ptr() -> *const u8 {
    SORTIE.with(|s| s.borrow().as_ptr())
}

/// Longueur du dernier tampon écrit (utile pour lire un message d'erreur).
#[no_mangle]
pub extern "C" fn sim_sortie_len() -> u32 {
    SORTIE.with(|s| s.borrow().len() as u32)
}

#[no_mangle]
pub extern "C" fn sim_fertilite_ptr() -> *const u8 {
    FERTILITE.with(|f| f.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_fertilite_len() -> u32 {
    FERTILITE.with(|f| f.borrow().len() as u32)
}

/// Description statique : ressources, types de bâtiments, habitats, dimensions.
#[no_mangle]
pub extern "C" fn sim_meta_json() -> u32 {
    avec(0, |s| {
        let c = &s.cat;
        let noms = |v: &[(usize, i64)]| v.iter().map(|(r, _)| c.ressources[*r].nom.as_str()).collect::<Vec<_>>();
        ecrire(&Meta {
            largeur: s.monde.largeur,
            hauteur: s.monde.hauteur,
            jours_par_an: c.sim.jours_par_an,
            distance_max: c.infra.distance_max,
            ressources: c.ressources.iter().map(|r| r.nom.as_str()).collect(),
            rations: (0..c.ressources.len()).map(|r| unites(c.ration_pop(r))).collect(),
            habitats: c
                .habitats
                .iter()
                .map(|h| HabitatMeta { nom: &h.nom, capacite: h.capacite_pops, atelier: h.atelier.is_some() })
                .collect(),
            batiments: c
                .batiments
                .iter()
                .map(|b| BatimentMeta {
                    nom: &b.nom,
                    pops: b.pops,
                    chantier: b.chantier,
                    fertile: b.fertile,
                    extrants: noms(&b.extrants_finaux()),
                    intrants: noms(&b.intrants_externes()),
                })
                .collect(),
            habitants_par_pop: c.demo.habitants_par_pop,
            seuil_croissance_microhab: s.seuil_croissance(),
        })
    })
}

/// Tiles occupées (chemins, hameaux, bâtiments) : à relire quand `version` change.
#[no_mangle]
pub extern "C" fn sim_carte_json() -> u32 {
    avec(0, |s| {
        let mut tuiles = Vec::new();
        for (i, t) in s.monde.tuiles.iter().enumerate() {
            let (x, y) = s.monde.xy(i);
            let (c, m) = match t.contenu {
                Contenu::Vide => continue,
                Contenu::Chemin { masque } => ("chemin", masque as u32),
                Contenu::Quartier(q) => ("hameau", q as u32),
                Contenu::Batiment(b) => ("bat", b as u32),
            };
            tuiles.push(TuileCarte { x, y, c, m, v: t.variante });
        }
        ecrire(&Carte { version: version_carte(s), tuiles })
    })
}

/// État courant. `depuis` : nombre d'événements déjà lus (on ne renvoie que les nouveaux).
#[no_mangle]
pub extern "C" fn sim_etat_json(depuis: u32) -> u32 {
    avec(0, |s| {
        let c = &s.cat;
        let eff = s.effectifs();
        let occ = s.occupation();
        let hameaux = s
            .monde
            .quartiers
            .iter()
            .enumerate()
            .map(|(q, h)| {
                let (x, y) = s.monde.xy(h.tuile);
                HameauVue {
                    x,
                    y,
                    habitat: c.habitats[h.habitat_id].nom.clone(),
                    capacite: h.capacite_pops,
                    pops: eff[q],
                    vital: f(h.vital_jour_ppm),
                    confort: f(h.confort_jour_ppm),
                    couverture: h.sat_par_ressource_ppm.iter().map(|&p| f(p)).collect(),
                    stock: h.stock.iter().map(|&m| unites(m)).collect(),
                    atelier: h.atelier,
                    vital_annuel: f(h.vital_annuel_ppm),
                    confort_annuel: f(h.confort_annuel_ppm),
                }
            })
            .collect();
        let lieux = s
            .monde
            .batiments
            .iter()
            .enumerate()
            .map(|(b, bat)| {
                let t = &c.batiments[bat.type_id];
                let (x, y) = s.monde.xy(bat.tuile);
                let chantier = bat.chantier.as_ref().map(|ch| {
                    let (cible, type_nom) = match ch.cible {
                        Cible::Terre(t) => ("terre", c.batiments[t].nom.clone()),
                        Cible::Hameau(q) => ("hameau", c.habitats[s.monde.quartiers[q].habitat_id].nom.clone()),
                    };
                    ChantierVue {
                        cible,
                        type_nom,
                        avancement: 1.0 - ch.travail_restant.max(0) as f64 / ch.travail_total.max(1) as f64,
                        equipe: bat.equipe,
                        presents: bat.presents,
                        restant_pop_jours: unites(ch.travail_restant.max(0)),
                    }
                });
                LieuVue {
                    type_nom: t.nom.clone(),
                    x,
                    y,
                    quartier: bat.quartier,
                    occ: occ[b],
                    postes: s.postes_de(b),
                    prod: f(bat.prod_ppm),
                    limitant: limitant_texte(s, bat.limitant),
                    fertilite: f(s.monde.tuiles[bat.tuile].fertilite_ppm),
                    entree: bat.stock_entree.iter().map(|&m| unites(m)).collect(),
                    sortie: bat.stock_sortie.iter().map(|&m| unites(m)).collect(),
                    chantier,
                }
            })
            .collect();
        let seuil = s.seuil_croissance() as f64;
        let depuis = (depuis as usize).min(s.evenements.len());
        ecrire(&Etat {
            jour: s.jour,
            version: version_carte(s),
            nb_pops: s.pops.len(),
            sans_poste: s.sans_poste(),
            chantiers: s.chantiers_en_cours(),
            hameaux,
            lieux,
            pops: s
                .pops
                .iter()
                .map(|p| (p.id, p.residence, p.emploi.map_or(-1, |e| e as i64), p.croissance as f64 / seuil * 100.0))
                .collect(),
            evenements: s.evenements[depuis..].iter().map(|e| (e.jour, e.texte.as_str())).collect(),
            nb_evenements: s.evenements.len(),
            produit: s.stats.produit.iter().map(|&m| unites(m)).collect(),
            consomme: s.stats.consomme_pops.iter().map(|&m| unites(m)).collect(),
            annuel: s
                .historique
                .iter()
                .map(|h| (h.annee, h.pops, h.sans_poste, h.naissances, h.deces, f(h.vital_moy_ppm), f(h.confort_moy_ppm)))
                .collect(),
        })
    })
}
