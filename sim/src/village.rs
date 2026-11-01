//! Générateur de village : déterministe (graine), aléatoire mais cohérent.
//!
//! Économie de subsistance de 1750 : des hameaux (2 pops, ou 3 pops dont une travaille à
//! l'atelier textile du hameau) et des terres exploitées (champs, pâturages, vergers, forêt,
//! lin, laine) disposés le long d'une croix de chemins de terre, directement ou au bout d'une
//! antenne d'une tile. Le nombre de terres de chaque type découle des besoins des hameaux
//! (jamais d'un ratio codé en dur) pour démarrer proche de l'équilibre.
//! La croix est compacte : tout reste à moins de `distance_max` tiles de chemin.
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
    /// Nombre total de hameaux (dont les hameaux textiles, calculés d'après les besoins).
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
            nb_hameaux: 10,
            occupation_initiale_pct: 80,
            largeur: 200,
            hauteur: 200,
            rayon_fertile: 3,
            rayon_sterile: 7,
        }
    }
}

#[derive(Clone, Copy)]
enum Item {
    /// Index dans `Catalogue::habitats`.
    Hameau(usize),
    Terre(usize),
}

/// Combien de bâtiments de chaque type, et combien de hameaux textiles.
#[derive(Clone, Debug)]
pub struct Composition {
    /// Par type de bâtiment (l'atelier est compté à raison d'un par hameau textile).
    pub batiments: Vec<usize>,
    pub hameaux_textiles: usize,
}

/// Dimensionne un village de `nb_hameaux` hameaux pleins : d'abord le nombre de hameaux
/// textiles nécessaires pour vêtir tout le monde, puis les terres, en remontant les chaînes
/// de production (intrants compris).
pub fn composition(cat: &Catalogue, nb_hameaux: usize) -> Composition {
    let nr = cat.ressources.len();
    let normal = 0;
    let textile = cat.habitats.iter().position(|h| h.atelier.is_some());
    let capacite = |t: usize| {
        cat.habitats[normal].capacite_pops as i64 * (nb_hameaux - t) as i64
            + textile.map_or(0, |h| cat.habitats[h].capacite_pops as i64 * t as i64)
    };
    let demande_de = |r: usize, t: usize| cat.ration_pop(r) as f64 * capacite(t) as f64;

    // Plus petit nombre de hameaux textiles dont les ateliers couvrent les besoins.
    let mut t_textiles = 0;
    if let Some(h) = textile {
        let atelier = cat.habitats[h].atelier.unwrap();
        let finaux = cat.batiments[atelier].extrants_finaux();
        t_textiles = (0..=nb_hameaux)
            .find(|&t| {
                finaux
                    .iter()
                    .all(|&(r, q)| (q * t as i64) as f64 >= demande_de(r, t))
            })
            .unwrap_or(nb_hameaux);
    }

    let mut demande = vec![0f64; nr];
    for b in &cat.besoins {
        demande[b.ressource] = demande_de(b.ressource, t_textiles);
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
            t_textiles
        } else {
            compte[t].max(besoin.round().max(1.0) as usize)
        };
        for (i, q_in) in cat.batiments[t].intrants_externes() {
            demande[i] += besoin * q_in as f64;
        }
    }
    Composition {
        batiments: compte,
        hameaux_textiles: t_textiles,
    }
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
    const CELLULE: usize = 6;
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
        equipe: 0,
        chantier: None,
        stock_entree: vec![0; nr],
        stock_sortie: vec![0; nr],
        presents: 0,
        prod_ppm: 0,
        limitant: Limitant::Aucun,
    }
}

fn poser(monde: &mut Monde, cat: &Catalogue, item: Item, t: usize, rng: &mut Rng) {
    match item {
        Item::Hameau(h) => {
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

fn essayer(cat: &Catalogue, p: &ParamsVillage, rng: &mut Rng) -> Result<Simulation, String> {
    // --- Éléments à placer ---
    let compo = composition(cat, p.nb_hameaux);
    let textile = cat.habitats.iter().position(|h| h.atelier.is_some());
    let mut items: Vec<Item> = (0..p.nb_hameaux - compo.hameaux_textiles)
        .map(|_| Item::Hameau(0))
        .collect();
    if let Some(h) = textile {
        items.extend((0..compo.hameaux_textiles).map(|_| Item::Hameau(h)));
    }
    for (t, &n) in compo.batiments.iter().enumerate() {
        let est_atelier = cat.habitats.iter().any(|h| h.atelier == Some(t));
        if !est_atelier && !cat.batiments[t].chantier {
            items.extend((0..n).map(|_| Item::Terre(t)));
        }
    }
    rng.melanger(&mut items);

    // --- Disposition en croix ---
    let (w, h) = (p.largeur, p.hauteur);
    let n_items = items.len();
    let mut l_base = (n_items * 14).div_ceil(80).max(3);
    let (cx, cy) = (w / 2, h / 2);
    if cx < l_base + 6 || cy < l_base + 6 {
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
            variante: rng.dans(12) as u8,
            fertilite_ppm: fertilite[i],
        })
        .collect();

    // Bras : E, S, O, N. Longueurs tirées autour de `l_base`, agrandies si les emplacements manquent.
    let dirs: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    let mut longueurs = [0usize; 4];
    for l in longueurs.iter_mut() {
        *l = (l_base + rng.dans(3)).saturating_sub(1).max(2);
    }
    while longueurs.iter().sum::<usize>() * 2 < n_items + n_items / 4 + 2 {
        l_base += 1;
        for l in longueurs.iter_mut() {
            *l += 1;
        }
    }
    let mut aretes: Vec<(usize, usize)> = Vec::new();
    let centre = monde.idx(cx, cy);
    monde.tuiles[centre].contenu = Contenu::Chemin { masque: 0 };
    // Emplacements : (tile libre, tile du bras où s'accrocher, direction vers l'extérieur du bras).
    let mut slots: Vec<(usize, usize, (i32, i32))> = Vec::new();
    for (d, &(dx, dy)) in dirs.iter().enumerate() {
        let mut prec = centre;
        for k in 1..=longueurs[d] as i32 {
            let (x, y) = ((cx as i32 + dx * k) as usize, (cy as i32 + dy * k) as usize);
            let t = monde.idx(x, y);
            monde.tuiles[t].contenu = Contenu::Chemin { masque: 0 };
            aretes.push((prec, t));
            prec = t;
        }
    }
    for (d, &(dx, dy)) in dirs.iter().enumerate() {
        for k in 1..=longueurs[d] as i32 {
            let (x, y) = ((cx as i32 + dx * k) as usize, (cy as i32 + dy * k) as usize);
            let arm = monde.idx(x, y);
            for (px, py) in [(-dy, dx), (dy, -dx)] {
                let (sx, sy) = (x as i32 + px, y as i32 + py);
                let s = monde.idx(sx as usize, sy as usize);
                if monde.tuiles[s].contenu == Contenu::Vide && !slots.iter().any(|e| e.0 == s) {
                    slots.push((s, arm, (px, py)));
                }
            }
        }
    }
    for item in items {
        let mut place = false;
        if rng.chance(30) && !slots.is_empty() {
            // Antenne d'une tile : l'emplacement devient chemin, le lieu est posé au-delà.
            let i = rng.dans(slots.len());
            let (s, arm, (px, py)) = slots[i];
            let (sx, sy) = monde.xy(s);
            let (ox, oy) = (sx as i32 + px, sy as i32 + py);
            if ox >= 0 && oy >= 0 && (ox as usize) < w && (oy as usize) < h {
                let o = monde.idx(ox as usize, oy as usize);
                if monde.tuiles[o].contenu == Contenu::Vide
                    && monde.tuiles[s].contenu == Contenu::Vide
                {
                    monde.tuiles[s].contenu = Contenu::Chemin { masque: 0 };
                    aretes.push((arm, s));
                    poser(&mut monde, cat, item, o, rng);
                    aretes.push((s, o));
                    slots.remove(i);
                    place = true;
                }
            }
        }
        if !place {
            let libres: Vec<usize> = (0..slots.len())
                .filter(|&i| monde.tuiles[slots[i].0].contenu == Contenu::Vide)
                .collect();
            if libres.is_empty() {
                return Err("plus d'emplacement le long de la croix".into());
            }
            let i = libres[rng.dans(libres.len())];
            let (s, arm, _) = slots[i];
            poser(&mut monde, cat, item, s, rng);
            aretes.push((arm, s));
            slots.remove(i);
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

    // --- Affectation aux postes (proportionnelle, pops les plus proches d'abord) ---
    let postes: Vec<u32> = (0..nb)
        .map(|b| cat.batiments[monde.batiments[b].type_id].pops)
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
    // Les ateliers d'abord : leur pop est celle du hameau (distance 0).
    let mut ordre_bat: Vec<usize> = (0..nb)
        .filter(|&b| monde.batiments[b].quartier.is_some())
        .collect();
    ordre_bat.extend((0..nb).filter(|&b| monde.batiments[b].quartier.is_none()));
    for &b in &ordre_bat {
        for _ in 0..cibles[b] {
            let Some(q) = (0..nq)
                .filter(|&q| restantes[q] > 0)
                .filter_map(|q| reseau.chemin(q, nq + b).map(|c| ((c.cout, q), q)))
                .min()
                .map(|(_, q)| q)
            else {
                break;
            };
            restantes[q] -= 1;
            pops.push(nouvelle(q, Some(b), rng));
        }
    }
    for q in 0..nq {
        for _ in 0..restantes[q] {
            pops.push(nouvelle(q, None, rng));
        }
    }
    Ok(Simulation::nouvelle(cat.clone(), monde, pops))
}
