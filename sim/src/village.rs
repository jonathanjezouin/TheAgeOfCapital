//! Générateur de village : déterministe (graine), aléatoire mais cohérent.
//!
//! Économie de subsistance de 1750 sur des tiles de 250 m : de petits hameaux (2 pops, textile fait à la
//! maison) et des terres exploitées de N tiles (champs, pâturages, vergers, forêt, lin, laine) disposés le
//! long d'une croix de chemins de terre (arêtes, voir docs/02 §9). Les hameaux sont près du centre ; chaque
//! terre est un bloc de tiles contiguës dont la tile « nœud » est reliée à un bras. Le nombre de tiles de
//! chaque type découle des besoins des hameaux (jamais d'un ratio codé en dur) pour démarrer proche de
//! l'équilibre. La croix est compacte : tout reste à moins de `distance_max` (coût des arêtes).
//! Un village plus grand que la croix ne tient pas : il faudrait plusieurs groupes de hameaux.

use crate::donnees::Catalogue;
use crate::monde::{Batiment, Contenu, Limitant, Monde, Quartier, Tuile};
use crate::reseau::Reseau;
use crate::rng::Rng;
use crate::simulation::{Pop, Simulation};
use crate::terrain::{generer_terrain, sites_village, CarteTerrain, ParamsTerrain};
use crate::PPM;

#[derive(Clone, Debug)]
pub struct ParamsVillage {
    pub graine: u64,
    /// Nombre total de hameaux.
    pub nb_hameaux: usize,
    /// Pops initiales, en % de la capacité de logement totale.
    pub occupation_initiale_pct: u32,
    pub largeur: usize,
    pub hauteur: usize,
}

impl Default for ParamsVillage {
    fn default() -> Self {
        ParamsVillage {
            graine: 1,
            nb_hameaux: 4,
            occupation_initiale_pct: 80,
            largeur: 200,
            hauteur: 200,
        }
    }
}

/// Combien de bâtiments de chaque type : des tiles pour les terres, un atelier par hameau.
#[derive(Clone, Debug)]
pub struct Composition {
    /// Par type de bâtiment : tiles de terre exploitée à fournir, ou nombre d'ateliers domestiques.
    pub batiments: Vec<usize>,
}

/// Dimensionne un village de `nb_hameaux` hameaux pleins : les terres, en remontant les chaînes
/// de production (intrants compris) d'après les besoins des pops. Chaque hameau fait lui-même
/// ses vêtements (atelier domestique), avec ses jours libres.
pub fn composition(cat: &Catalogue, nb_hameaux: usize) -> Composition {
    let nr = cat.ressources.len();
    let capacite = cat.habitats[0].capacite_pops as i64 * nb_hameaux as i64;
    let mut demande = vec![0f64; nr];
    for b in &cat.besoins {
        demande[b.ressource] = cat.ration_pop(b.ressource) as f64 * capacite as f64;
    }
    let producteur: Vec<Option<usize>> = (0..nr)
        .map(|r| cat.producteurs(r).first().copied())
        .collect();
    let consomme_par = |s: usize, r: usize| -> bool {
        producteur[s].map_or(false, |t| {
            cat.batiments[t]
                .intrants_externes()
                .iter()
                .any(|(x, _)| *x == r)
        })
    };
    let est_atelier = |t: usize| cat.habitats.iter().any(|h| h.atelier == Some(t));
    let mut fait = vec![false; nr];
    let mut compte = vec![0usize; cat.batiments.len()];
    for _ in 0..nr {
        // Une ressource est traitable quand tous ses consommateurs l'ont été.
        let Some(r) = (0..nr)
            .find(|&r| !fait[r] && (0..nr).all(|s| s == r || fait[s] || !consomme_par(s, r)))
        else {
            break;
        };
        fait[r] = true;
        let (Some(t), true) = (producteur[r], demande[r] > 0.0) else {
            continue;
        };
        let q_ext = cat.batiments[t]
            .extrants_finaux()
            .iter()
            .find(|(x, _)| *x == r)
            .unwrap()
            .1 as f64;
        let besoin = demande[r] / q_ext;
        compte[t] = if est_atelier(t) {
            nb_hameaux
        } else {
            compte[t].max(besoin.round().max(1.0) as usize)
        };
        for (i, q_in) in cat.batiments[t].intrants_externes() {
            demande[i] += besoin * q_in as f64;
        }
    }
    Composition { batiments: compte }
}


/// Qualité locale du sol (ppm de 1,0) : bruit de valeur lissé de 0,85 à 1,3, sans lien avec le village.
/// Le terrain (docs/07) et son aptitude par type de terre s'y ajoutent : la fertilité ne décroît plus
/// artificiellement avec la distance au centre. Pur entier.
fn champ_fertilite(p: &ParamsVillage, rng: &mut Rng) -> Vec<i64> {
    const CELLULE: usize = 24; // ~6 km : le bruit de fertilité varie à l'échelle du terroir (tile de 250 m)
    let (gw, gh) = (p.largeur / CELLULE + 2, p.hauteur / CELLULE + 2);
    let grille: Vec<i64> = (0..gw * gh).map(|_| rng.dans(1001) as i64).collect();
    let mut f = Vec::with_capacity(p.largeur * p.hauteur);
    for y in 0..p.hauteur {
        for x in 0..p.largeur {
            let (gx, gy, fx, fy) = (
                x / CELLULE,
                y / CELLULE,
                (x % CELLULE) as i64,
                (y % CELLULE) as i64,
            );
            let v = |i: usize, j: usize| grille[j * gw + i];
            let c = CELLULE as i64;
            let haut = v(gx, gy) * (c - fx) + v(gx + 1, gy) * fx;
            let bas = v(gx, gy + 1) * (c - fx) + v(gx + 1, gy + 1) * fx;
            let valeur = (haut * (c - fy) + bas * fy) / (c * c); // 0..=1000
            f.push(850_000 + valeur * 450);
        }
    }
    f
}

fn nouveau_hameau(tuile: usize, habitat_id: usize, cat: &Catalogue) -> Quartier {
    let nr = cat.ressources.len();
    Quartier {
        tuile,
        habitat_id,
        capacite_pops: cat.habitats[habitat_id].capacite_pops,
        atelier: None,
        stock: vec![0; nr],
        vital_jour_ppm: PPM,
        confort_jour_ppm: PPM,
        sat_par_ressource_ppm: vec![PPM; nr],
        cumul_vital_ppm: 0,
        cumul_confort_ppm: 0,
        jours_an: 0,
        vital_annuel_ppm: PPM,
        confort_annuel_ppm: PPM,
        vide_mois: 0,
        abandon_mois: None,
        detruit: false,
    }
}

fn nouveau_batiment(
    type_id: usize,
    tuile: usize,
    quartier: Option<usize>,
    cat: &Catalogue,
) -> Batiment {
    let nr = cat.ressources.len();
    Batiment {
        type_id,
        tuile,
        quartier,
        emprise: vec![tuile],
        equipe: 0,
        chantier: None,
        stock_entree: vec![0; nr],
        stock_sortie: vec![0; nr],
        presents: 0,
        mo_milli: 0,
        prod_ppm: if cat.batiments[type_id].par_tile { PPM } else { 0 },
        demande_milli: 0,
        sur_pied_ppm: if cat.batiments[type_id].par_tile { (cat.sim.jours_stock_sortie * PPM as f64 / 2.0) as i64 } else { 0 },
        mo_lisse_milli: 0,
        limitant: Limitant::Aucun,
        inactif_mois: 0,
        abandon_mois: None,
        progres_ref: 0,
    }
}

fn poser_hameau(monde: &mut Monde, cat: &Catalogue, h: usize, t: usize, rng: &mut Rng) {
    let q = monde.quartiers.len();
    monde.quartiers.push(nouveau_hameau(t, h, cat));
    monde.tuiles[t].contenu = Contenu::Quartier(q);
    if let Some(atelier) = cat.habitats[h].atelier {
        let b = monde.batiments.len();
        monde
            .batiments
            .push(nouveau_batiment(atelier, t, Some(q), cat));
        monde.quartiers[q].atelier = Some(b);
    }
    monde.tuiles[t].variante = rng.dans(4) as u8;
}

/// Génère un village. Si une disposition tirée au sort viole les contraintes (accès, distance),
/// on en tire une autre de façon déterministe à partir de la même graine.
pub fn generer(cat: Catalogue, p: &ParamsVillage) -> Result<Simulation, String> {
    let mut derniere = String::new();
    // Le terrain dépend de la seule graine ; le village se pose sur une fenêtre dégagée, près d'un point d'eau.
    let carte = generer_terrain(&ParamsTerrain::defaut(p.graine, p.largeur, p.hauteur));
    let sites = sites_village(&carte);
    if sites.is_empty() {
        return Err("aucun site dégagé pour le village sur cette carte".into());
    }
    // Le meilleur site d'abord ; si la disposition n'y tient pas, le suivant.
    for &(_, cx0, cy0) in &sites {
        let centre = (cx0, cy0);
        for essai in 0..32u64 {
            let mut rng = Rng::new(
                p.graine
                    .wrapping_add(essai.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
            );
            match essayer(&cat, p, &mut rng, &carte, centre) {
                Ok(sim) => return Ok(sim),
                Err(e) => derniere = e,
            }
        }
    }
    Err(format!(
        "aucune disposition valide ({derniere})"
    ))
}

fn essayer(
    cat: &Catalogue,
    p: &ParamsVillage,
    rng: &mut Rng,
    carte: &CarteTerrain,
    centre_village: (usize, usize),
) -> Result<Simulation, String> {
    let (w, h) = (p.largeur, p.hauteur);
    let (cx, cy) = centre_village;
    let mut monde = Monde {
        largeur: w,
        hauteur: h,
        tuiles: Vec::new(),
        quartiers: Vec::new(),
        batiments: Vec::new(),
    };
    let fertilite = champ_fertilite(p, rng);
    monde.tuiles = (0..w * h)
        .map(|i| Tuile {
            contenu: Contenu::Vide,
            aretes: 0,
            variante: rng.dans(12) as u8,
            fertilite_ppm: fertilite[i],
            terrain: carte.terrain[i],
            eau: carte.eau[i],
            aval: carte.aval[i],
            vacant_mois: 0,
        })
        .collect();

    // --- État initial (docs/07 §5) : un premier hameau au site choisi, puis le village se constitue avec les
    // mêmes règles de site que le jeu (terres, hameaux), chantiers achevés d'un coup. Plus de disposition codée.
    let compo = composition(cat, p.nb_hameaux);
    let centre = monde.idx(cx, cy);
    poser_hameau(&mut monde, cat, 0, centre, rng);
    let mut amorce = Simulation::nouvelle(cat.clone(), monde, Vec::new());
    amorce.peupler_initial(&compo.batiments, p.nb_hameaux)?;
    let mut monde = amorce.monde;
    for t in &mut monde.tuiles {
        t.variante = (t.variante % 12) as u8;
    }
    // Les terres posées par chantier démarrent comme celles d'un village en régime.
    for b in &mut monde.batiments {
        let ty = &cat.batiments[b.type_id];
        if ty.par_tile {
            b.prod_ppm = PPM;
            b.sur_pied_ppm = (cat.sim.jours_stock_sortie * PPM as f64 / 2.0) as i64;
        }
    }

// --- Pops initiales : réparties sur les hameaux (ordre tiré au hasard) ---
    let nq = monde.quartiers.len();
    let cap_totale: u32 = monde.quartiers.iter().map(|q| q.capacite_pops).sum();
    let total0 = ((cap_totale * p.occupation_initiale_pct + 50) / 100)
        .max(1)
        .min(cap_totale);
    let mut ordre: Vec<usize> = (0..nq).collect();
    rng.melanger(&mut ordre);
    let mut n0 = vec![0u32; nq];
    let mut reste = total0;
    'tour: loop {
        let mut progres = false;
        for &q in &ordre {
            if reste == 0 {
                break 'tour;
            }
            if n0[q] < monde.quartiers[q].capacite_pops {
                n0[q] += 1;
                reste -= 1;
                progres = true;
            }
        }
        if !progres {
            break;
        }
    }

    // --- Stocks initiaux ---
    for b in &mut monde.batiments {
        let t = &cat.batiments[b.type_id];
        for (r, q) in t.intrants_externes() {
            b.stock_entree[r] = Catalogue::jours_x(q, cat.sim.stock_initial_entree_jours);
        }
        for (r, q) in t.extrants_finaux() {
            b.stock_sortie[r] = Catalogue::jours_x(q, cat.sim.stock_initial_sortie_jours);
        }
    }

    // --- Accès : chaque hameau doit pouvoir s'approvisionner, chaque lieu être desservi ---
    let reseau = Reseau::construire(&monde, cat);
    let nb = monde.batiments.len();
    let produit = |b: usize, r: usize| {
        cat.batiments[monde.batiments[b].type_id]
            .extrants_finaux()
            .iter()
            .any(|(x, _)| *x == r)
    };
    for q in 0..nq {
        for besoin in &cat.besoins {
            if !(0..nb).any(|b| produit(b, besoin.ressource) && reseau.chemin(q, nq + b).is_some())
            {
                return Err(format!(
                    "hameau {q} sans accès à {}",
                    cat.ressources[besoin.ressource].nom
                ));
            }
        }
    }
    for b in 0..nb {
        // Les sites de chantier laissés inertes par la constitution du village ne sont pas des lieux de travail.
        if cat.batiments[monde.batiments[b].type_id].chantier {
            continue;
        }
        for (r, _) in cat.batiments[monde.batiments[b].type_id].intrants_externes() {
            if !(0..nb).any(|f| produit(f, r) && reseau.chemin(nq + b, nq + f).is_some()) {
                return Err(format!("lieu {b} sans accès à {}", cat.ressources[r].nom));
            }
        }
        if !(0..nq).any(|q| reseau.chemin(q, nq + b).is_some()) {
            return Err(format!("lieu {b} hors de portée de tout hameau"));
        }
    }

    // --- Pops initiales : sans emploi, le travail se répartit chaque jour (budget de jours) ---
    let seuil = cat.demo.habitants_par_pop * 1_000_000;
    let mut pops: Vec<Pop> = Vec::new();
    let mut id = 0u32;
    for q in 0..nq {
        for _ in 0..n0[q] {
            // Jauges décalées pour que les duplications ne soient pas synchrones.
            let mut pop = Pop::nouvelle(id, q, (rng.u64() % seuil as u64) as i64, cat.ressources.len());
            // Stock personnel de départ : quelques jours de ration.
            for b in &cat.besoins {
                pop.stock[b.ressource] = Catalogue::jours_x(b.par_pop, cat.sim.stock_initial_quartier_jours);
            }
            pops.push(pop);
            id += 1;
        }
    }
    Ok(Simulation::nouvelle(cat.clone(), monde, pops))
}
