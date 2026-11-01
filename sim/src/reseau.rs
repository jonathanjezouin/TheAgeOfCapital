//! Graphe de transport : segments entre tiles traversables, chemins précalculés
//! entre les « lieux » (quartiers puis bâtiments).

use crate::donnees::{Catalogue, Qte};
use crate::monde::{Contenu, Monde, DIRS};
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
    pub cout: i64,
}

#[derive(Clone, Debug)]
pub struct Chemin {
    pub cout: i64,
    pub segments: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Reseau {
    pub segments: Vec<Segment>,
    /// Nombre de lieux : quartiers (0..nq) puis bâtiments (nq..).
    pub nb_lieux: usize,
    chemins: Vec<Option<Chemin>>,
}

const AUCUN: usize = usize::MAX;

impl Reseau {
    pub fn chemin(&self, de: usize, vers: usize) -> Option<&Chemin> {
        self.chemins[de * self.nb_lieux + vers].as_ref()
    }

    /// Construit segments et chemins. Deux tiles sont reliées quand leurs
    /// demi-directions en vis-à-vis sont présentes des deux côtés ; un bâtiment
    /// est relié à toute tile traversable voisine qui pointe vers lui.
    pub fn construire(monde: &Monde, cat: &Catalogue) -> Reseau {
        let n_tuiles = monde.tuiles.len();
        let mut segments: Vec<Segment> = Vec::new();
        let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n_tuiles];

        for idx in 0..n_tuiles {
            let Some(m) = monde.masque(idx) else { continue };
            for d in [1usize, 2] {
                // Est et Sud seulement : chaque segment est créé une fois.
                let (bit, _, _, opp) = DIRS[d];
                if m & bit == 0 {
                    continue;
                }
                let Some(v) = monde.voisin(idx, d) else {
                    continue;
                };
                let Some(mv) = monde.masque(v) else { continue };
                if mv & opp == 0 {
                    continue;
                }
                let s = segments.len();
                segments.push(Segment {
                    a: idx,
                    b: v,
                    cap_nominale: cat.infra.capacite_nominale,
                    cap_max: cat.infra.capacite_max,
                    charge: 0,
                    cout: cat.infra.cout_tuile,
                });
                adj[idx].push((v, s));
                adj[v].push((idx, s));
            }
        }

        let nq = monde.quartiers.len();
        let nb_lieux = nq + monde.batiments.len();
        let acces = |lieu: usize| -> Vec<usize> {
            if lieu < nq {
                return vec![monde.quartiers[lieu].tuile];
            }
            let t = monde.batiments[lieu - nq].tuile;
            // Bâtiment domestique : il partage la tile (et les rues) de son hameau.
            if matches!(monde.tuiles[t].contenu, Contenu::Quartier(_)) {
                return vec![t];
            }
            let mut v = Vec::new();
            for d in 0..4 {
                if let Some(n) = monde.voisin(t, d) {
                    if let Some(m) = monde.masque(n) {
                        // La tile voisine doit pointer vers le bâtiment.
                        if m & DIRS[d].3 != 0 {
                            v.push(n);
                        }
                    }
                }
            }
            v
        };
        let acces_lieux: Vec<Vec<usize>> = (0..nb_lieux).map(acces).collect();

        let mut chemins: Vec<Option<Chemin>> = vec![None; nb_lieux * nb_lieux];
        for de in 0..nb_lieux {
            if acces_lieux[de].is_empty() {
                continue;
            }
            let (dist, prev) = dijkstra(&acces_lieux[de], &adj, &segments, n_tuiles);
            for vers in 0..nb_lieux {
                // Meilleure tile d'accès de la destination (ex æquo : plus petit index).
                let Some(&cible) = acces_lieux[vers]
                    .iter()
                    .filter(|&&t| dist[t] != i64::MAX)
                    .min_by_key(|&&t| (dist[t], t))
                else {
                    continue;
                };
                if dist[cible] > cat.sim.rayon_max_cout {
                    continue;
                }
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
        }
    }
}

fn dijkstra(
    sources: &[usize],
    adj: &[Vec<(usize, usize)>],
    segments: &[Segment],
    n: usize,
) -> (Vec<i64>, Vec<(usize, usize)>) {
    let mut dist = vec![i64::MAX; n];
    let mut prev = vec![(AUCUN, AUCUN); n];
    let mut tas = BinaryHeap::new();
    for &s in sources {
        dist[s] = 0;
        tas.push(Reverse((0i64, s)));
    }
    while let Some(Reverse((d, t))) = tas.pop() {
        if d > dist[t] {
            continue;
        }
        for &(v, s) in &adj[t] {
            let nd = d + segments[s].cout;
            // Départage déterministe : à coût égal, on garde le premier trouvé (ordre des segments).
            if nd < dist[v] {
                dist[v] = nd;
                prev[v] = (t, s);
                tas.push(Reverse((nd, v)));
            }
        }
    }
    (dist, prev)
}

/// Tiles occupées par un lieu — utilitaire de test / d'affichage.
pub fn contenu_est_lieu(c: Contenu) -> bool {
    matches!(c, Contenu::Quartier(_) | Contenu::Batiment(_))
}
