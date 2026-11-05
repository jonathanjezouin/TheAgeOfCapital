<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { RESSOURCES, TYPES, pct, un, couleurRatio } from '../libelles.js'

const d = computed(() => store.ressourceDetail)
const type = (t) => TYPES[t] ?? t
const nomR = computed(() => (RESSOURCES[d.value.ressource] ?? d.value.ressource))
const voir = (x, y) => { store.selectionPop = null; store.selection = { x, y } }

// Expéditions du jour regroupées par couple (origine, destination).
const flux = computed(() => {
  const m = new Map()
  for (const f of d.value.flux) {
    const k = f.de + '>' + f.vers
    const e = m.get(k) ?? { ...f, qte: 0 }
    e.qte += f.qte
    m.set(k, e)
  }
  return [...m.values()].sort((a, b) => b.qte - a.qte)
})
const t = computed(() => d.value.totaux)
const nomLieu = (xy, genre) => `${genre === 'hameau' ? 'hameau' : 'lieu'} (${xy[0]}, ${xy[1]})`
const bilan = computed(() => {
  const x = t.value
  return x.cumul_produit - x.cumul_consomme_pops - x.cumul_consomme_prod - x.cumul_perime
})
</script>

<template>
  <aside class="infos" aria-label="Suivi d'une ressource">
    <h2>Flux de marchandises</h2>
    <div class="puces">
      <button v-for="(nom, r) in store.meta.ressources" :key="nom" :class="{ actif: store.ressource === r }" @click="store.ressource = r">{{ RESSOURCES[nom] ?? nom }}</button>
    </div>

    <template v-if="d">
      <h3>{{ nomR }} — aujourd'hui</h3>
      <div class="ligne"><span>Produit</span><span>{{ un(t.produit_jour, 1) }}</span></div>
      <div class="ligne"><span>Consommé par les pops</span><span>{{ un(t.consomme_pops_jour, 1) }}</span></div>
      <div class="ligne"><span>Expédié aujourd'hui</span><span>{{ un(flux.reduce((s, f) => s + f.qte, 0), 1) }}</span></div>

      <h3>Où se trouve le stock</h3>
      <div class="ligne"><span>Chez les producteurs</span><span>{{ un(t.stock_producteurs, 0) }}</span></div>
      <div class="ligne"><span>Chez les pops</span><span>{{ un(t.stock_pops, 0) }}</span></div>
      <div class="ligne"><span>Dans les ateliers et chantiers</span><span>{{ un(t.stock_intrants, 0) }}</span></div>
      <div class="ligne"><span>En route (retards)</span><span>{{ un(t.en_transit, 0) }}</span></div>

      <h3>Depuis le début</h3>
      <div class="ligne"><span>Produit</span><span>{{ un(t.cumul_produit, 0) }}</span></div>
      <div class="ligne"><span>Mangé ou brûlé par les pops</span><span>{{ un(t.cumul_consomme_pops, 0) }}</span></div>
      <div class="ligne"><span>Utilisé par les ateliers</span><span>{{ un(t.cumul_consomme_prod, 0) }}</span></div>
      <div class="ligne"><span>Périmé</span><span>{{ un(t.cumul_perime, 0) }}</span></div>
      <div class="ligne"><span>Reste en stock (ou en route)</span><span>{{ un(bilan, 0) }}</span></div>

      <h3>Qui en produit</h3>
      <div v-for="p in d.producteurs" :key="p.lieu" class="pop">
        <button class="lien" @click="voir(p.x, p.y)"><b>{{ type(p.type_nom) }}</b> ({{ p.x }}, {{ p.y }})</button><br />
        <small>aujourd'hui {{ un(p.produit_jour, 1) }} · stock de sortie {{ un(p.stock_sortie, 0) }}</small>
      </div>
      <p v-if="!d.producteurs.length" class="doux">Personne n'en produit ni n'en garde en stock.</p>

      <h3>Où ça va (aujourd'hui)</h3>
      <div v-for="(f, i) in flux" :key="i" class="pop">
        <small>{{ nomLieu(f.de, 'lieu') }} → {{ nomLieu(f.vers, f.vers_genre) }} : <b>{{ un(f.qte, 1) }}</b></small>
      </div>
      <p v-if="!flux.length" class="doux">Aucune expédition aujourd'hui.</p>

      <h3>Qui en consomme</h3>
      <div v-for="h in d.hameaux" :key="h.hameau" class="pop">
        <button class="lien" @click="voir(h.x, h.y)"><b>Hameau</b> ({{ h.x }}, {{ h.y }})</button> <small>{{ h.pops }} pops</small><br />
        <small>stock des pops {{ un(h.stock, 0) }} · mangé {{ un(h.consomme_jour, 1) }} · reçu {{ un(h.recu_jour, 1) }}</small>
        <div class="barre"><i :style="{ width: Math.min(100, h.couverture * 100) + '%', background: couleurRatio(h.couverture) }"></i></div>
      </div>
      <template v-if="d.utilisateurs.length">
        <h3>Ateliers et chantiers qui l'utilisent</h3>
        <div v-for="u in d.utilisateurs" :key="u.lieu" class="pop">
          <button class="lien" @click="voir(u.x, u.y)"><b>{{ type(u.type_nom) }}</b> ({{ u.x }}, {{ u.y }})</button>
          <small> stock {{ un(u.stock_entree, 1) }}</small>
        </div>
      </template>
    </template>
  </aside>
</template>
