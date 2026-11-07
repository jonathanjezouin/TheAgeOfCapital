//! Réseau de marche et de transport (docs/02 §9.1, docs/07 §18).
//!
//! On marche sur **toute tile praticable** (hors eau), dans 8 directions, coûts entiers 10 / 14. Une **route**
//! (arête posée entre deux tiles voisines) garde ce coût ; hors route un pas coûte `cout_hors_route` fois plus.
//! Un lieu (hameau, terre, chantier) est accessible par **n'importe laquelle de ses tiles** : il n'y a plus de
//! « tile nœud ». Les chemins entre lieux sont précalculés par Dijkstra borné par `distance_max`.

use crate::donnees::{Catalogue, Qte, PPM};
use crate::monde::{cout_dir, Contenu, Monde, DIRS};
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Debug, Serialize)]
pub struct Segment {
    pub a: usize,
    pub b: usize,
    pub cap_nominale: Qte,
    pub cap_max: Qte,
    pub charge: Qte,
    /// Coût de l'arête : 10 (orthogonale) ou 14 (diagonale).
    pub cout: i64,
}

#[derive(Clone, Debug)]
pub struct Chemin {
    /// Coût total en unités (10 par côté, 14 par diagonale sur route, plus cher hors route) ; 0 sur une même tile.
    pub cout: i64,
    /// Segments de route empruntés (la congestion ne porte que sur eux).
    pub segments: Vec<usize>,
    /// Tiles parcourues, de la tile de départ à la tile d'arrivée.
    pub tuiles: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Reseau {
    pub segments: Vec<Segment>,
    /// Nombre de lieux : quartiers (0..nq) puis bâtiments (nq..).
    pub nb_lieux: usize,
    chemins: Vec<Option<Chemin>>,
    /// Voisins reliés par une route : (tile, segment).
    adj: Vec<Vec<(usize, usize)>>,
    /// Tiles par lesquelles on accède à chaque lieu (toutes les siennes ; vide pour un site inerte).
    acces_lieux: Vec<Vec<usize>>,
    largeur: usize,
    hauteur: usize,
    /// Tile praticable à pied (ni eau, sauf pont).
    praticable: Vec<bool>,
    hors_route_ppm: i64,
    limite: i64,
}

const AUCUN: usize = usize::MAX;

impl Reseau {
    pub fn chemin(&self, de: usize, vers: usize) -> Option<&Chemin> {
        self.chemins[de * self.nb_lieux + vers].as_ref()
    }

    /// Coût du pas de `t` vers sa voisine `v` dans la direction `d` ; `None` si `v` n'est pas praticable. Retourne aussi
    /// le segment de route emprunté, le cas échéant.
    fn pas(&self, t: usize, d: usize, v: usize, hors_route_ppm: i64, ponts: bool) -> Option<(i64, Option<usize>)> {
        // `ponts` : estimation d'un site à relier par route, l'eau se franchit alors par un pont (trois fois le prix d'un pas).
        if !ponts && !self.praticable[v] && !self.adj[t].iter().any(|&(w, _)| w == v) {
            return None;
        }
        let base = cout_dir(d) * if ponts && !self.praticable[v] { 3 } else { 1 };
        match self.adj[t].iter().find(|&&(w, _)| w == v) {
            Some(&(_, sg)) => Some((self.segments[sg].cout, Some(sg))),
            None => {
                // Hors route : on ne traverse pas non plus une tile d'eau sans pont (ni le départ depuis l'eau).
                if !ponts && !self.praticable[t] {
                    return None;
                }
                Some((base * hors_route_ppm / PPM, None))
            }
        }
    }

    fn voisin(&self, t: usize, d: usize) -> Option<usize> {
        let (x, y) = (t % self.largeur, t / self.largeur);
        let nx = x as i32 + DIRS[d].1;
        let ny = y as i32 + DIRS[d].2;
        if nx < 0 || ny < 0 || nx >= self.largeur as i32 || ny >= self.hauteur as i32 {
            None
        } else {
            Some(ny as usize * self.largeur + nx as usize)
        }
    }

    /// Construit routes et chemins entre lieux (voir l'en-tête du module).
    pub fn construire(monde: &Monde, cat: &Catalogue) -> Reseau {
        let n_tuiles = monde.tuiles.len();
        let mut segments: Vec<Segment> = Vec::new();
        let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_tuiles];

        for idx in 0..n_tuiles {
            let m = monde.masque(idx);
            if m == 0 {
                continue;
            }
            for (d, &(bit, _, _, opp)) in DIRS.iter().enumerate() {
                if m & bit == 0 {
                    continue;
                }
                let Some(v) = monde.voisin(idx, d) else {
                    continue;
                };
                // Chaque segment est créé une fois (depuis la tile d'index le plus petit).
                if v < idx || monde.masque(v) & opp == 0 {
                    continue;
                }
                let s = segments.len();
                segments.push(Segment {
                    a: idx,
                    b: v,
                    cap_nominale: cat.infra.capacite_nominale,
                    cap_max: cat.infra.capacite_max,
                    charge: 0,
                    cout: cout_dir(d),
                });
                adj[idx].push((v, s));
                adj[v].push((idx, s));
            }
        }

        let nq = monde.quartiers.len();
        let nb_lieux = nq + monde.batiments.len();
        let acces_lieux: Vec<Vec<usize>> = (0..nb_lieux)
            .map(|lieu| {
                if lieu < nq {
                    if monde.quartiers[lieu].detruit {
                        return Vec::new();
                    }
                    return vec![monde.quartiers[lieu].tuile];
                }
                let bat = &monde.batiments[lieu - nq];
                // Chantier terminé sans suite (site inerte) : hors du réseau.
                if cat.batiments[bat.type_id].chantier && bat.chantier.is_none() {
                    return Vec::new();
                }
                if bat.emprise.is_empty() {
                    vec![bat.tuile]
                } else {
                    bat.emprise.clone()
                }
            })
            .collect();
        let mut praticable: Vec<bool> = monde.tuiles.iter().map(|t| t.eau == crate::terrain::Eau::Aucune).collect();
        // Une tile d'eau portant une route (un pont) est praticable par la route seulement : voir `pas`.
        for (i, t) in monde.tuiles.iter().enumerate() {
            if t.eau != crate::terrain::Eau::Aucune && t.aretes != 0 {
                praticable[i] = false;
            }
            let _ = Contenu::Vide;
        }
        let mut res = Reseau {
            segments,
            nb_lieux,
            chemins: vec![None; nb_lieux * nb_lieux],
            adj,
            acces_lieux,
            largeur: monde.largeur,
            hauteur: monde.hauteur,
            praticable,
            hors_route_ppm: cat.infra.cout_hors_route_ppm,
            limite: cat.infra.distance_max,
        };

        // Un Dijkstra par lieu, borné : seul le voisinage est parcouru.
        let mut dist = vec![i64::MAX; n_tuiles];
        let mut prev = vec![(AUCUN, AUCUN); n_tuiles];
        let mut touchees: Vec<usize> = Vec::new();
        // Un chantier se rejoint comme si le chemin était déjà tracé : l'équipe aménage son accès en y allant.
        let en_chantier: Vec<bool> = (0..nb_lieux).map(|l| l >= nq && monde.batiments[l - nq].chantier.is_some()).collect();
        let a_des_chantiers = en_chantier.iter().any(|&c| c);
        let mut dist2 = vec![i64::MAX; n_tuiles];
        let mut prev2 = vec![(AUCUN, AUCUN); n_tuiles];
        let mut touchees2: Vec<usize> = Vec::new();
        for de in 0..nb_lieux {
            if res.acces_lieux[de].is_empty() {
                continue;
            }
            for &t in &touchees {
                dist[t] = i64::MAX;
                prev[t] = (AUCUN, AUCUN);
            }
            touchees.clear();
            let hr = res.hors_route_ppm;
            res.dijkstra(&res.acces_lieux[de].clone(), &mut dist, &mut prev, &mut touchees, hr, false);
            if a_des_chantiers {
                for &t in &touchees2 {
                    dist2[t] = i64::MAX;
                    prev2[t] = (AUCUN, AUCUN);
                }
                touchees2.clear();
                res.dijkstra(&res.acces_lieux[de].clone(), &mut dist2, &mut prev2, &mut touchees2, PPM, false);
            }
            for vers in 0..nb_lieux {
                let (dist, prev) = if en_chantier[vers] || en_chantier[de] { (&dist2, &prev2) } else { (&dist, &prev) };
                // Tile d'arrivée : la plus proche parmi celles du lieu.
                let Some(&cible) = res.acces_lieux[vers]
                    .iter()
                    .filter(|&&t| dist[t] != i64::MAX)
                    .min_by_key(|&&t| (dist[t], t))
                else {
                    continue;
                };
                let (mut segs, mut tuiles) = (Vec::new(), vec![cible]);
                let mut t = cible;
                while prev[t].0 != AUCUN {
                    if prev[t].1 != AUCUN {
                        segs.push(prev[t].1);
                    }
                    t = prev[t].0;
                    tuiles.push(t);
                }
                segs.reverse();
                tuiles.reverse();
                res.chemins[de * nb_lieux + vers] = Some(Chemin { cout: dist[cible], segments: segs, tuiles });
            }
        }
        res
    }

    /// Dijkstra depuis plusieurs sources, borné par `limite` ; renseigne `dist` et `prev` (tile précédente, segment).
    fn dijkstra(&self, sources: &[usize], dist: &mut [i64], prev: &mut [(usize, usize)], touchees: &mut Vec<usize>, hors_route_ppm: i64, ponts: bool) {
        let mut tas: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        for &s in sources {
            if dist[s] != 0 {
                dist[s] = 0;
                touchees.push(s);
                tas.push(Reverse((0, s)));
            }
        }
        while let Some(Reverse((c, t))) = tas.pop() {
            if c > dist[t] {
                continue;
            }
            for d in 0..8 {
                let Some(v) = self.voisin(t, d) else { continue };
                let Some((cout, sg)) = self.pas(t, d, v, hors_route_ppm, ponts) else { continue };
                let nc = c + cout;
                if nc > self.limite {
                    continue;
                }
                if nc < dist[v] {
                    if dist[v] == i64::MAX {
                        touchees.push(v);
                    }
                    dist[v] = nc;
                    prev[v] = (t, sg.unwrap_or(AUCUN));
                    tas.push(Reverse((nc, v)));
                }
            }
        }
    }

    /// Coût minimal (unités 10 / 14, hors route plus cher) depuis `source` jusqu'à chaque tile atteignable dans
    /// `distance_max` (`i64::MAX` sinon).
    pub fn distances_depuis(&self, source: usize) -> Vec<i64> {
        self.distances_depuis_tuiles(&[source])
    }

    /// Tiles par lesquelles on accède au lieu (les siennes).
    pub fn acces(&self, lieu: usize) -> &[usize] {
        &self.acces_lieux[lieu]
    }

    /// Comme `distances_depuis`, à partir de plusieurs tiles de départ.
    pub fn distances_depuis_tuiles(&self, sources: &[usize]) -> Vec<i64> {
        self.distances_avec(sources, self.hors_route_ppm, false)
    }

    /// Distances **comme si les routes étaient construites partout** (un pas hors route coûte comme un pas sur route) :
    /// ce que le seigneur vise quand il choisit un site qu'il compte relier par une route (docs/07 §18).
    pub fn distances_si_route(&self, sources: &[usize]) -> Vec<i64> {
        self.distances_avec(sources, PPM, true)
    }

    /// Distances vers un chantier : l'équipe aménage son accès en y allant (un pas hors route coûte comme sur route).
    pub fn distances_chantier(&self, source: usize) -> Vec<i64> {
        self.distances_avec(&[source], PPM, false)
    }

    fn distances_avec(&self, sources: &[usize], hors_route_ppm: i64, ponts: bool) -> Vec<i64> {
        let mut d = vec![i64::MAX; self.adj.len()];
        let mut prev = vec![(AUCUN, AUCUN); self.adj.len()];
        let mut touchees = Vec::new();
        self.dijkstra(sources, &mut d, &mut prev, &mut touchees, hors_route_ppm, ponts);
        d
    }

    /// La tile `t` est-elle sur une route (au moins une arête) ?
    pub fn sur_route(&self, t: usize) -> bool {
        !self.adj[t].is_empty()
    }
}

/// Tiles occupées par un lieu — utilitaire de test / d'affichage.
pub fn contenu_est_lieu(c: crate::monde::Contenu) -> bool {
    matches!(
        c,
        crate::monde::Contenu::Quartier(_) | crate::monde::Contenu::Batiment(_)
    )
}
