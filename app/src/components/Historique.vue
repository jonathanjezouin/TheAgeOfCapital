<script setup>
import { computed } from 'vue'
import { unites } from '../libelles.js'
const props = defineProps({ etat: Object })
const u = (q) => unites(q, props.etat.q, 1)
const travailles = (j) => j.labour + j.semis + j.recolte + j.transport_grain + j.coupe + j.transport_bois + j.vetement
const lignes = computed(() => props.etat.historique.slice(-12).reverse())
</script>

<template>
  <section>
    <h3>Années écoulées</h3>
    <p v-if="!lignes.length" class="aide">Aucune année terminée.</p>
    <table v-else>
      <thead><tr><th>An</th><th>Météo</th><th>Grain net</th><th>Mangé</th><th>Stock fin</th><th>Jours trav.</th></tr></thead>
      <tbody>
        <tr v-for="b in lignes" :key="b.annee" :class="{ bas: b.manque_grain > 0 }">
          <td>{{ b.annee }}</td><td>×{{ (b.meteo_pm / 1000).toFixed(2) }}</td>
          <td>{{ u(b.grain_brut - b.grain_preleve) }}</td><td>{{ u(b.grain_mange) }}</td>
          <td>{{ u(b.stock_fin.grain) }}</td>
          <td>{{ travailles(b.jours) }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>
