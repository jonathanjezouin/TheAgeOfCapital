//! Boucle quotidienne (voir docs/02 §5), démographie par jauge de pop (docs/02 §6)
//! et réorientation de la main-d'œuvre selon le besoin (docs/02 §3).

use crate::donnees::{mul_div, Catalogue, ParamsDemo, Qte, Traversabilite, PPM};
use crate::monde::{cout_dir, Batiment, Chantier, Cible, Contenu, Limitant, Monde, Quartier};
use crate::reseau::Reseau;
use serde::Serialize;
use std::collections::BTreeMap;

/// Ce qui pousse une pop vers un lieu de travail (docs/02 §10.4), par ordre de priorité.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Classe {
    /// Nourriture ou chauffage qui s'épuise.
    Vital,
    /// Viande, vêtements.
    Confort,
    /// Matière première dont un atelier manque pour vêtir le hameau.
    Intrant,
    /// Rien ne manque : la terre est entretenue pour garder ses stocks pleins.
    Entretien,
}

/// Pourquoi la pop travaille là aujourd'hui (ou pourquoi elle reste chez elle).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Motif {
    /// Pas encore de décision (pop née aujourd'hui).
    Aucun,
    /// Équipe d'un chantier : poste fixe.
    Chantier,
    /// Elle travaille pour une ressource qui lui manque.
    Besoin {
        ressource: usize,
        classe: Classe,
        /// Jours d'autonomie qui manquent pour atteindre la cible de stock, en milli-jours.
        manque_milli_jours: i64,
    },
    /// Aucune terre ne réclame assez de travail à sa portée : elle reste chez elle (travail domestique).
    Repos,
}

#[derive(Clone, Debug, Serialize)]
pub struct Pop {
    pub id: u32,
    /// Index du hameau de résidence.
    pub residence: usize,
    /// Lieu de travail d'aujourd'hui, un seul (None = chez elle). Équipe d'un chantier : poste fixe.
    pub emploi: Option<usize>,
    /// Jauge de croissance en micro-habitants : à +100 habitants la pop se duplique,
    /// à −100 elle disparaît. Alimentée chaque jour par la couverture des besoins.
    pub croissance: i64,
    /// Inventaire personnel, par ressource (milli-unités) : la pop consomme sur son propre stock.
    pub stock: Vec<Qte>,
    pub motif: Motif,
    /// Travail fourni aujourd'hui sur son lieu (milli pop-jours) et sa moyenne sur ~30 jours.
    pub travail_milli: i64,
    pub travail_lisse_milli: i64,
    /// Besoins couverts lors du dernier jour (ppm), et ration reçue par ressource.
    pub vital_jour_ppm: i64,
    pub confort_jour_ppm: i64,
    pub sat_par_ressource_ppm: Vec<i64>,
    /// Quantités consommées lors du dernier jour, par ressource (milli-unités).
    pub conso_jour: Vec<Qte>,
}

impl Pop {
    pub fn nouvelle(id: u32, residence: usize, croissance: i64, nb_ressources: usize) -> Pop {
        Pop {
            id,
            residence,
            emploi: None,
            croissance,
            stock: vec![0; nb_ressources],
            motif: Motif::Aucun,
            travail_milli: 0,
            travail_lisse_milli: 0,
            vital_jour_ppm: PPM,
            confort_jour_ppm: PPM,
            sat_par_ressource_ppm: vec![PPM; nb_ressources],
            conso_jour: vec![0; nb_ressources],
        }
    }
}

/// Une terre que la pop pourrait choisir, avec la raison de son rang (docs/02 §10.5).
#[derive(Clone, Debug, Serialize)]
pub struct Candidat {
    pub lieu: usize,
    /// Coût du trajet depuis son hameau (unités d'arête).
    pub cout: i64,
    pub ressource: usize,
    pub classe: Classe,
    pub manque_milli_jours: i64,
    /// Travail que la terre réclame encore (milli pop-jours).
    pub demande_milli: i64,
}

/// Jours d'autonomie d'une pop (milli-jours) : combien de temps son stock tient sans réapprovisionnement.
#[derive(Clone, Debug, Serialize)]
pub struct Autonomies {
    pub nutrition: i64,
    pub chauffage: i64,
    /// Par ressource de ration (i64::MAX si la ressource n'est pas une ration).
    pub par_ressource: Vec<i64>,
}

/// Partage `qte` au prorata de `poids` (somme exacte, plus grandes fractions d'abord).
fn partager(qte: Qte, poids: &[Qte]) -> Vec<Qte> {
    let total: Qte = poids.iter().sum();
    if total <= 0 || qte <= 0 {
        return vec![0; poids.len()];
    }
    let mut parts: Vec<Qte> = poids.iter().map(|&w| mul_div(qte, w, total)).collect();
    let mut restes: Vec<(i128, usize)> = poids
        .iter()
        .enumerate()
        .map(|(i, &w)| ((qte as i128 * w as i128) % total as i128, i))
        .collect();
    let mut reste = qte - parts.iter().sum::<Qte>();
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

/// Une marchandise partie d'un lieu vers un autre aujourd'hui (indices de lieux : hameaux puis bâtiments).
#[derive(Clone, Debug, Serialize)]
pub struct Flux {
    pub de: usize,
    pub vers: usize,
    pub ressource: usize,
    pub qte: Qte,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StatsJour {
    /// Marchandises expédiées aujourd'hui (fournisseur → client), pour suivre où va la production.
    pub flux: Vec<Flux>,
    /// Production du jour par bâtiment : (bâtiment, ressource, quantité).
    pub produits_par_lieu: Vec<(usize, usize, Qte)>,
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

/// Surcoût de pathfinding (unités d'arête) pour entrer dans une tile `couteuse` (docs/02 §10.1).
const SURCOUT_TRAVERSEE: i64 = 30;

/// Longueur maximale (coût) de la route neuve qui relie un hameau fondé au réseau existant.
const ROUTE_FONDATION_MAX: i64 = 80;

/// Emplacement choisi pour un nouveau lieu : tile libre, tile du réseau où l'on s'accroche,
/// et les tiles intermédiaires d'une route neuve entre les deux (dans l'ordre, éventuellement aucune).
#[derive(Clone, Debug)]
struct Site {
    tuile: usize,
    attache: usize,
    route: Vec<usize>,
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
    /// Temps libre de chaque hameau aujourd'hui, en milli pop-jours (ce que les terres n'ont pas pris).
    pub libre_milli: Vec<i64>,
    /// Temps libre de chaque hameau, moyenne mobile sur ~30 jours (milli pop-jours) : sert à compter les bras sans emploi.
    pub libre_lisse_milli: Vec<i64>,
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
            libre_milli: Vec::new(),
            libre_lisse_milli: Vec::new(),
        };
        sim.calculer_fournisseurs();
        sim.bilan.initial = sim.stock_total();
        sim
    }

    fn nq(&self) -> usize {
        self.monde.quartiers.len()
    }

    /// Postes d'un lieu : effectif de l'équipe pour un chantier, 0 sinon (budget de jours, docs/02 §8.3).
    pub fn postes_de(&self, b: usize) -> u32 {
        let bat = &self.monde.batiments[b];
        let t = &self.cat.batiments[bat.type_id];
        if t.chantier {
            bat.equipe
        } else {
            0 // ni terre ni atelier domestique n'ont de poste : le travail est un budget de jours
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

    /// Stock total par ressource : pops + réserves de hameau + bâtiments + marchandises en transit.
    pub fn stock_total(&self) -> Vec<Qte> {
        let mut t = vec![0; self.cat.ressources.len()];
        for q in &self.monde.quartiers {
            for (r, v) in q.stock.iter().enumerate() {
                t[r] += v;
            }
        }
        for p in &self.pops {
            for (r, v) in p.stock.iter().enumerate() {
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

    /// Bras sans emploi : temps libre moyen des hameaux (ce que les terres ne réclament pas), en pops entières.
    /// Une pop compte comme sans poste quand plus de 90 % de son temps reste libre (docs/02 §10.4).
    pub fn sans_poste(&self) -> usize {
        let eff = self.effectifs();
        self.libre_lisse_milli
            .iter()
            .enumerate()
            .map(|(q, &l)| (((l + 100) / 1000).max(0) as usize).min(eff.get(q).copied().unwrap_or(0) as usize))
            .sum()
    }

    /// Stock d'une ressource dans un hameau : pops qui y vivent + réserve commune.
    pub fn stock_hameau(&self, q: usize, res: usize) -> Qte {
        self.monde.quartiers[q].stock[res]
            + self.pops.iter().filter(|p| p.residence == q).map(|p| p.stock[res]).sum::<Qte>()
    }

    /// Verse `qte` aux pops du hameau `q`, au prorata de ce qui leur manque pour atteindre leur cible de
    /// stock (à parts égales si rien ne manque). Sans habitant, la marchandise reste dans la réserve du hameau.
    fn distribuer_hameau(&mut self, q: usize, res: usize, qte: Qte) {
        if qte <= 0 {
            return;
        }
        let idx: Vec<usize> = (0..self.pops.len()).filter(|&i| self.pops[i].residence == q).collect();
        if idx.is_empty() {
            self.monde.quartiers[q].stock[res] += qte;
            return;
        }
        let cible = Catalogue::jours_x(self.cat.ration_pop(res), self.cat.sim.jours_stock_quartier);
        let mut poids: Vec<Qte> = idx.iter().map(|&i| (cible - self.pops[i].stock[res]).max(0)).collect();
        if poids.iter().sum::<Qte>() == 0 {
            poids = vec![1; idx.len()];
        }
        let parts = partager(qte, &poids);
        for (k, &i) in idx.iter().enumerate() {
            self.pops[i].stock[res] += parts[k];
        }
    }

    /// Les réserves de hameau sans habitant sont versées aux pops qui s'y installent.
    fn vider_reserves(&mut self) {
        for q in 0..self.nq() {
            for r in 0..self.cat.ressources.len() {
                let v = std::mem::take(&mut self.monde.quartiers[q].stock[r]);
                if v > 0 {
                    self.distribuer_hameau(q, r, v);
                }
            }
        }
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
            self.distribuer_hameau(client, res, qte);
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
        self.stats.flux.clear();
        self.stats.produits_par_lieu.clear();

        self.vider_reserves();
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
                    let t = &self.cat.batiments[bat.type_id];
                    // Un atelier domestique ne commande que de quoi vêtir les pops de son hameau,
                    // pas de quoi tourner à plein régime (sinon il accapare la fibre).
                    let frac = match (t.domestique, bat.quartier) {
                        (true, Some(q)) => t
                            .extrants_finaux()
                            .iter()
                            .map(|&(r, qf)| {
                                mul_div(self.cat.ration_pop(r) * eff[q] as i64, PPM, qf.max(1))
                            })
                            .max()
                            .unwrap_or(PPM)
                            .min(PPM),
                        _ => PPM,
                    };
                    for (r, q) in t.intrants_externes() {
                        besoins.push((r, mul_div(q, frac, PPM), self.cat.sim.jours_stock_entree));
                    }
                }
            }
            besoins.sort_by_key(|b| b.0);
            for (res, requis, jours) in besoins {
                if requis <= 0 {
                    continue;
                }
                let stock = if client < nq {
                    self.stock_hameau(client, res)
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
        // 0 = tous les fournisseurs du rayon d'approvisionnement (`distance_max`), du plus proche au plus loin.
        let rondes = if self.cat.sim.nb_rondes == 0 { usize::MAX } else { self.cat.sim.nb_rondes };
        for k in 0..rondes {
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
            self.stats.flux.push(Flux { de: f, vers: c, ressource: res, qte });
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

    /// Autonomie d'une pop : combien de jours son stock personnel tient pour chaque besoin.
    pub fn autonomies(&self, p: &Pop) -> Autonomies {
        let cat = &self.cat;
        let nutri: Qte = (0..cat.ressources.len())
            .map(|r| p.stock[r] * cat.ressources[r].nutritive_milli / 1000)
            .sum();
        let jours = |stock: Qte, par_jour: Qte| if par_jour > 0 { stock * 1000 / par_jour } else { i64::MAX };
        let mut par = vec![i64::MAX; cat.ressources.len()];
        for b in &cat.besoins {
            par[b.ressource] = jours(p.stock[b.ressource], b.par_pop);
        }
        Autonomies {
            nutrition: jours(nutri, cat.vital.nourriture),
            chauffage: jours(p.stock[cat.vital.chauffage_ressource], cat.vital.chauffage),
            par_ressource: par,
        }
    }

    /// Terres à portée de la pop `i`, de la plus pressante à la moins pressante (docs/02 §10.4) :
    /// d'abord ce qui lui manque pour vivre (nourriture, chauffage), puis le confort, puis les matières
    /// premières dont un atelier manque, puis l'entretien des stocks ; à classe égale, la plus grande
    /// pénurie d'abord, puis la plus proche. `demande[b]` : travail encore réclamé par la terre `b`.
    pub fn candidats(&self, i: usize, demande: &[i64], tension: &[i64]) -> Vec<Candidat> {
        let nq = self.nq();
        let p = &self.pops[i];
        let a = self.autonomies(p);
        let cible = (self.cat.sim.jours_stock_quartier * 1000.0).round() as i64;
        let manque = |aut: i64| (cible - aut).max(0);
        let mut v: Vec<Candidat> = Vec::new();
        for (b, bat) in self.monde.batiments.iter().enumerate() {
            let t = &self.cat.batiments[bat.type_id];
            if !t.par_tile || bat.chantier.is_some() {
                continue;
            }
            let Some(ch) = self.reseau.chemin(p.residence, nq + b) else {
                continue;
            };
            let Some(&(r, _)) = t.extrants_finaux().first() else {
                continue;
            };
            let (classe, m) = if self.cat.ressources[r].nutritive_milli > 0 && manque(a.nutrition) > 0 {
                (Classe::Vital, manque(a.nutrition))
            } else if r == self.cat.vital.chauffage_ressource && manque(a.chauffage) > 0 {
                (Classe::Vital, manque(a.chauffage))
            } else if self.cat.confort.iter().any(|c| c.ressource == r) && manque(a.par_ressource[r]) > 0 {
                (Classe::Confort, manque(a.par_ressource[r]))
            } else if tension.get(r).copied().unwrap_or(0) > 0 {
                (Classe::Intrant, 0)
            } else {
                (Classe::Entretien, 0)
            };
            v.push(Candidat {
                lieu: b,
                cout: ch.cout,
                ressource: r,
                classe,
                manque_milli_jours: m,
                demande_milli: demande[b],
            });
        }
        v.sort_by_key(|c| (c.classe as u8, std::cmp::Reverse(c.manque_milli_jours), c.cout, c.lieu));
        v
    }

    /// Les candidats d'une pop d'après l'état de fin de journée (pour l'affichage) : le lieu choisi
    /// est celui de `Pop::emploi`, le premier de la liste qui réclamait assez de travail ce matin-là.
    pub fn expliquer_pop(&self, id: u32) -> Option<Vec<Candidat>> {
        let i = self.pops.iter().position(|p| p.id == id)?;
        let demande: Vec<i64> = self.monde.batiments.iter().map(|b| b.demande_milli).collect();
        Some(self.candidats(i, &demande, &self.poids_tension()))
    }

    /// Demande de travail de chaque terre (milli pop-jours) : de quoi remplir son stock de sortie.
    fn demandes_de_travail(&self) -> Vec<i64> {
        self.monde
            .batiments
            .iter()
            .map(|bat| {
                let t = &self.cat.batiments[bat.type_id];
                if !t.par_tile || bat.chantier.is_some() {
                    return 0;
                }
                let n = bat.emprise.len() as i64;
                let Some(&(r, q)) = t.extrants_finaux().first() else {
                    return 0;
                };
                let plafond = Catalogue::jours_x(q * n, self.cat.sim.jours_stock_sortie);
                let libre = (plafond - bat.stock_sortie[r]).max(0);
                let jours_de_place_ppm = (libre * PPM / (q * n).max(1)).min(bat.sur_pied_ppm);
                self.cat.besoin_mo_terre_milli(bat.type_id, &bat.emprise, &self.monde.tuiles) * jours_de_place_ppm / PPM
            })
            .collect()
    }

    /// Chaque pop libre choisit **un seul lieu** de travail pour la journée (docs/02 §10.4) :
    /// la terre la plus utile à ce qui lui manque, qui réclame encore assez de travail. Une pop fournit
    /// au plus un pop-jour ; le reste de sa journée est du temps libre (travail domestique à la maison).
    /// Les équipes de chantier ont un poste fixe. Les pops passent dans un ordre qui tourne chaque jour.
    fn resoudre_main_oeuvre(&mut self) {
        let nq = self.nq();
        for b in &mut self.monde.batiments {
            b.presents = 0;
            b.mo_milli = 0;
        }
        let poids = self.cat.infra.poids_trajet_pop;
        // 1. Équipes de chantier : trajet complet de chaque pop.
        let mut libres: Vec<usize> = Vec::new();
        for i in 0..self.pops.len() {
            self.pops[i].travail_milli = 0;
            match self.pops[i].emploi {
                Some(emploi) if self.est_chantier_type(emploi) => {
                    self.pops[i].motif = Motif::Chantier;
                    let res = self.pops[i].residence;
                    let ok = self.charger(res, nq + emploi, poids);
                    if ok >= poids {
                        self.monde.batiments[emploi].presents += 1;
                        self.pops[i].travail_milli = 1000;
                    } else {
                        // La charge partielle n'est pas rendue : la pop n'est pas arrivée mais a encombré la route.
                        self.stats.retards_pops += 1;
                    }
                }
                _ => {
                    self.pops[i].emploi = None;
                    libres.push(i);
                }
            }
        }
        // 2. Les pops libres choisissent, l'une après l'autre ; le travail réclamé par une terre baisse d'autant.
        let tension = self.poids_tension();
        let mut demande = self.demandes_de_travail();
        let seuil = self.cat.travail.seuil_deplacement_milli;
        let mut cap = vec![0i64; nq];
        libres.sort_by_key(|&i| self.pops[i].id);
        let depart = if libres.is_empty() { 0 } else { self.jour as usize % libres.len() };
        for k in 0..libres.len() {
            let i = libres[(k + depart) % libres.len()];
            let q = self.pops[i].residence;
            let choix = self
                .candidats(i, &demande, &tension)
                .into_iter()
                .find(|c| demande[c.lieu] >= seuil);
            match choix {
                Some(c) => {
                    let b = c.lieu;
                    let travail = demande[b].min(1000);
                    let ok = self.charger(q, nq + b, poids);
                    let eff = if poids > 0 { mul_div(travail, ok, poids) } else { travail };
                    if ok < poids {
                        self.stats.retards_pops += 1;
                    }
                    demande[b] -= travail;
                    self.monde.batiments[b].mo_milli += eff;
                    self.pops[i].emploi = Some(b);
                    self.pops[i].travail_milli = eff;
                    self.pops[i].motif = Motif::Besoin {
                        ressource: c.ressource,
                        classe: c.classe,
                        manque_milli_jours: c.manque_milli_jours,
                    };
                    cap[q] += 1000 - travail;
                }
                None => {
                    self.pops[i].motif = Motif::Repos;
                    cap[q] += 1000;
                }
            }
        }
        // 3. Bilan de la journée : travail réclamé et non fait, temps libre, moyennes mobiles.
        for (b, bat) in self.monde.batiments.iter_mut().enumerate() {
            bat.demande_milli = demande[b];
            bat.mo_lisse_milli += (bat.mo_milli - bat.mo_lisse_milli) / 30;
        }
        for p in &mut self.pops {
            p.travail_lisse_milli += (p.travail_milli - p.travail_lisse_milli) / 30;
        }
        self.libre_lisse_milli.resize(nq, 0);
        for q in 0..nq {
            self.libre_lisse_milli[q] += (cap[q] - self.libre_lisse_milli[q]) / 30;
        }
        self.libre_milli = cap;
    }

    fn produire(&mut self) {
        let libre = self.libre_milli.clone();
        let cat = &self.cat;
        let tuiles = &self.monde.tuiles;
        for (idx_b, b) in self.monde.batiments.iter_mut().enumerate() {
            let t = &cat.batiments[b.type_id];
            if t.chantier {
                Self::avancer_chantier(b, cat, &mut self.bilan);
                continue;
            }
            // Terre : N tiles, extrants et travail proportionnels à N ; rendement moyen de l'emprise
            // (fertilité de chaque tile × aptitude de son terrain pour ce type de terre, docs/07 §3.3).
            let n = if t.par_tile { b.emprise.len() as i64 } else { 1 };
            let fert = if t.fertile && !b.emprise.is_empty() {
                b.emprise.iter().map(|&i| cat.rendement_tile(b.type_id, &tuiles[i])).sum::<i64>() / b.emprise.len() as i64
            } else if t.fertile {
                cat.rendement_tile(b.type_id, &tuiles[b.tuile])
            } else {
                PPM
            };
            let f_mo = if t.domestique {
                // Travail domestique : temps libre du hameau / travail requis à plein régime.
                let q = b.quartier.expect("atelier domestique sans hameau");
                (libre.get(q).copied().unwrap_or(0) * PPM / (t.pops as i64 * 1000)).min(PPM)
            } else if t.par_tile {
                // Le travail arrive par visites : un pop-jour peut valoir plusieurs jours de production
                // (borné par la place restante dans le stock de sortie).
                let besoin = cat.besoin_mo_terre_milli(b.type_id, &b.emprise, tuiles).max(1);
                b.mo_milli * PPM / besoin
            } else {
                b.presents as i64 * PPM / t.pops as i64
            };
            let mut f_min = PPM;
            let mut limitant = Limitant::Aucun;
            let mut f_dernier = 0;
            for (si, e) in t.etapes.iter().enumerate() {
                let mut f = if t.par_tile { f_mo } else { f_mo.min(PPM) };
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
                    let plafond = Catalogue::jours_x(q * n, cat.sim.jours_stock_sortie);
                    let libre_stock = (plafond - b.stock_sortie[r]).max(0);
                    let f_out = libre_stock * PPM / (q * n).max(1);
                    if f_out < f {
                        f = f_out;
                        lim = Limitant::StockPlein;
                    }
                }
                // Une terre ne produit pas plus que ce qui a poussé.
                let f = if t.par_tile { f.min(b.sur_pied_ppm) } else { f }.max(0);
                for &(r, q) in &e.intrants {
                    let c = mul_div(q, f, PPM);
                    b.stock_entree[r] -= c;
                    self.bilan.consomme_prod[r] += c;
                }
                for &(r, q) in &e.extrants {
                    let p = mul_div(mul_div(q * n, f, PPM), fert, PPM);
                    self.bilan.produit[r] += p;
                    if t.consomme_ensuite(si, r) {
                        b.stock_entree[r] += p; // matière intermédiaire, consommée par l'étape suivante
                    } else {
                        b.stock_sortie[r] += p;
                        self.stats.produit[r] += p;
                        if p > 0 {
                            self.stats.produits_par_lieu.push((idx_b, r, p));
                        }
                    }
                }
                if f < f_min {
                    f_min = f;
                    limitant = lim;
                }
                f_dernier = f;
            }
            if t.par_tile {
                let plafond_pied = (cat.sim.jours_stock_sortie * PPM as f64) as i64;
                b.sur_pied_ppm = (b.sur_pied_ppm - f_dernier + PPM).clamp(0, plafond_pied);
                // Moyenne mobile : la production d'une terre est faite de visites espacées.
                b.prod_ppm += (f_dernier.min(PPM * 20) - b.prod_ppm) / 30;
                let sortie_pleine = t.extrants_finaux().first().map_or(false, |&(r, q)| {
                    b.stock_sortie[r] * 10 >= Catalogue::jours_x(q * n, cat.sim.jours_stock_sortie) * 9
                });
                b.limitant = if b.demande_milli >= cat.travail.seuil_deplacement_milli {
                    Limitant::MainOeuvre
                } else if sortie_pleine {
                    Limitant::StockPlein
                } else {
                    Limitant::Aucun
                };
            } else {
                b.prod_ppm = f_dernier;
                b.limitant = if f_min >= PPM {
                    Limitant::Aucun
                } else {
                    limitant
                };
            }
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
                    let n = self.monde.batiments[b].emprise.len();
                    self.journal(format!(
                        "Nouvelle terre exploitée : {nom} de {n} tiles en ({x}, {y})"
                    ));
                }
                Cible::Extension(terre) => {
                    // Les tiles défrichées passent à la terre ; le site de chantier redevient inerte.
                    let tuile = self.monde.batiments[b].tuile;
                    let nouvelles = std::mem::replace(&mut self.monde.batiments[b].emprise, vec![tuile]);
                    for &t in &nouvelles {
                        self.monde.tuiles[t].contenu = Contenu::Batiment(terre);
                    }
                    self.monde.batiments[terre].emprise.extend(nouvelles.iter().copied());
                    self.monde.batiments[b].equipe = 0;
                    let mat = self.cat.extension.materiau;
                    let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
                    self.monde.batiments[terre].stock_entree[mat] += reste;
                    let nom = self.cat.batiments[self.monde.batiments[terre].type_id].nom.clone();
                    let n = self.monde.batiments[terre].emprise.len();
                    self.journal(format!(
                        "Terre agrandie : {nom}, {} tiles de plus ({n} au total), près de ({x}, {y})",
                        nouvelles.len()
                    ));
                }
                Cible::Hameau(q) => {
                    let h = &self.cat.habitats[self.monde.quartiers[q].habitat_id];
                    let (cap, atelier, nom) = (h.capacite_fondation, h.atelier, h.nom.clone());
                    self.monde.quartiers[q].capacite_pops = cap;
                    self.monde.batiments[b].equipe = 0;
                    // Le matériau resté sur le chantier revient au hameau.
                    let mat = self.cat.extension.materiau;
                    let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
                    self.distribuer_hameau(q, mat, reste);
                    if let Some(at) = atelier {
                        self.monde.batiments[b].type_id = at;
                        self.monde.batiments[b].quartier = Some(q);
                        self.monde.quartiers[q].atelier = Some(b);
                    }
                    self.journal(format!("Nouveau hameau construit : {nom} en ({x}, {y})"));
                    self.migrer_vers(q, cap);
                }
                Cible::Maison(q) => {
                    self.monde.quartiers[q].capacite_pops += 1;
                    self.monde.batiments[b].equipe = 0;
                    // Le matériau resté sur le chantier revient au hameau.
                    let mat = self.cat.extension.materiau;
                    let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
                    self.distribuer_hameau(q, mat, reste);
                    let cap = self.monde.quartiers[q].capacite_pops;
                    self.journal(format!(
                        "Nouvelle maison dans le hameau en ({x}, {y}) : {cap} maisons"
                    ));
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

    /// Des pops sans poste quittent leur hameau pour s'établir dans le hameau neuf `q` (au plus `n`),
    /// les plus proches d'abord : c'est ainsi que le peuplement gagne de nouvelles terres.
    fn migrer_vers(&mut self, q: usize, n: u32) {
        let d = self.reseau.distances_depuis(self.monde.quartiers[q].tuile);
        let mut candidats: Vec<(i64, u32, usize)> = (0..self.pops.len())
            .filter(|&i| self.pops[i].emploi.is_none() && self.pops[i].residence != q)
            .map(|i| {
                let t = self.monde.quartiers[self.pops[i].residence].tuile;
                (d[t], self.pops[i].id, i)
            })
            .filter(|&(c, _, _)| c != i64::MAX)
            .collect();
        candidats.sort();
        for &(_, _, i) in candidats.iter().take(n as usize) {
            self.pops[i].residence = q;
        }
    }

    /// Chaque pop consomme sur son propre stock ; le hameau agrège la couverture de ses habitants.
    fn consommer(&mut self) {
        let nr = self.cat.ressources.len();
        let nq = self.nq();
        let mut n = vec![0i64; nq];
        let mut vital = vec![0i64; nq];
        let mut confort = vec![0i64; nq];
        let mut sat = vec![vec![0i64; nr]; nq];
        for p in &mut self.pops {
            let mut recu = vec![0; nr];
            for b in &self.cat.besoins {
                let r = p.stock[b.ressource].min(b.par_pop).max(0);
                p.stock[b.ressource] -= r;
                recu[b.ressource] = r;
                self.bilan.consomme_pops[b.ressource] += r;
                self.stats.consomme_pops[b.ressource] += r;
            }
            let ev = evaluer_besoins(&self.cat, 1, &recu);
            p.vital_jour_ppm = ev.vital_ppm;
            p.confort_jour_ppm = ev.confort_ppm;
            p.sat_par_ressource_ppm = ev.par_ressource_ppm;
            p.conso_jour = recu;
            let q = p.residence;
            n[q] += 1;
            vital[q] += p.vital_jour_ppm;
            confort[q] += p.confort_jour_ppm;
            for r in 0..nr {
                sat[q][r] += p.sat_par_ressource_ppm[r];
            }
        }
        for q in 0..nq {
            let quartier = &mut self.monde.quartiers[q];
            let (v, c, s) = if n[q] == 0 {
                (PPM, PPM, vec![PPM; nr])
            } else {
                (vital[q] / n[q], confort[q] / n[q], sat[q].iter().map(|x| x / n[q]).collect())
            };
            quartier.vital_jour_ppm = v;
            quartier.confort_jour_ppm = c;
            quartier.sat_par_ressource_ppm = s;
            quartier.cumul_vital_ppm += v;
            quartier.cumul_confort_ppm += c;
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
        for p in &mut self.pops {
            let taux = taux_croissance_ppm(&self.cat.demo, p.vital_jour_ppm, p.confort_jour_ppm);
            p.croissance += mul_div(seuil, taux, PPM * jours);
        }

        // Disparitions d'abord (elles libèrent du logement et des postes).
        let morts: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance <= -seuil)
            .collect();
        let mut heritages: Vec<(usize, Vec<Qte>)> = Vec::new();
        for &i in morts.iter().rev() {
            let p = self.pops.remove(i);
            heritages.push((p.residence, p.stock));
            self.deces_an += 1;
        }
        // Le stock d'une pop disparue revient aux habitants de son hameau (ou à sa réserve).
        for (q, stock) in heritages {
            for (r, v) in stock.into_iter().enumerate() {
                self.distribuer_hameau(q, r, v);
            }
        }

        // Duplications : la nouvelle pop se loge et prend le poste vacant le plus proche.
        let mut eff = self.effectifs();
        let parents: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance >= seuil)
            .collect();
        for i in parents {
            let q = self.pops[i].residence;
            match self.hameau_libre(q, &eff) {
                Some(h) => {
                    let id = self.prochain_pop_id;
                    self.prochain_pop_id += 1;
                    self.pops.push(Pop::nouvelle(id, h, 0, self.cat.ressources.len()));
                    eff[h] += 1;
                    self.pops[i].croissance -= seuil;
                    self.naissances_an += 1;
                }
                // Plus de logement : la jauge attend, plafonnée.
                None => self.pops[i].croissance = seuil,
            }
        }
    }

    // ---------- Tension ----------

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
        // Un intrant qui manque ne compte que si ce que le bâtiment produit manque lui aussi : un atelier
        // qui tourne au ralenti parce qu'on ne lui livre que de quoi vêtir son hameau n'est pas en tension.
        let rations = poids.clone();
        for bat in &self.monde.batiments {
            if let Limitant::Intrant(r) = bat.limitant {
                let t = &self.cat.batiments[bat.type_id];
                if t.extrants_finaux().iter().any(|&(x, _)| rations[x] > 0) {
                    poids[r] += PPM - bat.prod_ppm;
                }
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
                // Un atelier domestique existe dans tout hameau : en bâtir un de plus n'aide pas.
                !self.cat.batiments[a].domestique
                    && self.cat.batiments[a]
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
                    !self.cat.batiments[a].domestique
                        && self.cat.batiments[a]
                            .extrants_finaux()
                            .iter()
                            .any(|(x, _)| *x == r)
                })
            }
            Some(Chantier {
                cible: Cible::Extension(t),
                ..
            }) => self.cat.batiments[self.monde.batiments[*t].type_id]
                .extrants_finaux()
                .iter()
                .any(|(x, _)| *x == r),
            Some(Chantier {
                cible: Cible::Maison(_),
                ..
            }) => false,
            None => false,
        })
    }

    /// Décisions saisonnières de la communauté, après la réaffectation mensuelle (donc une fois
    /// tous les postes disponibles pris) : loger les pops qui attendent, puis défricher pour
    /// la ressource la plus en tension tant que des bras restent sans poste.
    pub fn etendre(&mut self) {
        // Les chantiers incomplets recrutent à nouveau.
        for b in 0..self.monde.batiments.len() {
            if self.monde.batiments[b].chantier.is_none() {
                continue;
            }
            let equipe = self.monde.batiments[b].equipe;
            let present = self.pops.iter().filter(|p| p.emploi == Some(b)).count() as u32;
            if present < equipe {
                self.recruter(b, equipe - present);
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
        if !attente.is_empty() {
            let motif = format!("{} pops attendent un logement", attente.len());
            let centres = self.centres(attente.iter().map(|&i| self.pops[i].residence));
            // 1a. Une maison de plus dans le hameau d'une pop qui attend, tant qu'il a de la place.
            for &centre in &centres {
                if self.lancer_maison(centre, &motif) {
                    return;
                }
            }
            // 1b. Sinon, un nouveau hameau (une seule fondation à la fois), sauf si une maison déjà
            // en chantier va loger ces pops.
            let maison_en_cours = centres.iter().any(|&c| self.maison_en_cours(c));
            if !hameau_en_cours && !maison_en_cours {
                let habitat = tensions
                    .iter()
                    .filter(|&&r| !self.producteurs_bloques_par_intrant(r))
                    .find_map(|&r| self.habitat_producteur(r))
                    .unwrap_or(0);
                for &centre in &centres {
                    if self.lancer_hameau(centre, habitat, &motif) {
                        return;
                    }
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
            // Les hameaux qui manquent de cette ressource d'abord (ils défrichent près d'eux), puis ceux
            // où des bras restent sans poste.
            let seuil_t = self.cat.mobilite.seuil_tension_ppm;
            let effs = self.effectifs();
            let mut manque: Vec<(i64, usize)> = (0..nq)
                .filter(|&q| effs[q] > 0 && self.monde.quartiers[q].sat_par_ressource_ppm[r] < seuil_t)
                .map(|q| (self.monde.quartiers[q].sat_par_ressource_ppm[r], q))
                .collect();
            manque.sort();
            let mut ordre: Vec<usize> = manque.into_iter().map(|(_, q)| q).collect();
            for &c in &centres {
                if !ordre.contains(&c) {
                    ordre.push(c);
                }
            }
            let centres_r = ordre;
            let mut lance = false;
            for &centre in &centres_r {
                lance = if let Some(t) = self.type_terre_pour(r) {
                    self.lancer_defrichement(centre, t, &motif)
                } else if let (Some(h), false) = (self.habitat_producteur(r), hameau_en_cours) {
                    self.lancer_hameau(centre, h, &motif)
                } else {
                    false
                };
                if lance {
                    break;
                }
            }
            // Plus de terre à portée des hameaux : les bras sans poste vont s'établir plus loin
            // (un hameau près d'une terre non défrichée, docs/02 §8.5).
            if !lance && !hameau_en_cours && self.type_terre_pour(r).is_some() {
                let motif = format!(
                    "plus de terre à portée pour {}, {} pops sans poste",
                    self.cat.ressources[r].nom, nb_sans
                );
                for &centre in &centres_r {
                    if self.lancer_hameau(centre, 0, &motif) {
                        lance = true;
                        break;
                    }
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
    /// soit relié par une arête à une tile du réseau, soit au bout d'une nouvelle tile intermédiaire
    /// (deux arêtes), dans 8 directions. Les coûts sont ceux des arêtes (10 / 14).
    /// Terre fertile : fertilité / distance ; sinon : le plus proche, sur la terre la plus pauvre.
    /// `requis` : groupes de ressources dont l'une au moins doit avoir un producteur à portée du site.
    fn meilleur_site(&self, centre: usize, fertile: bool, type_id: Option<usize>, requis: &[Vec<usize>]) -> Option<Site> {
        let nq = self.nq();
        let d = self
            .reseau
            .distances_depuis(self.monde.quartiers[centre].tuile);
        // Coût de chaque tile du réseau aux producteurs de chaque groupe requis.
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
        // Une tile traversée par une arête n'est pas encore constructible (docs/02 §9, à lever plus tard).
        let vide = |t: usize| {
            self.monde.tuiles[t].contenu == Contenu::Vide
                && self.monde.tuiles[t].aretes == 0
                && self.monde.tuiles[t].constructible()
        };
        // Le lieu posé à `supplement` d'arêtes de `attache` est-il desservi ?
        let desservi = |attache: usize, supplement: i64| {
            portees
                .iter()
                .all(|dp| dp[attache] != i64::MAX && dp[attache] + supplement <= dmax)
        };
        let mut best: Option<((i64, i64, i64, usize), Site)> = None;
        let essayer = |tuile: usize,
                       attache: usize,
                       route: &[usize],
                       dist: i64,
                       best: &mut Option<((i64, i64, i64, usize), Site)>| {
            let fert = self.fert_pour(type_id, tuile);
            let cle = if fertile {
                (
                    -(fert * 10_000 / (dist + 10)),
                    dist,
                    route.len() as i64,
                    tuile,
                )
            } else {
                (dist, route.len() as i64, fert, tuile)
            };
            if best.as_ref().map_or(true, |(k, _)| cle < *k) {
                *best = Some((
                    cle,
                    Site {
                        tuile,
                        attache,
                        route: route.to_vec(),
                    },
                ));
            }
        };
        // Dijkstra depuis tout le réseau atteignable : une route neuve ne traverse que les tiles dont le
        // type est traversable (habitat) ; elle contourne terres et chantiers (docs/02 §10.1) et ne rejoint
        // pas une autre arête ; le site lui-même doit être libre.
        let n = self.monde.tuiles.len();
        let mut cout = vec![i64::MAX; n];
        let mut prec = vec![usize::MAX; n];
        let mut racine = vec![usize::MAX; n];
        let mut tas: std::collections::BinaryHeap<std::cmp::Reverse<(i64, usize)>> =
            std::collections::BinaryHeap::new();
        // Terre : le site doit rester à portée du hameau demandeur (coût total ≤ dmax). Hameau : il se
        // fonde en périphérie du réseau, à `ROUTE_FONDATION_MAX` de route neuve au plus ; ce qui compte
        // est l'accès aux producteurs depuis le site (`desservi`).
        let limite_route = if fertile { dmax } else { ROUTE_FONDATION_MAX };
        for p in 0..n {
            if d[p] != i64::MAX {
                cout[p] = d[p];
                racine[p] = p;
                tas.push(std::cmp::Reverse((d[p], p)));
            }
        }
        while let Some(std::cmp::Reverse((c, t))) = tas.pop() {
            if c > cout[t] {
                continue;
            }
            for dir in 0..8 {
                let Some(v) = self.monde.voisin(t, dir) else {
                    continue;
                };
                if self.monde.tuiles[v].aretes != 0 || !self.monde.peut_relier(t, v) {
                    continue;
                }
                // Traversabilité (docs/02 §10.1) : une route neuve contourne les terres et les chantiers.
                let surcout = match self.traversabilite(v) {
                    Traversabilite::Libre => 0,
                    Traversabilite::Couteuse => SURCOUT_TRAVERSEE,
                    Traversabilite::Interdite => continue,
                };
                let nc = c + cout_dir(dir) + surcout;
                let ok = if fertile { nc <= dmax } else { nc - d[racine[t]] <= limite_route };
                if ok && nc < cout[v] {
                    cout[v] = nc;
                    prec[v] = t;
                    racine[v] = racine[t];
                    tas.push(std::cmp::Reverse((nc, v)));
                }
            }
        }
        for v in 0..n {
            if prec[v] == usize::MAX || !vide(v) {
                continue;
            }
            let r = racine[v];
            if !desservi(r, cout[v] - d[r]) {
                continue;
            }
            let mut route: Vec<usize> = Vec::new();
            let mut t = prec[v];
            while t != r {
                route.push(t);
                t = prec[t];
            }
            route.reverse();
            essayer(v, r, &route, cout[v], &mut best);
        }
        best.map(|(_, s)| s)
    }

    /// Traversabilité d'une tile par une route neuve, selon son contenu (docs/02 §10.1).
    pub fn traversabilite(&self, t: usize) -> Traversabilite {
        // Eau, forêt et marais ne se traversent pas (pas de pont ni de défrichement dans cette étape, docs/07 §3).
        if !self.monde.tuiles[t].constructible() {
            return Traversabilite::Interdite;
        }
        match self.monde.tuiles[t].contenu {
            Contenu::Vide | Contenu::Quartier(_) => Traversabilite::Libre,
            Contenu::Batiment(b) => self.cat.batiments[self.monde.batiments[b].type_id].traversable,
        }
    }

    fn poser_site(&mut self, site: &Site) {
        let mut prec = site.attache;
        for &r in &site.route {
            self.monde.relier(prec, r);
            prec = r;
        }
        self.monde.relier(prec, site.tuile);
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
            emprise: vec![tuile],
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
            mo_milli: 0,
            prod_ppm: 0,
            demande_milli: 0,
            sur_pied_ppm: 0,
            mo_lisse_milli: 0,
            limitant: Limitant::Aucun,
        }
    }

    /// Choisit jusqu'à `k` tiles libres contiguës (voisines par un côté) à `base`, de proche en proche,
    /// les plus fertiles d'abord. Une tile libre est vide et traversée par aucune arête.
    ///
    /// Score de chaque candidate (docs/02 §10.2, poids dans `[extension]`) : fertilité, plus un bonus par
    /// voisin déjà dans l'emprise (périmètre court : clôtures, trajets), moins la distance au hameau
    /// demandeur, moins une pénalité hors du terroir de la terre (hameau le plus proche différent).
    fn agrandir(&self, base: &[usize], k: usize, hameau: Option<usize>, type_id: Option<usize>) -> Vec<usize> {
        let libre = |t: usize| {
            self.monde.tuiles[t].contenu == Contenu::Vide
                && self.monde.tuiles[t].aretes == 0
                && self.monde.tuiles[t].constructible()
        };
        let ext = &self.cat.extension;
        let terroir_terre = base.first().and_then(|&t| self.terroir(t));
        let mut occupees: Vec<usize> = base.to_vec();
        let mut choisies: Vec<usize> = Vec::new();
        while choisies.len() < k {
            let mut meilleure: Option<((i64, std::cmp::Reverse<usize>), usize)> = None;
            for &t in &occupees {
                for d in [0usize, 2, 4, 6] {
                    let Some(v) = self.monde.voisin(t, d) else {
                        continue;
                    };
                    if !libre(v) || occupees.contains(&v) {
                        continue;
                    }
                    let voisins = [0usize, 2, 4, 6]
                        .iter()
                        .filter_map(|&e| self.monde.voisin(v, e))
                        .filter(|w| occupees.contains(w))
                        .count() as i64;
                    let mut score = self.fert_pour(type_id, v) + voisins * ext.poids_voisins_ppm;
                    if let Some(h) = hameau {
                        score -= self.distance_tiles(v, self.monde.quartiers[h].tuile)
                            * ext.poids_distance_ppm;
                    }
                    if terroir_terre.is_some() && self.terroir(v) != terroir_terre {
                        score -= ext.penalite_terroir_ppm;
                    }
                    let cle = (score, std::cmp::Reverse(v));
                    if meilleure.map_or(true, |(c, _)| cle > c) {
                        meilleure = Some((cle, v));
                    }
                }
            }
            let Some((_, v)) = meilleure else { break };
            occupees.push(v);
            choisies.push(v);
        }
        choisies
    }

    /// Distance entre deux tiles en tiles (octile : max + min / 2).
    fn distance_tiles(&self, a: usize, b: usize) -> i64 {
        let ((ax, ay), (bx, by)) = (self.monde.xy(a), self.monde.xy(b));
        let (dx, dy) = ((ax as i64 - bx as i64).abs(), (ay as i64 - by as i64).abs());
        dx.max(dy) + dx.min(dy) / 2
    }

    /// Terroir d'une tile : le hameau le plus proche (distance à vol d'oiseau ; égalité : le premier).
    fn terroir(&self, t: usize) -> Option<usize> {
        (0..self.monde.quartiers.len())
            .min_by_key(|&q| (self.distance_tiles(t, self.monde.quartiers[q].tuile), q))
    }

    /// Réutilise le site de chantier inerte posé sur `tuile`, ou en crée un.
    fn site_de_chantier(&mut self, tuile: usize, cible: Cible, cout: &crate::donnees::CoutCreation) -> usize {
        let type_chantier = self.cat.type_chantier();
        let inerte = (0..self.monde.batiments.len()).find(|&b| {
            let bat = &self.monde.batiments[b];
            bat.tuile == tuile && bat.type_id == type_chantier && bat.chantier.is_none()
        });
        match inerte {
            Some(b) => {
                let bat = &mut self.monde.batiments[b];
                bat.equipe = cout.equipe;
                bat.chantier = Some(Chantier {
                    cible,
                    travail_restant: cout.travail,
                    travail_total: cout.travail,
                    materiau_total: cout.materiau,
                });
                b
            }
            None => {
                let b = self.monde.batiments.len();
                let chantier = self.nouveau_chantier(tuile, cible, cout);
                self.monde.batiments.push(chantier);
                b
            }
        }
    }

    /// Coût d'un défrichement des tiles `tuiles` : travail (multiplié par le coût de construction du terrain
    /// de chaque tile, docs/07 §3.3) et matériau par tile, équipe du chantier.
    fn cout_defrichement(&self, type_id: usize, tuiles: &[usize]) -> Option<crate::donnees::CoutCreation> {
        let c = self.cat.batiments[type_id].cout_creation.clone()?;
        let travail = tuiles
            .iter()
            .map(|&t| c.travail * self.cat.cout_terrain_ppm[self.monde.tuiles[t].terrain as usize] / PPM)
            .sum();
        Some(crate::donnees::CoutCreation {
            travail,
            materiau: c.materiau * tuiles.len() as i64,
            equipe: c.equipe,
        })
    }

    /// Coût d'un chantier de logement sur la tile `tuile` : le travail suit le terrain.
    fn cout_logement(&self, mut cout: crate::donnees::CoutCreation, tuile: usize) -> crate::donnees::CoutCreation {
        cout.travail = cout.travail * self.cat.cout_terrain_ppm[self.monde.tuiles[tuile].terrain as usize] / PPM;
        cout
    }

    /// Rendement d'une tile pour une terre de ce type (fertilité × aptitude du terrain), ou fertilité seule.
    fn fert_pour(&self, type_id: Option<usize>, t: usize) -> i64 {
        match type_id {
            Some(ty) => self.cat.rendement_tile(ty, &self.monde.tuiles[t]),
            None => self.monde.tuiles[t].fertilite_ppm,
        }
    }

    /// Défricher pour la ressource d'un type de terre : d'abord agrandir une terre existante du même
    /// type (moins coûteux, sans route), sinon en fonder une nouvelle.
    fn lancer_defrichement(&mut self, centre: usize, type_id: usize, motif: &str) -> bool {
        let nq = self.nq();
        let max = self.cat.batiments[type_id].emprise_max as usize;
        // Terres de ce type à portée du hameau, avec de la place pour grandir.
        let mut candidates: Vec<(i64, usize)> = Vec::new();
        for b in 0..self.monde.batiments.len() {
            let bat = &self.monde.batiments[b];
            if bat.type_id != type_id || bat.chantier.is_some() || bat.emprise.len() >= max {
                continue;
            }
            let Some(ch) = self.reseau.chemin(centre, nq + b) else {
                continue;
            };
            let voisines = self.agrandir(&bat.emprise, 1, Some(centre), Some(type_id));
            let Some(&v) = voisines.first() else {
                continue;
            };
            // Meilleure fertilité d'abord, puis la plus proche.
            candidates.push((-self.fert_pour(Some(type_id), v) * 1000 / (ch.cout + 10), b));
        }
        candidates.sort();
        for &(_, b) in &candidates {
            if self.lancer_extension(centre, b, motif) {
                return true;
            }
        }
        self.lancer_terre(centre, type_id, motif)
    }

    /// Agrandit la terre `b` de quelques tiles contiguës (`tuiles_par_defrichement`).
    fn lancer_extension(&mut self, centre: usize, b: usize, motif: &str) -> bool {
        let nq = self.nq();
        let type_id = self.monde.batiments[b].type_id;
        let max = self.cat.batiments[type_id].emprise_max as usize;
        let k = (self.cat.extension.tuiles_par_defrichement as usize)
            .min(max.saturating_sub(self.monde.batiments[b].emprise.len()));
        let nouvelles = self.agrandir(&self.monde.batiments[b].emprise, k, Some(centre), Some(type_id));
        if nouvelles.is_empty() {
            return false;
        }
        let Some(cout) = self.cout_defrichement(type_id, &nouvelles) else {
            return false;
        };
        let node = self.monde.batiments[b].tuile;
        if cout.materiau > 0 {
            let mat = self.cat.extension.materiau;
            let livrable = (0..self.monde.batiments.len())
                .any(|f| self.livre(f, mat) && self.reseau.chemin(nq + b, nq + f).is_some());
            if !livrable {
                return false;
            }
        }
        let c = self.site_de_chantier(node, Cible::Extension(b), &cout);
        self.monde.batiments[c].emprise = nouvelles.clone();
        for &t in &nouvelles {
            self.monde.tuiles[t].contenu = Contenu::Batiment(c);
            self.monde.tuiles[t].variante = (c % 4) as u8;
        }
        let (x, y) = self.monde.xy(node);
        let nom = self.cat.batiments[type_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : agrandir {nom} de {} tiles près de ({x}, {y}) — {motif}",
            nouvelles.len()
        ));
        self.reconstruire_reseau();
        self.recruter(c, cout.equipe);
        true
    }

    /// Fonde une terre : un bloc de `tuiles_par_defrichement` tiles contiguës, dont la tile « nœud »
    /// est reliée au réseau.
    fn lancer_terre(&mut self, centre: usize, type_id: usize, motif: &str) -> bool {
        let k = self.cat.extension.tuiles_par_defrichement as usize;
        let Some(cout_tile) = self.cat.batiments[type_id].cout_creation.clone() else {
            return false;
        };
        let requis: Vec<Vec<usize>> = if cout_tile.materiau > 0 {
            vec![vec![self.cat.extension.materiau]]
        } else {
            Vec::new()
        };
        let Some(site) = self.meilleur_site(centre, true, Some(type_id), &requis) else {
            return false;
        };
        let reste = self.agrandir(&[site.tuile], k.saturating_sub(1), Some(centre), Some(type_id));
        let mut emprise = vec![site.tuile];
        emprise.extend(reste);
        let Some(cout) = self.cout_defrichement(type_id, &emprise) else {
            return false;
        };
        self.poser_site(&site);
        let b = self.monde.batiments.len();
        let mut chantier = self.nouveau_chantier(site.tuile, Cible::Terre(type_id), &cout);
        chantier.emprise = emprise.clone();
        self.monde.batiments.push(chantier);
        for &t in &emprise {
            self.monde.tuiles[t].contenu = Contenu::Batiment(b);
            self.monde.tuiles[t].variante = (b % 4) as u8;
        }
        let (x, y) = self.monde.xy(site.tuile);
        let nom = self.cat.batiments[type_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : défricher {nom} ({} tiles) en ({x}, {y}) — {motif}",
            emprise.len()
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe);
        true
    }

    /// Une maison est-elle en chantier dans le hameau `q` ?
    fn maison_en_cours(&self, q: usize) -> bool {
        self.monde.batiments.iter().any(|b| {
            matches!(
                b.chantier,
                Some(Chantier {
                    cible: Cible::Maison(h),
                    ..
                }) if h == q
            )
        })
    }

    /// Une maison de plus (une pop) dans le hameau `q`, au plafond de son type près. Une seule à la
    /// fois par hameau ; le matériau doit pouvoir lui être livré.
    fn lancer_maison(&mut self, q: usize, motif: &str) -> bool {
        let nq = self.nq();
        let habitat_id = self.monde.quartiers[q].habitat_id;
        let cap = self.monde.quartiers[q].capacite_pops;
        if cap == 0 || cap >= self.cat.habitats[habitat_id].capacite_max_pops {
            return false;
        }
        let tuile = self.monde.quartiers[q].tuile;
        if self
            .monde
            .batiments
            .iter()
            .any(|b| b.tuile == tuile && b.chantier.is_some())
        {
            return false;
        }
        let cout = self.cout_logement(self.cat.habitats[habitat_id].cout_creation.clone(), tuile);
        if cout.materiau > 0 {
            let mat = self.cat.extension.materiau;
            let livrable = (0..self.monde.batiments.len())
                .any(|b| self.livre(b, mat) && self.reseau.chemin(q, nq + b).is_some());
            if !livrable {
                return false;
            }
        }
        // On réutilise le site de chantier inerte de la tile, s'il y en a un.
        let type_chantier = self.cat.type_chantier();
        let inerte = (0..self.monde.batiments.len()).find(|&b| {
            let bat = &self.monde.batiments[b];
            bat.tuile == tuile && bat.type_id == type_chantier && bat.chantier.is_none()
        });
        let b = match inerte {
            Some(b) => {
                let bat = &mut self.monde.batiments[b];
                bat.equipe = cout.equipe;
                bat.chantier = Some(Chantier {
                    cible: Cible::Maison(q),
                    travail_restant: cout.travail,
                    travail_total: cout.travail,
                    materiau_total: cout.materiau,
                });
                b
            }
            None => {
                let b = self.monde.batiments.len();
                let chantier = self.nouveau_chantier(tuile, Cible::Maison(q), &cout);
                self.monde.batiments.push(chantier);
                b
            }
        };
        let (x, y) = self.monde.xy(tuile);
        self.journal(format!(
            "Chantier lancé : une maison de plus dans le hameau en ({x}, {y}) — {motif}"
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe);
        true
    }

    fn lancer_hameau(&mut self, centre: usize, habitat_id: usize, motif: &str) -> bool {
        let cout = self.cat.habitats[habitat_id].cout_creation.clone();
        // Un hameau doit pouvoir être bâti (matériau), chauffé et nourri à portée de son site.
        // Le hameau neuf doit trouver à portée de son site de quoi satisfaire chaque ration, hors ce que son
        // atelier domestique produit lui-même, ainsi que la fibre de cet atelier.
        let atelier = self.cat.habitats[habitat_id].atelier;
        let propres: Vec<usize> = atelier
            .map(|a| self.cat.batiments[a].extrants_finaux().iter().map(|&(r, _)| r).collect())
            .unwrap_or_default();
        let mut requis: Vec<Vec<usize>> = self
            .cat
            .besoins
            .iter()
            .filter(|b| !propres.contains(&b.ressource))
            .map(|b| vec![b.ressource])
            .collect();
        if let Some(a) = atelier {
            for (r, _) in self.cat.batiments[a].intrants_externes() {
                requis.push(vec![r]);
            }
        }
        if cout.materiau > 0 {
            requis.push(vec![self.cat.extension.materiau]);
        }
        let Some(site) = self.meilleur_site(centre, false, None, &requis) else {
            return false;
        };
        // Le travail de fondation suit le terrain du site (docs/07 §3.3).
        let cout = self.cout_logement(cout, site.tuile);
        self.poser_site(&site);
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
        self.recruter(b, cout.equipe);
        true
    }

    /// Constitue l'équipe d'un chantier : les pops non engagées les plus proches (coût, puis identifiant),
    /// sans prendre plus de la moitié des pops d'un hameau (les terres ont besoin de bras, docs/02 §8.3).
    /// Pour la fondation d'un hameau (`fondation = Some(q)`), les pops peuvent venir de plus loin que
    /// `distance_max` : elles s'installent sur le site (leur résidence devient le hameau neuf).
    fn recruter(&mut self, chantier: usize, n: u32) {
        let nq = self.nq();
        let fondation = match &self.monde.batiments[chantier].chantier {
            Some(Chantier {
                cible: Cible::Hameau(q),
                ..
            }) => Some(*q),
            _ => None,
        };
        let eff = self.effectifs();
        let mut equipe = vec![0u32; nq];
        for p in &self.pops {
            if let Some(e) = p.emploi {
                if self.est_chantier_type(e) {
                    equipe[p.residence] += 1;
                }
            }
        }
        let distances = fondation.map(|_| {
            self.reseau
                .distances_depuis(self.monde.batiments[chantier].tuile)
        });
        let mut libres: Vec<(i64, u32, usize)> = (0..self.pops.len())
            .filter(|&i| {
                self.pops[i]
                    .emploi
                    .map_or(true, |e| !self.est_chantier_type(e))
            })
            .filter_map(|i| match &distances {
                Some(d) => {
                    let c = d[self.monde.quartiers[self.pops[i].residence].tuile];
                    (c != i64::MAX).then_some((c, self.pops[i].id, i))
                }
                None => self
                    .reseau
                    .chemin(self.pops[i].residence, nq + chantier)
                    .map(|c| (c.cout, self.pops[i].id, i)),
            })
            .collect();
        libres.sort();
        let mut pris = 0;
        for &(_, _, i) in &libres {
            if pris >= n {
                break;
            }
            let q = self.pops[i].residence;
            if equipe[q] >= (eff[q] / 2).max(1) {
                continue;
            }
            equipe[q] += 1;
            self.pops[i].emploi = Some(chantier);
            if let Some(nouveau) = fondation {
                self.pops[i].residence = nouveau;
            }
            pris += 1;
        }
    }

    /// Plan ASCII du village (pour l'observation en ligne de commande).
    /// H hameau, C champ, P pâturage, V verger, F forêt, L lin, W laine,
    /// + chemin, minuscules = chantier de la lettre correspondante, # chantier de hameau.
    /// Vue d'ensemble du terrain (1 caractère pour 2 × 2 tiles) et parts de chaque terrain ; `V` : un hameau.
    pub fn terrain_ascii(&self) -> String {
        use crate::terrain::{Eau, Terrain};
        let m = &self.monde;
        let mut out = String::new();
        let mut parts = [0usize; 5];
        let mut eau = [0usize; 4];
        for t in &m.tuiles {
            parts[t.terrain as usize] += 1;
            eau[t.eau as usize] += 1;
        }
        let n = m.tuiles.len() as f64;
        out += &format!(
            "plaine {:.1} % · riche {:.1} % · rocailleuse {:.1} % · marécageuse {:.1} % · forêt {:.1} % · eau {:.1} % (ruisseau {}, rivière {}, fleuve {})\n",
            parts[0] as f64 * 100.0 / n, parts[1] as f64 * 100.0 / n, parts[2] as f64 * 100.0 / n,
            parts[3] as f64 * 100.0 / n, parts[4] as f64 * 100.0 / n,
            (eau[1] + eau[2] + eau[3]) as f64 * 100.0 / n, eau[1], eau[2], eau[3]
        );
        for y in (0..m.hauteur).step_by(4) {
            for x in (0..m.largeur).step_by(2) {
                let mut c = ' ';
                let mut rang = -1i32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (xx, yy) = (x + dx, y + dy);
                    if xx >= m.largeur || yy >= m.hauteur { continue; }
                    let t = &m.tuiles[yy * m.largeur + xx];
                    let (r, ch) = if matches!(t.contenu, Contenu::Quartier(_)) { (9, 'V') }
                        else if t.eau == Eau::Fleuve { (8, '#') }
                        else if t.eau == Eau::Riviere { (7, '=') }
                        else if t.eau == Eau::Ruisseau { (6, '~') }
                        else { match t.terrain {
                            Terrain::Foret => (4, 'T'),
                            Terrain::Marecageuse => (3, '%'),
                            Terrain::Rocailleuse => (2, '^'),
                            Terrain::PlaineRiche => (1, '"'),
                            Terrain::Plaine => (0, '.'),
                        } };
                    if r > rang { rang = r; c = ch; }
                }
                out.push(c);
            }
            out.push('\n');
        }
        out
    }

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
                Contenu::Vide if m.tuiles[i].aretes != 0 => '+',
                Contenu::Vide => ' ',
                Contenu::Quartier(q) => {
                    if m.quartiers[q].capacite_pops == 0 {
                        '#'
                    } else {
                        'H'
                    }
                }
                Contenu::Batiment(b) => match &m.batiments[b].chantier {
                    Some(Chantier {
                        cible: Cible::Terre(t),
                        ..
                    }) => lettre(*t).to_ascii_lowercase(),
                    Some(Chantier {
                        cible: Cible::Extension(terre),
                        ..
                    }) => lettre(m.batiments[*terre].type_id).to_ascii_lowercase(),
                    Some(_) => '#',
                    None => lettre(m.batiments[b].type_id),
                },
            }
        };
        let occupees: Vec<(usize, usize)> = (0..m.tuiles.len())
            .filter(|&i| m.tuiles[i].contenu != Contenu::Vide || m.tuiles[i].aretes != 0)
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
        for p in &mut self.pops {
            for r in 0..p.stock.len() {
                perdre(&mut p.stock[r], r);
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
