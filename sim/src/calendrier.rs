//! Calendrier : 365 jours (pas d'années bissextiles), le jour 0 est le lundi 1er janvier 1750.

pub const JOURS_PAR_AN: u32 = 365;
pub const ANNEE_DEPART: u32 = 1750;
pub const JOURS_DU_MOIS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
pub const NOMS_MOIS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre",
];
pub const NOMS_JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

/// (mois 1..=12, jour du mois 1..) pour un jour de l'année 0..365.
pub fn mois_jour(jour_de_l_annee: u32) -> (u32, u32) {
    let mut reste = jour_de_l_annee;
    for (i, n) in JOURS_DU_MOIS.iter().enumerate() {
        if reste < *n {
            return (i as u32 + 1, reste + 1);
        }
        reste -= n;
    }
    (12, 31)
}

/// Les mois froids où l'on brûle du bois (codé en dur, docs/08 §7) : novembre à mars.
pub fn mois_froid(mois: u32) -> bool {
    mois >= 11 || mois <= 3
}
