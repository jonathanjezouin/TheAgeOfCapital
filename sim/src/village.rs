//! Générateur de village : déterministe (graine), aléatoire mais cohérent.
//!
//! Dimensionné pour démarrer proche de l'équilibre : le nombre de bâtiments de
//! chaque type découle des besoins des quartiers (jamais d'un ratio codé en dur).
//! Disposition : un chemin principal horizontal, des bâtiments le long (deux côtés)
//! ou au bout d'antennes de 1 à 3 tiles. Cette version est volontairement simple ;
//! elle gagnera en finesse avec les villages suivants.

use crate::donnees::Catalogue;
use crate::monde::{Batiment, Contenu, Limitant, Monde, Quartier, Tuile, DIRS, N, S};
use crate::reseau::Reseau;
use crate::rng::Rng;
use crate::simulation::{Pop, Simulation};

#[derive(Clone, Debug)]
pub struct ParamsVillage {
    pub graine: u64,
    pub nb_quartiers: usize,
    /// Pops initiales par quartier, en % de la capacité de logement.
    pub occupation_initiale_pct: u32,
    pub largeur: usize,
    pub hauteur: usize,
}

impl Default for ParamsVillage {
    fn default() -> Self {
        ParamsVillage {
            graine: 1,
            nb_quartiers: 4,
            occupation_initiale_pct: 80,
            largeur: 200,
            hauteur: 200,
        }
    }
}

#[derive(Clone, Copy)]
enum Item {
    Quartier,
    Batiment(usize),
}

/// Nombre de bâtiments de chaque type pour couvrir les besoins de `nb_quartiers`
/// quartiers pleins, en remontant les chaînes de production (intrants compris).
pub fn composition(cat: &Catalogue, nb_quartiers: usize) -> Vec<usize> {
    let nr = cat.ressources.len();
    let mut demande = vec![0f64; nr];
    for r in 0..nr {
        demande[r] = (cat.besoin_pop(r)
            * cat.demo.capacite_quartier_pops as i64
            * nb_quartiers as i64) as f64;
    }
    let producteur: Vec<Option<usize>> = (0..nr)
        .map(|r| cat.producteurs(r).first().copied())
        .collect();
    // Ressources consommées par le producteur de s.
    let consomme_par = |s: usize, r: usize| -> bool {
        producteur[s].map_or(false, |t| {
            cat.batiments[t].intrants.iter().any(|(x, _)| *x == r)
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
            .extrants
            .iter()
            .find(|(x, _)| *x == r)
            .unwrap()
            .1 as f64;
        let besoin = demande[r] / q_ext;
        compte[t] = compte[t].max(besoin.round().max(1.0) as usize);
        for &(i, q_in) in &cat.batiments[t].intrants {
            demande[i] += besoin * q_in as f64;
        }
    }
    compte
}

pub fn generer(cat: Catalogue, p: &ParamsVillage) -> Result<Simulation, String> {
    let mut rng = Rng::new(p.graine);
    // --- Éléments à placer ---
    let mut items: Vec<Item> = (0..p.nb_quartiers).map(|_| Item::Quartier).collect();
    for (t, &n) in composition(&cat, p.nb_quartiers).iter().enumerate() {
        items.extend((0..n).map(|_| Item::Batiment(t)));
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
    let tuile = |m: &Monde, x: usize, y: usize| m.idx(x, y);

    for x in x0..x0 + cols {
        let t = tuile(&monde, x, cy);
        monde.tuiles[t].contenu = Contenu::Chemin { masque: 0 };
        if x > x0 {
            aretes.push((tuile(&monde, x - 1, cy), t));
        }
    }

    fn poser(monde: &mut Monde, cat: &Catalogue, item: Item, t: usize, rng: &mut Rng) {
        let nr = cat.ressources.len();
        match item {
            Item::Quartier => {
                let q = monde.quartiers.len();
                monde.quartiers.push(Quartier {
                    tuile: t,
                    stock: vec![0; nr],
                    croissance_milli: 0,
                    sat_jour_ppm: crate::PPM,
                    sat_nourriture_jour_ppm: crate::PPM,
                    sat_par_ressource_ppm: vec![crate::PPM; nr],
                    cumul_sat_ppm: 0,
                    cumul_sat_nourriture_ppm: 0,
                    jours_an: 0,
                    sat_annuelle_ppm: crate::PPM,
                    sat_nourriture_annuelle_ppm: crate::PPM,
                });
                monde.tuiles[t].contenu = Contenu::Quartier(q);
            }
            Item::Batiment(type_id) => {
                let b = monde.batiments.len();
                monde.batiments.push(Batiment {
                    type_id,
                    tuile: t,
                    stock_entree: vec![0; nr],
                    stock_sortie: vec![0; nr],
                    presents: 0,
                    prod_ppm: 0,
                    limitant: Limitant::Aucun,
                });
                monde.tuiles[t].contenu = Contenu::Batiment(b);
            }
        }
        monde.tuiles[t].variante = rng.dans(4) as u8;
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
                    let mut prec = tuile(&monde, x, cy);
                    for &y in &ys[..d as usize] {
                        let t = tuile(&monde, x, y);
                        monde.tuiles[t].contenu = Contenu::Chemin { masque: 0 };
                        aretes.push((prec, t));
                        prec = t;
                    }
                    let t = tuile(&monde, x, ys[d as usize]);
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
            let t = tuile(&monde, x, y);
            poser(&mut monde, &cat, item, t, &mut rng);
            aretes.push((tuile(&monde, x, cy), t));
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
    debug_assert!(N != S);

    // --- Stocks initiaux ---
    let cap = cat.demo.capacite_quartier_pops;
    let n0 = ((cap * p.occupation_initiale_pct + 50) / 100).max(1);
    for q in &mut monde.quartiers {
        for b in &cat.besoins {
            q.stock[b.ressource] =
                Catalogue::jours_x(b.par_pop * n0 as i64, cat.sim.stock_initial_quartier_jours);
        }
    }
    for b in &mut monde.batiments {
        let t = &cat.batiments[b.type_id];
        for &(r, q) in &t.intrants {
            b.stock_entree[r] = Catalogue::jours_x(q, cat.sim.stock_initial_entree_jours);
        }
        for &(r, q) in &t.extrants {
            b.stock_sortie[r] = Catalogue::jours_x(q, cat.sim.stock_initial_sortie_jours);
        }
    }

    // --- Pops et affectation aux postes ---
    let reseau = Reseau::construire(&monde, &cat);
    let nq = monde.quartiers.len();
    let nb = monde.batiments.len();
    for de in 0..nq + nb {
        for vers in 0..nq + nb {
            if reseau.chemin(de, vers).is_none() {
                return Err(format!("lieux {de} et {vers} non reliés dans le périmètre"));
            }
        }
    }
    let postes: Vec<u32> = monde
        .batiments
        .iter()
        .map(|b| cat.batiments[b.type_id].pops)
        .collect();
    let total_postes: u32 = postes.iter().sum();
    let total_pops = n0 * nq as u32;
    let cibles: Vec<u32> = if total_pops >= total_postes {
        postes.clone()
    } else {
        // Répartition proportionnelle (plus fortes fractions d'abord).
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
    let mut restantes = vec![n0; nq];
    let mut pops: Vec<Pop> = Vec::new();
    let mut id = 0u32;
    for (b, &t) in cibles.iter().enumerate() {
        for _ in 0..t {
            let q = (0..nq)
                .filter(|&q| restantes[q] > 0)
                .min_by_key(|&q| (reseau.chemin(q, nq + b).map_or(i64::MAX, |c| c.cout), q))
                .expect("assez de pops");
            restantes[q] -= 1;
            pops.push(Pop {
                id,
                residence: q,
                emploi: Some(b),
            });
            id += 1;
        }
    }
    for q in 0..nq {
        for _ in 0..restantes[q] {
            pops.push(Pop {
                id,
                residence: q,
                emploi: None,
            });
            id += 1;
        }
    }
    Ok(Simulation::nouvelle(cat, monde, pops))
}
