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
    /// Rayon (tiles) jusqu'où la terre garde sa pleine fertilité autour du centre.
    pub rayon_fertile: i64,
    /// Rayon (tiles) au-delà duquel la terre est la plus pauvre.
    pub rayon_sterile: i64,
}

impl Default for ParamsVillage {
    fn default() -> Self {
        ParamsVillage {
            graine: 1,
            nb_hameaux: 4,
            occupation_initiale_pct: 80,
            largeur: 200,
            hauteur: 200,
            rayon_fertile: 12,
            rayon_sterile: 28,
        }
    }
}

#[derive(Clone, Copy)]
enum Item {
    /// Index dans `Catalogue::habitats`.
    Hameau(usize),
    /// Type de terre et nombre de tiles.
    Terre(usize, usize),
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

fn isqrt(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = (n as f64).sqrt() as u64;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// Fertilité d'une tile (ppm) : bruit de valeur lissé (0,85 à 1,3), qui s'effondre
/// entre `rayon_fertile` et `rayon_sterile` autour du centre. Pur entier.
fn champ_fertilite(p: &ParamsVillage, rng: &mut Rng) -> Vec<i64> {
    const CELLULE: usize = 24; // ~6 km : le bruit de fertilité varie à l'échelle du terroir (tile de 250 m)
    let (gw, gh) = (p.largeur / CELLULE + 2, p.hauteur / CELLULE + 2);
    let grille: Vec<i64> = (0..gw * gh).map(|_| rng.dans(1001) as i64).collect();
    let (cx, cy) = (p.largeur as i64 / 2, p.hauteur as i64 / 2);
    let (r0, r1) = (p.rayon_fertile * 1000, p.rayon_sterile * 1000);
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
            let base = 850_000 + valeur * 450;
            let (dx, dy) = (x as i64 - cx, y as i64 - cy);
            let d = isqrt(((dx * dx + dy * dy) * 1_000_000) as u64) as i64;
            let facteur = if d <= r0 {
                PPM
            } else if d >= r1 {
                150_000
            } else {
                PPM - (PPM - 150_000) * (d - r0) / (r1 - r0)
            };
            f.push(base * facteur / PPM);
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
        prod_ppm: 0,
        limitant: Limitant::Aucun,
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
    for essai in 0..32u64 {
        let mut rng = Rng::new(
            p.graine
                .wrapping_add(essai.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        );
        match essayer(&cat, p, &mut rng) {
            Ok(sim) => return Ok(sim),
            Err(e) => derniere = e,
        }
    }
    Err(format!(
        "aucune disposition valide en 32 essais : {derniere}"
    ))
}

/// Découpe `tiles` tiles de terre en blocs d'au plus `max` tiles, de tailles voisines.
fn blocs(tiles: usize, max: usize) -> Vec<usize> {
    let n = tiles.div_ceil(max.max(1)).max(1);
    (0..n).map(|i| tiles / n + usize::from(i < tiles % n)).collect()
}

fn essayer(cat: &Catalogue, p: &ParamsVillage, rng: &mut Rng) -> Result<Simulation, String> {
    // --- Éléments à placer ---
    let compo = composition(cat, p.nb_hameaux);
    let hameaux: Vec<Item> = (0..p.nb_hameaux).map(|_| Item::Hameau(0)).collect();
    // Terres : un bloc de chaque type d'abord (pour que chaque ressource ait un producteur près du
    // centre), puis les autres blocs.
    let mut par_type: Vec<Vec<usize>> = Vec::new();
    for (t, &n) in compo.batiments.iter().enumerate() {
        let ty = &cat.batiments[t];
        if ty.par_tile && n > 0 {
            par_type.push(blocs(n, ty.emprise_max as usize).into_iter().map(|taille| t * 1000 + taille).collect());
        }
    }
    rng.melanger(&mut par_type);
    let mut terres: Vec<Item> = Vec::new();
    let mut rang = 0;
    loop {
        let mut vu = false;
        for liste in &par_type {
            if let Some(&c) = liste.get(rang) {
                terres.push(Item::Terre(c / 1000, c % 1000));
                vu = true;
            }
        }
        if !vu {
            break;
        }
        rang += 1;
    }

    // --- Disposition en croix ---
    let (w, h) = (p.largeur, p.hauteur);
    let (cx, cy) = (w / 2, h / 2);
    // Longueur des bras : le fond d'un bras reste à portée de tous les hameaux (coût 10 par tile).
    let longueur = ((cat.infra.distance_max / 10) as usize).saturating_sub(3).clamp(3, 8);
    if cx < longueur + 20 || cy < longueur + 20 {
        return Err("carte trop petite pour ce village".into());
    }
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
        })
        .collect();
    let dirs: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let centre = monde.idx(cx, cy);
    let tuile_a = |m: &Monde, d: usize, k: i32, lat: i32| -> Option<usize> {
        let (dx, dy) = dirs[d];
        let (px, py) = (-dy, dx);
        let (x, y) = (cx as i32 + dx * k + px * lat, cy as i32 + dy * k + py * lat);
        if x < 1 || y < 1 || x >= w as i32 - 1 || y >= h as i32 - 1 {
            None
        } else {
            Some(m.idx(x as usize, y as usize))
        }
    };
    for d in 0..4 {
        let mut prec = centre;
        for k in 1..=longueur as i32 {
            let t = tuile_a(&monde, d, k, 0).ok_or("bras hors de la carte")?;
            monde.relier(prec, t);
            prec = t;
        }
    }
    let libre = |m: &Monde, t: usize| m.tuiles[t].contenu == Contenu::Vide && m.tuiles[t].aretes == 0;

    // Hameaux : au plus près du centre, de part et d'autre des bras (k = 1 puis 2).
    let mut slots: Vec<(usize, usize)> = Vec::new(); // (tile du hameau, tile du bras)
    for k in 1..=2 {
        for d in 0..4 {
            for lat in [1, -1] {
                if let (Some(t), Some(arm)) = (tuile_a(&monde, d, k, lat), tuile_a(&monde, d, k, 0)) {
                    // Le coin entre deux bras est un emplacement commun aux deux : une seule fois.
                    if !slots.iter().any(|&(u, _)| u == t) {
                        slots.push((t, arm));
                    }
                }
            }
        }
    }
    // On remplit d'abord les emplacements de k = 1, dans un ordre tiré au sort.
    let k1 = slots.iter().filter(|&&(_, arm)| {
        // les emplacements de k = 1 sont ceux dont la tile de bras est voisine du centre
        let (ax, ay) = monde.xy(arm);
        ax.abs_diff(cx) + ay.abs_diff(cy) == 1
    }).count();
    let (proches, loin) = slots.split_at(k1.min(slots.len()));
    let mut proches = proches.to_vec();
    let mut loin = loin.to_vec();
    rng.melanger(&mut proches);
    rng.melanger(&mut loin);
    proches.extend(loin);
    if hameaux.len() > proches.len() {
        return Err("plus d'emplacement pour les hameaux".into());
    }
    for (item, &(t, arm)) in hameaux.iter().zip(&proches) {
        if let Item::Hameau(hh) = *item {
            poser_hameau(&mut monde, cat, hh, t, rng);
            monde.relier(arm, t);
        }
    }

    // Terres : blocs de largeur 3 (le long du bras) et de profondeur suffisante, au plus près du centre.
    let mut ordre_bras: Vec<usize> = (0..4).collect();
    for item in &terres {
        let Item::Terre(type_id, taille) = *item else {
            continue;
        };
        rng.melanger(&mut ordre_bras);
        let mut place = false;
        'recherche: for k in 3..=(longueur as i32) {
            for &d in &ordre_bras {
                for lat in [1i32, -1] {
                    for largeur in [3i32, 4, 2] {
                        if k + largeur - 1 > longueur as i32 {
                            continue;
                        }
                        let profondeur = (taille as i32 + largeur - 1) / largeur;
                        let mut tuiles: Vec<usize> = Vec::new();
                        let mut valide = true;
                        'bloc: for j in 0..profondeur {
                            for i in 0..largeur {
                                if tuiles.len() == taille {
                                    break 'bloc;
                                }
                                match tuile_a(&monde, d, k + i, lat * (1 + j)) {
                                    Some(t) if libre(&monde, t) => tuiles.push(t),
                                    _ => {
                                        valide = false;
                                        break 'bloc;
                                    }
                                }
                            }
                        }
                        if !valide || tuiles.len() != taille {
                            continue;
                        }
                        let arm = tuile_a(&monde, d, k, 0).unwrap();
                        let b = monde.batiments.len();
                        let mut bat = nouveau_batiment(type_id, tuiles[0], None, cat);
                        bat.emprise = tuiles.clone();
                        monde.batiments.push(bat);
                        for &t in &tuiles {
                            monde.tuiles[t].contenu = Contenu::Batiment(b);
                            monde.tuiles[t].variante = rng.dans(4) as u8;
                        }
                        monde.relier(arm, tuiles[0]);
                        place = true;
                        break 'recherche;
                    }
                }
            }
        }
        if !place {
            return Err("plus d'emplacement le long de la croix".into());
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
            pops.push(Pop {
                id,
                residence: q,
                emploi: None,
                croissance: (rng.u64() % seuil as u64) as i64,
            });
            id += 1;
        }
    }
    Ok(Simulation::nouvelle(cat.clone(), monde, pops))
}
