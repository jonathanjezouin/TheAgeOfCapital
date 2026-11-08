<script setup>
import { computed } from 'vue'
import { ACTIVITES, unites } from '../libelles.js'
const props = defineProps({ etat: Object })
const a = computed(() => ACTIVITES[props.etat.pop.activite])
const u = (q) => unites(q)
const s = computed(() => props.etat.pop.stocks)
</script>

<template>
  <section>
    <h3>La pop</h3>
    <div class="act" :style="{ borderColor: a.couleur }">
      <b :style="{ color: a.couleur }">{{ a.icone }} {{ a.nom }}</b>
      <span>{{ etat.pop.raison }}</span>
    </div>
    <div class="stocks">
      <div><label>Grain</label><b>{{ u(s.grain) }}</b></div>
      <div :class="{ bas: s.bois < etat.pop.seuil_bois }"><label>Bois</label><b>{{ u(s.bois) }}</b></div>
      <div :class="{ bas: s.vetement < etat.pop.seuil_vetement }"><label>Vêtements</label><b>{{ u(s.vetement) }}</b></div>
    </div>
    <p class="aide">Chaque jour : dimanche repos ; sinon terre (culture, puis transport du grain) ; sinon bois si stock sous le seuil ; sinon couture si moins de 2 vêtements ; sinon chômage.</p>
  </section>
</template>
