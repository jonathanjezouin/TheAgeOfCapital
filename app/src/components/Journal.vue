<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { dateJeu, traduire } from '../libelles.js'

const derniers = computed(() => store.evenements.slice(-60).reverse())

// Un événement mentionne souvent « (x, y) » : cliquer le sélectionne sur la carte.
function aller(texte) {
  const m = texte.match(/\((\d+), (\d+)\)/)
  if (m) store.selection = { x: +m[1], y: +m[2] }
}
const court = (j) => dateJeu(j).texte.replace(/^(\d+) (\S{3})\S*/, '$1 $2.')
</script>

<template>
  <section aria-label="Journal des événements">
    <h3 style="margin-top: 0">Journal</h3>
    <p v-if="!derniers.length" class="doux">Rien à signaler. Les chantiers apparaîtront ici avec leur motif.</p>
    <ul class="journal">
      <li v-for="(e, i) in derniers" :key="i" :title="traduire(e[1])" @click="aller(e[1])">
        <span class="quand">{{ court(e[0]) }}</span><span>{{ traduire(e[1]) }}</span>
      </li>
    </ul>
  </section>
</template>
