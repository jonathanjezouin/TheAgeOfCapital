//! Graphe de transport : segments entre tiles traversables, chemins précalculés
//! entre les « lieux » (quartiers puis bâtiments).

use crate::donnees::{Catalogue, Qte};
use crate::monde::{Contenu, Monde, DIRS};
use serde::Serialize;

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
    /// Voisins traversables de chaque tile : (tile, segment).
    adj: Vec<Vec<(usize, usize)>>,
    /// Tiles traversables par lesquelles on accède à chaque lieu.
    acces_lieux: Vec<Vec<usize>>,
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
                    cout: 1,
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
            let bat = &monde.batiments[lieu - nq];
            let t = bat.tuile;
            // Chantier terminé sans suite (hameau sans atelier) : lieu inerte, hors du réseau.
            if cat.batiments[bat.type_id].chantier && bat.chantier.is_none() {
                return Vec::new();
            }
            // Atelier embarqué : il partage la tile (et les rues) de son hameau.
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
        let tuile_lieu = |lieu: usize| -> usize {
            if lieu < nq {
                monde.quartiers[lieu].tuile
            } else {
                monde.batiments[lieu - nq].tuile
            }
        };

        let mut chemins: Vec<Option<Chemin>> = vec![None; nb_lieux * nb_lieux];
        // Recherche en largeur bornée par la distance limite (tous les segments coûtent 1), avec
        // des tableaux réutilisés d'un lieu à l'autre : seul le voisinage est parcouru.
        let mut dist = vec![i64::MAX; n_tuiles];
        let mut prev = vec![(AUCUN, AUCUN); n_tuiles];
        let mut touchees: Vec<usize> = Vec::new();
        let mut file = std::collections::VecDeque::new();
        let aretes_max = cat.infra.distance_max - 1;
        for de in 0..nb_lieux {
            if acces_lieux[de].is_empty() {
                continue;
            }
            for &t in &touchees {
                dist[t] = i64::MAX;
                prev[t] = (AUCUN, AUCUN);
            }
            touchees.clear();
            file.clear();
            for &t in &acces_lieux[de] {
                if dist[t] == i64::MAX {
                    dist[t] = 0;
                    touchees.push(t);
                    file.push_back(t);
                }
            }
            while let Some(t) = file.pop_front() {
                if dist[t] >= aretes_max {
                    continue;
                }
                for &(v, sg) in &adj[t] {
                    if dist[v] == i64::MAX {
                        dist[v] = dist[t] + 1;
                        prev[v] = (t, sg);
                        touchees.push(v);
                        file.push_back(v);
                    }
                }
            }
            for vers in 0..nb_lieux {
                // Meilleure tile d'accès de la destination (ex æquo : plus petit index).
                let Some(&cible) = acces_lieux[vers]
                    .iter()
                    .filter(|&&t| dist[t] != i64::MAX)
                    .min_by_key(|&&t| (dist[t], t))
                else {
                    continue;
                };
                // Distance = nombre de tiles chemin empruntées : arêtes + 1, ou 0 sur une même tile.
                let distance = if tuile_lieu(de) == tuile_lieu(vers) {
                    0
                } else {
                    dist[cible] + 1
                };
                if distance > cat.infra.distance_max {
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
                    cout: distance,
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

    /// Nombre de segments à franchir depuis `source` jusqu'à chaque tile traversable
    /// (`i64::MAX` si inaccessible).
    pub fn distances_depuis(&self, source: usize) -> Vec<i64> {
        self.distances_depuis_tuiles(&[source])
    }

    /// Tiles traversables par lesquelles on accède au lieu.
    pub fn acces(&self, lieu: usize) -> &[usize] {
        &self.acces_lieux[lieu]
    }

    /// Comme `distances_depuis`, à partir de plusieurs tiles de départ.
    pub fn distances_depuis_tuiles(&self, sources: &[usize]) -> Vec<i64> {
        let mut d = vec![i64::MAX; self.adj.len()];
        let mut file = std::collections::VecDeque::new();
        for &source in sources {
            d[source] = 0;
            file.push_back(source);
        }
        while let Some(t) = file.pop_front() {
            for &(v, _) in &self.adj[t] {
                if d[v] == i64::MAX {
                    d[v] = d[t] + 1;
                    file.push_back(v);
                }
            }
        }
        d
    }
}

/// Tiles occupées par un lieu — utilitaire de test / d'affichage.
pub fn contenu_est_lieu(c: Contenu) -> bool {
    matches!(c, Contenu::Quartier(_) | Contenu::Batiment(_))
}
