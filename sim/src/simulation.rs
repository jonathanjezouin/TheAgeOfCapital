//! Boucle quotidienne (voir docs/02 §5), démographie par jauge de pop (docs/02 §6)
//! et réorientation de la main-d'œuvre selon le besoin (docs/02 §3).

use crate::donnees::{mul_div, Catalogue, ParamsDemo, Qte, PPM};
use crate::monde::{Batiment, Chantier, Cible, Contenu, Limitant, Monde, Quartier, DIRS};
use crate::reseau::Reseau;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Pop {
    pub id: u32,
    /// Index du hameau de résidence.
    pub residence: usize,
    /// Index du bâtiment d'emploi (None = pop sans poste).
    pub emploi: Option<usize>,
    /// Jauge de croissance en micro-habitants : à +100 habitants la pop se duplique,
    /// à −100 elle disparaît. Alimentée chaque jour par la couverture des besoins.
    pub croissance: i64,
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
    pub sans_poste: usize,
    pub naissances: u32,
    pub deces: u32,
    pub vital_moy_ppm: i64,
    pub confort_moy_ppm: i64,
}

/// Fait marquant, pour le journal d'événements affiché au joueur.
#[derive(Clone, Debug, Serialize)]
pub struct Evenement {
    pub jour: u64,
    pub texte: String,
}

/// Emplacement choisi pour un nouveau lieu : tile libre, tile traversable où l'on s'accroche,
/// et éventuellement une tile de chemin à créer entre les deux.
#[derive(Clone, Copy, Debug)]
struct Site {
    tuile: usize,
    attache: usize,
    route: Option<usize>,
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
    pub naissances_an: u32,
    pub deces_an: u32,
    pub evenements: Vec<Evenement>,
    /// `fournisseurs[lieu_client][ressource]` = (coût, lieu fournisseur), du plus proche au plus loin.
    /// Un hameau est son propre fournisseur le plus proche (coût 0) : le producteur mange d'abord.
    fournisseurs: Vec<Vec<Vec<(i64, usize)>>>,
}

/// Résultat de l'évaluation des besoins d'un hameau sur une journée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    /// Besoins vitaux couverts (nourriture, chauffage) : min des deux, en ppm.
    pub vital_ppm: i64,
    /// Besoins de confort couverts (part de viande, vêtements) : min, en ppm.
    pub confort_ppm: i64,
    /// Couverture de la ration par ressource (affichage).
    pub par_ressource_ppm: Vec<i64>,
}

fn ratio(a: Qte, b: Qte) -> i64 {
    if b <= 0 {
        PPM
    } else {
        mul_div(a.max(0), PPM, b).min(PPM)
    }
}

/// Évalue ce qu'ont reçu `n` pops (`recu[r]` en milli-unités sur la journée).
/// Nourriture : grain, viande et fruit s'additionnent en unités nutritives.
pub fn evaluer_besoins(cat: &Catalogue, n: i64, recu: &[Qte]) -> Evaluation {
    let nr = cat.ressources.len();
    if n == 0 {
        return Evaluation {
            vital_ppm: PPM,
            confort_ppm: PPM,
            par_ressource_ppm: vec![PPM; nr],
        };
    }
    let nutrition: Qte = (0..nr)
        .map(|r| recu[r] * cat.ressources[r].nutritive_milli / 1000)
        .sum();
    let nourriture = ratio(nutrition, cat.vital.nourriture * n);
    let chauffage = ratio(recu[cat.vital.chauffage_ressource], cat.vital.chauffage * n);
    let confort = cat
        .confort
        .iter()
        .map(|c| ratio(recu[c.ressource], c.par_pop * n))
        .min()
        .unwrap_or(PPM);
    let mut par = vec![PPM; nr];
    for b in &cat.besoins {
        par[b.ressource] = ratio(recu[b.ressource], b.par_pop * n);
    }
    Evaluation {
        vital_ppm: nourriture.min(chauffage),
        confort_ppm: confort,
        par_ressource_ppm: par,
    }
}

/// Taux de croissance annuel (ppm) d'une pop.
/// - besoins vitaux couverts : de `croissance_insatisfaite` (confort nul) à `croissance_satisfaite` (confort plein) ;
/// - sinon : décroît linéairement jusqu'à −`mortalite_famine` quand les besoins vitaux sont nuls.
pub fn taux_croissance_ppm(d: &ParamsDemo, vital_ppm: i64, confort_ppm: i64) -> i64 {
    let bas = d.croissance_insatisfaite_ppm;
    let haut = d.croissance_satisfaite_ppm;
    let r = if vital_ppm >= PPM {
        bas + (haut - bas) * confort_ppm / PPM
    } else {
        bas - (bas + d.mortalite_famine_ppm) * (PPM - vital_ppm) / PPM
    };
    r * d.acceleration_ppm / PPM
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
            naissances_an: 0,
            deces_an: 0,
            evenements: Vec::new(),
            fournisseurs: Vec::new(),
        };
        sim.calculer_fournisseurs();
        sim.bilan.initial = sim.stock_total();
        sim
    }

    fn nq(&self) -> usize {
        self.monde.quartiers.len()
    }

    /// Postes d'un lieu : effectif de l'équipe pour un chantier, effectif du type sinon.
    pub fn postes_de(&self, b: usize) -> u32 {
        let bat = &self.monde.batiments[b];
        let t = &self.cat.batiments[bat.type_id];
        if t.chantier {
            bat.equipe
        } else {
            t.pops
        }
    }

    fn est_chantier_type(&self, b: usize) -> bool {
        self.cat.batiments[self.monde.batiments[b].type_id].chantier
    }

    fn journal(&mut self, texte: String) {
        if self.evenements.len() < 20_000 {
            self.evenements.push(Evenement {
                jour: self.jour,
                texte,
            });
        }
    }

    fn reconstruire_reseau(&mut self) {
        self.reseau = Reseau::construire(&self.monde, &self.cat);
        self.calculer_fournisseurs();
    }

    fn calculer_fournisseurs(&mut self) {
        let nq = self.nq();
        let nr = self.cat.ressources.len();
        let nb_lieux = nq + self.monde.batiments.len();
        // Producteurs de chaque ressource (lieux), calculés une fois.
        let producteurs: Vec<Vec<usize>> = (0..nr)
            .map(|r| {
                (0..self.monde.batiments.len())
                    .filter(|&b| self.cat.batiments[self.monde.batiments[b].type_id].livre(r))
                    .map(|b| nq + b)
                    .collect()
            })
            .collect();
        let mut f = vec![vec![Vec::new(); nr]; nb_lieux];
        for client in 0..nb_lieux {
            for res in 0..nr {
                let mut v: Vec<(i64, usize)> = producteurs[res]
                    .iter()
                    .filter(|&&p| p != client)
                    .filter_map(|&p| self.reseau.chemin(client, p).map(|ch| (ch.cout, p)))
                    .collect();
                v.sort();
                f[client][res] = v;
            }
        }
        self.fournisseurs = f;
    }

    // ---------- Observation ----------

    /// Stock total par ressource : hameaux + bâtiments + marchandises en transit.
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

    /// Nombre de pops résidant dans chaque hameau.
    pub fn effectifs(&self) -> Vec<u32> {
        let mut e = vec![0u32; self.nq()];
        for p in &self.pops {
            e[p.residence] += 1;
        }
        e
    }

    pub fn pops_du_quartier(&self, q: usize) -> usize {
        self.pops.iter().filter(|p| p.residence == q).count()
    }

    pub fn sans_poste(&self) -> usize {
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

    /// Seuil de la jauge de croissance, en micro-habitants.
    pub fn seuil_croissance(&self) -> i64 {
        self.cat.demo.habitants_par_pop * 1_000_000
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
        // 4. Production, avancement des chantiers.
        self.produire();
        self.terminer_chantiers();
        // 5. Consommation des pops et couverture des besoins.
        self.consommer();
        // 6. Jauges de croissance : naissances et disparitions de pops.
        self.croissance_quotidienne();
        // 7. Péremption, statistiques.
        self.perimer();
        self.finaliser_jour();

        self.jour += 1;
        if self.jour % self.cat.mobilite.periode_jours == 0 {
            self.reaffecter();
        }
        if self.jour % self.cat.extension.periode_jours == 0 {
            self.etendre();
        }
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
        let eff = self.effectifs();

        // En transit par (client, ressource), pour ne pas commander deux fois.
        let mut transit: BTreeMap<(usize, usize), Qte> = BTreeMap::new();
        for d in &self.differes {
            *transit.entry((d.client, d.ressource)).or_default() += d.qte;
        }

        let mut demandes: Vec<Demande> = Vec::new();
        for client in 0..nb_lieux {
            let mut besoins: Vec<(usize, Qte, f64)> = Vec::new(); // (ressource, requis/jour, jours de cible)
            if client < nq {
                for b in &self.cat.besoins {
                    besoins.push((
                        b.ressource,
                        b.par_pop * eff[client] as i64,
                        self.cat.sim.jours_stock_quartier,
                    ));
                }
            } else {
                let bat = &self.monde.batiments[client - nq];
                if let Some(ch) = &bat.chantier {
                    // Un chantier commande le matériau dont son équipe a besoin chaque jour.
                    let par_jour = mul_div(
                        ch.materiau_total,
                        bat.equipe as i64 * 1000,
                        ch.travail_total.max(1),
                    );
                    if par_jour > 0 {
                        besoins.push((
                            self.cat.extension.materiau,
                            par_jour,
                            self.cat.sim.jours_stock_entree,
                        ));
                    }
                } else {
                    for (r, q) in self.cat.batiments[bat.type_id].intrants_externes() {
                        besoins.push((r, q, self.cat.sim.jours_stock_entree));
                    }
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
        let tuiles = &self.monde.tuiles;
        for b in &mut self.monde.batiments {
            let t = &cat.batiments[b.type_id];
            if t.chantier {
                Self::avancer_chantier(b, cat, &mut self.bilan);
                continue;
            }
            // La fertilité de la tile module la production des terres exploitées.
            let fert = if t.fertile {
                tuiles[b.tuile].fertilite_ppm
            } else {
                PPM
            };
            let f_mo = b.presents as i64 * PPM / t.pops as i64;
            let mut f_min = PPM;
            let mut limitant = Limitant::Aucun;
            let mut f_dernier = 0;
            for (si, e) in t.etapes.iter().enumerate() {
                let mut f = f_mo.min(PPM);
                let mut lim = if f_mo < PPM {
                    Limitant::MainOeuvre
                } else {
                    Limitant::Aucun
                };
                for &(r, q) in &e.intrants {
                    let f_in = (b.stock_entree[r] * PPM / q).min(PPM);
                    if f_in < f {
                        f = f_in;
                        lim = Limitant::Intrant(r);
                    }
                }
                for &(r, q) in &e.extrants {
                    if t.consomme_ensuite(si, r) {
                        continue;
                    }
                    let plafond = Catalogue::jours_x(q, cat.sim.jours_stock_sortie);
                    let libre = (plafond - b.stock_sortie[r]).max(0);
                    let f_out = (libre * PPM / q).min(PPM);
                    if f_out < f {
                        f = f_out;
                        lim = Limitant::StockPlein;
                    }
                }
                let f = f.max(0);
                for &(r, q) in &e.intrants {
                    let c = mul_div(q, f, PPM);
                    b.stock_entree[r] -= c;
                    self.bilan.consomme_prod[r] += c;
                }
                for &(r, q) in &e.extrants {
                    let p = mul_div(mul_div(q, f, PPM), fert, PPM);
                    self.bilan.produit[r] += p;
                    if t.consomme_ensuite(si, r) {
                        b.stock_entree[r] += p; // matière intermédiaire, consommée par l'étape suivante
                    } else {
                        b.stock_sortie[r] += p;
                        self.stats.produit[r] += p;
                    }
                }
                if f < f_min {
                    f_min = f;
                    limitant = lim;
                }
                f_dernier = f;
            }
            b.prod_ppm = f_dernier;
            b.limitant = if f_min >= PPM {
                Limitant::Aucun
            } else {
                limitant
            };
        }
    }

    /// Un jour de travail sur un chantier : l'équipe présente avance, au rythme du matériau disponible.
    fn avancer_chantier(b: &mut Batiment, cat: &Catalogue, bilan: &mut Bilan) {
        b.prod_ppm = 0;
        let presents = b.presents as i64;
        let equipe = b.equipe;
        let Some(ch) = b.chantier.as_mut() else {
            b.limitant = Limitant::Aucun;
            return;
        };
        let mat = cat.extension.materiau;
        // Matériau nécessaire pour la journée de travail de l'équipe présente.
        let besoin = mul_div(ch.materiau_total, presents * 1000, ch.travail_total.max(1));
        let f_mat = if besoin > 0 {
            (b.stock_entree[mat] * PPM / besoin).min(PPM)
        } else {
            PPM
        };
        let progres = (presents * 1000 * f_mat / PPM).min(ch.travail_restant.max(0)); // milli-pop-jours
        let conso =
            mul_div(ch.materiau_total, progres, ch.travail_total.max(1)).min(b.stock_entree[mat]);
        b.stock_entree[mat] -= conso;
        bilan.consomme_prod[mat] += conso;
        ch.travail_restant -= progres;
        b.limitant = if f_mat < PPM {
            Limitant::Intrant(mat)
        } else if (b.presents) < equipe {
            Limitant::MainOeuvre
        } else {
            Limitant::Aucun
        };
    }

    /// Met en service les chantiers dont le travail est achevé : l'équipe devient la première main-d'œuvre.
    fn terminer_chantiers(&mut self) {
        let finis: Vec<usize> = (0..self.monde.batiments.len())
            .filter(|&b| {
                self.monde.batiments[b]
                    .chantier
                    .as_ref()
                    .map_or(false, |c| c.travail_restant <= 0)
            })
            .collect();
        if finis.is_empty() {
            return;
        }
        for b in finis {
            let ch = self.monde.batiments[b].chantier.take().expect("chantier");
            let (x, y) = self.monde.xy(self.monde.batiments[b].tuile);
            match ch.cible {
                Cible::Terre(t) => {
                    self.monde.batiments[b].type_id = t;
                    self.monde.batiments[b].equipe = 0;
                    let nom = self.cat.batiments[t].nom.clone();
                    self.journal(format!("Nouvelle terre exploitée : {nom} en ({x}, {y})"));
                }
                Cible::Hameau(q) => {
                    let h = &self.cat.habitats[self.monde.quartiers[q].habitat_id];
                    let (cap, atelier, nom) = (h.capacite_pops, h.atelier, h.nom.clone());
                    self.monde.quartiers[q].capacite_pops = cap;
                    self.monde.batiments[b].equipe = 0;
                    // Le matériau resté sur le chantier revient au hameau.
                    let mat = self.cat.extension.materiau;
                    let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
                    self.monde.quartiers[q].stock[mat] += reste;
                    if let Some(at) = atelier {
                        self.monde.batiments[b].type_id = at;
                        self.monde.batiments[b].quartier = Some(q);
                        self.monde.quartiers[q].atelier = Some(b);
                    }
                    self.journal(format!("Nouveau hameau construit : {nom} en ({x}, {y})"));
                }
            }
            // Les bras de l'équipe en surnombre redeviennent sans poste.
            let postes = self.postes_de(b);
            let mut equipe: Vec<usize> = (0..self.pops.len())
                .filter(|&i| self.pops[i].emploi == Some(b))
                .collect();
            equipe.sort_by_key(|&i| self.pops[i].id);
            for &i in equipe.iter().skip(postes as usize) {
                self.pops[i].emploi = None;
            }
        }
        self.calculer_fournisseurs();
    }

    fn consommer(&mut self) {
        let nr = self.cat.ressources.len();
        let eff = self.effectifs();
        for q in 0..self.nq() {
            let n = eff[q] as i64;
            let quartier = &mut self.monde.quartiers[q];
            let mut recu = vec![0; nr];
            for b in &self.cat.besoins {
                let r = quartier.stock[b.ressource].min(b.par_pop * n).max(0);
                quartier.stock[b.ressource] -= r;
                recu[b.ressource] = r;
                self.bilan.consomme_pops[b.ressource] += r;
                self.stats.consomme_pops[b.ressource] += r;
            }
            let ev = evaluer_besoins(&self.cat, n, &recu);
            quartier.vital_jour_ppm = ev.vital_ppm;
            quartier.confort_jour_ppm = ev.confort_ppm;
            quartier.sat_par_ressource_ppm = ev.par_ressource_ppm;
            quartier.cumul_vital_ppm += ev.vital_ppm;
            quartier.cumul_confort_ppm += ev.confort_ppm;
            quartier.jours_an += 1;
        }
    }

    // ---------- Démographie : une jauge par pop ----------

    /// Hameau où loger une nouvelle pop : celui du parent s'il a de la place,
    /// sinon le plus proche qui en a (coût, remplissage, index).
    fn hameau_libre(&self, q: usize, eff: &[u32]) -> Option<usize> {
        if eff[q] < self.monde.quartiers[q].capacite_pops {
            return Some(q);
        }
        (0..self.nq())
            .filter(|&h| eff[h] < self.monde.quartiers[h].capacite_pops)
            .filter_map(|h| self.reseau.chemin(q, h).map(|c| ((c.cout, eff[h], h), h)))
            .min()
            .map(|(_, h)| h)
    }

    fn croissance_quotidienne(&mut self) {
        let seuil = self.seuil_croissance();
        let jours = self.cat.sim.jours_par_an;
        let taux: Vec<i64> = self
            .monde
            .quartiers
            .iter()
            .map(|q| taux_croissance_ppm(&self.cat.demo, q.vital_jour_ppm, q.confort_jour_ppm))
            .collect();
        for p in &mut self.pops {
            p.croissance += mul_div(seuil, taux[p.residence], PPM * jours);
        }

        // Disparitions d'abord (elles libèrent du logement et des postes).
        let morts: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance <= -seuil)
            .collect();
        for &i in morts.iter().rev() {
            self.pops.remove(i);
            self.deces_an += 1;
        }

        // Duplications : la nouvelle pop se loge et prend le poste vacant le plus proche.
        let mut eff = self.effectifs();
        let mut occ = self.occupation();
        let parents: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance >= seuil)
            .collect();
        for i in parents {
            let q = self.pops[i].residence;
            match self.hameau_libre(q, &eff) {
                Some(h) => {
                    let emploi = self.meilleur_poste(h, &occ);
                    if let Some(b) = emploi {
                        occ[b] += 1;
                    }
                    let id = self.prochain_pop_id;
                    self.prochain_pop_id += 1;
                    self.pops.push(Pop {
                        id,
                        residence: h,
                        emploi,
                        croissance: 0,
                    });
                    eff[h] += 1;
                    self.pops[i].croissance -= seuil;
                    self.naissances_an += 1;
                }
                // Plus de logement : la jauge attend, plafonnée.
                None => self.pops[i].croissance = seuil,
            }
        }
    }

    // ---------- Postes ----------

    /// Poste vacant le plus proche du hameau : (distance, taux de remplissage, index) minimal.
    fn meilleur_poste(&self, q: usize, occ: &[u32]) -> Option<usize> {
        let nq = self.nq();
        let mut best: Option<((i64, i64, usize), usize)> = None;
        for b in 0..self.monde.batiments.len() {
            let postes = self.postes_de(b);
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

    /// Tension par ressource : ration non couverte dans un hameau, ou intrant qui manque à un producteur.
    fn poids_tension(&self) -> Vec<i64> {
        let nr = self.cat.ressources.len();
        let eff = self.effectifs();
        let seuil = self.cat.mobilite.seuil_tension_ppm;
        let mut poids = vec![0i64; nr];
        for (q, qu) in self.monde.quartiers.iter().enumerate() {
            if eff[q] == 0 {
                continue;
            }
            for b in &self.cat.besoins {
                let s = qu.sat_par_ressource_ppm[b.ressource];
                if s < seuil {
                    poids[b.ressource] += (PPM - s) * eff[q] as i64;
                }
            }
        }
        for bat in &self.monde.batiments {
            if let Limitant::Intrant(r) = bat.limitant {
                poids[r] += PPM - bat.prod_ppm;
            }
        }
        poids
    }

    /// Ressources en tension, de la plus pressante à la moins pressante.
    fn tensions(&self) -> Vec<usize> {
        let poids = self.poids_tension();
        let mut ordre: Vec<usize> = (0..poids.len()).filter(|&r| poids[r] > 0).collect();
        ordre.sort_by(|a, b| poids[*b].cmp(&poids[*a]).then(a.cmp(b)));
        ordre
    }

    fn livre(&self, b: usize, r: usize) -> bool {
        self.cat.batiments[self.monde.batiments[b].type_id].livre(r)
    }

    /// Réorientation mensuelle des bras selon le besoin (entraide du hameau, sans salaire) :
    /// 1. les pops sans poste prennent les postes vacants ;
    /// 2. pour chaque ressource en tension, une pop quitte un lieu dont le stock est plein
    ///    pour rejoindre un lieu qui produit la ressource manquante et a un poste libre.
    fn reaffecter(&mut self) {
        let nq = self.nq();
        let nb = self.monde.batiments.len();
        let mut occ = self.occupation();
        for i in 0..self.pops.len() {
            if self.pops[i].emploi.is_none() {
                if let Some(b) = self.meilleur_poste(self.pops[i].residence, &occ) {
                    self.pops[i].emploi = Some(b);
                    occ[b] += 1;
                }
            }
        }

        let mut utilises = vec![false; nb];
        for r in self.tensions() {
            for _ in 0..self.cat.mobilite.max_par_ressource {
                let receveur = (0..nb)
                    .filter(|&b| !utilises[b] && self.livre(b, r) && occ[b] < self.postes_de(b))
                    .min_by_key(|&b| (occ[b] as i64 * 1000 / self.postes_de(b) as i64, b));
                let Some(rb) = receveur else { break };
                let donneur = (0..nb)
                    .filter(|&b| {
                        !utilises[b]
                            && b != rb
                            && occ[b] > 0
                            && !self.est_chantier_type(b)
                            && self.monde.batiments[b].limitant == Limitant::StockPlein
                    })
                    .min_by_key(|&b| (self.monde.batiments[b].prod_ppm, b));
                let Some(db) = donneur else { break };
                let pop = (0..self.pops.len())
                    .filter(|&i| self.pops[i].emploi == Some(db))
                    .filter_map(|i| {
                        self.reseau
                            .chemin(self.pops[i].residence, nq + rb)
                            .map(|c| ((c.cout, self.pops[i].id), i))
                    })
                    .min();
                let Some((_, i)) = pop else { break };
                self.pops[i].emploi = Some(rb);
                occ[db] -= 1;
                occ[rb] += 1;
                utilises[db] = true;
                utilises[rb] = true;
            }
        }
    }

    // ---------- Extension du village ----------

    pub fn chantiers_en_cours(&self) -> usize {
        self.monde
            .batiments
            .iter()
            .filter(|b| b.chantier.is_some())
            .count()
    }

    /// Habitat dont l'atelier livre la ressource.
    fn habitat_producteur(&self, r: usize) -> Option<usize> {
        self.cat.habitats.iter().position(|h| {
            h.atelier.map_or(false, |a| {
                self.cat.batiments[a]
                    .extrants_finaux()
                    .iter()
                    .any(|(x, _)| *x == r)
            })
        })
    }

    /// Type de terre exploitée qu'on peut défricher pour produire la ressource.
    fn type_terre_pour(&self, r: usize) -> Option<usize> {
        (0..self.cat.batiments.len()).find(|&t| {
            let ty = &self.cat.batiments[t];
            !ty.chantier
                && ty.cout_creation.is_some()
                && ty.extrants_finaux().iter().any(|(x, _)| *x == r)
        })
    }

    /// Les producteurs de la ressource sont-ils bloqués par un intrant manquant ? Alors la cause
    /// racine est ailleurs (la fibre, pas les vêtements) : en bâtir davantage ne servirait à rien.
    fn producteurs_bloques_par_intrant(&self, r: usize) -> bool {
        self.monde
            .batiments
            .iter()
            .enumerate()
            .any(|(b, bat)| self.livre(b, r) && matches!(bat.limitant, Limitant::Intrant(_)))
    }

    /// Un chantier produira-t-il déjà cette ressource ?
    fn chantier_pour(&self, r: usize) -> bool {
        self.monde.batiments.iter().any(|b| match &b.chantier {
            Some(Chantier {
                cible: Cible::Terre(t),
                ..
            }) => self.cat.batiments[*t]
                .extrants_finaux()
                .iter()
                .any(|(x, _)| *x == r),
            Some(Chantier {
                cible: Cible::Hameau(q),
                ..
            }) => {
                let h = &self.cat.habitats[self.monde.quartiers[*q].habitat_id];
                h.atelier.map_or(false, |a| {
                    self.cat.batiments[a]
                        .extrants_finaux()
                        .iter()
                        .any(|(x, _)| *x == r)
                })
            }
            None => false,
        })
    }

    /// Décisions saisonnières de la communauté, après la réaffectation mensuelle (donc une fois
    /// tous les postes disponibles pris) : loger les pops qui attendent, puis défricher pour
    /// la ressource la plus en tension tant que des bras restent sans poste.
    pub fn etendre(&mut self) {
        // Les chantiers incomplets recrutent à nouveau ; vide, un chantier réquisitionne (corvée).
        for b in 0..self.monde.batiments.len() {
            if self.monde.batiments[b].chantier.is_none() {
                continue;
            }
            let equipe = self.monde.batiments[b].equipe;
            let present = self.pops.iter().filter(|p| p.emploi == Some(b)).count() as u32;
            if present < equipe {
                self.recruter(b, equipe - present, present == 0);
            }
        }
        if self.chantiers_en_cours() >= self.cat.extension.chantiers_max {
            return;
        }
        let nq = self.nq();
        let seuil = self.seuil_croissance();
        let tensions = self.tensions();
        let hameau_en_cours = (0..nq).any(|q| self.monde.quartiers[q].capacite_pops == 0);

        // 1. Logement : des pops à jauge pleine attendent une place.
        let attente: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance >= seuil)
            .collect();
        if !attente.is_empty() && !hameau_en_cours {
            let habitat = tensions
                .iter()
                .filter(|&&r| !self.producteurs_bloques_par_intrant(r))
                .find_map(|&r| self.habitat_producteur(r))
                .unwrap_or(0);
            let motif = format!("{} pops attendent un logement", attente.len());
            for centre in self.centres(attente.iter().map(|&i| self.pops[i].residence)) {
                if self.lancer_hameau(centre, habitat, &motif) {
                    return;
                }
            }
        }

        // 2. Terres : des bras restent sans poste alors qu'une ressource manque.
        let sans: Vec<usize> = self
            .pops
            .iter()
            .filter(|p| p.emploi.is_none())
            .map(|p| p.residence)
            .collect();
        if sans.is_empty() {
            return;
        }
        let centres = self.centres(sans.into_iter());
        let nb_sans = self.sans_poste();
        for r in tensions {
            if self.chantier_pour(r) || self.producteurs_bloques_par_intrant(r) {
                continue;
            }
            let motif = format!(
                "{} manque, {} pops sans poste",
                self.cat.ressources[r].nom, nb_sans
            );
            let mut lance = false;
            for &centre in &centres {
                lance = if let Some(t) = self.type_terre_pour(r) {
                    self.lancer_terre(centre, t, &motif)
                } else if let (Some(h), false) = (self.habitat_producteur(r), hameau_en_cours) {
                    self.lancer_hameau(centre, h, &motif)
                } else {
                    false
                };
                if lance {
                    break;
                }
            }
            if lance {
                return;
            }
        }
    }

    /// Hameaux d'origine à essayer, sans doublon, dans l'ordre où on les rencontre.
    fn centres(&self, residences: impl Iterator<Item = usize>) -> Vec<usize> {
        let mut v: Vec<usize> = Vec::new();
        for q in residences {
            if !v.contains(&q) {
                v.push(q);
            }
        }
        v
    }

    /// Meilleur emplacement pour un nouveau lieu, à portée du hameau `centre` :
    /// soit accolé à une tile traversable, soit au bout d'une nouvelle tile de chemin.
    /// Terre fertile : fertilité / distance maximale ; sinon : le plus proche, sur la terre la plus pauvre.
    /// `requis` : groupes de ressources dont l'une au moins doit avoir un producteur à portée du site.
    fn meilleur_site(&self, centre: usize, fertile: bool, requis: &[Vec<usize>]) -> Option<Site> {
        let nq = self.nq();
        let d = self
            .reseau
            .distances_depuis(self.monde.quartiers[centre].tuile);
        // Distance (en segments) de chaque tile aux producteurs de chaque groupe requis.
        let portees: Vec<Vec<i64>> = requis
            .iter()
            .map(|groupe| {
                let mut sources: Vec<usize> = Vec::new();
                for b in 0..self.monde.batiments.len() {
                    if groupe.iter().any(|&r| self.livre(b, r)) {
                        sources.extend_from_slice(self.reseau.acces(nq + b));
                    }
                }
                self.reseau.distances_depuis_tuiles(&sources)
            })
            .collect();
        let dmax = self.cat.infra.distance_max;
        let vide = |t: usize| self.monde.tuiles[t].contenu == Contenu::Vide;
        // Le lieu posé au bout de `attache` (+ une tile de route éventuelle) est-il desservi ?
        // Un hameau est lui-même traversable : sa propre tile compte une tile de plus dans le trajet.
        let extra = if fertile { 0 } else { 1 };
        let desservi = |attache: usize, route: bool| {
            portees
                .iter()
                .all(|dp| dp[attache] != i64::MAX && dp[attache] + route as i64 + 1 + extra <= dmax)
        };
        let mut best: Option<((i64, i64, i64, usize), Site)> = None;
        let essayer = |tuile: usize,
                       attache: usize,
                       route: Option<usize>,
                       dist: i64,
                       best: &mut Option<((i64, i64, i64, usize), Site)>| {
            let fert = self.monde.tuiles[tuile].fertilite_ppm;
            let cle = if fertile {
                (
                    -(fert * 1000 / (dist + 1)),
                    dist,
                    route.is_some() as i64,
                    tuile,
                )
            } else {
                (dist, route.is_some() as i64, fert, tuile)
            };
            if best.as_ref().map_or(true, |(k, _)| cle < *k) {
                *best = Some((
                    cle,
                    Site {
                        tuile,
                        attache,
                        route,
                    },
                ));
            }
        };
        for p in 0..self.monde.tuiles.len() {
            if d[p] == i64::MAX {
                continue;
            }
            for dir in 0..4 {
                let Some(t1) = self.monde.voisin(p, dir) else {
                    continue;
                };
                if !vide(t1) {
                    continue;
                }
                if d[p] + 1 + extra <= dmax && desservi(p, false) {
                    essayer(t1, p, None, d[p] + 1, &mut best);
                }
                if d[p] + 2 + extra <= dmax && desservi(p, true) {
                    for dir2 in 0..4 {
                        if DIRS[dir2].3 == DIRS[dir].0 {
                            continue; // pas de demi-tour
                        }
                        let Some(t2) = self.monde.voisin(t1, dir2) else {
                            continue;
                        };
                        if vide(t2) {
                            essayer(t2, p, Some(t1), d[p] + 2, &mut best);
                        }
                    }
                }
            }
        }
        best.map(|(_, s)| s)
    }

    /// Relie deux tiles voisines en posant les demi-directions des tiles de chemin.
    fn relier(&mut self, a: usize, b: usize) {
        let (ax, ay) = self.monde.xy(a);
        let (bx, by) = self.monde.xy(b);
        let (dx, dy) = (bx as i32 - ax as i32, by as i32 - ay as i32);
        let d = DIRS
            .iter()
            .position(|&(_, ddx, ddy, _)| ddx == dx && ddy == dy)
            .expect("tiles non voisines");
        if let Contenu::Chemin { masque } = &mut self.monde.tuiles[a].contenu {
            *masque |= DIRS[d].0;
        }
        if let Contenu::Chemin { masque } = &mut self.monde.tuiles[b].contenu {
            *masque |= DIRS[d].3;
        }
    }

    fn poser_site(&mut self, site: Site) {
        match site.route {
            Some(r) => {
                self.monde.tuiles[r].contenu = Contenu::Chemin { masque: 0 };
                self.relier(site.attache, r);
                self.relier(r, site.tuile);
            }
            None => self.relier(site.attache, site.tuile),
        }
    }

    fn nouveau_chantier(
        &self,
        tuile: usize,
        cible: Cible,
        cout: &crate::donnees::CoutCreation,
    ) -> Batiment {
        let nr = self.cat.ressources.len();
        Batiment {
            type_id: self.cat.type_chantier(),
            tuile,
            quartier: None,
            equipe: cout.equipe,
            chantier: Some(Chantier {
                cible,
                travail_restant: cout.travail,
                travail_total: cout.travail,
                materiau_total: cout.materiau,
            }),
            stock_entree: vec![0; nr],
            stock_sortie: vec![0; nr],
            presents: 0,
            prod_ppm: 0,
            limitant: Limitant::Aucun,
        }
    }

    fn lancer_terre(&mut self, centre: usize, type_id: usize, motif: &str) -> bool {
        let Some(cout) = self.cat.batiments[type_id].cout_creation.clone() else {
            return false;
        };
        let requis: Vec<Vec<usize>> = if cout.materiau > 0 {
            vec![vec![self.cat.extension.materiau]]
        } else {
            Vec::new()
        };
        let Some(site) = self.meilleur_site(centre, true, &requis) else {
            return false;
        };
        self.poser_site(site);
        let b = self.monde.batiments.len();
        let chantier = self.nouveau_chantier(site.tuile, Cible::Terre(type_id), &cout);
        self.monde.batiments.push(chantier);
        self.monde.tuiles[site.tuile].contenu = Contenu::Batiment(b);
        self.monde.tuiles[site.tuile].variante = (b % 4) as u8;
        let (x, y) = self.monde.xy(site.tuile);
        let nom = self.cat.batiments[type_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : défricher {nom} en ({x}, {y}) — {motif}"
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe, false);
        true
    }

    fn lancer_hameau(&mut self, centre: usize, habitat_id: usize, motif: &str) -> bool {
        let cout = self.cat.habitats[habitat_id].cout_creation.clone();
        // Un hameau doit pouvoir être bâti (matériau), chauffé et nourri à portée de son site.
        let nutritives: Vec<usize> = (0..self.cat.ressources.len())
            .filter(|&r| self.cat.ressources[r].nutritive_milli > 0)
            .collect();
        let mut requis = vec![vec![self.cat.vital.chauffage_ressource], nutritives];
        if cout.materiau > 0 {
            requis.push(vec![self.cat.extension.materiau]);
        }
        let Some(site) = self.meilleur_site(centre, false, &requis) else {
            return false;
        };
        self.poser_site(site);
        // Un hameau s'ajoute aux quartiers : les lieux « bâtiment » se décalent d'un cran.
        let nq = self.nq();
        for d in &mut self.differes {
            if d.fournisseur >= nq {
                d.fournisseur += 1;
            }
            if d.client >= nq {
                d.client += 1;
            }
        }
        let q = nq;
        let nr = self.cat.ressources.len();
        self.monde.quartiers.push(Quartier {
            tuile: site.tuile,
            habitat_id,
            capacite_pops: 0,
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
        });
        let b = self.monde.batiments.len();
        let chantier = self.nouveau_chantier(site.tuile, Cible::Hameau(q), &cout);
        self.monde.batiments.push(chantier);
        self.monde.tuiles[site.tuile].contenu = Contenu::Quartier(q);
        self.monde.tuiles[site.tuile].variante = (q % 4) as u8;
        let (x, y) = self.monde.xy(site.tuile);
        let nom = self.cat.habitats[habitat_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : bâtir un {nom} en ({x}, {y}) — {motif}"
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe, false);
        true
    }

    /// Constitue l'équipe d'un chantier : d'abord les pops sans poste les plus proches ; à défaut,
    /// des bras pris dans les lieux dont le stock couvre largement les besoins.
    fn recruter(&mut self, chantier: usize, n: u32, corvee: bool) {
        let nq = self.nq();
        let mut libres: Vec<(i64, u32, usize)> = (0..self.pops.len())
            .filter(|&i| self.pops[i].emploi.is_none())
            .filter_map(|i| {
                self.reseau
                    .chemin(self.pops[i].residence, nq + chantier)
                    .map(|c| (c.cout, self.pops[i].id, i))
            })
            .collect();
        libres.sort();
        let mut pris = 0;
        for &(_, _, i) in libres.iter().take(n as usize) {
            self.pops[i].emploi = Some(chantier);
            pris += 1;
        }
        if pris >= n {
            return;
        }
        // Corvée : un chantier resté sans équipe prend des bras même aux lieux sans excédent.
        let seuil = if corvee {
            0
        } else {
            (self.cat.extension.couverture_donneur_jours * 1000.0) as i64
        };
        let mut donneurs: Vec<(i64, i64, u32, usize)> = Vec::new();
        for i in 0..self.pops.len() {
            let Some(db) = self.pops[i].emploi else {
                continue;
            };
            if db == chantier || self.est_chantier_type(db) {
                continue;
            }
            let couverture = self.couverture_milli_jours(db);
            let Some(ch) = self.reseau.chemin(self.pops[i].residence, nq + chantier) else {
                continue;
            };
            if couverture >= seuil {
                donneurs.push((-couverture, ch.cout, self.pops[i].id, i));
            }
        }
        donneurs.sort();
        for &(_, _, _, i) in donneurs.iter().take((n - pris) as usize) {
            self.pops[i].emploi = Some(chantier);
        }
    }

    /// Jours de production en stock de sortie (×1000) pour le premier extrant final du lieu.
    fn couverture_milli_jours(&self, b: usize) -> i64 {
        let bat = &self.monde.batiments[b];
        match self.cat.batiments[bat.type_id].extrants_finaux().first() {
            Some(&(r, q)) if q > 0 => bat.stock_sortie[r] * 1000 / q,
            _ => 0,
        }
    }

    /// Plan ASCII du village (pour l'observation en ligne de commande).
    /// H hameau, T hameau textile, C champ, P pâturage, V verger, F forêt, L lin, W laine,
    /// + chemin, minuscules = chantier de la lettre correspondante, # chantier de hameau.
    pub fn carte_ascii(&self) -> String {
        let m = &self.monde;
        let lettre = |t: usize| -> char {
            match self.cat.batiments[t].nom.as_str() {
                "champ" => 'C',
                "paturage" => 'P',
                "verger" => 'V',
                "foret_exploitee" => 'F',
                "champ_lin" => 'L',
                "elevage_laine" => 'W',
                n => n.chars().next().unwrap_or('?').to_ascii_uppercase(),
            }
        };
        let glyphe = |i: usize| -> char {
            match m.tuiles[i].contenu {
                Contenu::Vide => ' ',
                Contenu::Chemin { .. } => '+',
                Contenu::Quartier(q) => {
                    if m.quartiers[q].capacite_pops == 0 {
                        '#'
                    } else if m.quartiers[q].atelier.is_some() {
                        'T'
                    } else {
                        'H'
                    }
                }
                Contenu::Batiment(b) => match &m.batiments[b].chantier {
                    Some(Chantier {
                        cible: Cible::Terre(t),
                        ..
                    }) => lettre(*t).to_ascii_lowercase(),
                    Some(_) => '#',
                    None => lettre(m.batiments[b].type_id),
                },
            }
        };
        let occupees: Vec<(usize, usize)> = (0..m.tuiles.len())
            .filter(|&i| m.tuiles[i].contenu != Contenu::Vide)
            .map(|i| m.xy(i))
            .collect();
        if occupees.is_empty() {
            return String::new();
        }
        let (x0, x1) = (
            occupees.iter().map(|p| p.0).min().unwrap(),
            occupees.iter().map(|p| p.0).max().unwrap(),
        );
        let (y0, y1) = (
            occupees.iter().map(|p| p.1).min().unwrap(),
            occupees.iter().map(|p| p.1).max().unwrap(),
        );
        let mut out = String::new();
        for y in y0..=y1 {
            let ligne: String = (x0..=x1).map(|x| glyphe(m.idx(x, y))).collect();
            out.push_str(ligne.trim_end());
            out.push('\n');
        }
        out
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

    fn fin_annee(&mut self) {
        let mut somme_v = 0i64;
        let mut somme_c = 0i64;
        let mut actifs = 0i64;
        let eff = self.effectifs();
        for (q, qu) in self.monde.quartiers.iter_mut().enumerate() {
            let jours = qu.jours_an.max(1);
            qu.vital_annuel_ppm = qu.cumul_vital_ppm / jours;
            qu.confort_annuel_ppm = qu.cumul_confort_ppm / jours;
            qu.cumul_vital_ppm = 0;
            qu.cumul_confort_ppm = 0;
            qu.jours_an = 0;
            if eff[q] > 0 {
                somme_v += qu.vital_annuel_ppm;
                somme_c += qu.confort_annuel_ppm;
                actifs += 1;
            }
        }
        self.historique.push(ResumeAn {
            annee: self.jour / self.cat.sim.jours_par_an as u64,
            pops: self.pops.len(),
            sans_poste: self.sans_poste(),
            naissances: self.naissances_an,
            deces: self.deces_an,
            vital_moy_ppm: if actifs > 0 { somme_v / actifs } else { 0 },
            confort_moy_ppm: if actifs > 0 { somme_c / actifs } else { 0 },
        });
        self.naissances_an = 0;
        self.deces_an = 0;
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
        // Pas de biais d'ordre : trois demandes égales reçoivent (à 1 près) la même part.
        let q = repartir_pro_rata(101, &[50, 50, 50]);
        assert_eq!(q.iter().sum::<Qte>(), 101);
        assert!(q.iter().max().unwrap() - q.iter().min().unwrap() <= 1);
    }

    #[test]
    fn pro_rata_suffisant_ou_vide() {
        assert_eq!(repartir_pro_rata(500, &[10, 20]), vec![10, 20]);
        assert_eq!(repartir_pro_rata(0, &[10, 20]), vec![0, 0]);
    }

    /// Reçu d'une pop pour une journée, par nom de ressource (en unités).
    fn recu(cat: &Catalogue, n: i64, parts: &[(&str, f64)]) -> Vec<Qte> {
        let mut v = vec![0; cat.ressources.len()];
        for (nom, q) in parts {
            v[cat.res_id(nom).unwrap()] = (q * 1000.0 * n as f64).round() as Qte;
        }
        v
    }

    #[test]
    fn besoins_pleinement_couverts() {
        let c = Catalogue::defaut();
        let r = recu(
            &c,
            2,
            &[
                ("grain", 5.0),
                ("viande", 1.0),
                ("fruit", 1.0),
                ("bois", 2.0),
                ("vetements", 0.5),
            ],
        );
        let e = evaluer_besoins(&c, 2, &r);
        assert_eq!((e.vital_ppm, e.confort_ppm), (PPM, PPM));
    }

    #[test]
    fn vital_couvert_sans_confort_est_insatisfait() {
        let c = Catalogue::defaut();
        // Grain et bois seulement : on survit, mais ni viande ni vêtements.
        let e = evaluer_besoins(&c, 2, &recu(&c, 2, &[("grain", 5.0), ("bois", 2.0)]));
        assert_eq!((e.vital_ppm, e.confort_ppm), (PPM, 0));
    }

    #[test]
    fn les_aliments_sont_interchangeables_pour_le_vital() {
        let c = Catalogue::defaut();
        // 3 grain + 1 viande + 1 fruit = 5 unités nutritives : le vital est couvert.
        let e = evaluer_besoins(
            &c,
            1,
            &recu(
                &c,
                1,
                &[
                    ("grain", 3.0),
                    ("viande", 1.0),
                    ("fruit", 1.0),
                    ("bois", 2.0),
                ],
            ),
        );
        assert_eq!(e.vital_ppm, PPM);
        // 2 grain seulement : 40 % du vital.
        let e = evaluer_besoins(&c, 1, &recu(&c, 1, &[("grain", 2.0), ("bois", 2.0)]));
        assert_eq!(e.vital_ppm, 400_000);
    }

    #[test]
    fn sans_chauffage_le_vital_tombe() {
        let c = Catalogue::defaut();
        let e = evaluer_besoins(
            &c,
            1,
            &recu(
                &c,
                1,
                &[("grain", 5.0), ("viande", 1.0), ("vetements", 0.5)],
            ),
        );
        assert_eq!(e.vital_ppm, 0);
    }

    #[test]
    fn taux_de_croissance_par_regime() {
        let d = &Catalogue::defaut().demo;
        assert_eq!(taux_croissance_ppm(d, PPM, PPM), 15_000); // satisfaite : +1,5 %/an
        assert_eq!(taux_croissance_ppm(d, PPM, 0), 5_000); // insatisfaite mais vitale couverte : +0,5 %/an
        assert_eq!(taux_croissance_ppm(d, 0, 0), -80_000); // famine totale
        assert!(taux_croissance_ppm(d, 900_000, PPM) < 5_000);
        assert!(taux_croissance_ppm(d, 500_000, PPM) < taux_croissance_ppm(d, 900_000, PPM));
    }
}
