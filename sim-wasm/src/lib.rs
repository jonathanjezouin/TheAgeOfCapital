//! Pont WebAssembly : une simulation globale, un tampon JSON que le JavaScript lit.

use sim::Simulation;
use std::cell::RefCell;

thread_local! {
    static SIM: RefCell<Simulation> = RefCell::new(Simulation::new());
    static SORTIE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// `meteo_pm` = 0 : météo aléatoire ; sinon météo fixe en ‰ (1000 = année moyenne).
#[no_mangle]
pub extern "C" fn sim_creer(graine: u32, meteo_pm: u32) {
    let p = sim::Params {
        graine: graine as u64,
        meteo_fixe_pm: if meteo_pm == 0 { None } else { Some(meteo_pm as i64) },
        ..Default::default()
    };
    SIM.with(|s| *s.borrow_mut() = Simulation::avec(p));
}

#[no_mangle]
pub extern "C" fn sim_avancer(jours: u32) {
    SIM.with(|s| {
        let mut s = s.borrow_mut();
        for _ in 0..jours {
            s.tick();
        }
    });
}

/// Écrit l'état dans le tampon et renvoie sa longueur.
#[no_mangle]
pub extern "C" fn sim_etat_json() -> u32 {
    let json = SIM.with(|s| s.borrow().etat_json()).into_bytes();
    let n = json.len() as u32;
    SORTIE.with(|o| *o.borrow_mut() = json);
    n
}

#[no_mangle]
pub extern "C" fn sim_sortie_ptr() -> *const u8 {
    SORTIE.with(|o| o.borrow().as_ptr())
}
