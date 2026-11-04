//! Générateur de terrain (docs/07 §3) : terrains, rivières et forêts d'une plaine à champs ouverts.
//!
//! Déterministe (graine) et entièrement en entiers. Une tile = 250 m : une carte de 200 × 200 tiles couvre
//! 50 km × 50 km (2 500 km²). Le générateur produit des **couches** à basse fréquence (altitude, humidité,
//! boisement, pierrosité, limon), puis **classe chaque tile** dans un terrain. Les parts de chaque terrain
//! sont des paramètres (`ParamsTerrain`), pas des règles de jeu.
//!
//! Les rivières descendent la pente de l'altitude, de proche en proche et par côtés (4-connexes) : une
//! rivière 4-connexe ne peut pas être traversée par une arête diagonale de route, donc la coupure est
//! étanche tant qu'il n'y a pas de pont (docs/07 §3.3).

use crate::monde::DIRS;
use crate::rng::Rng;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Terrain {
    Plaine = 0,
    /// Variante riche de la plaine (limons) : meilleurs rendements des céréales.
    PlaineRiche = 1,
    Rocailleuse = 2,
    Marecageuse = 3,
    Foret = 4,
}

impl Terrain {
    /// Tous les terrains, dans l'ordre de leur index (`terrain as usize`).
    pub const TOUS: [Terrain; 5] = [
        Terrain::Plaine,
        Terrain::PlaineRiche,
        Terrain::Rocailleuse,
        Terrain::Marecageuse,
        Terrain::Foret,
    ];

    /// Index d'un terrain d'après son nom (`plaine_riche`…), pour les fichiers de données.
    pub fn index_de(nom: &str) -> Option<usize> {
        Self::TOUS.iter().position(|t| t.nom() == nom)
    }

    pub fn nom(self) -> &'static str {
        match self {
            Terrain::Plaine => "plaine",
            Terrain::PlaineRiche => "plaine_riche",
            Terrain::Rocailleuse => "plaine_rocailleuse",
            Terrain::Marecageuse => "plaine_marecageuse",
            Terrain::Foret => "foret",
        }
    }

    /// Peut-on bâtir (hameau, terre exploitée, route) sur ce terrain dans l'état actuel du jeu ?
    /// Forêts (défrichement) et marais (drainage) restent fermés jusqu'à ce que ces chantiers existent
    /// (docs/07 §8, étape 3).
    pub fn constructible(self) -> bool {
        matches!(self, Terrain::Plaine | Terrain::PlaineRiche | Terrain::Rocailleuse)
    }
}

/// Gabarit du cours d'eau qui occupe une tile (cosmétique et structurant : coupure, point d'eau).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Eau {
    Aucune = 0,
    Ruisseau = 1,
    Riviere = 2,
    Fleuve = 3,
}

/// Paramètres du générateur. Les parts sont en % de la carte (hors cours d'eau).
#[derive(Clone, Debug)]
pub struct ParamsTerrain {
    pub graine: u64,
    pub largeur: usize,
    pub hauteur: usize,
    pub pct_marais: usize,
    pub pct_foret: usize,
    pub pct_rocailleuse: usize,
    pub pct_riche: usize,
    /// Affluents principaux (rivières) et ruisseaux, en plus du fleuve.
    pub nb_affluents: usize,
    pub nb_ruisseaux: usize,
    /// Tiles en amont (confluences comprises) à partir desquelles un cours est une rivière / un fleuve.
    pub seuil_riviere: u32,
    pub seuil_fleuve: u32,
}

impl ParamsTerrain {
    /// Valeurs réalistes pour une plaine du Bassin parisien vers 1750 : ~12 % de forêt, un fleuve, quelques
    /// rivières, des ruisseaux tous les 5 à 8 km.
    pub fn defaut(graine: u64, largeur: usize, hauteur: usize) -> Self {
        ParamsTerrain {
            graine,
            largeur,
            hauteur,
            pct_marais: 4,
            pct_foret: 12,
            pct_rocailleuse: 15,
            pct_riche: 26,
            nb_affluents: 3,
            nb_ruisseaux: 6,
            seuil_riviere: 70,
            seuil_fleuve: 240,
        }
    }
}

/// Résultat : un terrain, un gabarit d'eau et la direction de l'aval pour chaque tile.
#[derive(Clone, Debug)]
pub struct CarteTerrain {
    pub largeur: usize,
    pub hauteur: usize,
    pub terrain: Vec<Terrain>,
    pub eau: Vec<Eau>,
    /// Pour une tile d'eau : direction (index de `DIRS`, 0 / 2 / 4 / 6) vers la tile aval, ou 255 (embouchure
    /// au bord de la carte, ou pas d'eau).
    pub aval: Vec<u8>,
    /// Altitude relative (0 à 1000) : pas de relief rendu pour l'instant, utile aux couches suivantes.
    pub altitude: Vec<i16>,
}

impl CarteTerrain {
    pub fn constructible(&self, i: usize) -> bool {
        self.eau[i] == Eau::Aucune && self.terrain[i].constructible()
    }
}

// ---------- Bruit de valeur entier ----------

fn hachage(x: i64, y: i64, s: u64) -> i64 {
    let mut z = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ s.wrapping_mul(0x1656_67B1_9E37_79F9);
    z ^= z >> 32;
    z = z.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    z ^= z >> 32;
    (z % 1001) as i64
}

/// Bruit de valeur lissé (0 à 1000) sur une grille de pas `c` tiles.
fn bruit(x: i64, y: i64, c: i64, s: u64) -> i64 {
    let (gx, gy, fx, fy) = (x / c, y / c, x % c, y % c);
    let lisse = |f: i64| f * f * (3 * c - 2 * f) / (c * c);
    let (sx, sy) = (lisse(fx), lisse(fy));
    let v = |i: i64, j: i64| hachage(gx + i, gy + j, s);
    let haut = v(0, 0) * (c - sx) + v(1, 0) * sx;
    let bas = v(0, 1) * (c - sx) + v(1, 1) * sx;
    (haut * (c - sy) + bas * sy) / (c * c)
}

/// Trois octaves (c, c/2, c/4), 0 à 1000.
fn fractal(x: i64, y: i64, c: i64, s: u64) -> i64 {
    (4 * bruit(x, y, c, s) + 2 * bruit(x, y, (c / 2).max(2), s ^ 0xA5) + bruit(x, y, (c / 4).max(2), s ^ 0x5A)) / 7
}

/// Les `n` meilleures tiles de `candidats` d'après `score` (égalités : indice croissant).
fn meilleures(candidats: &[usize], score: &[i64], n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = candidats.to_vec();
    v.sort_by(|&a, &b| score[b].cmp(&score[a]).then(a.cmp(&b)));
    v.truncate(n);
    v
}

/// Distance (en côtés de tile, 4-connexe) à la tile d'eau la plus proche ; `u16::MAX` hors de portée.
pub fn distance_eau(largeur: usize, hauteur: usize, eau: &[Eau]) -> Vec<u16> {
    let mut d = vec![u16::MAX; largeur * hauteur];
    let mut file = std::collections::VecDeque::new();
    for (i, e) in eau.iter().enumerate() {
        if *e != Eau::Aucune {
            d[i] = 0;
            file.push_back(i);
        }
    }
    while let Some(i) = file.pop_front() {
        let (x, y) = (i % largeur, i / largeur);
        for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < 0 || ny < 0 || nx >= largeur as i64 || ny >= hauteur as i64 {
                continue;
            }
            let j = ny as usize * largeur + nx as usize;
            if d[j] == u16::MAX {
                d[j] = d[i] + 1;
                file.push_back(j);
            }
        }
    }
    d
}

pub fn generer_terrain(p: &ParamsTerrain) -> CarteTerrain {
    let (w, h) = (p.largeur, p.hauteur);
    let n = w * h;
    let mut rng = Rng::new(p.graine ^ 0x7E55_A1);
    let s = |k: u64| p.graine.wrapping_mul(0x9E37_79B9).wrapping_add(k * 0x1234_5678_9ABC);

    // --- Altitude : une pente générale (sens tiré au sort) et un relief doux ---
    const SENS: [(i64, i64); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];
    let (fx, fy) = SENS[rng.dans(8)];
    let (cx, cy) = (w as i64 / 2, h as i64 / 2);
    let alt_brute: Vec<i64> = (0..n)
        .map(|i| {
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let proj = (x - cx) * fx + (y - cy) * fy;
            -proj * 9 + fractal(x, y, 36, s(1)) * 420 / 1000 + fractal(x, y, 13, s(2)) * 80 / 1000
        })
        .collect();
    let (mn, mx) = (*alt_brute.iter().min().unwrap(), *alt_brute.iter().max().unwrap());
    let altitude: Vec<i16> = alt_brute.iter().map(|&a| ((a - mn) * 1000 / (mx - mn).max(1)) as i16).collect();

    // --- Rivières : descente de pente, par côtés, avec un peu de méandre ---
    let mut eau = vec![Eau::Aucune; n];
    let mut suivant: Vec<usize> = vec![usize::MAX; n]; // tile aval d'une tile d'eau
    // Écoulement : inondation par priorité depuis les bords (aucune cuvette : chaque tile a un parent plus bas,
    // qui mène toujours au bord de la carte). Un bruit fin sur l'altitude donne les méandres.
    let alt_fine: Vec<i64> = (0..n)
        .map(|i| alt_brute[i] + hachage((i % w) as i64, (i / w) as i64, s(40)) * 90 / 1000)
        .collect();
    let mut parent = vec![usize::MAX; n];
    {
        let mut rempli = vec![i64::MIN; n];
        let mut tas = std::collections::BinaryHeap::new();
        for i in 0..n {
            let (x, y) = (i % w, i / w);
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                rempli[i] = alt_fine[i];
                tas.push(std::cmp::Reverse((alt_fine[i], i)));
            }
        }
        while let Some(std::cmp::Reverse((a, i))) = tas.pop() {
            if a > rempli[i] {
                continue;
            }
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if rempli[j] != i64::MIN {
                    continue;
                }
                rempli[j] = alt_fine[j].max(rempli[i] + 1);
                parent[j] = i;
                tas.push(std::cmp::Reverse((rempli[j], j)));
            }
        }
    }
    // Un cours part de sa source, suit les parents jusqu'au bord de la carte ou jusqu'à un cours existant.
    let tracer = |depart: usize, eau: &mut Vec<Eau>, suivant: &mut Vec<usize>| {
        let mut cur = depart;
        loop {
            let deja = eau[cur] != Eau::Aucune;
            eau[cur] = Eau::Ruisseau;
            if deja || parent[cur] == usize::MAX {
                break;
            }
            suivant[cur] = parent[cur];
            cur = parent[cur];
        }
    };
    // Le fleuve part de la bande la plus haute ; affluents et ruisseaux, d'un peu partout en amont.
    let proj_de = |i: usize| ((i % w) as i64 - cx) * fx + ((i / w) as i64 - cy) * fy;
    let amont: Vec<usize> = {
        let mut v: Vec<usize> = (0..n).filter(|&i| {
            let (x, y) = (i % w, i / w);
            x > 4 && y > 4 && x < w - 5 && y < h - 5
        }).collect();
        v.sort_by_key(|&i| (proj_de(i), i));
        v
    };
    let pris = |amont: &[usize], frac_lo: usize, frac_hi: usize, rng: &mut Rng| -> usize {
        let (a, b) = (amont.len() * frac_lo / 100, amont.len() * frac_hi / 100);
        amont[a + rng.dans((b - a).max(1))]
    };
    let mut departs: Vec<usize> = vec![pris(&amont, 0, 12, &mut rng)]; // fleuve
    for _ in 0..p.nb_affluents {
        departs.push(pris(&amont, 5, 50, &mut rng));
    }
    for _ in 0..p.nb_ruisseaux {
        departs.push(pris(&amont, 8, 75, &mut rng));
    }
    for (k, &d) in departs.iter().enumerate() {
        // Un cours ne démarre pas à moins de 6 tiles d'un cours existant (sauf le premier).
        if k > 0 {
            let (x, y) = ((d % w) as i64, (d / w) as i64);
            let trop_pres = (-6i64..=6).any(|dy| (-6i64..=6).any(|dx| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 && eau[ny as usize * w + nx as usize] != Eau::Aucune
            }));
            if trop_pres {
                continue;
            }
        }
        tracer(d, &mut eau, &mut suivant);
    }

    // --- Gabarit : nombre de tiles d'eau en amont (accumulation, de l'amont vers l'aval) ---
    let mut entrants = vec![0u32; n];
    for i in 0..n {
        if suivant[i] != usize::MAX {
            entrants[suivant[i]] += 1;
        }
    }
    let mut amont_n = vec![1u32; n];
    let mut file: Vec<usize> = (0..n).filter(|&i| eau[i] != Eau::Aucune && entrants[i] == 0).collect();
    while let Some(i) = file.pop() {
        let j = suivant[i];
        if j == usize::MAX {
            continue;
        }
        amont_n[j] += amont_n[i];
        entrants[j] -= 1;
        if entrants[j] == 0 {
            file.push(j);
        }
    }
    for i in 0..n {
        if eau[i] != Eau::Aucune {
            eau[i] = if amont_n[i] >= p.seuil_fleuve {
                Eau::Fleuve
            } else if amont_n[i] >= p.seuil_riviere {
                Eau::Riviere
            } else {
                Eau::Ruisseau
            };
        }
    }
    let aval: Vec<u8> = (0..n)
        .map(|i| {
            if eau[i] == Eau::Aucune || suivant[i] == usize::MAX {
                return 255;
            }
            (0..8)
                .find(|&d| {
                    let (x, y) = ((i % w) as i64 + DIRS[d].1 as i64, (i / w) as i64 + DIRS[d].2 as i64);
                    x >= 0 && y >= 0 && (y as usize) * w + x as usize == suivant[i]
                })
                .map_or(255, |d| d as u8)
        })
        .collect();

    // --- Terrains : on retire les parts dans l'ordre marais, forêt, rocailleuse, riche ---
    let dist = distance_eau(w, h, &eau);
    let proche = |i: usize, seuils: &[(u16, i64)]| -> i64 {
        seuils.iter().find(|&&(d, _)| dist[i] <= d).map_or(0, |&(_, b)| b)
    };
    let xy = |i: usize| ((i % w) as i64, (i / w) as i64);
    let part = |pct: usize| n * pct / 100;
    let mut libres: Vec<usize> = (0..n).filter(|&i| eau[i] == Eau::Aucune).collect();
    let mut terrain = vec![Terrain::Plaine; n];

    let mut attribuer = |t: Terrain, score: &dyn Fn(usize) -> i64, pct: usize, libres: &mut Vec<usize>| {
        let sc: Vec<i64> = {
            let mut v = vec![i64::MIN; n];
            for &i in libres.iter() {
                v[i] = score(i);
            }
            v
        };
        let prises = meilleures(libres, &sc, part(pct));
        for &i in &prises {
            terrain[i] = t;
        }
        let set: std::collections::HashSet<usize> = prises.into_iter().collect();
        libres.retain(|i| !set.contains(i));
    };
    // Marais : bas-fonds humides près des cours d'eau, plus une région marécageuse (type Dombes).
    attribuer(
        Terrain::Marecageuse,
        &|i| {
            let (x, y) = xy(i);
            fractal(x, y, 12, s(3)) + fractal(x, y, 64, s(4)) * 6 / 10 + proche(i, &[(2, 250), (5, 140), (9, 60)])
        },
        p.pct_marais,
        &mut libres,
    );
    // Forêt : massifs (bruit à grande échelle) ; jamais sur les meilleurs limons à ce stade.
    attribuer(
        Terrain::Foret,
        &|i| {
            let (x, y) = xy(i);
            fractal(x, y, 9, s(5)) + fractal(x, y, 52, s(6)) * 5 / 10
        },
        p.pct_foret,
        &mut libres,
    );
    attribuer(
        Terrain::Rocailleuse,
        &|i| {
            let (x, y) = xy(i);
            fractal(x, y, 22, s(7)) + fractal(x, y, 70, s(8)) * 5 / 10
        },
        p.pct_rocailleuse,
        &mut libres,
    );
    // Plaine riche : limons, favorisés par les alluvions près des rivières.
    attribuer(
        Terrain::PlaineRiche,
        &|i| {
            let (x, y) = xy(i);
            fractal(x, y, 26, s(9)) + proche(i, &[(2, 220), (6, 110)])
        },
        p.pct_riche,
        &mut libres,
    );

    CarteTerrain { largeur: w, hauteur: h, terrain, eau, aval, altitude }
}

/// Choisit l'emplacement du village de départ : une fenêtre dégagée (ni eau, ni forêt, ni marais) de rayon
/// `rayon`, à bonne distance d'un point d'eau (ni les pieds dans l'eau, ni sans eau à portée), de préférence
/// sur de la plaine riche et près du centre de la carte. Renvoie `(x, y)`.
pub fn choisir_centre(c: &CarteTerrain, rayon: usize) -> Option<(usize, usize)> {
    candidats_centre(c, rayon, 1).first().map(|&(_, x, y)| (x, y))
}

/// Les `n` meilleurs sites de village, espacés d'au moins 12 tiles : `(score, x, y)`, meilleur d'abord.
pub fn candidats_centre(c: &CarteTerrain, rayon: usize, n: usize) -> Vec<(i64, usize, usize)> {
    let (w, h) = (c.largeur, c.hauteur);
    if w < 2 * rayon + 4 || h < 2 * rayon + 4 {
        return Vec::new();
    }
    // Image intégrale des tiles non constructibles.
    let stride = w + 1;
    let mut somme = vec![0i32; (w + 1) * (h + 1)];
    for y in 0..h {
        for x in 0..w {
            let bloque = i32::from(!c.constructible(y * w + x));
            somme[(y + 1) * stride + x + 1] = bloque + somme[y * stride + x + 1] + somme[(y + 1) * stride + x] - somme[y * stride + x];
        }
    }
    let rect = |x0: usize, y0: usize, x1: usize, y1: usize| {
        somme[y1 * stride + x1] - somme[y0 * stride + x1] - somme[y1 * stride + x0] + somme[y0 * stride + x0]
    };
    let dist = distance_eau(w, h, &c.eau);
    let r = rayon;
    let mut tous: Vec<(i64, usize, usize)> = Vec::new();
    let marge = (r + 2).max(22).min(w / 3).min(h / 3);
    for y in marge..(h - marge) {
        for x in marge..(w - marge) {
            if rect(x - r, y - r, x + r + 1, y + r + 1) != 0 {
                continue;
            }
            let d = dist[y * w + x] as i64;
            // La fenêtre dégagée exclut l'eau : le point d'eau le plus proche est donc au-delà de `rayon`. On le
            // veut juste au bord de la fenêtre (3 km environ) : à portée du terroir, sans couper le village.
            let hors_zone = if d <= r as i64 { 0 } else if d > r as i64 + 3 { d - r as i64 - 3 } else { 0 };
            let mut sc = -hors_zone * 14 - ((x as i64 - w as i64 / 2).abs() + (y as i64 - h as i64 / 2).abs()) / 6;
            // Plaine riche à portée (rayon 10) : un peu de bonus, échantillonné tous les 2 tiles.
            for dy in (-10i64..=10).step_by(2) {
                for dx in (-10i64..=10).step_by(2) {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx >= 0 && ny >= 0 && nx < w as i64 && ny < h as i64 {
                        match c.terrain[ny as usize * w + nx as usize] {
                            Terrain::PlaineRiche => sc += 1,
                            // Un village s'installe d'abord sur de bonnes terres : la rocaille à portée le pénalise.
                            Terrain::Rocailleuse => sc -= 3,
                            _ => {}
                        }
                    }
                }
            }
            tous.push((sc, x, y));
        }
    }
    tous.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut gardes: Vec<(i64, usize, usize)> = Vec::new();
    for t in tous {
        if gardes.len() == n {
            break;
        }
        if gardes.iter().all(|g| g.1.abs_diff(t.1).max(g.2.abs_diff(t.2)) >= 12) {
            gardes.push(t);
        }
    }
    gardes
}

/// Sites possibles du village de départ, du meilleur au pire : fenêtres dégagées de rayon 12 puis 10, celles qui ont
/// un point d'eau à moins de 17 tiles (4 km) d'abord. Renvoie `(rayon, x, y)`.
pub fn sites_village(c: &CarteTerrain) -> Vec<(usize, usize, usize)> {
    let dist = distance_eau(c.largeur, c.hauteur, &c.eau);
    let (mut avec_eau, mut sans_eau) = (Vec::new(), Vec::new());
    for r in [12usize, 10] {
        for (_, x, y) in candidats_centre(c, r, 6) {
            if dist[y * c.largeur + x] <= 17 {
                avec_eau.push((r, x, y));
            } else {
                sans_eau.push((r, x, y));
            }
        }
    }
    avec_eau.extend(sans_eau);
    avec_eau
}

/// Le meilleur site du village de départ (voir `sites_village`).
pub fn site_village(c: &CarteTerrain) -> Option<(usize, usize, usize)> {
    sites_village(c).first().copied()
}
