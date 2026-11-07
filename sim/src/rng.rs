//! Générateur pseudo-aléatoire déterministe (splitmix64) : même graine, même histoire.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(graine: u64) -> Self {
        Rng(graine)
    }

    pub fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier uniforme dans `0..n`.
    pub fn sous(&mut self, n: u64) -> u64 {
        self.suivant() % n
    }
}
