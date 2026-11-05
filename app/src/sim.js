// Pont vers le moteur Rust (WebAssembly) : un instantané JSON par appel.
export async function chargerSim(url) {
  const r = await fetch(url)
  if (!r.ok) throw new Error(`moteur introuvable (${url} : ${r.status})`)
  const { instance } = await WebAssembly.instantiate(await r.arrayBuffer(), {})
  const w = instance.exports
  const dec = new TextDecoder()
  return {
    creer: () => w.sim_creer(),
    avancer: (jours) => w.sim_avancer(jours),
    etat() {
      const n = w.sim_etat_json()
      return JSON.parse(dec.decode(new Uint8Array(w.memory.buffer, w.sim_sortie_ptr(), n)))
    },
  }
}
