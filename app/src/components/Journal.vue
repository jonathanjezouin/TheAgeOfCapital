<script setup>
import { computed } from 'vue'
import { dateCourte } from '../libelles.js'
const props = defineProps({ etat: Object })
const lignes = computed(() => [...props.etat.journal].reverse().slice(0, 25))
</script>

<template>
  <section>
    <h3>Journal</h3>
    <ul class="journal"><li v-for="(e, i) in lignes" :key="i"><time>{{ dateCourte(e.jour) }}</time> {{ e.texte }}</li></ul>
  </section>
</template>
