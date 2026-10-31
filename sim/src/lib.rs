//! Moteur de simulation de The Age Of Capital (POC-A : boucle physique).
//!
//! Crate pure : aucune dépendance à Electron / napi. Déterministe : même graine,
//! même résultat (quantités en entiers, itérations ordonnées).

pub mod donnees;
pub mod monde;
pub mod reseau;
pub mod rng;
pub mod simulation;
pub mod village;

pub use donnees::{Catalogue, Qte, PPM, U};
pub use simulation::Simulation;
pub use village::{generer, ParamsVillage};
