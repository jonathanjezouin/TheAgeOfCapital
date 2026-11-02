//! Graphe de transport : arêtes entre tiles voisines (8 directions, coûts entiers 10 / 14),
//! chemins précalculés entre les « lieux » (quartiers puis bâtiments) par Dijkstra borné
//! (docs/02 §9.1).

use crate::donnees::{Catalogue, Qte};
use crate::monde::{cout_dir, Monde, DIRS};
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
    /// Coût total en unités (10 par côté, 14 par diagonale) ; 0 sur une même tile.
    pub cout: i64,
    pub segments: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Reseau {
    pub segments: Vec<Segment>,
    /// Nombre de lieux : quartiers (0..nq) puis bâtiments (nq..).
    pub nb_lieux: usize,
    chemins: Vec<Option<Chemin>>,
    /// Voisins reliés de chaque tile : (tile, segment).
    adj: Vec<Vec<(usize, usize)>>,
    /// Tiles du réseau par lesquelles on accède à chaque lieu (sa propre tile, si elle est desservie).
    acces_lieux: Vec<Vec<usize>>,
}

const AUCUN: usize = usize::MAX;

/// Dijkstra à coûts entiers depuis plusieurs sources, borné par `limite` (inclus).
/// `dist` et `prev` sont réutilisables d'un appel à l'autre : seules les tiles touchées sont réinitialisées
/// par l'appelant via `touchees`.
fn dijkstra(
    adj: &[Vec<(usize, usize)>],
    segments: &[Segment],
    sources: &[usize],
    limite: i64,
    dist: &mut [i64],
    prev: &mut [(usize, usize)],
    touchees: &mut Vec<usize>,
) {
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
        for &(v, sg) in &adj[t] {
            let nc = c + segments[sg].cout;
            if nc > limite {
                continue;
            }
            if nc < dist[v] {
                if dist[v] == i64::MAX {
                    touchees.push(v);
                }
                dist[v] = nc;
                prev[v] = (t, sg);
                tas.push(Reverse((nc, v)));
            }
        }
    }
}

impl Reseau {
    pub fn chemin(&self, de: usize, vers: usize) -> Option<&Chemin> {
        self.chemins[de * self.nb_lieux + vers].as_ref()
    }

    /// Construit segments et chemins. Deux tiles sont reliées quand leurs demi-arêtes en
    /// vis-à-vis sont présentes des deux côtés ; un lieu est desservi si sa tile est sur le réseau.
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
        let tuile_lieu = |lieu: usize| -> usize {
            if lieu < nq {
                monde.quartiers[lieu].tuile
            } else {
                monde.batiments[lieu - nq].tuile
            }
        };
        let acces = |lieu: usize| -> Vec<usize> {
            if lieu >= nq {
                let bat = &monde.batiments[lieu - nq];
                // Chantier terminé sans suite (hameau sans atelier) : lieu inerte, hors du réseau.
                if cat.batiments[bat.type_id].chantier && bat.chantier.is_none() {
                    return Vec::new();
                }
            }
            let t = tuile_lieu(lieu);
            if monde.masque(t) != 0 {
                vec![t]
            } else {
                Vec::new()
            }
        };
        let acces_lieux: Vec<Vec<usize>> = (0..nb_lieux).map(acces).collect();

        let mut chemins: Vec<Option<Chemin>> = vec![None; nb_lieux * nb_lieux];
        // Tableaux réutilisés d'un lieu à l'autre : seul le voisinage est parcouru.
        let mut dist = vec![i64::MAX; n_tuiles];
        let mut prev = vec![(AUCUN, AUCUN); n_tuiles];
        let mut touchees: Vec<usize> = Vec::new();
        let limite = cat.infra.distance_max;
        for de in 0..nb_lieux {
            if acces_lieux[de].is_empty() {
                continue;
            }
            for &t in &touchees {
                dist[t] = i64::MAX;
                prev[t] = (AUCUN, AUCUN);
            }
            touchees.clear();
            dijkstra(
                &adj,
                &segments,
                &acces_lieux[de],
                limite,
                &mut dist,
                &mut prev,
                &mut touchees,
            );
            for vers in 0..nb_lieux {
                // Tile d'accès de la destination (une seule : sa propre tile).
                let Some(&cible) = acces_lieux[vers].iter().find(|&&t| dist[t] != i64::MAX) else {
                    continue;
                };
                let mut segs = Vec::new();
                let mut t = cible;
                while prev[t].0 != AUCUN {
                    segs.push(prev[t].1);
                    t = prev[t].0;
                }
                segs.reverse();
                chemins[de * nb_lieux + vers] = Some(Chemin {
                    cout: dist[cible],
                    segments: segs,
                });
            }
        }
        Reseau {
            segments,
            nb_lieux,
            chemins,
            adj,
            acces_lieux,
        }
    }

    /// Coût minimal (unités 10 / 14) depuis `source` jusqu'à chaque tile du réseau
    /// (`i64::MAX` si inaccessible).
    pub fn distances_depuis(&self, source: usize) -> Vec<i64> {
        self.distances_depuis_tuiles(&[source])
    }

    /// Tiles du réseau par lesquelles on accède au lieu.
    pub fn acces(&self, lieu: usize) -> &[usize] {
        &self.acces_lieux[lieu]
    }

    /// Comme `distances_depuis`, à partir de plusieurs tiles de départ.
    pub fn distances_depuis_tuiles(&self, sources: &[usize]) -> Vec<i64> {
        let mut d = vec![i64::MAX; self.adj.len()];
        let mut tas: BinaryHeap<Reverse<(i64, usize)>> = BinaryHeap::new();
        for &s in sources {
            d[s] = 0;
            tas.push(Reverse((0, s)));
        }
        while let Some(Reverse((c, t))) = tas.pop() {
            if c > d[t] {
                continue;
            }
            for &(v, sg) in &self.adj[t] {
                let nc = c + self.segments[sg].cout;
                if nc < d[v] {
                    d[v] = nc;
                    tas.push(Reverse((nc, v)));
                }
            }
        }
        d
    }
}

/// Tiles occupées par un lieu — utilitaire de test / d'affichage.
pub fn contenu_est_lieu(c: crate::monde::Contenu) -> bool {
    matches!(
        c,
        crate::monde::Contenu::Quartier(_) | crate::monde::Contenu::Batiment(_)
    )
}
