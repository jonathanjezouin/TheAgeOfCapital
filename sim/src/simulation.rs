//! Boucle quotidienne (voir docs/02 §5), démographie par jauge de pop (docs/02 §6)
//! et réorientation de la main-d'œuvre selon le besoin (docs/02 §3).

use crate::donnees::{mul_div, Catalogue, ParamsDemo, Qte, Traversabilite, PPM};
use crate::terrain::Eau;
use crate::monde::{Tuile, cout_dir, Batiment, Chantier, Cible, Contenu, Limitant, Monde, Quartier};
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
    /// Statut : serf (docs/07 §15) ; `None` = libre.
    pub statut: Option<crate::monde::Statut>,
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
            statut: Some(crate::monde::Statut::Serf),
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

/// Longueur maximale (coût) de la route neuve qui relie un hameau fondé au réseau existant.
/// Coût d'un côté de tile sur route (unités d'arête).
const COUT_COTE: i64 = 10;
/// Part (en %) du besoin nominal que le générateur d'état initial couvre, en tiles *équivalentes* (tiles pondérées
/// par leur rendement réel : qualité du sol × aptitude du terrain).
const MARGE_TERRES_PCT: usize = 150;

/// Emplacement choisi pour un nouveau lieu : tile libre, tile du réseau où l'on s'accroche,
/// et les tiles intermédiaires d'une route neuve entre les deux (dans l'ordre, éventuellement aucune).
#[derive(Clone, Debug)]
struct Site {
    tuile: usize,
    /// Tile du réseau où s'accroche la route neuve ; `usize::MAX` : pas de route (le lieu s'atteint à pied).
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
    /// Trajets de travail récents (hameau de résidence, terre) -> pop-jours, depuis la dernière décision de routes.
    pub trajets: BTreeMap<(usize, usize), u32>,
    /// Distance (en côtés de tile) de chaque tile au cours d'eau le plus proche : fixe, elle sert à valoriser les sites.
    pub dist_eau: Vec<u16>,
}

/// Mesures du peuplement : voir `Simulation::stats_peuplement`.
#[derive(Clone, Debug, Default)]
pub struct StatsPeuplement {
    /// Effectif de chaque hameau vivant (capacité > 0, ni abandonné ni détruit), trié.
    pub tailles: Vec<u32>,
    pub vides: u32,
    pub abandonnes: u32,
    pub detruits: u32,
    pub pops: u32,
    pub terres_abandonnees: u32,
    pub tiles_en_friche: u32,
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
        let eaux: Vec<crate::terrain::Eau> = monde.tuiles.iter().map(|t| t.eau).collect();
        let dist_eau = crate::terrain::distance_eau(monde.largeur, monde.hauteur, &eaux);
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
            trajets: BTreeMap::new(),
            dist_eau,
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

    pub fn reconstruire_reseau(&mut self) {
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
                    .filter(|&b| self.livre(b, r))
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
        for p in &self.monde.personnages {
            for (r, v) in p.grenier.iter().enumerate() {
                t[r] += v;
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
        self.train_de_vie();
        // 6. Jauges de croissance : naissances et disparitions de pops.
        self.croissance_quotidienne();
        // 7. Péremption, statistiques.
        self.perimer();
        self.finaliser_jour();

        self.jour += 1;
        if self.jour % 30 == 0 {
            self.entretien_mensuel();
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
                    // Un chantier commande le matériau dont son équipe a besoin chaque jour, sauf s'il a un maître
                    // d'ouvrage : c'est alors son grenier qui le fournit (voir `produire`).
                    let par_jour = if ch.maitre.is_some() { 0 } else { mul_div(
                        ch.materiau_total,
                        bat.equipe as i64 * 1000,
                        ch.travail_total.max(1),
                    ) };
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
                if !t.par_tile || bat.chantier.is_some() || bat.abandon_mois.is_some() {
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
        // Mémoire des trajets : de quoi décider d'une route là où l'on marche beaucoup (docs/07 §18).
        for i in 0..self.pops.len() {
            if let Some(e) = self.pops[i].emploi {
                if !self.est_chantier_type(e) {
                    *self.trajets.entry((self.pops[i].residence, e)).or_default() += 1;
                }
            }
        }
    }

    fn produire(&mut self) {
        let libre = self.libre_milli.clone();
        let cat = &self.cat;
        let tuiles = &self.monde.tuiles;
        // Le maître d'ouvrage paie : son grenier fournit le matériau dont l'équipe présente a besoin aujourd'hui.
        let mat = cat.extension.materiau;
        for b in self.monde.batiments.iter_mut() {
            let Some(ch) = b.chantier.as_ref() else { continue };
            let Some(m) = ch.maitre else { continue };
            let besoin = mul_div(ch.materiau_total, b.presents as i64 * 1000, ch.travail_total.max(1));
            let manque = (besoin - b.stock_entree[mat]).max(0);
            if let Some(p) = self.monde.personnages.get_mut(m) {
                let don = manque.min(p.grenier[mat]);
                p.grenier[mat] -= don;
                b.stock_entree[mat] += don;
            }
        }
        let champart_ppm = cat.seigneurie.champart_ppm;
        for (idx_b, b) in self.monde.batiments.iter_mut().enumerate() {
            let t = &cat.batiments[b.type_id];
            if t.chantier {
                Self::avancer_chantier(b, cat, &mut self.bilan);
                continue;
            }
            // Une terre abandonnée ne produit plus.
            if b.abandon_mois.is_some() {
                b.prod_ppm = 0;
                b.limitant = Limitant::Aucun;
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
                        // Champart : le seigneur prélève sa part en nature sur ce que produit une terre.
                        let mut part = 0;
                        if t.par_tile && cat.seigneurie.champart_ressources.contains(&r) {
                            if let Some(pn) = self.monde.personnages.get_mut(0) {
                                part = mul_div(p, champart_ppm, PPM);
                                pn.grenier[r] += part;
                            }
                        }
                        b.stock_sortie[r] += p - part;
                        self.stats.produit[r] += p - part;
                        if p > 0 {
                            self.stats.produits_par_lieu.push((idx_b, r, p - part));
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

    /// Convertit le terrain des tiles `tuiles` à la fin d'un chantier (défrichement d'une forêt, drainage d'un
    /// marais, docs/07 §12) : le terrain devient celui de la conversion, et le bois récupéré va au hameau le plus
    /// proche. `conserve` : la terre garde son terrain (forêt exploitée) ; rien à faire.
    fn convertir(&mut self, tuiles: &[usize], conserve: bool) {
        if conserve {
            return;
        }
        let (mut bois, mut forets, mut marais) = (0, 0, 0);
        for &t in tuiles {
            let Some(c) = self.cat.conversion[self.monde.tuiles[t].terrain as usize] else {
                continue;
            };
            match self.monde.tuiles[t].terrain {
                crate::terrain::Terrain::Foret => forets += 1,
                _ => marais += 1,
            }
            self.monde.tuiles[t].terrain = crate::terrain::Terrain::TOUS[c.vers];
            bois += c.bois;
        }
        if forets + marais == 0 {
            return;
        }
        let Some(&t0) = tuiles.first() else { return };
        let (x, y) = self.monde.xy(t0);
        if let Some(q) = self.terroir(t0) {
            let mat = self.cat.extension.materiau;
            self.distribuer_hameau(q, mat, bois);
            // Le bois du défrichement est une production : le bilan de matière reste juste.
            self.bilan.produit[mat] += bois;
        }
        let mut quoi = Vec::new();
        if forets > 0 {
            quoi.push(format!("{forets} tiles de forêt défrichées"));
        }
        if marais > 0 {
            quoi.push(format!("{marais} tiles de marais drainées"));
        }
        self.journal(format!("Terrain converti près de ({x}, {y}) : {} ({} bois récupérés)", quoi.join(", "), bois / 1000));
    }

    /// Le matériau resté sur le chantier `b` revient à son maître d'ouvrage (grenier) ; sans maître, au hameau `q`.
    fn rendre_au_maitre_ou_hameau(&mut self, b: usize, q: usize) {
        let mat = self.cat.extension.materiau;
        let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
        self.rendre_matiere(b, mat, reste, Some(q));
    }

    fn rendre_au_maitre(&mut self, b: usize) {
        let mat = self.cat.extension.materiau;
        let reste = std::mem::take(&mut self.monde.batiments[b].stock_entree[mat]);
        self.rendre_matiere(b, mat, reste, None);
    }

    fn rendre_matiere(&mut self, b: usize, mat: usize, reste: Qte, hameau: Option<usize>) {
        if reste <= 0 {
            return;
        }
        let maitre = self.monde.batiments[b].chantier.as_ref().and_then(|c| c.maitre);
        // Le chantier est déjà terminé (`chantier` retiré) : on retrouve le maître par défaut, le seigneur.
        let maitre = maitre.or_else(|| self.monde.personnages.first().map(|p| p.id));
        if let Some(p) = maitre.and_then(|m| self.monde.personnages.get_mut(m)) {
            p.grenier[mat] += reste;
        } else if let Some(q) = hameau {
            self.distribuer_hameau(q, mat, reste);
        } else {
            self.monde.batiments[b].stock_entree[mat] += reste;
        }
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
                    let emprise = self.monde.batiments[b].emprise.clone();
                    self.convertir(&emprise, self.cat.batiments[t].conserve_terrain);
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
                    let conserve = self.cat.batiments[self.monde.batiments[terre].type_id].conserve_terrain;
                    self.convertir(&nouvelles, conserve);
                    self.rendre_au_maitre(b);
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
                    self.monde.quartiers[q].capacite_memoire = cap;
                    self.monde.batiments[b].equipe = 0;
                    let site = self.monde.batiments[b].tuile;
                    self.convertir(&[site], false);
                    // Le matériau resté sur le chantier revient au maître d'ouvrage (ou au hameau, sans maître).
                    self.rendre_au_maitre_ou_hameau(b, q);
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
                    self.monde.quartiers[q].capacite_memoire = self.monde.quartiers[q].capacite_pops;
                    self.monde.batiments[b].equipe = 0;
                    self.rendre_au_maitre_ou_hameau(b, q);
                    let cap = self.monde.quartiers[q].capacite_pops;
                    self.journal(format!(
                        "Nouvelle maison dans le hameau en ({x}, {y}) : {cap} maisons"
                    ));
                }
                Cible::RemiseHameau(q) => {
                    let h = &mut self.monde.quartiers[q];
                    h.abandon_mois = None;
                    h.vide_mois = 0;
                    h.capacite_pops = h.capacite_memoire.max(1);
                    self.monde.batiments[b].equipe = 0;
                    self.rendre_au_maitre(b);
                    self.journal(format!("Hameau remis en état en ({x}, {y})"));
                }
                Cible::Route(q, t) => {
                    self.monde.batiments[b].equipe = 0;
                    self.poser_route_entre(q, t);
                }
                Cible::RemiseTerre(t) => {
                    let bat = &mut self.monde.batiments[t];
                    bat.abandon_mois = None;
                    bat.inactif_mois = 0;
                    self.monde.batiments[b].equipe = 0;
                    self.rendre_au_maitre(b);
                    let nom = self.cat.batiments[self.monde.batiments[t].type_id].nom.clone();
                    self.journal(format!("Terre remise en service : {nom} en ({x}, {y})"));
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
        // Les pops déjà établies sur le site (l'équipe de fondation) comptent dans la capacité du hameau neuf.
        let deja = self.pops.iter().filter(|p| p.residence == q).count() as u32;
        let mut restant: Vec<u32> = self.effectifs();
        let mut pris = 0;
        for &(_, _, i) in &candidats {
            if pris >= n.saturating_sub(deja) {
                break;
            }
            let o = self.pops[i].residence;
            if restant[o] <= 1 {
                continue; // jamais la dernière pop d'un hameau
            }
            restant[o] -= 1;
            self.pops[i].residence = q;
            pris += 1;
        }
    }

    /// Chaque pop consomme sur son propre stock ; le hameau agrège la couverture de ses habitants.
    /// Le seigneur vit sur son grenier : chaque jour, la ration de `train_de_vie_pops` pops (grain, bois…) y est prélevée.
    fn train_de_vie(&mut self) {
        let milli = self.cat.seigneurie.train_de_vie_milli;
        let Some(p) = self.monde.personnages.get_mut(0) else { return };
        for b in &self.cat.besoins {
            let c = mul_div(b.par_pop, milli, 1000).min(p.grenier[b.ressource]);
            p.grenier[b.ressource] -= c;
            self.bilan.consomme_pops[b.ressource] += c;
        }
    }

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
                // Un chantier privé de matériau pèse sur la ressource qui lui manque : le village doit en produire plus.
                if t.chantier {
                    poids[r] += PPM;
                }
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
        self.monde.batiments[b].abandon_mois.is_none() && self.cat.batiments[self.monde.batiments[b].type_id].livre(r)
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
                cible: Cible::Maison(_) | Cible::RemiseHameau(_) | Cible::RemiseTerre(_) | Cible::Route(..),
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
        // Un chantier qui attend son matériau ne bloque pas la place d'un autre : c'est justement l'autre qui doit
        // produire ce matériau (la forêt qui manque), sinon le village reste bloqué.
        let actifs = self
            .monde
            .batiments
            .iter()
            .filter(|b| b.chantier.is_some() && !matches!(b.limitant, Limitant::Intrant(_)))
            .count();
        if actifs >= self.cat.extension.chantiers_max {
            return;
        }
        // Routes : là où l'on marche beaucoup hors route (60 pop-jours de marche en une période), le seigneur ordonne une route.
        if self.decider_routes(60) {
            return;
        }
        // Pas de chantier sans bras pour le mener : le seigneur n'ordonne que ce que les pops (corvée) peuvent faire.
        if self.bras_recrutables() == 0 {
            return;
        }
        let nq = self.nq();
        let seuil = self.seuil_croissance();
        let tensions = self.tensions();
        let hameau_en_cours = (0..nq).any(|q| {
            let h = &self.monde.quartiers[q];
            h.capacite_pops == 0 && h.abandon_mois.is_none() && !h.detruit
        });

        // 1. Logement : des pops à jauge pleine attendent une place.
        let attente: Vec<usize> = (0..self.pops.len())
            .filter(|&i| self.pops[i].croissance >= seuil)
            .collect();
        if !attente.is_empty() {
            let motif = format!("{} pops attendent un logement", attente.len());
            let centres = self.centres(attente.iter().map(|&i| self.pops[i].residence));
            // Pour chaque hameau d'une pop qui attend : une maison de plus là où l'on est, ou un hameau neuf
            // ailleurs s'il vaut `seuil_fondation` fois mieux (la famille reste, sauf gain net, docs/07 §4.3-4.4).
            let maison_en_cours = centres.iter().any(|&c| self.maison_en_cours(c));
            let habitat = tensions
                .iter()
                .filter(|&&r| !self.producteurs_bloques_par_intrant(r))
                .find_map(|&r| self.habitat_producteur(r))
                .unwrap_or(0);
            for &centre in &centres {
                // Remettre en état un hameau abandonné à portée coûte moins qu'en fonder un : le seigneur le préfère.
                if !hameau_en_cours {
                    if let Some(h) = self.hameau_a_remettre(centre) {
                        if self.lancer_remise_hameau(h, &motif) {
                            return;
                        }
                    }
                }
                let reste = self.valeur_habitat(self.monde.quartiers[centre].tuile, 1);
                let fonder = !hameau_en_cours && !maison_en_cours && {
                    self.site_hameau(centre, habitat).map_or(false, |site| {
                        self.valeur_habitat(site.tuile, 1) * PPM >= reste * self.cat.peuplement.seuil_fondation_ppm
                    })
                };
                if fonder && self.lancer_hameau(centre, habitat, &motif) {
                    return;
                }
                if self.lancer_maison(centre, &motif) {
                    return;
                }
            }
            // Plus de place où l'on est (densité maximale) : fonder ailleurs, quelle qu'en soit la valeur relative.
            if !hameau_en_cours && !maison_en_cours {
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

    /// Meilleur emplacement pour un nouveau lieu, à portée de **marche** du hameau `centre` (toute tile praticable,
    /// la route réduisant le coût, docs/07 §18). Terre fertile : rendement / distance ; hameau : valeur du site.
    /// `requis` : groupes de ressources dont l'une au moins doit avoir un producteur à portée de marche du site.
    /// Si l'accès dépasse `route_seuil_acces` de `distance_max`, le seigneur ordonne aussi une route (`planifier_route`).
    fn meilleur_site(&self, centre: usize, fertile: bool, type_id: Option<usize>, requis: &[Vec<usize>]) -> Option<Site> {
        let nq = self.nq();
        let d_marche = self.reseau.distances_depuis(self.monde.quartiers[centre].tuile);
        // Le seigneur choisit un site qu'il compte relier par une route : on admet donc les sites à portée *si la route
        // existait* ; une route n'est ordonnée que si la marche seule dépasse le seuil (voir plus bas).
        let d = self.reseau.distances_si_route(&[self.monde.quartiers[centre].tuile]);
        // Coût (route supposée construite) de chaque tile aux producteurs de chaque groupe requis.
        let portees: Vec<Vec<i64>> = requis
            .iter()
            .map(|groupe| {
                let mut sources: Vec<usize> = Vec::new();
                for b in 0..self.monde.batiments.len() {
                    if groupe.iter().any(|&r| self.livre(b, r)) {
                        sources.extend_from_slice(self.reseau.acces(nq + b));
                    }
                }
                self.reseau.distances_si_route(&sources)
            })
            .collect();
        let dmax = self.cat.infra.distance_max;
        let seuil = dmax * self.cat.infra.route_seuil_ppm / PPM;
        let memoire = self.cat.abandon.memoire_site_mois;
        // Une tile traversée par une route n'est pas encore constructible (docs/02 §9, à lever plus tard).
        let vide = |t: usize| {
            self.monde.tuiles[t].contenu == Contenu::Vide
                && self.monde.tuiles[t].aretes == 0
                && self.monde.tuiles[t].constructible()
                // Mémoire du lieu : on ne refonde pas un hameau là où un projet vient d'échouer (docs/07 §14).
                && (fertile || self.monde.tuiles[t].vacant_mois == 0 || self.monde.tuiles[t].vacant_mois as u32 >= memoire)
        };
        let mut best: Option<((i64, i64, usize), usize)> = None;
        for v in 0..self.monde.tuiles.len() {
            if d[v] == i64::MAX || !vide(v) || !portees.iter().all(|dp| dp[v] != i64::MAX) {
                continue;
            }
            let dist = d[v];
            let fert = self.fert_pour(type_id, v);
            // Travaux propres au site : conversion du terrain (défrichement, drainage) et route estimée si l'accès est long.
            let route_est = if d_marche[v] > seuil { (dist.min(d_marche[v]) / COUT_COTE).max(1) * self.cat.infra.route_travail_milli / 2 } else { 0 };
            let travaux = route_est + self.cat.travail_conversion(type_id, &[v], &self.monde.tuiles);
            let cle = if fertile {
                (-(fert * 10_000 / (dist + 10) * 100_000 / (100_000 + travaux)), dist, v)
            } else {
                // Hameau : le site vaut par la terre libre qu'il partage, l'eau et les voisins (docs/07 §4.1) ;
                // les travaux en rabattent la valeur.
                let valeur = self.valeur_habitat(v, 1);
                (-(valeur * 100_000 / (100_000 + travaux)), dist + travaux / self.cat.extension.travail_par_unite_route_milli, v)
            };
            if best.map_or(true, |(k, _)| cle < k) {
                best = Some((cle, v));
            }
        }
        let (_, tuile) = best?;
        if d_marche[tuile] > seuil {
            if let Some((attache, route)) = self.planifier_route(tuile) {
                return Some(Site { tuile, attache, route });
            }
        }
        Some(Site { tuile, attache: usize::MAX, route: Vec::new() })
    }

    /// Plan d'une route neuve de la tile `v` au réseau le plus proche (route existante ou hameau) : renvoie la tile du
    /// réseau où elle s'accroche et les tiles intermédiaires, de l'accroche vers `v`. Elle peut franchir l'eau par un
    /// pont ou traverser une terre en détruisant les tiles (travail et pénalité d'ordre), jamais un chantier. Elle
    /// rejoint une route existante au lieu de la longer.
    fn planifier_route(&self, v: usize) -> Option<(usize, Vec<usize>)> {
        let n = self.monde.tuiles.len();
        let limite = 3 * self.cat.infra.distance_max;
        let unite = self.cat.extension.travail_par_unite_route_milli;
        let reseau_t = |t: usize| {
            self.monde.tuiles[t].aretes != 0 || matches!(self.monde.tuiles[t].contenu, Contenu::Quartier(_))
        };
        let mut cout = vec![i64::MAX; n];
        let mut prec = vec![usize::MAX; n];
        let mut tas: std::collections::BinaryHeap<std::cmp::Reverse<(i64, usize)>> = std::collections::BinaryHeap::new();
        cout[v] = 0;
        tas.push(std::cmp::Reverse((0, v)));
        while let Some(std::cmp::Reverse((c, t))) = tas.pop() {
            if c > cout[t] {
                continue;
            }
            if t != v && reseau_t(t) {
                // Arrivée : de l'accroche `t` on suit les précédents jusqu'à `v` ; ce sont les tiles intermédiaires.
                let mut route = Vec::new();
                let mut x = prec[t];
                while x != v && x != usize::MAX {
                    route.push(x);
                    x = prec[x];
                }
                return Some((t, route));
            }
            for dir in 0..8 {
                let Some(w) = self.monde.voisin(t, dir) else { continue };
                if !self.monde.peut_relier(t, w) {
                    continue;
                }
                let tw = &self.monde.tuiles[w];
                let penalite = if tw.eau != Eau::Aucune {
                    self.cat.pont_milli[tw.eau as usize] / unite
                } else if reseau_t(w) {
                    0
                } else if let Some(wd) = self.destruction_de(w) {
                    wd / unite
                } else {
                    match tw.contenu {
                        Contenu::Vide => 0,
                        Contenu::Quartier(_) => 0,
                        Contenu::Batiment(_) => continue, // chantier, ou terre indestructible : on contourne
                    }
                };
                let nc = c + cout_dir(dir) + penalite;
                if nc <= limite && nc < cout[w] {
                    cout[w] = nc;
                    prec[w] = t;
                    tas.push(std::cmp::Reverse((nc, w)));
                }
            }
        }
        None
    }

    /// Traversabilité d'une tile par une route neuve, selon son contenu (docs/02 §10.1).
    pub fn traversabilite(&self, t: usize) -> Traversabilite {
        // L'eau se franchit par un pont (voir `meilleur_site`) ; forêt et marais, par un chemin comme le reste.
        match self.monde.tuiles[t].contenu {
            Contenu::Vide | Contenu::Quartier(_) => Traversabilite::Libre,
            Contenu::Batiment(b) => self.cat.batiments[self.monde.batiments[b].type_id].traversable,
        }
    }

    /// Travail (milli pop-jours) pour détruire la tile `t` si une route doit la traverser : une tile de terre qui n'est ni
    /// le nœud de la terre, ni en chantier. `None` : intraversable (hameau, chantier, nœud, terre indestructible).
    fn destruction_de(&self, t: usize) -> Option<Qte> {
        let Contenu::Batiment(b) = self.monde.tuiles[t].contenu else {
            return None;
        };
        let bat = &self.monde.batiments[b];
        let ty = &self.cat.batiments[bat.type_id];
        if !ty.par_tile || ty.chantier || bat.chantier.is_some() || bat.tuile == t || ty.destruction_milli <= 0 {
            return None;
        }
        Some(ty.destruction_milli)
    }

    /// Travail des destructions d'une route neuve (tiles de terre traversées).
    fn travail_destructions(&self, route: &[usize]) -> Qte {
        route.iter().filter_map(|&t| self.destruction_de(t)).sum()
    }

    /// Travail de la route elle-même : ponts et destructions.
    fn travail_route(&self, route: &[usize]) -> Qte {
        self.travail_ponts(route) + self.travail_destructions(route)
    }

    /// Pose la route d'un site. Les tiles de terre traversées sont détruites (la terre peut se scinder : les tiles
    /// qui ne se touchent plus forment une nouvelle terre), puis la route est tracée.
    fn poser_site(&mut self, site: &Site, relier_fin: bool) {
        let a_detruire: Vec<usize> = site.route.iter().copied().filter(|&t| self.destruction_de(t).is_some()).collect();
        let touchees = self.detruire_tuiles(&a_detruire);
        if site.attache != usize::MAX {
            let mut prec = site.attache;
            for &r in &site.route {
                self.monde.relier(prec, r);
                prec = r;
            }
            // Une terre n'a pas d'entrée : la route s'arrête à sa lisière. Seul un hameau est relié.
            if relier_fin {
                self.monde.relier(prec, site.tuile);
            }
        }
        for b in touchees {
            self.scinder(b, &site.route);
        }
    }

    /// Trace une route de `attache` à `tuile` par les tiles `route` (intermédiaires), en détruisant les tiles de terre
    /// traversées ; reconstruit le réseau. Primitive des chantiers de route, et de tout décideur qui en ordonne une.
    pub fn tracer_route(&mut self, attache: usize, route: &[usize], tuile: usize) {
        self.poser_site(&Site { tuile, attache, route: route.to_vec() }, true);
        self.reconstruire_reseau();
    }

    /// Travail (milli pop-jours) des destructions et ponts d'une route donnée (pour les décideurs et les tests).
    pub fn travail_route_pour_test(&self, route: &[usize]) -> Qte {
        self.travail_route(route)
    }

    /// Détruit des tiles de terre : elles sortent de leur terre et deviennent libres. Renvoie les terres touchées.
    /// Ne reconnecte rien : `scinder` sépare ensuite les morceaux devenus disjoints.
    pub fn detruire_tuiles(&mut self, tuiles: &[usize]) -> Vec<usize> {
        let mut touchees: Vec<usize> = Vec::new();
        for &t in tuiles {
            let Contenu::Batiment(b) = self.monde.tuiles[t].contenu else {
                continue;
            };
            self.monde.batiments[b].emprise.retain(|&x| x != t);
            self.monde.tuiles[t].contenu = Contenu::Vide;
            self.monde.tuiles[t].vacant_mois = 0;
            if !touchees.contains(&b) {
                touchees.push(b);
            }
        }
        touchees
    }

    /// Sépare la terre `b` en terres distinctes si sa destruction partielle a coupé son emprise : chaque morceau de
    /// tiles contiguës devient une terre, qui garde sa part du stock. `acces` : tiles de route tout juste posées, par
    /// lesquelles un morceau est relié au réseau.
    pub fn scinder(&mut self, b: usize, acces: &[usize]) {
        let emprise = self.monde.batiments[b].emprise.clone();
        let noeud = self.monde.batiments[b].tuile;
        let ens: std::collections::BTreeSet<usize> = emprise.iter().copied().collect();
        let mut vu: std::collections::BTreeSet<usize> = Default::default();
        let mut morceaux: Vec<Vec<usize>> = Vec::new();
        for &t in &emprise {
            if vu.contains(&t) {
                continue;
            }
            let mut comp = vec![t];
            vu.insert(t);
            let mut k = 0;
            while k < comp.len() {
                for d in [0usize, 2, 4, 6] {
                    if let Some(v) = self.monde.voisin(comp[k], d) {
                        if ens.contains(&v) && vu.insert(v) {
                            comp.push(v);
                        }
                    }
                }
                k += 1;
            }
            morceaux.push(comp);
        }
        if morceaux.len() <= 1 {
            return;
        }
        let total: i64 = emprise.len() as i64;
        let nr = self.cat.ressources.len();
        let (principal, autres): (Vec<_>, Vec<_>) = morceaux.into_iter().partition(|c| c.contains(&noeud));
        let mut reste = principal.into_iter().next().unwrap_or_default();
        for comp in autres {
            let part = comp.len() as i64;
            let nouveau = self.monde.batiments.len();
            let mut bat = self.monde.batiments[b].clone();
            for r in 0..nr {
                let (e, so) = (self.monde.batiments[b].stock_entree[r], self.monde.batiments[b].stock_sortie[r]);
                let (pe, ps) = (e * part / total, so * part / total);
                bat.stock_entree[r] = pe;
                bat.stock_sortie[r] = ps;
                self.monde.batiments[b].stock_entree[r] -= pe;
                self.monde.batiments[b].stock_sortie[r] -= ps;
            }
            // Plus de nœud : le nouveau morceau est accessible par toutes ses tiles ; la première sert de repère.
            let choix = (comp[0], None::<usize>);
            let _ = acces;
            bat.tuile = choix.0;
            // Le nouveau morceau repart à zéro : il n'hérite ni du travail reçu aujourd'hui ni des présents du jour.
            bat.mo_milli = 0;
            bat.presents = 0;
            bat.emprise = comp.clone();
            bat.equipe = 0;
            bat.chantier = None;
            self.monde.batiments.push(bat);
            for &t in &comp {
                self.monde.tuiles[t].contenu = Contenu::Batiment(nouveau);
            }
            let nom = self.cat.batiments[self.monde.batiments[b].type_id].nom.clone();
            let (x, y) = self.monde.xy(choix.0);
            self.journal(format!("Une terre coupée par un chemin : {nom} de {part} tiles devient une terre à part en ({x}, {y})"));
        }
        reste.sort_unstable();
        self.monde.batiments[b].emprise = reste;
    }

    /// Mesures du peuplement (docs/07 §6, §14) : tailles des hameaux vivants, hameaux vides, abandonnés, détruits, terres
    /// abandonnées, tiles en friche et forêts reconquises.
    pub fn stats_peuplement(&self) -> StatsPeuplement {
        let eff = self.effectifs();
        let mut st = StatsPeuplement::default();
        for (q, h) in self.monde.quartiers.iter().enumerate() {
            if h.detruit {
                st.detruits += 1;
            } else if h.abandon_mois.is_some() {
                st.abandonnes += 1;
            } else if h.capacite_pops > 0 {
                st.tailles.push(eff[q]);
                if eff[q] == 0 {
                    st.vides += 1;
                }
            }
        }
        st.pops = self.pops.len() as u32;
        st.terres_abandonnees = self.monde.batiments.iter().filter(|b| b.abandon_mois.is_some()).count() as u32;
        st.tiles_en_friche = self.monde.tuiles.iter().filter(|t| t.vacant_mois > 0).count() as u32;
        st.tailles.sort_unstable();
        st
    }

    // ---------- Usure, abandon et repousse (docs/07 §14) ----------

    /// Une fois par mois : détecte les terres sans travail et les hameaux sans pop, fait vieillir ce qui est abandonné,
    /// détruit ce qui est en ruine, et laisse la forêt reprendre les tiles libérées.
    fn entretien_mensuel(&mut self) {
        let ab = self.cat.abandon.clone();
        let eff = self.effectifs();
        let tensions = self.poids_tension();
        for b in 0..self.monde.batiments.len() {
            let (type_id, chantier, abandon) = {
                let bat = &self.monde.batiments[b];
                (bat.type_id, bat.chantier.is_some(), bat.abandon_mois)
            };
            if chantier {
                // Un chantier sans progrès depuis trop longtemps est abandonné : l'équipe est libérée.
                let bat = &mut self.monde.batiments[b];
                let restant = bat.chantier.as_ref().map_or(0, |c| c.travail_restant);
                if restant < bat.progres_ref {
                    bat.progres_ref = restant;
                    bat.inactif_mois = 0;
                } else {
                    bat.inactif_mois += 1;
                    if bat.inactif_mois >= ab.chantier_bloque_mois {
                        self.abandonner_chantier(b);
                    }
                }
                continue;
            }
            if !self.cat.batiments[type_id].par_tile {
                continue;
            }
            if let Some(m) = abandon {
                self.monde.batiments[b].abandon_mois = Some(m + 1);
                if m + 1 >= ab.terre_friche_mois {
                    self.liberer_terre(b);
                }
                continue;
            }
            let besoin = self.cat.besoin_mo_terre_milli(type_id, &self.monde.batiments[b].emprise, &self.monde.tuiles);
            // Une terre dont la ressource manque au village n'est jamais « inutile », même si personne n'y travaille.
            let utile = self.cat.batiments[type_id].extrants_finaux().iter().any(|&(r, _)| tensions[r] > 0);
            let bat = &mut self.monde.batiments[b];
            // Moins de 2 % du travail dont la terre aurait besoin, et rien qui en manque : on n'en a pas l'usage.
            if bat.mo_lisse_milli * 50 < besoin && !utile {
                bat.inactif_mois += 1;
            } else {
                bat.inactif_mois = 0;
            }
            if bat.inactif_mois >= ab.terre_inactive_mois {
                self.abandonner_terre(b);
            }
        }
        for q in 0..self.monde.quartiers.len() {
            let h = &mut self.monde.quartiers[q];
            if h.detruit {
                continue;
            }
            if let Some(m) = h.abandon_mois {
                h.abandon_mois = Some(m + 1);
                if m + 1 >= ab.hameau_ruine_mois {
                    self.detruire_hameau(q);
                }
            } else if eff[q] == 0 && h.capacite_pops > 0 {
                h.vide_mois += 1;
                if h.vide_mois >= ab.hameau_vide_mois {
                    self.abandonner_hameau(q);
                }
            } else {
                h.vide_mois = 0;
            }
        }
        self.repousse_mensuelle();
    }

    /// Nombre de pops qu'un nouveau chantier pourrait recruter : celles qui ne sont pas déjà en chantier, sans prendre plus
    /// de la moitié (au moins une) des pops d'un hameau (les terres ont besoin de bras, docs/02 §8.3).
    fn bras_recrutables(&self) -> u32 {
        self.bras_recrutables_depuis(None, false)
    }

    /// Comme `bras_recrutables`, mais seulement les pops des hameaux à portée de `centre` : celles qu'un chantier proche
    /// de ce hameau peut effectivement recruter.
    /// `fondation` : une fondation de hameau ne prend jamais la dernière pop d'un hameau (voir `recruter`).
    fn bras_recrutables_depuis(&self, centre: Option<usize>, fondation: bool) -> u32 {
        let eff = self.effectifs();
        let mut equipe = vec![0u32; self.nq()];
        for p in &self.pops {
            if p.emploi.map_or(false, |e| self.est_chantier_type(e)) {
                equipe[p.residence] += 1;
            }
        }
        (0..self.nq())
            .filter(|&q| centre.map_or(true, |c| c == q || self.reseau.chemin(c, q).is_some()))
            .map(|q| {
                let libres = eff[q].saturating_sub(equipe[q]).saturating_sub(u32::from(fondation));
                ((eff[q] / 2).max(1)).saturating_sub(equipe[q]).min(libres)
            })
            .sum()
    }

    /// Chemin à pied du hameau `q` à la terre `t` : tiles de route à poser (celles du trajet qui n'en ont pas encore), coût en
    /// travail (route + destruction des tiles de terre traversées). Le dernier pas (dans la terre) n'est pas goudronné.
    fn plan_route_entre(&self, q: usize, t: usize) -> Option<(Vec<usize>, Qte)> {
        let nq = self.nq();
        let ch = self.reseau.chemin(q, nq + t)?;
        let n = ch.tuiles.len();
        if n < 3 {
            return None;
        }
        // On pose la route sur tout le trajet, sauf la dernière tile (celle de la terre atteinte).
        let mut tuiles: Vec<usize> = ch.tuiles[..n - 1].to_vec();
        // La route s'arrête devant une terre qu'on ne peut pas couper (tile de référence, chantier) : jamais dedans.
        if let Some(k) = tuiles.iter().position(|&x| matches!(self.monde.tuiles[x].contenu, Contenu::Batiment(_)) && self.destruction_de(x).is_none()) {
            tuiles.truncate(k);
        }
        if tuiles.len() < 3 {
            return None;
        }
        let neuves: Vec<usize> = tuiles
            .windows(2)
            .filter(|w| {
                let d = self.monde.direction(w[0], w[1]).unwrap_or(0);
                !(self.monde.masque(w[0]) & crate::monde::DIRS[d].0 != 0 && self.monde.masque(w[1]) & crate::monde::DIRS[d].3 != 0)
            })
            .map(|w| w[1])
            .collect();
        if neuves.is_empty() {
            return None;
        }
        let travail = neuves.len() as i64 * self.cat.infra.route_travail_milli + self.travail_destructions(&neuves);
        Some((tuiles, travail))
    }

    /// Pose la route décidée entre le hameau `q` et la terre `t` (fin d'un chantier de route).
    fn poser_route_entre(&mut self, q: usize, t: usize) {
        let Some((tuiles, _)) = self.plan_route_entre(q, t) else { return };
        let n = tuiles.len();
        let (attache, route, bout) = (tuiles[0], tuiles[1..n].to_vec(), tuiles[n - 1]);
        let a_detruire: Vec<usize> = route.iter().copied().filter(|&x| self.destruction_de(x).is_some()).collect();
        let touchees = self.detruire_tuiles(&a_detruire);
        let mut prec = attache;
        for &r in &route {
            self.monde.relier(prec, r);
            prec = r;
        }
        for b in touchees {
            self.scinder(b, &route);
        }
        let (x, y) = self.monde.xy(bout);
        self.journal(format!("Route terminée vers ({x}, {y}) : {} tiles", route.len()));
        self.reconstruire_reseau();
    }

    /// Le seigneur fait tracer une route là où l'on marche beaucoup hors route : le trajet de travail le plus répété dont l'accès
    /// dépasse `route_seuil_acces` de `distance_max`. Un chantier de route à la fois.
    fn decider_routes(&mut self, motif_seuil_trajets: u32) -> bool {
        let seuil = self.cat.infra.distance_max * self.cat.infra.route_seuil_ppm / PPM;
        let nq = self.nq();
        let en_cours = self.monde.batiments.iter().any(|b| matches!(b.chantier.as_ref().map(|c| c.cible), Some(Cible::Route(..))));
        let mut candidats: Vec<(u32, usize, usize)> = self
            .trajets
            .iter()
            .filter(|&(_, &n)| n >= motif_seuil_trajets)
            .map(|(&(q, b), &n)| (n, q, b))
            .collect();
        self.trajets.clear();
        if en_cours {
            return false;
        }
        candidats.sort_by(|a, b| b.cmp(a));
        for (n, q, b) in candidats {
            if b >= self.monde.batiments.len() || q >= nq || self.monde.batiments[b].chantier.is_some() || self.monde.batiments[b].abandon_mois.is_some() {
                continue;
            }
            let Some(ch) = self.reseau.chemin(q, nq + b) else { continue };
            if ch.cout <= seuil {
                continue;
            }
            let Some((_, travail)) = self.plan_route_entre(q, b) else { continue };
            if self.bras_recrutables_depuis(Some(q), false) == 0 {
                return false;
            }
            let cout = crate::donnees::CoutCreation { travail, materiau: 0, equipe: 1 };
            let noeud = self.monde.batiments[b].tuile;
            let c = self.site_de_chantier(noeud, Cible::Route(q, b), &cout);
            let (x, y) = self.monde.xy(noeud);
            self.journal(format!(
                "Chantier lancé : route vers la terre en ({x}, {y}) ({} pop-jours) — {n} jours de marche en {} jours",
                travail / 1000,
                self.cat.extension.periode_jours
            ));
            self.reconstruire_reseau();
            self.recruter(c, cout.equipe);
            return true;
        }
        false
    }

    /// Part du coût de construction que coûte la remise en état, selon l'usure (0 à 1 en ppm).
    fn part_remise_ppm(&self, usure_ppm: i64) -> i64 {
        let a = &self.cat.abandon;
        a.remise_min_ppm + (a.remise_max_ppm - a.remise_min_ppm) * usure_ppm.clamp(0, PPM) / PPM
    }

    fn cout_remise(&self, plein: crate::donnees::CoutCreation, usure_ppm: i64) -> crate::donnees::CoutCreation {
        let part = self.part_remise_ppm(usure_ppm);
        crate::donnees::CoutCreation {
            travail: plein.travail * part / PPM,
            materiau: plein.materiau * part / PPM,
            equipe: plein.equipe,
        }
    }

    /// Hameau abandonné (pas encore détruit) le plus proche de `centre`, le moins usé d'abord : celui qu'on remet en état.
    fn hameau_a_remettre(&self, centre: usize) -> Option<usize> {
        let ruine = self.cat.abandon.hameau_ruine_mois as i64;
        (0..self.nq())
            .filter(|&h| {
                let q = &self.monde.quartiers[h];
                !q.detruit && q.abandon_mois.is_some() && self.reseau.chemin(centre, h).is_some()
            })
            .min_by_key(|&h| (self.monde.quartiers[h].abandon_mois.unwrap_or(0) as i64 * PPM / ruine, h))
    }

    /// Remet en état un hameau abandonné : le seigneur ordonne les réparations, moins chères qu'une fondation, de plus
    /// en plus chères avec l'usure, tant que le hameau n'est pas en ruine (docs/07 §14, §15).
    fn lancer_remise_hameau(&mut self, q: usize, motif: &str) -> bool {
        if self.bras_recrutables_depuis(Some(q), false) == 0 {
            return false;
        }
        let h = &self.monde.quartiers[q];
        let Some(abandon) = h.abandon_mois else { return false };
        if h.detruit {
            return false;
        }
        let tuile = h.tuile;
        let maisons = h.capacite_memoire.max(1) as i64;
        let un = self.cout_logement(self.cat.habitats[h.habitat_id].cout_creation.clone(), tuile);
        let plein = crate::donnees::CoutCreation { travail: un.travail * maisons, materiau: un.materiau * maisons, equipe: un.equipe };
        let usure = abandon as i64 * PPM / self.cat.abandon.hameau_ruine_mois as i64;
        let cout = self.cout_remise(plein, usure);
        if !self.assez_de_materiau(q, cout.materiau)
            || self.monde.batiments.iter().any(|b| b.tuile == tuile && b.chantier.is_some())
        {
            return false;
        }
        let b = self.site_de_chantier(tuile, Cible::RemiseHameau(q), &cout);
        let (x, y) = self.monde.xy(tuile);
        self.journal(format!(
            "Chantier lancé : remettre en état le hameau en ({x}, {y}) ({} % de sa construction) — {motif}",
            self.part_remise_ppm(usure) * 100 / PPM
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe);
        true
    }

    /// Remet en service une terre abandonnée (pas encore redevenue friche).
    fn lancer_remise_terre(&mut self, centre: usize, type_id: usize, motif: &str) -> bool {
        let nq = self.nq();
        let friche = self.cat.abandon.terre_friche_mois as i64;
        let Some(t) = (0..self.monde.batiments.len())
            .filter(|&b| {
                let bat = &self.monde.batiments[b];
                bat.type_id == type_id && bat.abandon_mois.is_some() && bat.chantier.is_none() && self.reseau.chemin(centre, nq + b).is_some()
            })
            .min_by_key(|&b| (self.monde.batiments[b].abandon_mois.unwrap_or(0) as i64 * PPM / friche, b))
        else {
            return false;
        };
        let emprise = self.monde.batiments[t].emprise.clone();
        let Some(plein) = self.cout_defrichement(type_id, &emprise) else { return false };
        let usure = self.monde.batiments[t].abandon_mois.unwrap_or(0) as i64 * PPM / friche;
        let cout = self.cout_remise(plein, usure);
        if !self.assez_de_materiau(centre, cout.materiau) {
            return false;
        }
        let noeud = self.monde.batiments[t].tuile;
        let b = self.site_de_chantier(noeud, Cible::RemiseTerre(t), &cout);
        let (x, y) = self.monde.xy(noeud);
        let nom = self.cat.batiments[type_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : remettre en service {nom} ({} tiles) en ({x}, {y}) ({} % du défrichement) — {motif}",
            emprise.len(),
            self.part_remise_ppm(usure) * 100 / PPM
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe);
        true
    }

    /// Abandonne le chantier `b` : l'équipe est libérée, le site redevient inerte, les tiles réservées sont libres
    /// (friche) ; un hameau en fondation est détruit et ses pops retournent dans un autre hameau.
    fn abandonner_chantier(&mut self, b: usize) {
        let cause = match self.monde.batiments[b].limitant {
            Limitant::Intrant(_) => "faute de bois",
            Limitant::MainOeuvre => "faute de bras",
            _ => "sans progrès",
        };
        let Some(ch) = self.monde.batiments[b].chantier.take() else {
            return;
        };
        let nr = self.cat.ressources.len();
        let noeud = self.monde.batiments[b].tuile;
        let (x, y) = self.monde.xy(noeud);
        let emprise = std::mem::replace(&mut self.monde.batiments[b].emprise, vec![noeud]);
        let mut libres: Vec<usize> = Vec::new();
        let quoi = match ch.cible {
            Cible::Terre(_) => {
                libres = emprise;
                "terre"
            }
            Cible::Extension(_) => {
                libres = emprise;
                "agrandissement"
            }
            Cible::Hameau(q) => {
                libres.push(noeud);
                self.monde.quartiers[q].detruit = true;
                self.monde.quartiers[q].capacite_pops = 0;
                let alive: Vec<usize> = (0..self.nq())
                    .filter(|&h| h != q && !self.monde.quartiers[h].detruit && self.monde.quartiers[h].abandon_mois.is_none() && self.monde.quartiers[h].capacite_pops > 0)
                    .collect();
                for i in 0..self.pops.len() {
                    if self.pops[i].residence == q {
                        let tuile = self.monde.quartiers[q].tuile;
                        let h = alive
                            .iter()
                            .copied()
                            .min_by_key(|&h| (self.distance_tiles(tuile, self.monde.quartiers[h].tuile), h))
                            .unwrap_or(q);
                        self.pops[i].residence = h;
                    }
                }
                "hameau"
            }
            Cible::Maison(_) => "maison",
            Cible::RemiseHameau(_) => "remise en état d'un hameau",
            Cible::RemiseTerre(_) => "remise en service d'une terre",
            Cible::Route(..) => "route",
        };
        for &t in &libres {
            self.monde.tuiles[t].contenu = Contenu::Vide;
            self.monde.tuiles[t].vacant_mois = 1;
        }
        let bat = &mut self.monde.batiments[b];
        for r in 0..nr {
            self.bilan.perime[r] += bat.stock_entree[r] + bat.stock_sortie[r];
            bat.stock_entree[r] = 0;
            bat.stock_sortie[r] = 0;
        }
        bat.equipe = 0;
        bat.inactif_mois = 0;
        for p in &mut self.pops {
            if p.emploi == Some(b) {
                p.emploi = None;
            }
        }
        self.journal(format!("Chantier abandonné ({quoi}) en ({x}, {y}) : {cause}, sans progrès depuis trop longtemps"));
        self.reconstruire_reseau();
    }

    /// Abandonne une terre (gratuit) : elle ne produit plus ni n'attire de travail ; elle redevient friche après
    /// `terre_friche_ans`. Appelé par l'entretien quand personne ne la travaille, et par le décideur (phase 6).
    pub fn abandonner_terre(&mut self, b: usize) {
        if self.monde.batiments[b].abandon_mois.is_some() {
            return;
        }
        self.monde.batiments[b].abandon_mois = Some(0);
        let (x, y) = self.monde.xy(self.monde.batiments[b].tuile);
        let nom = self.cat.batiments[self.monde.batiments[b].type_id].nom.clone();
        self.journal(format!("Terre abandonnée : {nom} en ({x}, {y})"));
    }

    /// Abandonne un hameau : plus de logement, il s'use puis sera détruit après `hameau_ruine_ans`.
    pub fn abandonner_hameau(&mut self, q: usize) {
        let h = &mut self.monde.quartiers[q];
        if h.abandon_mois.is_some() || h.detruit {
            return;
        }
        h.abandon_mois = Some(0);
        h.capacite_memoire = h.capacite_pops.max(h.capacite_memoire);
        h.capacite_pops = 0;
        let (x, y) = self.monde.xy(self.monde.quartiers[q].tuile);
        self.journal(format!("Hameau abandonné en ({x}, {y})"));
    }

    /// La terre `b` abandonnée a fini de s'user : ses tiles redeviennent friche (libres, repoussées plus tard en forêt).
    fn liberer_terre(&mut self, b: usize) {
        // Un agrandissement en cours de cette terre n'a plus d'objet.
        let en_cours: Vec<usize> = (0..self.monde.batiments.len())
            .filter(|&c| matches!(self.monde.batiments[c].chantier.as_ref().map(|ch| ch.cible), Some(Cible::Extension(t)) if t == b))
            .collect();
        for c in en_cours {
            self.abandonner_chantier(c);
        }
        let nr = self.cat.ressources.len();
        let emprise = std::mem::take(&mut self.monde.batiments[b].emprise);
        let noeud = self.monde.batiments[b].tuile;
        for &t in &emprise {
            self.monde.tuiles[t].contenu = Contenu::Vide;
            self.monde.tuiles[t].vacant_mois = 1;
        }
        let nom = self.cat.batiments[self.monde.batiments[b].type_id].nom.clone();
        let inerte = self.cat.type_chantier();
        let bat = &mut self.monde.batiments[b];
        for r in 0..nr {
            self.bilan.perime[r] += bat.stock_entree[r] + bat.stock_sortie[r];
            bat.stock_entree[r] = 0;
            bat.stock_sortie[r] = 0;
        }
        bat.type_id = inerte; // site inerte, réutilisé par un prochain chantier sur ce nœud
        bat.emprise = vec![noeud];
        bat.abandon_mois = None;
        bat.inactif_mois = 0;
        bat.prod_ppm = 0;
        bat.sur_pied_ppm = 0;
        bat.equipe = 0;
        let (x, y) = self.monde.xy(noeud);
        self.journal(format!("Terre redevenue friche : {nom} de {} tiles en ({x}, {y})", emprise.len()));
    }

    /// Le hameau abandonné `q` est en ruine : il est détruit, sa tile est libre.
    fn detruire_hameau(&mut self, q: usize) {
        let nr = self.cat.ressources.len();
        let tuile = self.monde.quartiers[q].tuile;
        self.monde.tuiles[tuile].contenu = Contenu::Vide;
        self.monde.tuiles[tuile].vacant_mois = 1;
        for r in 0..nr {
            self.bilan.perime[r] += self.monde.quartiers[q].stock[r];
            self.monde.quartiers[q].stock[r] = 0;
        }
        if let Some(a) = self.monde.quartiers[q].atelier.take() {
            let inerte = self.cat.type_chantier();
            let bat = &mut self.monde.batiments[a];
            for r in 0..nr {
                self.bilan.perime[r] += bat.stock_entree[r] + bat.stock_sortie[r];
                bat.stock_entree[r] = 0;
                bat.stock_sortie[r] = 0;
            }
            bat.type_id = inerte;
            bat.quartier = None;
            bat.prod_ppm = 0;
        }
        self.monde.quartiers[q].detruit = true;
        let (x, y) = self.monde.xy(tuile);
        self.journal(format!("Hameau en ruine détruit en ({x}, {y})"));
    }

    /// Facteur de durée de repousse d'une tile (ppm) : plus rapide en lisière d'une forêt, plus lent isolée.
    pub fn facteur_repousse_ppm(&self, tuile: usize) -> i64 {
        let forets = (0..8)
            .filter_map(|d| self.monde.voisin(tuile, d))
            .filter(|&v| self.monde.tuiles[v].terrain == crate::terrain::Terrain::Foret && self.monde.tuiles[v].eau == Eau::Aucune)
            .count();
        match forets {
            0 => self.cat.abandon.lisiere_isolee_ppm,
            1 | 2 => self.cat.abandon.lisiere_moyenne_ppm,
            _ => self.cat.abandon.lisiere_forte_ppm,
        }
    }

    /// Mois qu'il reste avant qu'une tile libérée redevienne forêt (`None` : elle ne repousse pas, ou n'est pas vacante).
    pub fn repousse_restante_mois(&self, tuile: usize) -> Option<i64> {
        let t = &self.monde.tuiles[tuile];
        if t.vacant_mois == 0 || t.contenu != Contenu::Vide || t.eau != Eau::Aucune || !t.terrain.ouvert() {
            return None;
        }
        let total = self.cat.abandon.foret_apres_mois as i64 * self.facteur_repousse_ppm(tuile) / PPM;
        Some((total - t.vacant_mois as i64).max(0))
    }

    fn repousse_mensuelle(&mut self) {
        let mini = self.cat.abandon.foret_apres_mois as i64 * self.cat.abandon.lisiere_forte_ppm / PPM;
        let mut a_boiser: Vec<usize> = Vec::new();
        for i in 0..self.monde.tuiles.len() {
            let t = &mut self.monde.tuiles[i];
            if t.vacant_mois == 0 {
                continue;
            }
            // Réoccupée, ou d'un terrain qui ne repousse pas : on arrête de compter.
            if t.contenu != Contenu::Vide || t.aretes != 0 || t.eau != Eau::Aucune || !t.terrain.ouvert() {
                t.vacant_mois = 0;
                continue;
            }
            t.vacant_mois = t.vacant_mois.saturating_add(1);
            if t.vacant_mois as i64 >= mini {
                a_boiser.push(i);
            }
        }
        for i in a_boiser {
            if self.repousse_restante_mois(i) == Some(0) {
                self.monde.tuiles[i].terrain = crate::terrain::Terrain::Foret;
                self.monde.tuiles[i].vacant_mois = 0;
            }
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
            emprise: vec![tuile],
            quartier: None,
            equipe: cout.equipe,
            chantier: Some(Chantier {
                cible,
                travail_restant: cout.travail,
                travail_total: cout.travail,
                materiau_total: cout.materiau,
                maitre: Some(0),
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
            inactif_mois: 0,
            abandon_mois: None,
            progres_ref: cout.travail,
        }
    }

    /// Choisit jusqu'à `k` tiles libres contiguës (voisines par un côté) à `base`, de proche en proche,
    /// les plus fertiles d'abord. Une tile libre est vide et traversée par aucune arête.
    ///
    /// Score de chaque candidate (docs/02 §10.2, poids dans `[extension]`) : fertilité, plus un bonus par
    /// voisin déjà dans l'emprise (périmètre court : clôtures, trajets), moins la distance au hameau
    /// demandeur, moins une pénalité hors du terroir de la terre (hameau le plus proche différent).
    fn agrandir(&self, base: &[usize], k: usize, hameau: Option<usize>, type_id: Option<usize>, interdits: &[usize]) -> Vec<usize> {
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
                    if !libre(v) || occupees.contains(&v) || interdits.contains(&v) {
                        continue;
                    }
                    let voisins = [0usize, 2, 4, 6]
                        .iter()
                        .filter_map(|&e| self.monde.voisin(v, e))
                        .filter(|w| occupees.contains(w))
                        .count() as i64;
                    let mut score = self.fert_pour(type_id, v) + voisins * ext.poids_voisins_ppm
                        - self.cat.travail_conversion(type_id, &[v], &self.monde.tuiles) / 1000
                            * ext.poids_conversion_ppm;
                    // En amas : on s'éloigne peu du centre de la terre (forme de disque, pas de bande), et on
                    // se range volontiers contre une route ou un hameau (docs/07 §18).
                    let n = occupees.len() as i64;
                    let (sx, sy) = occupees.iter().fold((0i64, 0i64), |(a, b), &o| {
                        let (x, y) = self.monde.xy(o);
                        (a + x as i64, b + y as i64)
                    });
                    let (vx, vy) = self.monde.xy(v);
                    let (dx, dy) = (vx as i64 * n - sx, vy as i64 * n - sy);
                    let d2_100 = (dx * dx + dy * dy) * 100 / (n * n).max(1);
                    score -= ext.poids_centre_ppm * d2_100 / 100;
                    let contre_route = (0..8).filter_map(|e| self.monde.voisin(v, e)).any(|w| {
                        self.monde.tuiles[w].aretes != 0 || matches!(self.monde.tuiles[w].contenu, Contenu::Quartier(_))
                    });
                    if contre_route {
                        score += ext.poids_route_ppm;
                    }
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
            .filter(|&q| !self.monde.quartiers[q].detruit)
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
                bat.progres_ref = cout.travail;
                bat.inactif_mois = 0;
                bat.chantier = Some(Chantier {
                    cible,
                    travail_restant: cout.travail,
                    travail_total: cout.travail,
                    materiau_total: cout.materiau,
                    maitre: Some(0),
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
        let travail: Qte = tuiles
            .iter()
            .map(|&t| c.travail * self.cat.cout_terrain_ppm[self.monde.tuiles[t].terrain as usize] / PPM)
            .sum();
        let travail = travail + self.cat.travail_conversion(Some(type_id), tuiles, &self.monde.tuiles);
        Some(crate::donnees::CoutCreation {
            travail,
            materiau: c.materiau * tuiles.len() as i64,
            equipe: c.equipe,
        })
    }

    /// Coût d'un chantier de logement sur la tile `tuile` : le travail suit le terrain.
    fn cout_logement(&self, mut cout: crate::donnees::CoutCreation, tuile: usize) -> crate::donnees::CoutCreation {
        cout.travail = cout.travail * self.cat.cout_terrain_ppm[self.monde.tuiles[tuile].terrain as usize] / PPM
            + self.cat.travail_conversion(None, &[tuile], &self.monde.tuiles);
        cout
    }

    /// Rendement d'une tile pour une terre de ce type, une fois le terrain converti (défrichement, drainage),
    /// ou fertilité seule.
    fn fert_pour(&self, type_id: Option<usize>, t: usize) -> i64 {
        let tu = self.monde.tuiles[t];
        match type_id {
            Some(ty) => {
                let apres = Tuile { terrain: self.cat.terrain_apres(ty, tu.terrain), ..tu };
                self.cat.rendement_tile(ty, &apres)
            }
            None => tu.fertilite_ppm,
        }
    }

    /// Travail des ponts et gués d'une route neuve : une somme par tile d'eau traversée (milli pop-jours).
    fn travail_ponts(&self, route: &[usize]) -> Qte {
        route.iter().map(|&t| self.cat.pont_milli[self.monde.tuiles[t].eau as usize]).sum()
    }

    /// Défricher pour la ressource d'un type de terre : d'abord agrandir une terre existante du même
    /// type (moins coûteux, sans route), sinon en fonder une nouvelle.
    fn lancer_defrichement(&mut self, centre: usize, type_id: usize, motif: &str) -> bool {
        if self.pops.len() > 0 && self.bras_recrutables_depuis(Some(centre), false) == 0 {
            return false;
        }
        // Une terre abandonnée de ce type à portée se remet en service à moindre coût : d'abord elle.
        if self.lancer_remise_terre(centre, type_id, motif) {
            return true;
        }
        let nq = self.nq();
        let max = self.cat.batiments[type_id].emprise_max as usize;
        // Terres de ce type à portée du hameau, avec de la place pour grandir.
        let mut candidates: Vec<(i64, usize)> = Vec::new();
        for b in 0..self.monde.batiments.len() {
            let bat = &self.monde.batiments[b];
            if bat.type_id != type_id || bat.chantier.is_some() || bat.abandon_mois.is_some() || bat.emprise.len() >= max {
                continue;
            }
            let Some(ch) = self.reseau.chemin(centre, nq + b) else {
                continue;
            };
            let voisines = self.agrandir(&bat.emprise, 1, Some(centre), Some(type_id), &[]);
            let Some(&v) = voisines.first() else {
                continue;
            };
            // Meilleure fertilité d'abord, puis la plus proche.
            candidates.push((-self.fert_pour(Some(type_id), v) * 1000 / (ch.cout + 10), b));
        }
        candidates.sort();
        // Agrandir une terre est moins cher qu'en fonder une (pas de route) tant que le terrain est ouvert ; si
        // l'agrandissement exige de défricher ou de drainer, on compare avec une terre neuve (pont compris).
        let mut etendre = !candidates.is_empty();
        if let Some(&(_, b0)) = candidates.first() {
            let tuiles = self.agrandir(
                &self.monde.batiments[b0].emprise,
                (self.cat.extension.tuiles_par_defrichement as usize)
                    .min(self.cat.batiments[type_id].emprise_max as usize - self.monde.batiments[b0].emprise.len().min(self.cat.batiments[type_id].emprise_max as usize)),
                Some(centre),
                Some(type_id),
                &[],
            );
            let conversion = self.cat.travail_conversion(Some(type_id), &tuiles, &self.monde.tuiles);
            if conversion > 0 {
                if let (Some(c_ext), Some((_, _, c_terre, _))) =
                    (self.cout_defrichement(type_id, &tuiles), self.plan_terre(centre, type_id))
                {
                    etendre = c_ext.travail <= c_terre.travail * tuiles.len().max(1) as i64
                        / self.cat.extension.tuiles_par_defrichement as i64;
                }
            }
        }
        if etendre {
            for &(_, b) in &candidates {
                if self.lancer_extension(centre, b, motif) {
                    return true;
                }
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
        let nouvelles = self.agrandir(&self.monde.batiments[b].emprise, k, Some(centre), Some(type_id), &[]);
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
            if !livrable || !self.assez_de_materiau(nq + b, cout.materiau) {
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
        let Some((site, emprise, cout, ponts)) = self.plan_terre(centre, type_id) else {
            return false;
        };
        let nb_destructions = site.route.iter().filter(|&&t| self.destruction_de(t).is_some()).count();
        self.poser_site(&site, false);
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
            "Chantier lancé : défricher {nom} ({} tiles) en ({x}, {y}){}{} — {motif}",
            emprise.len(),
            if ponts > 0 { format!(", avec franchissement de cours d'eau ({} pop-jours de pont)", ponts / 1000) } else { String::new() },
            if nb_destructions > 0 { format!(", en détruisant {nb_destructions} tiles de terre") } else { String::new() }
        ));
        self.reconstruire_reseau();
        self.recruter(b, cout.equipe);
        true
    }

    /// Projet de fondation d'une terre : site, tiles, coût complet (défrichement, conversion du terrain,
    /// ponts) et travail de pont. Ne modifie rien.
    fn plan_terre(&self, centre: usize, type_id: usize) -> Option<(Site, Vec<usize>, crate::donnees::CoutCreation, Qte)> {
        let k = self.cat.extension.tuiles_par_defrichement as usize;
        let cout_tile = self.cat.batiments[type_id].cout_creation.clone()?;
        if !self.assez_de_materiau(centre, cout_tile.materiau * k as i64) {
            return None;
        }
        let requis: Vec<Vec<usize>> = if cout_tile.materiau > 0 {
            vec![vec![self.cat.extension.materiau]]
        } else {
            Vec::new()
        };
        let site = self.meilleur_site(centre, true, Some(type_id), &requis)?;
        // La route du site ne passe pas par les tiles de la terre : on les exclut de l'emprise.
        let reste = self.agrandir(&[site.tuile], k.saturating_sub(1), Some(centre), Some(type_id), &site.route);
        let mut emprise = vec![site.tuile];
        emprise.extend(reste);
        // Une terre d'une seule tile n'en est pas une : si la place manque autour du site, on renonce.
        if emprise.len() < 2 && k >= 2 {
            return None;
        }
        let mut cout = self.cout_defrichement(type_id, &emprise)?;
        let ponts = self.travail_ponts(&site.route);
        cout.travail += self.travail_route(&site.route);
        Some((site, emprise, cout, ponts))
    }

    /// Achève d'un coup tous les chantiers en cours (génération de l'état initial : le village se constitue sans durée).
    fn achever_chantiers(&mut self) {
        for b in &mut self.monde.batiments {
            if let Some(ch) = b.chantier.as_mut() {
                ch.travail_restant = 0;
            }
        }
        self.terminer_chantiers();
    }

    /// État initial (docs/07 §5) : à partir du premier hameau, constitue le village avec les règles de site du jeu.
    /// `besoins[t]` : tiles de terre exploitée à fournir par type de bâtiment (voir `village::composition`).
    /// D'abord un bloc de chaque type de terre, puis les hameaux (qui exigent des producteurs à portée), puis le
    /// reste des terres, en tournant entre les hameaux. Les chantiers sont achevés aussitôt.
    pub(crate) fn peupler_initial(&mut self, besoins: &[usize], nb_hameaux: usize) -> Result<(), String> {
        // Tiles équivalentes (en ppm de tile) : chaque tile compte pour son rendement réel, de sorte qu'un village sur
        // sol pauvre pose plus de terre qu'un village sur limon (docs/07 §4).
        let tuiles_de = |s: &Self, t: usize| -> usize {
            s.monde
                .batiments
                .iter()
                .filter(|b| b.type_id == t && b.chantier.is_none())
                .flat_map(|b| b.emprise.iter())
                .map(|&i| s.cat.rendement_tile(t, &s.monde.tuiles[i]) as usize)
                .sum()
        };
        // Les terres qui n'ont pas besoin de bois d'abord : le bois est le matériau des autres chantiers.
        let mut ordre: Vec<usize> = (0..self.cat.batiments.len())
            .filter(|&t| self.cat.batiments[t].par_tile && besoins.get(t).copied().unwrap_or(0) > 0)
            .collect();
        ordre.sort_by_key(|&t| (self.cat.batiments[t].cout_creation.as_ref().map_or(false, |c| c.materiau > 0), t));
        let cap = self.cat.habitats[0].capacite_pops;
        let mut tour = 0usize;
        let lancer_terre_de = |s: &mut Self, t: usize, tour: &mut usize, motif: &str| -> bool {
            let nq = s.nq();
            for k in 0..nq {
                let c = (*tour + k) % nq;
                if s.lancer_defrichement(c, t, motif) {
                    s.achever_chantiers();
                    *tour += 1;
                    return true;
                }
            }
            false
        };
        // Le village se constitue par étapes : à chaque étape, une part des terres, puis un hameau de plus. Le
        // terroir déjà occupé rabat la valeur des sites proches : les hameaux s'écartent d'eux-mêmes (docs/07 §4).
        for etape in 1..=nb_hameaux.max(1) {
            for &t in &ordre {
                // Une marge sur le besoin nominal : le rendement réel des tiles (rocaille, qualité du sol) est inférieur à 1,0.
                let cible = (besoins[t] * PPM as usize * MARGE_TERRES_PCT / 100 * etape).div_ceil(nb_hameaux.max(1)).max(1);
                // Au moins un bloc de chaque type dès la première étape (les hameaux exigent des producteurs).
                for _ in 0..2000 {
                    if tuiles_de(self, t) >= cible {
                        break;
                    }
                    if !lancer_terre_de(self, t, &mut tour, "état initial") {
                        if tuiles_de(self, t) == 0 {
                            return Err(format!("aucune place pour {}", self.cat.batiments[t].nom));
                        }
                        break;
                    }
                }
            }
            if etape < nb_hameaux {
                let nq = self.nq();
                let mut fait = false;
                for k in 0..nq {
                    if self.lancer_hameau((tour + k) % nq, 0, "état initial") {
                        self.achever_chantiers();
                        fait = true;
                        break;
                    }
                }
                if !fait {
                    if false { eprintln!("étape {etape} nq {} types {:?} besoins {:?}", self.nq(), (0..self.cat.batiments.len()).map(|t| tuiles_de(self, t)).collect::<Vec<_>>(), besoins); let h = self.monde.quartiers[0].tuile; let libres = (0..self.monde.tuiles.len()).filter(|&i| { let tu = &self.monde.tuiles[i]; tu.contenu == Contenu::Vide && tu.aretes == 0 && tu.constructible() && self.distance_tiles(i, h) <= 14 }).count(); eprintln!("libres à 14 : {libres} ; hameaux {:?}", (0..self.nq()).map(|q| self.monde.xy(self.monde.quartiers[q].tuile)).collect::<Vec<_>>()); }
                    return Err("aucun site pour un hameau de plus".into());
                }
                let q = self.nq() - 1;
                self.monde.quartiers[q].capacite_pops = cap;
                tour += 1;
            }
        }
        self.monde.quartiers[0].capacite_pops = cap;
        for &t in &ordre {
            if tuiles_de(self, t) * 2 < besoins[t] * PPM as usize * MARGE_TERRES_PCT / 100 {
                return Err(format!("trop peu de terre pour {} : {} sur {}", self.cat.batiments[t].nom, tuiles_de(self, t) / PPM as usize, besoins[t]));
            }
        }
        self.evenements.clear();
        Ok(())
    }

    /// Le seigneur peut-il payer le matériau de ce chantier ? C'est son grenier qui le fournit : il ne lance que ce
    /// qu'il peut payer (docs/07 §15). Pendant la constitution de l'état initial, les chantiers sont achevés d'un coup.
    fn assez_de_materiau(&self, _lieu: usize, besoin: Qte) -> bool {
        if besoin <= 0 || self.pops.is_empty() {
            return true;
        }
        let mat = self.cat.extension.materiau;
        let engage: Qte = self
            .monde
            .batiments
            .iter()
            .filter_map(|b| b.chantier.as_ref())
            .map(|c| c.materiau_total)
            .sum();
        self.monde.personnages.first().map_or(true, |p| p.grenier[mat] >= engage + besoin)
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
        if !self.pops.is_empty() && self.bras_recrutables_depuis(Some(q), false) == 0 {
            return false;
        }
        let nq = self.nq();
        let habitat_id = self.monde.quartiers[q].habitat_id;
        let cap = self.monde.quartiers[q].capacite_pops;
        // Seule la densité maximale par tile borne un hameau (docs/07 §4.2) ; c'est la valeur du site qui décide.
        if cap == 0 || cap >= self.cat.habitat.densite_max_pops {
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
            if !livrable || !self.assez_de_materiau(q, cout.materiau) {
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
                    maitre: Some(0),
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

    /// Valeur d'un site de vie pour `nouveaux` pops de plus (docs/07 §4.1, §4.2) : la terre libre à portée
    /// (rendement d'une terre nourricière, décroissant avec la distance, forêt et marais comptés après leur
    /// coût de conversion), **partagée entre les pops du voisinage**, corrigée par l'eau à portée et par la
    /// présence de voisins. Elle baisse à mesure que le terroir se remplit : c'est ce qui limite un noyau et
    /// pousse à fonder ailleurs. Aucune taille n'est codée.
    pub fn valeur_habitat(&self, tuile: usize, nouveaux: i64) -> i64 {
        let pe = &self.cat.peuplement;
        let (cx, cy) = self.monde.xy(tuile);
        let r = pe.rayon_terroir;
        let type_nourricier = self.type_nourricier();
        let mut terroir: i64 = 0;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let (x, y) = (cx as i64 + dx, cy as i64 + dy);
                if x < 0 || y < 0 || x >= self.monde.largeur as i64 || y >= self.monde.hauteur as i64 {
                    continue;
                }
                let t = y as usize * self.monde.largeur + x as usize;
                let tu = &self.monde.tuiles[t];
                if tu.contenu != Contenu::Vide || tu.aretes != 0 || !tu.constructible() {
                    continue;
                }
                // Une tile à convertir vaut moins : on rabat sa valeur du travail de conversion.
                let travaux = self.cat.travail_conversion(type_nourricier, &[t], &self.monde.tuiles);
                let f = self.fert_pour(type_nourricier, t) * 100_000 / (100_000 + travaux);
                terroir += f / (dx.abs().max(dy.abs()) + 3);
            }
        }
        // Pops du voisinage (terroir partagé) et pops tout près (voisins).
        let (mut pops, mut proches) = (0i64, 0i64);
        for p in &self.pops {
            let t = self.monde.quartiers[p.residence].tuile;
            let d = self.distance_tiles(tuile, t);
            if d <= r {
                pops += 1;
            }
            if d <= pe.rayon_voisinage {
                proches += 1;
            }
        }
        let d_eau = self.dist_eau[tuile] as i64;
        let eau = if d_eau <= pe.eau_proche {
            PPM
        } else {
            (PPM * (pe.eau_portee - d_eau).max(0) / (pe.eau_portee - pe.eau_proche)).max(PPM / 10)
        };
        let voisins = PPM + proches * pe.poids_voisins_ppm;
        terroir * eau / PPM * voisins / PPM / (pops + nouveaux).max(1)
    }

    /// Type de terre qui nourrit : la première terre fertile qui livre une ressource nutritive.
    fn type_nourricier(&self) -> Option<usize> {
        (0..self.cat.batiments.len()).find(|&t| {
            let ty = &self.cat.batiments[t];
            ty.fertile && ty.par_tile && ty.extrants_finaux().iter().any(|&(r, _)| self.cat.ressources[r].nutritive_milli > 0)
        })
    }

    /// Site et ressources requises pour fonder un hameau de ce type depuis `centre` (sans rien modifier).
    fn site_hameau(&self, centre: usize, habitat_id: usize) -> Option<Site> {
        let cout = &self.cat.habitats[habitat_id].cout_creation;
        // Un hameau doit pouvoir être nourri (n'importe quelle ressource nutritive) et chauffé à portée de son site, et
        // se faire livrer son matériau de construction. Le textile (lin, laine) n'est pas exigé : un hameau
        // sans fibre à portée verra ses pops pousser à défricher la terre qui lui manque (docs/07 §4.1).
        let nourriture: Vec<usize> = (0..self.cat.ressources.len())
            .filter(|&r| self.cat.ressources[r].nutritive_milli > 0 && !self.cat.producteurs(r).is_empty())
            .collect();
        let mut requis: Vec<Vec<usize>> = vec![nourriture, vec![self.cat.vital.chauffage_ressource]];
        if cout.materiau > 0 {
            requis.push(vec![self.cat.extension.materiau]);
        }
        self.meilleur_site(centre, false, None, &requis)
    }

    fn lancer_hameau(&mut self, centre: usize, habitat_id: usize, motif: &str) -> bool {
        if !self.pops.is_empty() && self.bras_recrutables_depuis(Some(centre), true) == 0 {
            return false;
        }
        let cout = self.cat.habitats[habitat_id].cout_creation.clone();
        // Un hameau doit pouvoir être bâti (matériau réellement disponible), chauffé et nourri à portée de son site.
        if !self.assez_de_materiau(centre, cout.materiau) {
            return false;
        }
        let Some(site) = self.site_hameau(centre, habitat_id) else {
            return false;
        };
        // Le travail de fondation suit le terrain du site (docs/07 §3.3).
        let mut cout = self.cout_logement(cout, site.tuile);
        let ponts = self.travail_ponts(&site.route);
        let destructions = self.travail_destructions(&site.route);
        let nb_destructions = site.route.iter().filter(|&&t| self.destruction_de(t).is_some()).count();
        cout.travail += ponts + destructions;
        self.poser_site(&site, true);
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
            vide_mois: 0,
            abandon_mois: None,
            detruit: false,
            capacite_memoire: 0,
        });
        let b = self.monde.batiments.len();
        let chantier = self.nouveau_chantier(site.tuile, Cible::Hameau(q), &cout);
        self.monde.batiments.push(chantier);
        self.monde.tuiles[site.tuile].contenu = Contenu::Quartier(q);
        self.monde.tuiles[site.tuile].variante = (q % 4) as u8;
        let (x, y) = self.monde.xy(site.tuile);
        let nom = self.cat.habitats[habitat_id].nom.clone();
        self.journal(format!(
            "Chantier lancé : bâtir un {nom} en ({x}, {y}){}{} — {motif}",
            if ponts > 0 { format!(", avec franchissement de cours d'eau ({} pop-jours de pont)", ponts / 1000) } else { String::new() },
            if nb_destructions > 0 { format!(", en détruisant {nb_destructions} tiles de terre ({} pop-jours)", destructions / 1000) } else { String::new() }
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
                .distances_chantier(self.monde.batiments[chantier].tuile)
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
            // Fonder un hameau ne doit pas vider celui d'où l'on part : on ne prend jamais la dernière pop d'un hameau
            // (sinon la fondation déserte l'ancien hameau, qui s'use et est détruit : migration en chaîne).
            if fondation.is_some() && eff[q].saturating_sub(equipe[q]) <= 1 {
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
