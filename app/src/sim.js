// Pont JavaScript vers le moteur Rust compilé en WebAssembly (voir sim-wasm/src/lib.rs).
// Une seule simulation ; chaque appel qui écrit renvoie la longueur du JSON à lire dans le tampon.

export async function chargerSim(urlWasm) {
  const reponse = await fetch(urlWasm)
  if (!reponse.ok) throw new Error(`moteur introuvable (${urlWasm} : ${reponse.status})`)
  const { instance } = await WebAssembly.instantiate(await reponse.arrayBuffer(), {})
  const w = instance.exports
  const decoder = new TextDecoder()
  const lire = (n) => decoder.decode(new Uint8Array(w.memory.buffer, w.sim_sortie_ptr(), n))
  const json = (n) => JSON.parse(lire(n))

  return {
    /** Crée un village. Lève une erreur si aucune disposition valide n'est trouvée. */
    creer({ graine = 1, hameaux = 4, rayonFertile = 12, rayonSterile = 28, occupation = 80 } = {}) {
      if (w.sim_creer(graine >>> 0, hameaux, rayonFertile, rayonSterile, occupation) !== 0) {
        throw new Error(lire(w.sim_sortie_len()))
      }
    },
    avancer: (jours) => w.sim_avancer(jours),
    meta: () => json(w.sim_meta_json()),
    carte: () => json(w.sim_carte_json()),
    /** `depuis` : nombre d'événements déjà reçus (le moteur ne renvoie que les nouveaux). */
    etat: (depuis = 0) => json(w.sim_etat_json(depuis)),
    /** Détail d'une pop : stock, rations, lieux possibles et raison du choix du jour (null si elle a disparu). */
    pop: (id) => json(w.sim_pop_json(id)),
    /** Où une ressource est produite, stockée, expédiée et consommée. */
    ressource: (r) => json(w.sim_ressource_json(r)),
    /** Tiles du chemin du hameau `q` au bâtiment `b`. */
    chemin: (q, b) => json(w.sim_chemin_json(q, b)),
    /** Fertilité de chaque tile en % (0 à 130), ligne par ligne. */
    /** Terrain de chaque tile : 2 octets (terrain ; gabarit d'eau bits 0-1 et sens de l'aval bits 2-4), ligne par ligne. */
    terrain: () => new Uint8Array(w.memory.buffer, w.sim_terrain_ptr(), w.sim_terrain_len()).slice(),
    fertilite: () => new Uint8Array(w.memory.buffer, w.sim_fertilite_ptr(), w.sim_fertilite_len()).slice(),
  }
}
