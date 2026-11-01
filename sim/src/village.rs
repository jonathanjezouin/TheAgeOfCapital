//! Générateur de village : déterministe (graine), aléatoire mais cohérent.
//!
//! Économie de subsistance de 1750 : des hameaux (logement à faible densité, avec
//! leur travail textile à domicile) et des terres exploitées (champs, pâturages,
//! vergers, forêt, lin, laine) le long d'un chemin principal, directement ou au bout
//! d'antennes de 1 à 3 tiles. Le nombre de terres de chaque type découle des besoins
//! des hameaux (jamais d'un ratio codé en dur) pour démarrer proche de l'équilibre.
//! Version volontairement simple : elle gagnera en finesse avec les villages suivants.

use crate::donnees::Catalogue;
use crate::monde::{Batiment, Contenu, Limitant, Monde, Quartier, Tuile, DIRS};
use crate::reseau::Reseau;
use crate::rng::Rng;
use crate::simulation::{Pop, Simulation};
use crate::PPM;

#[derive(Clone, Debug)]
pub struct ParamsVillage {
    pub graine: u64,
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
            nb_hameaux: 10,
            occupation_initiale_pct: 80,
            largeur: 200,
            hauteur: 200,
        }
    }
}

#[derive(Clone, Copy)]
enum Item {
    Hameau,
    Terre(usize),
}

/// Nombre de bâtiments de chaque type pour couvrir les besoins de `nb_hameaux`
/// hameaux pleins, en remontant les chaînes de production (intrants compris).
/// Les bâtiments domestiques sont comptés à raison d'un par hameau.
pub fn composition(cat: &Catalogue, nb_hameaux: usize) -> Vec<usize> {
    let nr = cat.ressources.len();
    let mut demande = vec![0f64; nr];
    for b in &cat.besoins {
        demande[b.ressource] =
            (b.par_pop * cat.habitat.capacite_pops as i64 * nb_hameaux as i64) as f64;
    }
    let producteur: Vec<Option<usize>> = (0..nr)
        .map(|r| cat.producteurs(r).first().copied())
        .collect();
    // Le producteur de s consomme-t-il r de l'extérieur ?
    let consomme_par = |s: usize, r: usize| -> bool {
        producteur[s].map_or(false, |t| {
            cat.batiments[t]
                .intrants_externes()
                .iter()
                .any(|(x, _)| *x == r)
        })
    };
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
        compte[t] = if cat.batiments[t].domestique {
            nb_hameaux
        } else {
            compte[t].max(besoin.round().max(1.0) as usize)
        };
        for (i, q_in) in cat.batiments[t].intrants_externes() {
            demande[i] += besoin * q_in as f64;
        }
    }
    compte
}

fn nouveau_hameau(tuile: usize, cat: &Catalogue) -> Quartier {
    let nr = cat.ressources.len();
    Quartier {
        tuile,
        capacite_pops: cat.habitat.capacite_pops,
        domestique: None,
        stock: vec![0; nr],
        vital_jour_ppm: PPM,
        confort_jour_ppm: PPM,
        sat_par_ressource_ppm: vec![PPM; nr],
        cumul_vital_ppm: 0,
        cumul_confort_ppm: 0,
        jours_an: 0,
        vital_annuel_ppm: PPM,
        confort_annuel_ppm: PPM,
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
        stock_entree: vec![0; nr],
        stock_sortie: vec![0; nr],
        presents: 0,
        prod_ppm: 0,
        limitant: Limitant::Aucun,
    }
}

fn poser(monde: &mut Monde, cat: &Catalogue, item: Item, t: usize, rng: &mut Rng) {
    match item {
        Item::Hameau => {
            let q = monde.quartiers.len();
            monde.quartiers.push(nouveau_hameau(t, cat));
            monde.tuiles[t].contenu = Contenu::Quartier(q);
            if let Some(type_id) = cat.habitat.domestique {
                let b = monde.batiments.len();
                monde
                    .batiments
                    .push(nouveau_batiment(type_id, t, Some(q), cat));
                monde.quartiers[q].domestique = Some(b);
            }
        }
        Item::Terre(type_id) => {
            let b = monde.batiments.len();
            monde
                .batiments
                .push(nouveau_batiment(type_id, t, None, cat));
            monde.tuiles[t].contenu = Contenu::Batiment(b);
        }
    }
    monde.tuiles[t].variante = rng.dans(4) as u8;
}

pub fn generer(cat: Catalogue, p: &ParamsVillage) -> Result<Simulation, String> {
    let mut rng = Rng::new(p.graine);

    // --- Éléments à placer ---
    let mut items: Vec<Item> = (0..p.nb_hameaux).map(|_| Item::Hameau).collect();
    for (t, &n) in composition(&cat, p.nb_hameaux).iter().enumerate() {
        if !cat.batiments[t].domestique {
            items.extend((0..n).map(|_| Item::Terre(t)));
        }
    }
    rng.melanger(&mut items);

    // --- Disposition ---
    let (w, h) = (p.largeur, p.hauteur);
    let cols = items.len().div_ceil(2) + 4;
    if cols + 2 > w || h < 12 {
        return Err("carte trop petite pour ce village".into());
    }
    let (cx, cy) = (w / 2, h / 2);
    let x0 = cx - cols / 2;
    let mut monde = Monde {
        largeur: w,
        hauteur: h,
        tuiles: (0..w * h)
            .map(|_| Tuile {
                contenu: Contenu::Vide,
                variante: rng.dans(12) as u8,
            })
            .collect(),
        quartiers: Vec::new(),
        batiments: Vec::new(),
    };
    let mut aretes: Vec<(usize, usize)> = Vec::new();

    for x in x0..x0 + cols {
        let t = monde.idx(x, cy);
        monde.tuiles[t].contenu = Contenu::Chemin { masque: 0 };
        if x > x0 {
            aretes.push((monde.idx(x - 1, cy), t));
        }
    }

    let libre = |m: &Monde, x: usize, y: usize| m.tuiles[m.idx(x, y)].contenu == Contenu::Vide;
    for item in items {
        let mut place = false;
        if rng.chance(35) {
            for _ in 0..8 {
                let x = x0 + rng.dans(cols);
                let cote: i64 = if rng.chance(50) { 1 } else { -1 };
                let d = 1 + rng.dans(3) as i64; // longueur de l'antenne
                let ys: Vec<usize> = (1..=d + 1)
                    .map(|k| (cy as i64 + cote * k) as usize)
                    .collect();
                if ys.iter().all(|&y| libre(&monde, x, y)) {
                    let mut prec = monde.idx(x, cy);
                    for &y in &ys[..d as usize] {
                        let t = monde.idx(x, y);
                        monde.tuiles[t].contenu = Contenu::Chemin { masque: 0 };
                        aretes.push((prec, t));
                        prec = t;
                    }
                    let t = monde.idx(x, ys[d as usize]);
                    poser(&mut monde, &cat, item, t, &mut rng);
                    aretes.push((prec, t));
                    place = true;
                    break;
                }
            }
        }
        if !place {
            let mut slots: Vec<(usize, usize)> = Vec::new();
            for x in x0..x0 + cols {
                for y in [cy - 1, cy + 1] {
                    if libre(&monde, x, y) {
                        slots.push((x, y));
                    }
                }
            }
            if slots.is_empty() {
                return Err("plus d'emplacement le long du chemin principal".into());
            }
            let (x, y) = slots[rng.dans(slots.len())];
            let t = monde.idx(x, y);
            poser(&mut monde, &cat, item, t, &mut rng);
            aretes.push((monde.idx(x, cy), t));
        }
    }

    // --- Demi-directions des chemins ---
    for (a, b) in aretes {
        let (ax, ay) = monde.xy(a);
        let (bx, by) = monde.xy(b);
        let (dx, dy) = (bx as i32 - ax as i32, by as i32 - ay as i32);
        let d = DIRS
            .iter()
            .position(|&(_, ddx, ddy, _)| ddx == dx && ddy == dy)
            .expect("arête non orthogonale");
        if let Contenu::Chemin { masque } = &mut monde.tuiles[a].contenu {
            *masque |= DIRS[d].0;
        }
        if let Contenu::Chemin { masque } = &mut monde.tuiles[b].contenu {
            *masque |= DIRS[d].3;
        }
    }

    // --- Pops initiales : réparties sur les hameaux (ordre tiré au hasard) ---
    let nq = monde.quartiers.len();
    let cap_totale = nq as u32 * cat.habitat.capacite_pops;
    let total0 = ((cap_totale * p.occupation_initiale_pct + 50) / 100)
        .max(1)
        .min(cap_totale);
    let mut ordre: Vec<usize> = (0..nq).collect();
    rng.melanger(&mut ordre);
    let mut n0 = vec![0u32; nq];
    for i in 0..total0 as usize {
        n0[ordre[i % nq]] += 1;
    }

    // --- Stocks initiaux ---
    for (q, qu) in monde.quartiers.iter_mut().enumerate() {
        for b in &cat.besoins {
            qu.stock[b.ressource] = Catalogue::jours_x(
                b.par_pop * n0[q] as i64,
                cat.sim.stock_initial_quartier_jours,
            );
        }
    }
    for b in &mut monde.batiments {
        let t = &cat.batiments[b.type_id];
        for (r, q) in t.intrants_externes() {
            b.stock_entree[r] = Catalogue::jours_x(q, cat.sim.stock_initial_entree_jours);
        }
        for (r, q) in t.extrants_finaux() {
            b.stock_sortie[r] = Catalogue::jours_x(q, cat.sim.stock_initial_sortie_jours);
        }
    }

    // --- Connexité ---
    let reseau = Reseau::construire(&monde, &cat);
    let nb = monde.batiments.len();
    for de in 0..nq + nb {
        for vers in 0..nq + nb {
            if reseau.chemin(de, vers).is_none() {
                return Err(format!("lieux {de} et {vers} non reliés dans le périmètre"));
            }
        }
    }

    // --- Affectation aux postes (proportionnelle, pops les plus proches d'abord) ---
    let travaillees: Vec<usize> = (0..nb)
        .filter(|&b| !cat.batiments[monde.batiments[b].type_id].domestique)
        .collect();
    let postes: Vec<u32> = travaillees
        .iter()
        .map(|&b| cat.batiments[monde.batiments[b].type_id].pops)
        .collect();
    let total_postes: u32 = postes.iter().sum();
    let total_pops: u32 = n0.iter().sum();
    let cibles: Vec<u32> = if total_pops >= total_postes {
        postes.clone()
    } else {
        let mut c: Vec<u32> = postes
            .iter()
            .map(|&x| x * total_pops / total_postes)
            .collect();
        let mut fr: Vec<(u32, usize)> = postes
            .iter()
            .enumerate()
            .map(|(i, &x)| ((x * total_pops) % total_postes, i))
            .collect();
        fr.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut reste = total_pops - c.iter().sum::<u32>();
        for (_, i) in fr {
            if reste == 0 {
                break;
            }
            c[i] += 1;
            reste -= 1;
        }
        c
    };
    let seuil = cat.demo.habitants_par_pop * 1_000_000;
    let mut restantes = n0.clone();
    let mut pops: Vec<Pop> = Vec::new();
    let mut id = 0u32;
    let mut nouvelle = |residence: usize, emploi: Option<usize>, rng: &mut Rng| {
        // Jauges décalées pour que les duplications ne soient pas synchrones.
        let p = Pop {
            id,
            residence,
            emploi,
            croissance: (rng.u64() % seuil as u64) as i64,
        };
        id += 1;
        p
    };
    for (k, &b) in travaillees.iter().enumerate() {
        for _ in 0..cibles[k] {
            let q = (0..nq)
                .filter(|&q| restantes[q] > 0)
                .min_by_key(|&q| (reseau.chemin(q, nq + b).map_or(i64::MAX, |c| c.cout), q))
                .expect("assez de pops");
            restantes[q] -= 1;
            pops.push(nouvelle(q, Some(b), &mut rng));
        }
    }
    for q in 0..nq {
        for _ in 0..restantes[q] {
            pops.push(nouvelle(q, None, &mut rng));
        }
    }
    Ok(Simulation::nouvelle(cat, monde, pops))
}
