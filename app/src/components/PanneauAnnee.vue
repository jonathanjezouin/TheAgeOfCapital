<script setup>
import { computed } from 'vue'
import { ACTIVITES, POSTES, unites } from '../libelles.js'
const props = defineProps({ etat: Object })
const b = computed(() => props.etat.bilan)
const u = (q) => unites(q, props.etat.q)
const total = computed(() => Object.values(b.value.jours).reduce((a, v) => a + v, 0) || 1)
</script>

<template>
  <section>
    <h3>Année {{ b.annee }} <small>météo ×{{ (b.meteo_pm / 1000).toFixed(2) }}</small></h3>
    <div class="barre">
      <span v-for="p in POSTES" :key="p" :style="{ flex: b.jours[p], background: ACTIVITES[p].couleur }" :title="ACTIVITES[p].nom + ' : ' + b.jours[p] + ' j'"></span>
    </div>
    <ul class="legende">
      <li v-for="p in POSTES" :key="p" v-show="b.jours[p]"><i :style="{ background: ACTIVITES[p].couleur }"></i>{{ ACTIVITES[p].nom }} <b>{{ b.jours[p] }} j</b></li>
    </ul>
    <dl>
      <dt>Grain récolté (brut)</dt><dd>{{ u(b.grain_brut) }}</dd>
      <dt>dont prélevé</dt><dd>{{ u(b.grain_preleve) }}</dd>
      <dt>Grain mangé</dt><dd>{{ u(b.grain_mange) }}</dd>
      <dt>Bois coupé (brut / prélevé)</dt><dd>{{ u(b.bois_brut) }} / {{ u(b.bois_preleve) }}</dd>
      <dt>Bois brûlé</dt><dd>{{ u(b.bois_brule) }}</dd>
      <dt>Vêtements faits / usés</dt><dd>{{ b.vetements_faits }} / {{ u(b.vetement_use) }}</dd>
      <dt>Manques (grain, bois, vêt.)</dt><dd :class="{ bas: b.manque_grain + b.manque_bois + b.manque_vetement > 0 }">{{ u(b.manque_grain) }}, {{ u(b.manque_bois) }}, {{ u(b.manque_vetement) }}</dd>
    </dl>
  </section>
</template>
