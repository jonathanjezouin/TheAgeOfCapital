//! Pont WebAssembly du moteur `sim`.
//!
//! Interface C minimale (pas de `wasm-bindgen`) : une simulation globale, des fonctions qui écrivent
//! un instantané JSON dans un tampon interne que le JavaScript lit (`sim_sortie_ptr` + longueur).
//! Le moteur n'est pas modifié : seules des *vues* de son état sont sérialisées ici.

use serde::Serialize;
use sim::monde::{Cible, Contenu, Limitant};
use sim::simulation::{taux_croissance_ppm, Classe, Motif};
use sim::{generer, Catalogue, ParamsVillage, Simulation, PPM};
use std::cell::RefCell;

thread_local! {
    static SIM: RefCell<Option<Simulation>> = const { RefCell::new(None) };
    static SORTIE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static FERTILITE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// Deux octets par tile : terrain, puis gabarit d'eau (bits 0-1) et sens de l'aval (bits 2-4, 0 = aucun, sinon 1 + direction / 2).
    /// Aptitude de chaque tile pour un type de terre (% de 1,0, 0 à 255), à la demande.
    static APTITUDE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    /// Deux octets par tile : années de vacance (0 : aucune), puis années restantes avant la forêt (255 : ne repousse pas).
    static VACANCE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static TERRAIN: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
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
    /// Multiplicateur du travail des chantiers par terrain.
    cout_terrain: Vec<f64>,
    /// Conversion par terrain (défrichement, drainage) : pop-jours par tile et bois récupéré (0 : aucune).
    conversion_travail: Vec<f64>,
    conversion_bois: Vec<f64>,
    /// Terrain obtenu par la conversion (index), -1 si aucune.
    conversion_vers: Vec<i32>,
    /// Travail d'un pont ou d'un gué par gabarit d'eau (aucune, ruisseau, rivière, fleuve), en pop-jours.
    pont: Vec<f64>,
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
    /// Rendement et travail requis par terrain (plaine, plaine riche, rocailleuse, marécageuse, forêt), en multiples de 1,0.
    aptitude: Vec<f64>,
    travail_terrain: Vec<f64>,
    /// La terre garde le terrain de ses tiles (pas de conversion).
    conserve_terrain: bool,
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
    /// Masque de demi-directions orthogonales N=1, E=2, S=4, O=8 (chemin, ancien format du rendu) ;
    /// indice du hameau ou du bâtiment sinon.
    m: u32,
    /// Demi-arêtes dans les 8 directions (bits E, SE, S, SO, O, NO, N, NE) : le réseau est une
    /// couche d'arêtes posée sur les tiles (docs/02 §9). Présent sur toute tile desservie.
    a: u32,
    v: u8,
}

/// Demi-arêtes (8 bits : E, SE, S, SO, O, NO, N, NE) vers l'ancien masque orthogonal N, E, S, O,
/// tant que le rendu ne dessine pas encore les diagonales.
fn masque_orthogonal(aretes: u8) -> u32 {
    let mut m = 0;
    if aretes & (1 << 6) != 0 {
        m |= 1; // N
    }
    if aretes & (1 << 0) != 0 {
        m |= 2; // E
    }
    if aretes & (1 << 2) != 0 {
        m |= 4; // S
    }
    if aretes & (1 << 4) != 0 {
        m |= 8; // O
    }
    m
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
    /// Travail requis à plein régime, en pops (équipe pour un chantier, jours par tile pour une terre).
    postes: f64,
    /// Travail reçu en moyenne (30 jours), en pops par jour (terre, atelier domestique).
    recu: f64,
    /// Travail reçu aujourd'hui, en pops.
    recu_jour: f64,
    /// Travail encore réclamé en fin de journée (pop-jours) : ce que personne n'est venu faire.
    demande: f64,
    /// Tiles occupées (terre exploitée sur N tiles) ; la première est la tile nœud.
    emprise: Vec<(usize, usize)>,
    prod: f64,
    limitant: String,
    fertilite: f64,
    entree: Vec<f64>,
    sortie: Vec<f64>,
    chantier: Option<ChantierVue>,
    /// Terre abandonnée : part du délai de friche écoulée (0 à 1) ; `None` si elle est en service.
    abandon: Option<f64>,
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
    /// Hameau abandonné : part du délai de ruine écoulée (0 à 1) ; `None` s'il est occupé.
    abandon: Option<f64>,
    detruit: bool,
}

#[derive(Serialize)]
struct PopVue {
    id: u32,
    residence: usize,
    /// Lieu de travail d'aujourd'hui (-1 : chez elle).
    emploi: i64,
    /// Jauge de croissance en % du seuil (−100..100).
    jauge: f64,
    /// Travail fourni aujourd'hui (pop-jours).
    travail: f64,
    /// "chantier" | "besoin" | "repos" | "aucun"
    motif: &'static str,
    /// Pour "besoin" : "vital" | "confort" | "intrant" | "entretien".
    classe: &'static str,
    /// Pour "besoin" : ressource visée (-1 sinon).
    ressource: i64,
    vital: f64,
    confort: f64,
}

fn motif_texte(m: Motif) -> (&'static str, &'static str, i64) {
    match m {
        Motif::Aucun => ("aucun", "", -1),
        Motif::Chantier => ("chantier", "", -1),
        Motif::Repos => ("repos", "", -1),
        Motif::Besoin { ressource, classe, .. } => (
            "besoin",
            match classe {
                Classe::Vital => "vital",
                Classe::Confort => "confort",
                Classe::Intrant => "intrant",
                Classe::Entretien => "entretien",
            },
            ressource as i64,
        ),
    }
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
    pops: Vec<PopVue>,
    /// Le décideur du POC (docs/07 §15) : son grenier, par ressource, en unités.
    seigneur: Option<SeigneurVue>,
    nb_serfs: usize,
    evenements: Vec<(u64, &'a str)>,
    nb_evenements: usize,
    produit: Vec<f64>,
    consomme: Vec<f64>,
    /// [année, pops, sans poste, naissances, décès, vital moyen, confort moyen]
    annuel: Vec<(u64, usize, usize, u32, u32, f64, f64)>,
}

#[derive(Serialize)]
struct SeigneurVue {
    nom: String,
    poste: String,
    grenier: Vec<f64>,
    /// Chantiers dont il est le maître d'ouvrage.
    chantiers: usize,
    /// Part de la production prélevée en nature (champart).
    champart: f64,
}

fn version_carte(s: &Simulation) -> u64 {
    let mut v = s.monde.quartiers.len() as u64 * 1_000_003 + s.monde.batiments.len() as u64 * 10_007;
    for b in &s.monde.batiments {
        v = v.wrapping_mul(31).wrapping_add(b.type_id as u64 + b.chantier.is_some() as u64 * 977 + b.abandon_mois.is_some() as u64 * 7919 + b.emprise.len() as u64 * 131);
    }
    for t in &s.monde.tuiles {
        if t.aretes != 0 {
            v = v.wrapping_mul(131).wrapping_add(t.aretes as u64 + 1);
        }
    }
    for q in &s.monde.quartiers {
        v = v.wrapping_mul(37).wrapping_add(q.capacite_pops as u64 + q.abandon_mois.is_some() as u64 * 4001 + q.detruit as u64 * 8009);
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
pub extern "C" fn sim_creer(graine: u32, hameaux: u32, occupation_pct: u32) -> i32 {
    let p = ParamsVillage {
        graine: graine as u64,
        nb_hameaux: hameaux as usize,
        occupation_initiale_pct: occupation_pct,
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

/// Calcule l'aptitude de chaque tile pour un type de terre (fertilité locale × aptitude du terrain, en % de 1,0)
/// et renvoie le nombre d'octets à lire à `sim_aptitude_ptr`. 0 si le type n'existe pas.
#[no_mangle]
pub extern "C" fn sim_aptitude(type_id: u32) -> u32 {
    avec(0, |s| {
        let t = type_id as usize;
        if t >= s.cat.batiments.len() {
            return 0;
        }
        let v: Vec<u8> = s.monde.tuiles.iter().map(|x| (s.cat.rendement_tile(t, x) / 10_000).clamp(0, 255) as u8).collect();
        let n = v.len() as u32;
        APTITUDE.with(|a| *a.borrow_mut() = v);
        n
    })
}

#[no_mangle]
pub extern "C" fn sim_aptitude_ptr() -> *const u8 {
    APTITUDE.with(|f| f.borrow().as_ptr())
}

/// Relit le terrain de chaque tile (il change quand une forêt est défrichée ou un marais drainé) et renvoie le
/// nombre d'octets à lire à `sim_terrain_ptr`.
#[no_mangle]
pub extern "C" fn sim_terrain() -> u32 {
    avec(0, |s| {
        let terrain: Vec<u8> = s
            .monde
            .tuiles
            .iter()
            .flat_map(|t| {
                let aval = if t.aval < 8 { 1 + t.aval / 2 } else { 0 };
                [t.terrain as u8, (t.eau as u8) | (aval << 2)]
            })
            .collect();
        let n = terrain.len() as u32;
        TERRAIN.with(|f| *f.borrow_mut() = terrain);
        n
    })
}

/// Relit la vacance des tiles (friche) et renvoie le nombre d'octets à lire à `sim_vacance_ptr`.
#[no_mangle]
pub extern "C" fn sim_vacance() -> u32 {
    avec(0, |s| {
        let v: Vec<u8> = (0..s.monde.tuiles.len())
            .flat_map(|i| {
                let t = &s.monde.tuiles[i];
                if t.vacant_mois == 0 {
                    return [0u8, 0u8];
                }
                let ans = (t.vacant_mois / 12).min(254) as u8;
                let reste = s.repousse_restante_mois(i).map_or(255u8, |m| ((m + 11) / 12).min(254) as u8);
                [ans.max(1), reste]
            })
            .collect();
        let n = v.len() as u32;
        VACANCE.with(|f| *f.borrow_mut() = v);
        n
    })
}

#[no_mangle]
pub extern "C" fn sim_vacance_ptr() -> *const u8 {
    VACANCE.with(|f| f.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_terrain_ptr() -> *const u8 {
    TERRAIN.with(|f| f.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn sim_terrain_len() -> u32 {
    TERRAIN.with(|f| f.borrow().len() as u32)
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
                    conserve_terrain: b.conserve_terrain,
                    aptitude: b.aptitude_ppm.iter().map(|&v| v as f64 / 1e6).collect(),
                    travail_terrain: b.travail_terrain_ppm.iter().map(|&v| v as f64 / 1e6).collect(),
                    extrants: noms(&b.extrants_finaux()),
                    intrants: noms(&b.intrants_externes()),
                })
                .collect(),
            habitants_par_pop: c.demo.habitants_par_pop,
            cout_terrain: c.cout_terrain_ppm.iter().map(|&v| v as f64 / 1e6).collect(),
            conversion_travail: c.conversion.iter().map(|x| x.map_or(0.0, |c| c.travail as f64 / 1000.0)).collect(),
            conversion_vers: c.conversion.iter().map(|x| x.map_or(-1, |c| c.vers as i32)).collect(),
            conversion_bois: c.conversion.iter().map(|x| x.map_or(0.0, |c| c.bois as f64 / 1000.0)).collect(),
            pont: c.pont_milli.iter().map(|&v| v as f64 / 1000.0).collect(),
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
                Contenu::Vide if t.aretes == 0 => continue,
                Contenu::Vide => ("chemin", masque_orthogonal(t.aretes)),
                Contenu::Quartier(q) => ("hameau", q as u32),
                Contenu::Batiment(b) => ("bat", b as u32),
            };
            tuiles.push(TuileCarte { x, y, c, m, a: t.aretes as u32, v: t.variante });
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
                    stock: (0..c.ressources.len()).map(|r| unites(s.stock_hameau(q, r))).collect(),
                    atelier: h.atelier,
                    abandon: h.abandon_mois.map(|m| (m as f64 / s.cat.abandon.hameau_ruine_mois as f64).min(1.0)),
                    detruit: h.detruit,
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
                        Cible::Maison(q) => ("maison", c.habitats[s.monde.quartiers[q].habitat_id].nom.clone()),
                        Cible::Extension(terre) => ("extension", c.batiments[s.monde.batiments[terre].type_id].nom.clone()),
                        Cible::RemiseHameau(q) => ("remise_hameau", c.habitats[s.monde.quartiers[q].habitat_id].nom.clone()),
                        Cible::RemiseTerre(t) => ("remise_terre", c.batiments[s.monde.batiments[t].type_id].nom.clone()),
                        Cible::Route(_, t) => ("route", c.batiments[s.monde.batiments[t].type_id].nom.clone()),
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
                    postes: if bat.chantier.is_some() {
                        bat.equipe as f64
                    } else {
                        c.besoin_mo_terre_milli(bat.type_id, &bat.emprise, &s.monde.tuiles) as f64 / 1000.0
                    },
                    recu: match bat.quartier {
                        Some(q) if t.domestique => s.libre_milli.get(q).map_or(0.0, |&m| m as f64 / 1000.0),
                        _ if t.par_tile => bat.mo_lisse_milli as f64 / 1000.0,
                        _ => bat.mo_milli as f64 / 1000.0,
                    },
                    recu_jour: bat.mo_milli as f64 / 1000.0,
                    demande: bat.demande_milli as f64 / 1000.0,
                    emprise: bat.emprise.iter().map(|&i| s.monde.xy(i)).collect(),
                    prod: f(bat.prod_ppm),
                    limitant: limitant_texte(s, bat.limitant),
                    // Rendement moyen de l'emprise : fertilité locale × aptitude du terrain pour ce type de terre.
                    fertilite: if bat.emprise.is_empty() {
                        f(c.rendement_tile(bat.type_id, &s.monde.tuiles[bat.tuile]))
                    } else {
                        f(bat.emprise.iter().map(|&i| c.rendement_tile(bat.type_id, &s.monde.tuiles[i])).sum::<i64>() / bat.emprise.len() as i64)
                    },
                    entree: bat.stock_entree.iter().map(|&m| unites(m)).collect(),
                    sortie: bat.stock_sortie.iter().map(|&m| unites(m)).collect(),
                    chantier,
                    abandon: bat.abandon_mois.map(|m| (m as f64 / s.cat.abandon.terre_friche_mois as f64).min(1.0)),
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
            seigneur: s.monde.personnages.first().map(|p| SeigneurVue {
                nom: p.nom.clone(),
                poste: p.poste.clone(),
                grenier: p.grenier.iter().map(|&m| unites(m)).collect(),
                chantiers: s.monde.batiments.iter().filter(|b| b.chantier.as_ref().map_or(false, |c| c.maitre == Some(p.id))).count(),
                champart: f(s.cat.seigneurie.champart_ppm),
            }),
            nb_serfs: s.pops.iter().filter(|p| p.statut == Some(sim::monde::Statut::Serf)).count(),
            hameaux,
            lieux,
            pops: s
                .pops
                .iter()
                .map(|p| {
                    let (motif, classe, ressource) = motif_texte(p.motif);
                    PopVue {
                        id: p.id,
                        residence: p.residence,
                        emploi: p.emploi.map_or(-1, |e| e as i64),
                        jauge: p.croissance as f64 / seuil * 100.0,
                        travail: unites(p.travail_milli),
                        motif,
                        classe,
                        ressource,
                        vital: f(p.vital_jour_ppm),
                        confort: f(p.confort_jour_ppm),
                    }
                })
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

// ---------- Vues détaillées : une pop, une ressource, un trajet ----------

#[derive(Serialize)]
struct RationVue {
    ressource: String,
    /// Ration normale par jour (unités).
    ration: f64,
    /// Stock personnel (unités) et jours qu'il tient (None : sans objet).
    stock: f64,
    autonomie_jours: Option<f64>,
    consomme_jour: f64,
    couverture: f64,
}

#[derive(Serialize)]
struct CandidatVue {
    lieu: usize,
    type_nom: String,
    x: usize,
    y: usize,
    /// Distance en tiles de chemin (coût / 10).
    distance: f64,
    ressource: String,
    classe: &'static str,
    manque_jours: f64,
    /// Travail encore réclamé en fin de journée (pop-jours).
    demande: f64,
    /// "choisi" | "complet" (la terre ne réclame plus assez de travail) | "moins_pressant"
    statut: &'static str,
}

#[derive(Serialize)]
struct HorsPortee {
    type_nom: String,
    x: usize,
    y: usize,
}

#[derive(Serialize)]
struct PopDetail {
    id: u32,
    residence: usize,
    x: usize,
    y: usize,
    emploi: i64,
    motif: &'static str,
    classe: &'static str,
    ressource: String,
    manque_jours: f64,
    travail: f64,
    travail_moyen: f64,
    vital: f64,
    confort: f64,
    /// Croissance en % du seuil, et taux annuel net (%) d'après la couverture de la pop.
    jauge: f64,
    taux_annuel: f64,
    nutrition_jours: f64,
    chauffage_jours: f64,
    cible_jours: f64,
    seuil_deplacement: f64,
    rations: Vec<RationVue>,
    candidats: Vec<CandidatVue>,
    hors_portee: Vec<HorsPortee>,
}

/// Détail d'une pop : ce qu'elle possède, ce qu'elle consomme, où elle peut travailler et pourquoi elle a choisi.
#[no_mangle]
pub extern "C" fn sim_pop_json(id: u32) -> u32 {
    avec(0, |s| {
        let Some(i) = s.pops.iter().position(|p| p.id == id) else {
            return ecrire(&serde_json::Value::Null);
        };
        let p = &s.pops[i];
        let c = &s.cat;
        let nq = s.monde.quartiers.len();
        let a = s.autonomies(p);
        let (motif, classe, res) = motif_texte(p.motif);
        let manque = match p.motif {
            Motif::Besoin { manque_milli_jours, .. } => manque_milli_jours as f64 / 1000.0,
            _ => 0.0,
        };
        let jours = |m: i64| if m == i64::MAX { f64::INFINITY } else { m as f64 / 1000.0 };
        let (hx, hy) = s.monde.xy(s.monde.quartiers[p.residence].tuile);
        let seuil = c.travail.seuil_deplacement_milli;
        let rations = c
            .besoins
            .iter()
            .map(|b| RationVue {
                ressource: c.ressources[b.ressource].nom.clone(),
                ration: unites(b.par_pop),
                stock: unites(p.stock[b.ressource]),
                autonomie_jours: Some(jours(a.par_ressource[b.ressource])).filter(|j| j.is_finite()),
                consomme_jour: unites(p.conso_jour[b.ressource]),
                couverture: f(p.sat_par_ressource_ppm[b.ressource]),
            })
            .collect();
        let cands = s.expliquer_pop(id).unwrap_or_default();
        let candidats = cands
            .iter()
            .map(|cd| {
                let bat = &s.monde.batiments[cd.lieu];
                let (x, y) = s.monde.xy(bat.tuile);
                let statut = if p.emploi == Some(cd.lieu) {
                    "choisi"
                } else if cd.demande_milli < seuil {
                    "complet"
                } else {
                    "moins_pressant"
                };
                CandidatVue {
                    lieu: cd.lieu,
                    type_nom: c.batiments[bat.type_id].nom.clone(),
                    x,
                    y,
                    distance: cd.cout as f64 / 10.0,
                    ressource: c.ressources[cd.ressource].nom.clone(),
                    classe: match cd.classe {
                        Classe::Vital => "vital",
                        Classe::Confort => "confort",
                        Classe::Intrant => "intrant",
                        Classe::Entretien => "entretien",
                    },
                    manque_jours: cd.manque_milli_jours as f64 / 1000.0,
                    demande: unites(cd.demande_milli),
                    statut,
                }
            })
            .collect::<Vec<_>>();
        let hors_portee = s
            .monde
            .batiments
            .iter()
            .enumerate()
            .filter(|(b, bat)| {
                c.batiments[bat.type_id].par_tile && bat.chantier.is_none() && s.reseau.chemin(p.residence, nq + b).is_none()
            })
            .map(|(_, bat)| {
                let (x, y) = s.monde.xy(bat.tuile);
                HorsPortee { type_nom: c.batiments[bat.type_id].nom.clone(), x, y }
            })
            .collect();
        ecrire(&PopDetail {
            id,
            residence: p.residence,
            x: hx,
            y: hy,
            emploi: p.emploi.map_or(-1, |e| e as i64),
            motif,
            classe,
            ressource: if res >= 0 { c.ressources[res as usize].nom.clone() } else { String::new() },
            manque_jours: manque,
            travail: unites(p.travail_milli),
            travail_moyen: unites(p.travail_lisse_milli),
            vital: f(p.vital_jour_ppm),
            confort: f(p.confort_jour_ppm),
            jauge: p.croissance as f64 / s.seuil_croissance() as f64 * 100.0,
            taux_annuel: taux_croissance_ppm(&c.demo, p.vital_jour_ppm, p.confort_jour_ppm) as f64 / PPM as f64 * 100.0,
            nutrition_jours: jours(a.nutrition),
            chauffage_jours: jours(a.chauffage),
            cible_jours: c.sim.jours_stock_quartier,
            seuil_deplacement: unites(seuil),
            rations,
            candidats,
            hors_portee,
        })
    })
}

#[derive(Serialize)]
struct FluxVue {
    de: (usize, usize),
    vers: (usize, usize),
    /// "lieu" ou "hameau" pour l'arrivée.
    vers_genre: &'static str,
    qte: f64,
}

#[derive(Serialize)]
struct ProducteurVue {
    lieu: usize,
    type_nom: String,
    x: usize,
    y: usize,
    produit_jour: f64,
    stock_sortie: f64,
}

#[derive(Serialize)]
struct ConsommateurVue {
    hameau: usize,
    x: usize,
    y: usize,
    pops: u32,
    /// Stock des pops du hameau, leur consommation et leur réception du jour (unités).
    stock: f64,
    consomme_jour: f64,
    recu_jour: f64,
    couverture: f64,
}

#[derive(Serialize)]
struct UtilisateurVue {
    lieu: usize,
    type_nom: String,
    x: usize,
    y: usize,
    stock_entree: f64,
}

#[derive(Serialize)]
struct Totaux {
    stock_producteurs: f64,
    stock_pops: f64,
    stock_intrants: f64,
    en_transit: f64,
    produit_jour: f64,
    consomme_pops_jour: f64,
    cumul_produit: f64,
    cumul_consomme_pops: f64,
    cumul_consomme_prod: f64,
    cumul_perime: f64,
}

#[derive(Serialize)]
struct RessourceVue {
    ressource: String,
    producteurs: Vec<ProducteurVue>,
    hameaux: Vec<ConsommateurVue>,
    utilisateurs: Vec<UtilisateurVue>,
    flux: Vec<FluxVue>,
    totaux: Totaux,
}

/// Où une ressource est produite, stockée, expédiée et consommée (dernier jour et cumul).
#[no_mangle]
pub extern "C" fn sim_ressource_json(r: u32) -> u32 {
    avec(0, |s| {
        let r = r as usize;
        let c = &s.cat;
        if r >= c.ressources.len() {
            return ecrire(&serde_json::Value::Null);
        }
        let nq = s.monde.quartiers.len();
        let xy_lieu = |l: usize| {
            if l < nq {
                s.monde.xy(s.monde.quartiers[l].tuile)
            } else {
                s.monde.xy(s.monde.batiments[l - nq].tuile)
            }
        };
        let mut produit_par_lieu = vec![0i64; s.monde.batiments.len()];
        for &(b, res, q) in &s.stats.produits_par_lieu {
            if res == r {
                produit_par_lieu[b] += q;
            }
        }
        let producteurs = s
            .monde
            .batiments
            .iter()
            .enumerate()
            .filter(|(b, bat)| bat.stock_sortie[r] > 0 || produit_par_lieu[*b] > 0)
            .map(|(b, bat)| {
                let (x, y) = s.monde.xy(bat.tuile);
                ProducteurVue {
                    lieu: b,
                    type_nom: c.batiments[bat.type_id].nom.clone(),
                    x,
                    y,
                    produit_jour: unites(produit_par_lieu[b]),
                    stock_sortie: unites(bat.stock_sortie[r]),
                }
            })
            .collect::<Vec<_>>();
        let mut recu = vec![0i64; nq];
        for fl in &s.stats.flux {
            if fl.ressource == r && fl.vers < nq {
                recu[fl.vers] += fl.qte;
            }
        }
        let hameaux = (0..nq)
            .map(|q| {
                let membres = s.pops.iter().filter(|p| p.residence == q);
                let (mut n, mut stock, mut conso) = (0u32, s.monde.quartiers[q].stock[r], 0i64);
                for p in membres {
                    n += 1;
                    stock += p.stock[r];
                    conso += p.conso_jour[r];
                }
                let (x, y) = s.monde.xy(s.monde.quartiers[q].tuile);
                ConsommateurVue {
                    hameau: q,
                    x,
                    y,
                    pops: n,
                    stock: unites(stock),
                    consomme_jour: unites(conso),
                    recu_jour: unites(recu[q]),
                    couverture: f(s.monde.quartiers[q].sat_par_ressource_ppm[r]),
                }
            })
            .filter(|h| h.pops > 0 || h.stock > 0.0)
            .collect::<Vec<_>>();
        let utilisateurs = s
            .monde
            .batiments
            .iter()
            .enumerate()
            .filter(|(_, bat)| bat.stock_entree[r] > 0)
            .map(|(b, bat)| {
                let (x, y) = s.monde.xy(bat.tuile);
                UtilisateurVue { lieu: b, type_nom: c.batiments[bat.type_id].nom.clone(), x, y, stock_entree: unites(bat.stock_entree[r]) }
            })
            .collect();
        let flux = s
            .stats
            .flux
            .iter()
            .filter(|fl| fl.ressource == r && fl.qte > 0)
            .map(|fl| FluxVue {
                de: xy_lieu(fl.de),
                vers: xy_lieu(fl.vers),
                vers_genre: if fl.vers < nq { "hameau" } else { "lieu" },
                qte: unites(fl.qte),
            })
            .collect();
        let totaux = Totaux {
            stock_producteurs: producteurs.iter().map(|p| p.stock_sortie).sum(),
            stock_pops: hameaux.iter().map(|h| h.stock).sum(),
            stock_intrants: s.monde.batiments.iter().map(|b| unites(b.stock_entree[r])).sum(),
            en_transit: s.differes.iter().filter(|d| d.ressource == r).map(|d| unites(d.qte)).sum(),
            produit_jour: unites(s.stats.produit[r]),
            consomme_pops_jour: unites(s.stats.consomme_pops[r]),
            cumul_produit: unites(s.bilan.produit[r]),
            cumul_consomme_pops: unites(s.bilan.consomme_pops[r]),
            cumul_consomme_prod: unites(s.bilan.consomme_prod[r]),
            cumul_perime: unites(s.bilan.perime[r]),
        };
        ecrire(&RessourceVue { ressource: c.ressources[r].nom.clone(), producteurs, hameaux, utilisateurs, flux, totaux })
    })
}

/// Tiles du chemin du hameau `q` au bâtiment `b` (de l'un à l'autre), en coordonnées de tiles.
#[no_mangle]
pub extern "C" fn sim_chemin_json(q: u32, b: u32) -> u32 {
    avec(0, |s| {
        let nq = s.monde.quartiers.len();
        let (q, b) = (q as usize, b as usize);
        if q >= nq || b >= s.monde.batiments.len() {
            return ecrire(&Vec::<(usize, usize)>::new());
        }
        let Some(ch) = s.reseau.chemin(q, nq + b) else {
            return ecrire(&Vec::<(usize, usize)>::new());
        };
        let pts: Vec<(usize, usize)> = ch.tuiles.iter().map(|&t| s.monde.xy(t)).collect();
        ecrire(&pts)
    })
}
