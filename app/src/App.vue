<script setup>
import { onMounted, onBeforeUnmount, ref } from 'vue'
import { chargerSim } from './sim.js'
import { Carte } from './carte.js'
import PanneauTile from './components/PanneauTile.vue'
import PanneauPop from './components/PanneauPop.vue'
import PanneauAnnee from './components/PanneauAnnee.vue'
import Courbes from './components/Courbes.vue'
import Historique from './components/Historique.vue'
import Journal from './components/Journal.vue'

const VITESSES = [{ nom: '1 j/s', jps: 1 }, { nom: '5 j/s', jps: 5 }, { nom: '1 mois/s', jps: 30 }, { nom: '1 an/s', jps: 365 }]

const zone = ref(null)
const erreur = ref('')
const etat = ref(null)
const choix = ref(null)
const enPause = ref(true)
const vitesse = ref(1)
const meteo = ref(0) // 0 : aléatoire ; 1000 : année moyenne
const graine = ref(1)
let sim, carte, raf, dernier = 0, reste = 0

const dateTxt = () => {
  const d = etat.value?.date
  return d ? `${d.nom_jour} ${d.jour_du_mois} ${d.nom_mois} ${d.annee}` : ''
}

function majCarte() { carte.mettreAJour(etat.value, choix.value) }
function lire() { etat.value = sim.etat(); majCarte() }

function boucle(t) {
  raf = requestAnimationFrame(boucle)
  const dt = Math.min(0.25, (t - dernier) / 1000); dernier = t
  if (enPause.value) return
  reste += dt * VITESSES[vitesse.value].jps
  const jours = Math.floor(reste)
  if (jours > 0) { reste -= jours; sim.avancer(jours); lire() }
}

function selectionner(i) { choix.value = i; majCarte() }
function reinit() { sim.creer(graine.value, meteo.value); enPause.value = true; reste = 0; lire() }
function pas(j) { sim.avancer(j); lire() }

onMounted(async () => {
  try {
    sim = await chargerSim('./sim_wasm.wasm')
    carte = new Carte()
    await carte.init(zone.value, './assets/atlas.png', './assets/atlas.json', selectionner)
    sim.creer(graine.value, meteo.value)
    etat.value = sim.etat()
    choix.value = etat.value.tiles.findIndex((t) => t.type === 'ferme')
    carte.construire(etat.value)
    majCarte()
    window.__app = { sim, etat, enPause, vitesse, lire, choix, selectionner } // captures et débogage
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
      <span v-if="etat" class="meteo">météo de l'année ×{{ (etat.meteo_pm / 1000).toFixed(2) }}</span>
      <span class="sep"></span>
      <label class="petit">météo <select v-model.number="meteo" @change="reinit"><option :value="0">aléatoire</option><option :value="1000">année moyenne</option></select></label>
      <label class="petit">graine <input type="number" v-model.number="graine" min="1" @change="reinit" /></label>
      <button @click="enPause = !enPause">{{ enPause ? '▶ Lecture' : '⏸ Pause' }}</button>
      <button @click="pas(1)" :disabled="!enPause">+1 jour</button>
      <button @click="pas(7)" :disabled="!enPause">+1 sem.</button>
      <select v-model.number="vitesse"><option v-for="(v, i) in VITESSES" :key="i" :value="i">{{ v.nom }}</option></select>
      <button @click="reinit">↺</button>
    </header>
    <main>
      <div class="carte" ref="zone"></div>
      <aside v-if="etat">
        <PanneauPop :etat="etat" />
        <PanneauTile :etat="etat" :choix="choix" />
        <PanneauAnnee :etat="etat" />
        <Courbes :etat="etat" />
        <Historique :etat="etat" />
        <Journal :etat="etat" />
      </aside>
    </main>
    <p v-if="erreur" class="erreur">{{ erreur }}</p>
  </div>
</template>

<style scoped>
.app { display: flex; flex-direction: column; height: 100%; }
header { display: flex; gap: 8px; align-items: center; padding: 8px 12px; background: var(--panneau); border-bottom: 1px solid #0006; flex-wrap: wrap; }
.date { color: var(--accent); font-variant-numeric: tabular-nums; min-width: 12em; }
.meteo { color: var(--discret); font-size: 12px; }
.sep { flex: 1; }
.petit { color: var(--discret); font-size: 12px; display: flex; gap: 4px; align-items: center; }
input[type=number] { width: 52px; }
button, select, input { background: #353a2c; color: var(--texte); border: 1px solid #4a503d; border-radius: 4px; padding: 4px 8px; font: inherit; cursor: pointer; }
button:disabled { opacity: .4; cursor: default; }
main { flex: 1; display: flex; min-height: 0; }
.carte { flex: 1; min-width: 0; position: relative; overflow: hidden; }
.carte :deep(canvas) { position: absolute; inset: 0; }
aside { width: 380px; overflow-y: auto; background: var(--panneau); border-left: 1px solid #0006; }
.erreur { position: absolute; top: 60px; left: 12px; color: #f88; }
@media (max-width: 760px) { main { flex-direction: column; } aside { width: auto; height: 50%; border-left: 0; } }
</style>
