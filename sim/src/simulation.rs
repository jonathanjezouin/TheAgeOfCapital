//! Boucle quotidienne (voir docs/02 §5) et démographie annuelle (docs/02 §6).

use crate::donnees::{mul_div, Catalogue, Qte, PPM};
use crate::monde::{Limitant, Monde};
use crate::reseau::Reseau;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Pop {
    pub id: u32,
    /// Index du quartier de résidence.
    pub residence: usize,
    /// Index du bâtiment d'emploi (None = chômeuse).
    pub emploi: Option<usize>,
}

/// Marchandise partie du stock du fournisseur mais pas encore arrivée (congestion).
#[derive(Clone, Debug, Serialize)]
pub struct Differe {
    pub fournisseur: usize,
    pub client: usize,
    pub ressource: usize,
    pub qte: Qte,
}

/// Compteurs cumulés pour vérifier la conservation de la matière.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Bilan {
    pub initial: Vec<Qte>,
    pub produit: Vec<Qte>,
    pub consomme_pops: Vec<Qte>,
    pub consomme_prod: Vec<Qte>,
    pub perime: Vec<Qte>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StatsJour {
    pub produit: Vec<Qte>,
    pub consomme_pops: Vec<Qte>,
    pub nb_differes: usize,
    pub qte_differee: Qte,
    pub retards_pops: u32,
    /// Charge maximale d'un segment / capacité nominale, en ppm.
    pub charge_max_ppm: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResumeAn {
    pub annee: u64,
    pub pops: usize,
    pub chomeurs: usize,
    pub naissances: u32,
    pub deces: u32,
    pub sat_moy_ppm: i64,
    pub sat_nourriture_moy_ppm: i64,
}

#[derive(Clone, Debug)]
pub struct Simulation {
    pub cat: Catalogue,
    pub monde: Monde,
    pub reseau: Reseau,
    pub pops: Vec<Pop>,
    pub prochain_pop_id: u32,
    pub jour: u64,
    pub differes: Vec<Differe>,
    pub bilan: Bilan,
    pub stats: StatsJour,
    pub historique: Vec<ResumeAn>,
    /// `fournisseurs[lieu_client][ressource]` = (coût, lieu fournisseur), du plus proche au plus loin.
    fournisseurs: Vec<Vec<Vec<(i64, usize)>>>,
}

/// Répartit `dispo` entre des demandes au prorata (rationnement), sans biais d'ordre :
/// parts entières, le reste attribué aux plus grandes fractions (ex æquo : premier indice).
pub fn repartir_pro_rata(dispo: Qte, demandes: &[Qte]) -> Vec<Qte> {
    let total: Qte = demandes.iter().sum();
    if total <= dispo {
        return demandes.to_vec();
    }
    if dispo <= 0 || total <= 0 {
        return vec![0; demandes.len()];
    }
    let mut parts: Vec<Qte> = demandes.iter().map(|&d| mul_div(dispo, d, total)).collect();
    let mut restes: Vec<(i128, usize)> = demandes
        .iter()
        .enumerate()
        .map(|(i, &d)| ((dispo as i128 * d as i128) % total as i128, i))
        .collect();
    let mut reste = dispo - parts.iter().sum::<Qte>();
    restes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in restes {
        if reste == 0 {
            break;
        }
        parts[i] += 1;
        reste -= 1;
    }
    parts
}

impl Simulation {
    pub fn nouvelle(cat: Catalogue, monde: Monde, pops: Vec<Pop>) -> Simulation {
        let reseau = Reseau::construire(&monde, &cat);
        let nr = cat.ressources.len();
        let prochain_pop_id = pops.iter().map(|p| p.id + 1).max().unwrap_or(0);
        let mut sim = Simulation {
            cat,
            monde,
            reseau,
            pops,
            prochain_pop_id,
            jour: 0,
            differes: Vec::new(),
            bilan: Bilan {
                initial: vec![0; nr],
                produit: vec![0; nr],
                consomme_pops: vec![0; nr],
                consomme_prod: vec![0; nr],
                perime: vec![0; nr],
            },
            stats: StatsJour {
                produit: vec![0; nr],
                consomme_pops: vec![0; nr],
                ..Default::default()
            },
            historique: Vec::new(),
            fournisseurs: Vec::new(),
        };
        sim.calculer_fournisseurs();
        sim.bilan.initial = sim.stock_total();
        sim
    }

    fn nq(&self) -> usize {
        self.monde.quartiers.len()
    }

    fn calculer_fournisseurs(&mut self) {
        let nq = self.nq();
        let nr = self.cat.ressources.len();
        let nb_lieux = nq + self.monde.batiments.len();
        let mut f = vec![vec![Vec::new(); nr]; nb_lieux];
        for client in 0..nb_lieux {
            for res in 0..nr {
                let mut v: Vec<(i64, usize)> = Vec::new();
                for (b, bat) in self.monde.batiments.iter().enumerate() {
                    let produit = self.cat.batiments[bat.type_id]
                        .extrants
                        .iter()
                        .any(|(r, _)| *r == res);
                    if !produit || nq + b == client {
                        continue;
                    }
                    if let Some(ch) = self.reseau.chemin(client, nq + b) {
                        v.push((ch.cout, nq + b));
                    }
                }
                v.sort();
                f[client][res] = v;
            }
        }
        self.fournisseurs = f;
    }

    // ---------- Observation ----------

    /// Stock total par ressource : quartiers + bâtiments + marchandises en transit.
    pub fn stock_total(&self) -> Vec<Qte> {
        let mut t = vec![0; self.cat.ressources.len()];
        for q in &self.monde.quartiers {
            for (r, v) in q.stock.iter().enumerate() {
                t[r] += v;
            }
        }
        for b in &self.monde.batiments {
            for r in 0..t.len() {
                t[r] += b.stock_entree[r] + b.stock_sortie[r];
            }
        }
        for d in &self.differes {
            t[d.ressource] += d.qte;
        }
        t
    }

    /// Écart de conservation par ressource (doit être nul) :
    /// stock = initial + produit − consommé − intrants consommés − périmé.
    pub fn ecart_conservation(&self) -> Vec<Qte> {
        let s = self.stock_total();
        (0..s.len())
            .map(|r| {
                s[r] - (self.bilan.initial[r] + self.bilan.produit[r]
                    - self.bilan.consomme_pops[r]
                    - self.bilan.consomme_prod[r]
                    - self.bilan.perime[r])
            })
            .collect()
    }

    pub fn pops_du_quartier(&self, q: usize) -> usize {
        self.pops.iter().filter(|p| p.residence == q).count()
    }

    pub fn chomeurs(&self) -> usize {
        self.pops.iter().filter(|p| p.emploi.is_none()).count()
    }

    /// Effectif par bâtiment (pops affectées, présentes ou non).
    pub fn occupation(&self) -> Vec<u32> {
        let mut occ = vec![0u32; self.monde.batiments.len()];
        for p in &self.pops {
            if let Some(b) = p.emploi {
                occ[b] += 1;
            }
        }
        occ
    }

    pub fn capacite_stock_quartier(&self, res: usize) -> Qte {
        let requis = self.cat.besoin_pop(res) * self.cat.demo.capacite_quartier_pops as i64;
        Catalogue::jours_x(requis, self.cat.sim.jours_stock_quartier)
    }

    // ---------- Transport ----------

    /// Charge le chemin `de → vers` avec `qte`, dans la limite de la capacité max
    /// des segments. Retourne la quantité effectivement acheminée aujourd'hui.
    fn charger(&mut self, de: usize, vers: usize, qte: Qte) -> Qte {
        let Some(ch) = self.reseau.chemin(de, vers) else {
            return 0;
        };
        let segs = ch.segments.clone();
        let marge = segs
            .iter()
            .map(|&s| (self.reseau.segments[s].cap_max - self.reseau.segments[s].charge).max(0))
            .min()
            .unwrap_or(qte);
        let ok = qte.min(marge);
        for s in segs {
            self.reseau.segments[s].charge += ok;
        }
        ok
    }

    fn crediter(&mut self, client: usize, res: usize, qte: Qte) {
        let nq = self.nq();
        if client < nq {
            self.monde.quartiers[client].stock[res] += qte;
        } else {
            self.monde.batiments[client - nq].stock_entree[res] += qte;
        }
    }

    // ---------- Un jour ----------

    pub fn jour_suivant(&mut self) {
        let nr = self.cat.ressources.len();
        for s in &mut self.reseau.segments {
            s.charge = 0;
        }
        self.stats.produit = vec![0; nr];
        self.stats.consomme_pops = vec![0; nr];
        self.stats.retards_pops = 0;

        // 1. Livraison des différés de la veille (priorité sur les nouveaux flux).
        let anciens = std::mem::take(&mut self.differes);
        for mut d in anciens {
            let ok = self.charger(d.fournisseur, d.client, d.qte);
            if ok > 0 {
                self.crediter(d.client, d.ressource, ok);
                d.qte -= ok;
            }
            if d.qte > 0 {
                self.differes.push(d);
            }
        }

        // 2. Marchandises : appel d'offres local, rationnement au pro rata, transport.
        self.resoudre_contrats();

        // 3. Main-d'œuvre : trajets domicile-travail.
        self.resoudre_main_oeuvre();

        // 4. Production.
        self.produire();

        // 5. Consommation des pops (satisfaction).
        self.consommer();

        // 6. Péremption, statistiques.
        self.perimer();
        self.finaliser_jour();

        self.jour += 1;
        if self.jour % self.cat.sim.jours_par_an as u64 == 0 {
            self.fin_annee();
        }
    }

    fn resoudre_contrats(&mut self) {
        struct Demande {
            client: usize,
            res: usize,
            reste: Qte,
        }
        let nq = self.nq();
        let nb_lieux = nq + self.monde.batiments.len();

        // En transit par (client, ressource), pour ne pas commander deux fois.
        let mut transit: BTreeMap<(usize, usize), Qte> = BTreeMap::new();
        for d in &self.differes {
            *transit.entry((d.client, d.ressource)).or_default() += d.qte;
        }

        let mut demandes: Vec<Demande> = Vec::new();
        for client in 0..nb_lieux {
            let mut besoins: Vec<(usize, Qte, f64)> = Vec::new(); // (ressource, requis/jour, jours de cible)
            if client < nq {
                let n = self.pops_du_quartier(client) as i64;
                for b in &self.cat.besoins {
                    besoins.push((
                        b.ressource,
                        b.par_pop * n,
                        self.cat.sim.jours_stock_quartier,
                    ));
                }
            } else {
                let t = &self.cat.batiments[self.monde.batiments[client - nq].type_id];
                for &(r, q) in &t.intrants {
                    besoins.push((r, q, self.cat.sim.jours_stock_entree));
                }
            }
            besoins.sort_by_key(|b| b.0);
            for (res, requis, jours) in besoins {
                if requis <= 0 {
                    continue;
                }
                let stock = if client < nq {
                    self.monde.quartiers[client].stock[res]
                } else {
                    self.monde.batiments[client - nq].stock_entree[res]
                };
                let en_route = transit.get(&(client, res)).copied().unwrap_or(0);
                let cible = Catalogue::jours_x(requis, jours);
                let commande = (cible - stock - en_route).clamp(0, 2 * requis);
                if commande > 0 {
                    demandes.push(Demande {
                        client,
                        res,
                        reste: commande,
                    });
                }
            }
        }

        // Stock disponible chez chaque fournisseur.
        let mut dispo: Vec<Vec<Qte>> = self
            .monde
            .batiments
            .iter()
            .map(|b| b.stock_sortie.clone())
            .collect();
        let mut transferts: Vec<(usize, usize, usize, Qte)> = Vec::new(); // (fournisseur, client, res, qte)
        for k in 0..self.cat.sim.nb_rondes {
            let mut groupes: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
            for (i, d) in demandes.iter().enumerate() {
                if d.reste <= 0 {
                    continue;
                }
                if let Some(&(_, f)) = self.fournisseurs[d.client][d.res].get(k) {
                    groupes.entry((f, d.res)).or_default().push(i);
                }
            }
            if groupes.is_empty() {
                break;
            }
            for ((f, res), idxs) in groupes {
                let av = dispo[f - nq][res];
                let dem: Vec<Qte> = idxs.iter().map(|&i| demandes[i].reste).collect();
                let parts = repartir_pro_rata(av, &dem);
                for (j, &i) in idxs.iter().enumerate() {
                    if parts[j] > 0 {
                        transferts.push((f, demandes[i].client, res, parts[j]));
                        demandes[i].reste -= parts[j];
                        dispo[f - nq][res] -= parts[j];
                    }
                }
            }
        }

        // Transport : le stock du fournisseur est débité, l'arrivée dépend de la congestion.
        for (f, c, res, qte) in transferts {
            self.monde.batiments[f - nq].stock_sortie[res] -= qte;
            let ok = self.charger(f, c, qte);
            if ok > 0 {
                self.crediter(c, res, ok);
            }
            if ok < qte {
                self.differes.push(Differe {
                    fournisseur: f,
                    client: c,
                    ressource: res,
                    qte: qte - ok,
                });
            }
        }
    }

    fn resoudre_main_oeuvre(&mut self) {
        let nq = self.nq();
        for b in &mut self.monde.batiments {
            b.presents = 0;
        }
        let poids = self.cat.infra.poids_trajet_pop;
        for i in 0..self.pops.len() {
            let Some(emploi) = self.pops[i].emploi else {
                continue;
            };
            let res = self.pops[i].residence;
            let ok = self.charger(res, nq + emploi, poids);
            if ok >= poids {
                self.monde.batiments[emploi].presents += 1;
            } else {
                // La charge partielle n'est pas rendue : la pop n'est pas arrivée mais a encombré la route.
                self.stats.retards_pops += 1;
            }
        }
    }

    fn produire(&mut self) {
        let cat = &self.cat;
        for b in &mut self.monde.batiments {
            let t = &cat.batiments[b.type_id];
            let mut f = PPM;
            let mut limitant = Limitant::Aucun;
            let f_mo = b.presents as i64 * PPM / t.pops as i64;
            if f_mo < f {
                f = f_mo;
                limitant = Limitant::MainOeuvre;
            }
            for &(r, q) in &t.intrants {
                let f_in = (b.stock_entree[r] * PPM / q).min(PPM);
                if f_in < f {
                    f = f_in;
                    limitant = Limitant::Intrant(r);
                }
            }
            for &(r, q) in &t.extrants {
                let plafond = Catalogue::jours_x(q, cat.sim.jours_stock_sortie);
                let libre = (plafond - b.stock_sortie[r]).max(0);
                let f_out = (libre * PPM / q).min(PPM);
                if f_out < f {
                    f = f_out;
                    limitant = Limitant::StockPlein;
                }
            }
            let f = f.max(0);
            for &(r, q) in &t.intrants {
                let c = mul_div(q, f, PPM);
                b.stock_entree[r] -= c;
                self.bilan.consomme_prod[r] += c;
            }
            for &(r, q) in &t.extrants {
                let p = mul_div(q, f, PPM);
                b.stock_sortie[r] += p;
                self.bilan.produit[r] += p;
                self.stats.produit[r] += p;
            }
            b.prod_ppm = f;
            b.limitant = if f >= PPM { Limitant::Aucun } else { limitant };
        }
    }

    fn consommer(&mut self) {
        let nr = self.cat.ressources.len();
        for q in 0..self.nq() {
            let n = self.pops_du_quartier(q) as i64;
            let quartier = &mut self.monde.quartiers[q];
            quartier.jours_an += 1;
            if n == 0 {
                quartier.sat_jour_ppm = PPM;
                quartier.sat_nourriture_jour_ppm = PPM;
                quartier.sat_par_ressource_ppm = vec![PPM; nr];
                quartier.cumul_sat_ppm += PPM;
                quartier.cumul_sat_nourriture_ppm += PPM;
                continue;
            }
            let mut sat = 0i64;
            let mut sat_n = 0i64;
            let mut poids_n = 0i64;
            let mut par_res = vec![PPM; nr];
            for b in &self.cat.besoins {
                let requis = b.par_pop * n;
                let recu = quartier.stock[b.ressource].min(requis).max(0);
                quartier.stock[b.ressource] -= recu;
                self.bilan.consomme_pops[b.ressource] += recu;
                self.stats.consomme_pops[b.ressource] += recu;
                let s = if requis > 0 { recu * PPM / requis } else { PPM };
                par_res[b.ressource] = s;
                sat += mul_div(b.poids_ppm, s, PPM);
                if b.nourriture {
                    sat_n += mul_div(b.poids_ppm, s, PPM);
                    poids_n += b.poids_ppm;
                }
            }
            let sat_n = if poids_n > 0 {
                sat_n * PPM / poids_n
            } else {
                PPM
            };
            quartier.sat_jour_ppm = sat;
            quartier.sat_nourriture_jour_ppm = sat_n;
            quartier.sat_par_ressource_ppm = par_res;
            quartier.cumul_sat_ppm += sat;
            quartier.cumul_sat_nourriture_ppm += sat_n;
        }
    }

    fn perimer(&mut self) {
        let cat = &self.cat;
        let bilan = &mut self.bilan;
        let mut perdre = |stock: &mut Qte, r: usize| {
            let p = cat.ressources[r].peremption_ppm;
            if p > 0 && *stock > 0 {
                let perte = mul_div(*stock, p, PPM);
                *stock -= perte;
                bilan.perime[r] += perte;
            }
        };
        for q in &mut self.monde.quartiers {
            for r in 0..q.stock.len() {
                perdre(&mut q.stock[r], r);
            }
        }
        for b in &mut self.monde.batiments {
            for r in 0..b.stock_sortie.len() {
                perdre(&mut b.stock_sortie[r], r);
                perdre(&mut b.stock_entree[r], r);
            }
        }
    }

    fn finaliser_jour(&mut self) {
        self.stats.nb_differes = self.differes.len();
        self.stats.qte_differee = self.differes.iter().map(|d| d.qte).sum();
        self.stats.charge_max_ppm = self
            .reseau
            .segments
            .iter()
            .map(|s| s.charge * PPM / s.cap_nominale.max(1))
            .max()
            .unwrap_or(0);
    }

    // ---------- Démographie annuelle ----------

    /// Poste vacant le plus proche du quartier : (coût, taux de remplissage, index) minimal.
    fn meilleur_poste(&self, q: usize, occ: &[u32]) -> Option<usize> {
        let nq = self.nq();
        let mut best: Option<((i64, i64, usize), usize)> = None;
        for (b, bat) in self.monde.batiments.iter().enumerate() {
            let postes = self.cat.batiments[bat.type_id].pops;
            if occ[b] >= postes {
                continue;
            }
            let Some(ch) = self.reseau.chemin(q, nq + b) else {
                continue;
            };
            let cle = (ch.cout, occ[b] as i64 * 1000 / postes as i64, b);
            if best.map_or(true, |(k, _)| cle < k) {
                best = Some((cle, b));
            }
        }
        best.map(|(_, b)| b)
    }

    fn fin_annee(&mut self) {
        let nq = self.nq();
        let d = self.cat.demo.clone();
        let seuil_milli = d.habitants_par_pop * 1000;
        let mut naissances = 0u32;
        let mut deces = 0u32;
        let mut somme_sat = 0i64;
        let mut somme_sat_n = 0i64;
        let mut nb_q_actifs = 0i64;

        for q in 0..nq {
            let jours = self.monde.quartiers[q].jours_an.max(1);
            let sat = self.monde.quartiers[q].cumul_sat_ppm / jours;
            let sat_n = self.monde.quartiers[q].cumul_sat_nourriture_ppm / jours;
            {
                let qu = &mut self.monde.quartiers[q];
                qu.sat_annuelle_ppm = sat;
                qu.sat_nourriture_annuelle_ppm = sat_n;
                qu.cumul_sat_ppm = 0;
                qu.cumul_sat_nourriture_ppm = 0;
                qu.jours_an = 0;
            }
            let n = self.pops_du_quartier(q) as i64;
            if n > 0 {
                somme_sat += sat;
                somme_sat_n += sat_n;
                nb_q_actifs += 1;
            }

            // Taux annuels (ppm). Natalité : sat globale ; mortalité : sat nourriture seule.
            let nat = d.naissance_max_ppm
                * ((sat - d.seuil_sat_naissance_ppm).max(0) * PPM
                    / (PPM - d.seuil_sat_naissance_ppm))
                    .min(PPM)
                / PPM;
            let mort = d.mortalite_base_ppm
                + (d.mortalite_max_ppm - d.mortalite_base_ppm) * (PPM - sat_n) / PPM;
            let net = (nat - mort) * d.acceleration_ppm / PPM;
            let hab = n * d.habitants_par_pop;
            self.monde.quartiers[q].croissance_milli += mul_div(hab * 1000, net, PPM);

            while self.monde.quartiers[q].croissance_milli >= seuil_milli {
                if (self.pops_du_quartier(q) as u32) < d.capacite_quartier_pops {
                    let occ = self.occupation();
                    let emploi = self.meilleur_poste(q, &occ);
                    let id = self.prochain_pop_id;
                    self.prochain_pop_id += 1;
                    self.pops.push(Pop {
                        id,
                        residence: q,
                        emploi,
                    });
                    self.monde.quartiers[q].croissance_milli -= seuil_milli;
                    naissances += 1;
                } else {
                    self.monde.quartiers[q].croissance_milli = seuil_milli; // plafonné par le logement
                    break;
                }
            }
            while self.monde.quartiers[q].croissance_milli <= -seuil_milli {
                // Une chômeuse d'abord, sinon la dernière pop du quartier (identifiant le plus élevé).
                let cible = self
                    .pops
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.residence == q)
                    .max_by_key(|(_, p)| (p.emploi.is_none(), p.id))
                    .map(|(i, _)| i);
                match cible {
                    Some(i) => {
                        self.pops.remove(i);
                        self.monde.quartiers[q].croissance_milli += seuil_milli;
                        deces += 1;
                    }
                    None => {
                        self.monde.quartiers[q].croissance_milli = 0;
                        break;
                    }
                }
            }
        }

        // Les chômeuses prennent les postes devenus vacants.
        let mut occ = self.occupation();
        for i in 0..self.pops.len() {
            if self.pops[i].emploi.is_none() {
                if let Some(b) = self.meilleur_poste(self.pops[i].residence, &occ) {
                    self.pops[i].emploi = Some(b);
                    occ[b] += 1;
                }
            }
        }

        let annee = self.jour / self.cat.sim.jours_par_an as u64;
        self.historique.push(ResumeAn {
            annee,
            pops: self.pops.len(),
            chomeurs: self.chomeurs(),
            naissances,
            deces,
            sat_moy_ppm: if nb_q_actifs > 0 {
                somme_sat / nb_q_actifs
            } else {
                0
            },
            sat_nourriture_moy_ppm: if nb_q_actifs > 0 {
                somme_sat_n / nb_q_actifs
            } else {
                0
            },
        });
    }

    pub fn avancer(&mut self, jours: u64) {
        for _ in 0..jours {
            self.jour_suivant();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_rata_conserve_et_borne() {
        let p = repartir_pro_rata(100, &[60, 60, 30]);
        assert_eq!(p.iter().sum::<Qte>(), 100);
        assert!(p.iter().zip([60, 60, 30]).all(|(a, b)| *a <= b));
        // Pas de biais d'ordre : deux demandes égales reçoivent (à 1 près) la même part.
        let q = repartir_pro_rata(101, &[50, 50, 50]);
        assert_eq!(q.iter().sum::<Qte>(), 101);
        assert!(q.iter().max().unwrap() - q.iter().min().unwrap() <= 1);
    }

    #[test]
    fn pro_rata_suffisant_ou_vide() {
        assert_eq!(repartir_pro_rata(500, &[10, 20]), vec![10, 20]);
        assert_eq!(repartir_pro_rata(0, &[10, 20]), vec![0, 0]);
    }
}
