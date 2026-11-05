<script setup>
import { onMounted, onBeforeUnmount, ref } from 'vue'
import { chargerSim } from './sim.js'
import { Carte } from './carte.js'

const MOIS = ['janvier', 'février', 'mars', 'avril', 'mai', 'juin', 'juillet', 'août', 'septembre', 'octobre', 'novembre', 'décembre']
const VITESSES = [{ nom: '1 j/s', jps: 1 }, { nom: '10 j/s', jps: 10 }, { nom: '1 mois/s', jps: 30 }, { nom: '1 an/s', jps: 365 }]

const zone = ref(null)
const erreur = ref('')
const etat = ref(null)
const enPause = ref(true)
const vitesse = ref(2)
let sim, carte, raf, dernier = 0, reste = 0

const dateTxt = () => {
  if (!etat.value) return ''
  const j = etat.value.jour_de_l_annee
  const m = Math.min(11, Math.floor(j / 30.42))
  return `${etat.value.annee}, ${MOIS[m]}`
}

function rafraichir() { etat.value = sim.etat(); carte.dessiner(etat.value) }

function boucle(t) {
  raf = requestAnimationFrame(boucle)
  const dt = (t - dernier) / 1000; dernier = t
  if (enPause.value) return
  reste += dt * VITESSES[vitesse.value].jps
  const jours = Math.floor(reste)
  if (jours > 0) { reste -= jours; sim.avancer(jours); etat.value = sim.etat() }
}

const reinit = () => { sim.creer(); enPause.value = true; reste = 0; rafraichir() }
const pas = (j) => { sim.avancer(j); etat.value = sim.etat() }

onMounted(async () => {
  try {
    sim = await chargerSim('./sim_wasm.wasm')
    carte = new Carte()
    await carte.init(zone.value, './assets/atlas.png', './assets/atlas.json')
    rafraichir()
    window.__sim = { sim, etat, enPause, vitesse } // aide aux captures et au débogage
    raf = requestAnimationFrame(boucle)
  } catch (e) { erreur.value = String(e) }
})
onBeforeUnmount(() => cancelAnimationFrame(raf))
</script>

<template>
  <div class="app">
    <header>
      <strong>The Age Of Capital</strong>
      <span class="date">{{ dateTxt() }}</span>
      <span class="sep"></span>
      <button @click="enPause = !enPause">{{ enPause ? '▶ Lecture' : '⏸ Pause' }}</button>
      <button @click="pas(1)" :disabled="!enPause">+1 jour</button>
      <select v-model.number="vitesse"><option v-for="(v, i) in VITESSES" :key="i" :value="i">{{ v.nom }}</option></select>
      <button @click="reinit">↺</button>
    </header>
    <main>
      <div class="carte" ref="zone"></div>
      <aside>
        <h3>Village</h3>
        <dl v-if="etat">
          <dt>Jour</dt><dd>{{ etat.jour }}</dd>
          <dt>Population</dt><dd>{{ etat.pops }} pop</dd>
          <dt>Tiles</dt><dd>{{ etat.tiles.length }}</dd>
        </dl>
      </aside>
    </main>
    <p v-if="erreur" class="erreur">{{ erreur }}</p>
  </div>
</template>

<style scoped>
.app { display: flex; flex-direction: column; height: 100%; }
header { display: flex; gap: 8px; align-items: center; padding: 8px 12px; background: var(--panneau); border-bottom: 1px solid #0006; }
.date { color: var(--accent); font-variant-numeric: tabular-nums; min-width: 9em; }
.sep { flex: 1; }
button, select { background: #353a2c; color: var(--texte); border: 1px solid #4a503d; border-radius: 4px; padding: 4px 10px; font: inherit; cursor: pointer; }
button:disabled { opacity: .4; cursor: default; }
main { flex: 1; display: flex; min-height: 0; }
.carte { flex: 1; min-width: 0; position: relative; overflow: hidden; }
.carte :deep(canvas) { position: absolute; inset: 0; }
aside { width: 220px; padding: 12px; background: var(--panneau); border-left: 1px solid #0006; }
h3 { margin: 0 0 8px; }
dl { display: grid; grid-template-columns: auto 1fr; gap: 4px 12px; margin: 0; }
dt { color: var(--discret); } dd { margin: 0; text-align: right; font-variant-numeric: tabular-nums; }
.erreur { position: absolute; top: 60px; left: 12px; color: #f88; }
</style>
