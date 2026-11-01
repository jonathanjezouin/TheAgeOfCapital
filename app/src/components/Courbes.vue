<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { RESSOURCES, un } from '../libelles.js'

const W = 420, H = 96, M = 22
const serie = computed(() => store.etat?.annuel ?? [])
const bornes = computed(() => {
  const s = serie.value
  const max = Math.max(10, ...s.map((p) => p[1]))
  return { max, n: Math.max(1, s.length - 1) }
})
const chemin = (i) => {
  const s = serie.value
  if (s.length < 2) return ''
  const { max, n } = bornes.value
  return s.map((p, k) => `${k ? 'L' : 'M'}${(M + (k / n) * (W - M - 4)).toFixed(1)},${(H - 14 - (p[i] / max) * (H - 24)).toFixed(1)}`).join(' ')
}

const flux = computed(() => {
  const e = store.etat
  if (!e || !store.meta) return []
  return store.meta.ressources
    .map((nom, r) => ({ nom, produit: e.produit[r], consomme: e.consomme[r] }))
    .filter((x) => x.produit > 0 || x.consomme > 0)
})
const max = computed(() => Math.max(1, ...flux.value.flatMap((x) => [x.produit, x.consomme])))
</script>

<template>
  <section aria-label="Population">
    <h3 style="margin-top: 0">Population par année</h3>
    <svg :viewBox="`0 0 ${W} ${H}`" width="100%" height="110" role="img" aria-label="Courbe de la population (pops) et des pops sans poste">
      <line :x1="M" :y1="H - 14" :x2="W - 4" :y2="H - 14" stroke="var(--bord)" />
      <line :x1="M" :y1="10" :x2="M" :y2="H - 14" stroke="var(--bord)" />
      <text :x="M - 4" y="16" text-anchor="end" font-size="10" fill="var(--doux)">{{ bornes.max }}</text>
      <text :x="M - 4" :y="H - 12" text-anchor="end" font-size="10" fill="var(--doux)">0</text>
      <text :x="W - 4" :y="H - 2" text-anchor="end" font-size="10" fill="var(--doux)">{{ serie.length ? `an ${serie.length}` : '' }}</text>
      <path :d="chemin(1)" fill="none" stroke="var(--accent)" stroke-width="2" />
      <path :d="chemin(2)" fill="none" stroke="var(--mauvais)" stroke-width="1.5" />
    </svg>
    <small><span class="pastille" style="background: var(--accent)"></span>pops
      <span class="pastille" style="background: var(--mauvais); margin-left: 10px"></span>sans poste</small>
  </section>
  <section aria-label="Production et consommation">
    <h3 style="margin-top: 0">Dernier jour : produit / consommé</h3>
    <div v-for="x in flux" :key="x.nom" class="rang" style="grid-template-columns: 5.2em 1fr 6.6em">
      <span>{{ RESSOURCES[x.nom] ?? x.nom }}</span>
      <span>
        <div class="barre" style="height: 6px"><i :style="{ width: (x.produit / max) * 100 + '%', background: 'var(--bon)' }"></i></div>
        <div class="barre" style="height: 6px; margin-top: 2px"><i :style="{ width: (x.consomme / max) * 100 + '%', background: 'var(--moyen)' }"></i></div>
      </span>
      <span class="val doux">{{ un(x.produit, 0) }} / {{ un(x.consomme, 0) }}</span>
    </div>
    <small><span class="pastille" style="background: var(--bon)"></span>produit
      <span class="pastille" style="background: var(--moyen); margin-left: 10px"></span>consommé par les pops</small>
  </section>
</template>
