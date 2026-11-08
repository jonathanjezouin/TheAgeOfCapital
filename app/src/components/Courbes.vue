<script setup>
import { computed } from 'vue'
const props = defineProps({ etat: Object })
const ANNEES = 8
const W = 300, H = 110
const pts = computed(() => {
  const fin = props.etat.jours_ecoules
  const debut = Math.max(0, fin - ANNEES * 365)
  return props.etat.serie.filter((p) => p.jour >= debut)
})
const max = computed(() => Math.max(10, ...pts.value.map((p) => Math.max(p.grain, p.bois, p.vetement))))
const chemin = (cle) => {
  const p = pts.value
  if (p.length < 2) return ''
  const j0 = p[0].jour, j1 = p[p.length - 1].jour || 1
  return p.map((v, i) => `${i ? 'L' : 'M'}${((v.jour - j0) / Math.max(1, j1 - j0)) * W},${H - (v[cle] / max.value) * H}`).join(' ')
}
const COULEURS = { grain: '#e0b43a', bois: '#3f8f5a', vetement: '#8d5fb0' }
</script>

<template>
  <section>
    <h3>Stocks à la ferme <small>{{ ANNEES }} dernières années</small></h3>
    <svg :viewBox="`0 0 ${W} ${H}`" class="courbe" preserveAspectRatio="none">
      <path v-for="(c, k) in COULEURS" :key="k" :d="chemin(k)" :stroke="c" fill="none" stroke-width="1.6" vector-effect="non-scaling-stroke" />
    </svg>
    <ul class="legende"><li v-for="(c, k) in COULEURS" :key="k"><i :style="{ background: c }"></i>{{ { grain: 'Grain', bois: 'Bois', vetement: 'Vêtements' }[k] }}</li><li>max {{ max }}</li></ul>
  </section>
</template>
