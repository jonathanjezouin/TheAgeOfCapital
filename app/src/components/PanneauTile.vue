<script setup>
import { computed } from 'vue'
import { PHASES, ACTIVITES, MOIS, unites } from '../libelles.js'
const props = defineProps({ etat: Object, choix: Number })
const t = computed(() => (props.choix != null ? props.etat.tiles[props.choix] : null))
const u = (q) => unites(q, props.etat.q)
const r = computed(() => props.etat.regles)
const surPlace = computed(() => props.etat.pop.tile === props.choix && props.etat.pop.activite !== 'aucune')
const cycle = [['nu', 'Labour', 'oct.-nov.'], ['laboure', 'Semis', 'mars-avr.'], ['seme', 'Récolte', 'juil.-août']]
</script>

<template>
  <section v-if="t">
    <h3>Tile ({{ t.x }}, {{ t.y }}) — {{ { plaine: 'Plaine libre', ferme: 'Ferme', champ: 'Champ', foret: 'Forêt' }[t.type] }}</h3>
    <p v-if="surPlace" class="pop">La pop est ici aujourd'hui : {{ ACTIVITES[etat.pop.activite].nom.toLowerCase() }}.</p>

    <template v-if="t.type === 'plaine'">
      <p>Terre sans usage. Rien n'y est produit, rien n'y est consommé.</p>
      <dl><dt>Distance à la ferme</dt><dd>{{ t.distance }} tiles</dd></dl>
    </template>

    <template v-else-if="t.type === 'ferme'">
      <p>La maison de la pop. Les récoltes y sont portées, on y consomme et on y coud.</p>
      <dl>
        <dt>Grain</dt><dd>{{ u(etat.pop.stocks.grain) }}</dd>
        <dt>Bois</dt><dd>{{ u(etat.pop.stocks.bois) }} <small>(seuil {{ u(etat.pop.seuil_bois) }})</small></dd>
        <dt>Vêtements</dt><dd>{{ u(etat.pop.stocks.vetement) }} <small>(seuil {{ u(etat.pop.seuil_vetement) }})</small></dd>
        <dt>Couture en cours</dt><dd>{{ etat.pop.travail_vetement }} / {{ r.vetement_j }} j</dd>
      </dl>
      <p class="aide">Chaque mois : 1 grain mangé, 1 bois brûlé de novembre à mars, 20 % de chances qu'un vêtement s'use. Sous le seuil, la pop travaille pour reconstituer le stock.</p>
    </template>

    <template v-else-if="t.type === 'champ'">
      <p>Cycle annuel :</p>
      <ol class="cycle">
        <li v-for="c in cycle" :key="c[0]" :class="{ actif: t.phase === c[0] }"><b>{{ c[1] }}</b><span>{{ c[2] }}</span></li>
      </ol>
      <dl>
        <dt>État</dt><dd>{{ PHASES[t.phase].nom }}</dd>
        <dt>Travail fait</dt><dd>{{ t.travail }} / {{ t.travail_requis }} j <small>({{ PHASES[t.phase].fenetre }})</small></dd>
        <dt>Distance à la ferme</dt><dd>{{ t.distance }} tiles</dd>
        <dt>Grain à porter</dt><dd>{{ u(t.grain) }}</dd>
        <dt>Coût de transport</dt><dd>{{ t.distance * r.k_grain }} j par grain</dd>
      </dl>
      <p class="aide">À la fin de la récolte : 3 grains (la météo peut en doubler ou en faire perdre), dont {{ r.prelevement_pct }} % prélevés par le seigneur. Le reste attend dans le champ d'être transporté. Une étape non faite à temps fait retomber le champ en friche.</p>
    </template>

    <template v-else-if="t.type === 'foret'">
      <p>Forêt exploitée de façon soutenable : jamais rasée, elle repousse au 1ᵉʳ janvier.</p>
      <dl>
        <dt>Bois coupable cette année</dt><dd>{{ u(t.disponible) }} / 3</dd>
        <dt>Coupe en cours</dt><dd>{{ t.travail }} / {{ t.travail_requis }} j</dd>
        <dt>Bois coupé à porter</dt><dd>{{ u(t.coupe) }}</dd>
        <dt>Distance à la ferme</dt><dd>{{ t.distance }} tiles</dd>
        <dt>Coût de transport</dt><dd>{{ t.distance * r.k_bois }} j par unité</dd>
      </dl>
      <p class="aide">Une unité = {{ r.coupe_j_par_unite }} j de coupe, puis transport. {{ r.prelevement_pct }} % sont prélevés par le seigneur à la coupe.</p>
    </template>
  </section>
  <section v-else><h3>Sélection</h3><p class="aide">Clique une tile de la carte pour la détailler.</p></section>
</template>
