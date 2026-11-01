<script setup>
import { computed } from 'vue'
import { store, VITESSES } from '../store.js'
import { dateJeu } from '../libelles.js'

const emit = defineEmits(['pas', 'nouveau'])
const date = computed(() => {
  const j = store.etat ? store.etat.jour : 0
  return dateJeu(j).texte
})
const habitants = computed(() => (store.etat ? store.etat.nb_pops * (store.meta?.habitants_par_pop ?? 100) : 0))
const MODES = [
  { id: 'normal', label: 'Normal' },
  { id: 'fertilite', label: 'Fertilité' },
  { id: 'besoins', label: 'Besoins' },
]
</script>

<template>
  <header class="haut">
    <span class="titre">Village 1750</span>
    <span class="date" aria-live="off">{{ date }}</span>

    <span class="groupe" role="group" aria-label="Vitesse de simulation">
      <button :class="{ actif: store.enPause }" title="Pause (espace)" @click="store.enPause = !store.enPause">
        {{ store.enPause ? '▶' : '⏸' }}
      </button>
      <button title="Avancer d'un jour" @click="emit('pas')">+1 j</button>
      <button
        v-for="v in VITESSES" :key="v.jps" :class="{ actif: !store.enPause && store.vitesse === v.jps }"
        :title="v.titre" @click="store.vitesse = v.jps; store.enPause = false"
      >{{ v.label }}</button>
    </span>

    <span class="groupe" role="group" aria-label="Mode de carte">
      <label>Carte</label>
      <button v-for="m in MODES" :key="m.id" :class="{ actif: store.mode === m.id }" @click="store.mode = m.id">{{ m.label }}</button>
    </span>

    <span class="stat">Population <b>{{ store.etat?.nb_pops ?? 0 }}</b> pops ({{ habitants.toLocaleString('fr-FR') }} hab.)</span>
    <span class="stat">Sans poste <b>{{ store.etat?.sans_poste ?? 0 }}</b></span>
    <span class="stat">Chantiers <b>{{ store.etat?.chantiers ?? 0 }}</b></span>

    <span class="groupe">
      <label for="graine">Graine</label>
      <input id="graine" v-model.number="store.graine" type="number" min="0" step="1" />
      <label for="hameaux">Hameaux</label>
      <input id="hameaux" v-model.number="store.hameaux" type="number" min="4" max="24" step="1" />
      <button title="Génère un nouveau village avec cette graine" @click="emit('nouveau')">Nouveau</button>
    </span>
  </header>
</template>
