<script setup>
import { markRaw, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import BarreHaut from './components/BarreHaut.vue'
import PanneauInfos from './components/PanneauInfos.vue'
import Journal from './components/Journal.vue'
import Courbes from './components/Courbes.vue'
import PanneauPop from './components/PanneauPop.vue'
import PanneauRessource from './components/PanneauRessource.vue'
import { chargerSim } from './sim.js'
import { Carte } from './carte.js'
import { store } from './store.js'
import { TYPES } from './libelles.js'

const zone = ref(null)
let sim = null
let carte = null
let accumule = 0 // fraction de jour pas encore simulée
let vueSale = true // l'état simulé a changé : relire l'état
let dernierEtat = 0

const URLS = { atlasPng: './assets/atlas.png', atlasJson: './assets/atlas.json' }

function lireEtat() {
  const e = sim.etat(store.evenements.length)
  if (e.evenements.length) store.evenements.push(...e.evenements)
  store.etat = markRaw(e)
  if (!store.carte || store.carte.version !== e.version) lireCarte(e)
  lireDetails()
  carte.rafraichir(e)
}

/** Vues détaillées demandées au moteur : la pop inspectée, la ressource suivie. */
function lireDetails() {
  if (store.selectionPop != null) {
    const d = sim.pop(store.selectionPop)
    if (!d) store.selectionPop = null
    store.popDetail = d ? markRaw(d) : null
  } else {
    store.popDetail = null
  }
  store.ressourceDetail = store.mode === 'ressource' ? markRaw(sim.ressource(store.ressource)) : null
}

function lireCarte(etat) {
  const c = sim.carte()
  const W = store.meta.largeur
  const grille = new Map(c.tuiles.map((t) => [t.y * W + t.x, t]))
  store.carte = markRaw({ version: c.version, tuiles: c.tuiles, grille })
  // Un défrichement ou un drainage change le terrain : on le relit et on repeint les tiles concernées.
  store.terrain = sim.terrain()
  store.vacance = sim.vacance()
  carte.majSol(store.terrain)
  carte.appliquerCarte(c, etat)
}

function nouvellePartie() {
  store.erreur = null
  try {
    sim.creer({ graine: store.graine, hameaux: store.hameaux })
  } catch (e) {
    store.erreur = `Aucun village valide pour cette graine : ${e.message}`
    return
  }
  store.evenements.length = 0
  store.selection = null
  store.selectionPop = null
  store.carte = null
  store.fertilite = sim.fertilite()
  store.aptitude = sim.aptitude(store.typeAptitude)
  carte.remplacerFertilite(store.aptitude)
  store.terrain = sim.terrain()
  store.vacance = sim.vacance()
  carte.construireSol(store.terrain)
  accumule = 0
  lireEtat()
  carte.centrer(store.carte)
}

function pasUnJour() {
  store.enPause = true
  sim.avancer(1)
  vueSale = true
}

function boucle(ticker) {
  if (!sim || !carte || !store.etat) return
  const dt = ticker.deltaMS / 1000
  if (!store.enPause) {
    accumule += dt * store.vitesse
    const n = Math.floor(accumule)
    accumule -= n
    if (n > 0) {
      sim.avancer(Math.min(n, 800))
      vueSale = true
    }
  }
  store.fraction = accumule
  // Heure affichée : en « jour/nuit » le jour dure 24 s ; sinon on montre la mi-journée (pops au travail).
  const heure = store.vitesse <= 0.5 ? (store.enPause ? store.heure : accumule * 24) : 12
  const heureChangee = Math.abs(heure - store.heure) > 0.05
  store.heure = heure
  const maintenant = performance.now()
  if (vueSale && maintenant - dernierEtat > 90) {
    dernierEtat = maintenant
    vueSale = false
    lireEtat()
  } else if (heureChangee) {
    carte.rafraichir(store.etat)
  }
}

function touche(e) {
  if (e.code === 'Space' && e.target === document.body) {
    e.preventDefault()
    store.enPause = !store.enPause
  }
}

onMounted(async () => {
  window.addEventListener('keydown', touche)
  try {
    sim = await chargerSim('./sim_wasm.wasm')
    sim.creer({ graine: store.graine, hameaux: store.hameaux })
    store.meta = markRaw(sim.meta())
    store.fertilite = sim.fertilite()
    store.terrain = sim.terrain()
    store.vacance = sim.vacance()
    store.typeAptitude = store.meta.batiments.findIndex((b) => b.nom === 'champ')
    store.aptitude = sim.aptitude(store.typeAptitude)
    carte = await new Carte().init(zone.value, URLS, store.meta, store.aptitude, store.terrain)
    carte.sim = sim
    window.__carte = carte
    window.__sim = sim
    window.__store = store
    window.__rafraichir = lireEtat
    lireEtat()
    // On centre une fois la mise en page terminée (le canvas n'a sa taille définitive qu'au 1er rendu).
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))
    carte.app.resize()
    carte.centrer(store.carte)
    carte.app.ticker.add(boucle)
    store.pret = true
  } catch (e) {
    store.erreur = e?.message ?? String(e)
    console.error(e)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', touche)
  carte?.detruire()
})

// Types de terre dont le rendement dépend du terrain, proposés dans le mode « Aptitude ».
const terres = () => store.meta.batiments.map((b, i) => ({ i, nom: TYPES[b.nom] ?? b.nom, fertile: b.fertile })).filter((t) => t.fertile)
watch(() => store.typeAptitude, (t) => {
  if (!sim || !carte || t == null) return
  store.aptitude = sim.aptitude(t)
  carte.remplacerFertilite(store.aptitude)
})

watch(() => [store.mode, store.selection, store.selectionPop, store.ressource], () => {
  if (!store.etat || !carte) return
  lireDetails()
  carte.rafraichir(store.etat)
})
watch(() => store.vitesse, () => carte?.majNuit())
</script>

<template>
  <div class="jeu">
    <BarreHaut class="haut" @pas="pasUnJour" @nouveau="nouvellePartie" />
    <div ref="zone" class="zone-carte">
      <div v-if="!store.pret || store.erreur" class="etat-chargement">
        <div>
          <p v-if="store.erreur" class="erreur">{{ store.erreur }}</p>
          <p v-else>Génération du village…</p>
        </div>
      </div>
      <div v-if="store.pret && store.survol" class="infobulle">({{ store.survol.x }}, {{ store.survol.y }})</div>
      <div v-if="store.pret && store.mode === 'fertilite'" class="legende">
        Aptitude du terrain pour
        <select v-model.number="store.typeAptitude" class="choix-terre" aria-label="Type de terre">
          <option v-for="t in terres()" :key="t.i" :value="t.i">{{ t.nom }}</option>
        </select>
        <div class="degrade" style="background: linear-gradient(90deg, rgb(150,84,56), rgb(196,150,70), rgb(150,176,70), rgb(74,150,70), rgb(30,110,76))"></div>
        pauvre <span style="float: right">riche</span>
      </div>
      <div v-if="store.pret && store.mode === 'ressource'" class="legende">
        Suivi d'une ressource : <span style="color: #7ab648">producteurs</span> · flèches = expéditions du jour · hameaux colorés par couverture · <span style="color: #e0a03a">ateliers</span> qui l'utilisent
      </div>
      <div v-if="store.pret && store.mode === 'besoins'" class="legende">
        Hameaux : <span style="color: #e0725f">vital manquant</span> · <span style="color: #e0b04a">confort insuffisant</span> · <span style="color: #7ab648">tout couvert</span><br />
        Terres : production (rouge → vert)
      </div>
    </div>
    <template v-if="store.pret">
      <PanneauPop v-if="store.popDetail" />
      <PanneauRessource v-else-if="store.mode === 'ressource' && !store.selection" />
      <PanneauInfos v-else />
    </template>
    <aside v-else class="infos"></aside>
    <div class="bas" v-if="store.pret">
      <Journal />
      <Courbes />
    </div>
  </div>
</template>
