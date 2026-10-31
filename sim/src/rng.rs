//! Générateur pseudo-aléatoire déterministe (SplitMix64), sans dépendance.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(graine: u64) -> Self {
        Rng(graine ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier dans `0..n` (n > 0).
    pub fn dans(&mut self, n: usize) -> usize {
        (self.u64() % n as u64) as usize
    }

    /// Vrai avec la probabilité `pct` %.
    pub fn chance(&mut self, pct: u32) -> bool {
        (self.u64() % 100) < pct as u64
    }

    pub fn melanger<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.dans(i + 1);
            v.swap(i, j);
        }
    }
}
