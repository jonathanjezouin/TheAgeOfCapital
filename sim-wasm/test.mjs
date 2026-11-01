// Test de fumée du pont WebAssembly : node test.mjs [chemin.wasm]
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const chemin = process.argv[2] ?? "target/wasm32-unknown-unknown/release/sim_wasm.wasm";
const { instance } = await WebAssembly.instantiate(readFileSync(chemin), {});
const w = instance.exports;
const lire = (n) => new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.sim_sortie_ptr(), n));
const json = (n) => JSON.parse(lire(n));

assert.equal(w.sim_creer(1, 10, 3, 7, 80), 0, "création du village");
const meta = json(w.sim_meta_json());
assert.equal(meta.largeur, 200);
assert.equal(meta.ressources.length, 8);
assert.deepEqual(meta.habitats.map((h) => h.capacite), [2, 3]);

const carte = json(w.sim_carte_json());
assert.ok(carte.tuiles.length > 20);
assert.ok(carte.tuiles.some((t) => t.c === "chemin" && t.m > 0));

let etat = json(w.sim_etat_json(0));
assert.equal(etat.jour, 0);
assert.equal(etat.hameaux.length, 10);
const pops0 = etat.nb_pops;

w.sim_avancer(365 * 120);
etat = json(w.sim_etat_json(0));
assert.equal(etat.jour, 365 * 120);
assert.ok(etat.nb_pops > pops0, `croissance ${pops0} -> ${etat.nb_pops}`);
assert.ok(etat.nb_evenements > 0 && etat.evenements.length === etat.nb_evenements);
assert.equal(json(w.sim_etat_json(etat.nb_evenements)).evenements.length, 0, "seulement les nouveaux événements");
assert.ok(json(w.sim_carte_json()).version !== carte.version, "la carte a changé (chantiers terminés)");
assert.equal(w.sim_fertilite_len(), 200 * 200);

// Déterminisme : même graine, même état.
w.sim_creer(1, 10, 3, 7, 80); w.sim_avancer(365 * 120);
const etat2 = json(w.sim_etat_json(0));
assert.equal(JSON.stringify(etat2.pops), JSON.stringify(etat.pops));
console.log(`OK : ${meta.batiments.length} types, ${carte.tuiles.length} tiles, ${etat.nb_pops} pops à 120 ans, ${etat.nb_evenements} événements`);
