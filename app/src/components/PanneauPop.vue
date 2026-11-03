<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { RESSOURCES, TYPES, pct, un, couleurRatio } from '../libelles.js'

const d = computed(() => store.popDetail)
const nom = (r) => (RESSOURCES[r] ?? r).toLowerCase()
const type = (t) => TYPES[t] ?? t

// Ce que fait la pop aujourd'hui, et pourquoi.
const lieu = computed(() => (d.value.emploi >= 0 ? store.etat.lieux[d.value.emploi] : null))
const resume = computed(() => {
  const x = d.value
  if (x.motif === 'chantier') return `Elle fait partie de l'équipe du chantier en (${lieu.value.x}, ${lieu.value.y}) : un poste fixe.`
  if (x.motif === 'repos') return 'Aucune terre à sa portée ne réclame de bras aujourd\'hui : elle reste chez elle (travail domestique, temps libre).'
  if (x.motif === 'besoin') {
    const r = nom(x.ressource)
    if (x.classe === 'vital') return `Il lui reste peu de ${x.ressource === 'bois' ? 'bois de chauffe' : 'nourriture'} (${un(x.manque_jours, 1)} jours de moins que les ${un(x.cible_jours ?? d.value.cible_jours, 0)} qu'elle vise) : elle va là où l'on produit du ${r}.`
    if (x.classe === 'confort') return `Elle manque de ${r} pour son confort (${un(x.manque_jours, 1)} jours sous sa cible) : elle va là où l'on en produit.`
    if (x.classe === 'intrant') return `Les ateliers manquent de ${r} pour vêtir le hameau : elle va en produire.`
    return `Elle va là où il reste du travail à faire (${r}). Rien de pressant ne lui manque, ou les terres qui produisent ce qui lui manque n'ont plus besoin d'elle aujourd'hui.`
  }
  return 'Pas encore de décision.'
})
const statut = {
  choisi: { texte: 'choisie', couleur: 'var(--bon)' },
  moins_pressant: { texte: 'moins pressante', couleur: 'var(--moyen)' },
  complet: { texte: 'ne réclame pas assez de travail aujourd\'hui', couleur: 'var(--doux)' },
}
const classeTexte = { vital: 'vital', confort: 'confort', intrant: 'matière première', entretien: 'entretien' }
const rations = computed(() => d.value.rations)
const fermer = () => { store.selectionPop = null }
const voirLieu = (c) => { store.selectionPop = null; store.selection = { x: c.x, y: c.y } }
</script>

<template>
  <aside class="infos" aria-label="Informations sur une pop">
    <small><button class="lien" @click="fermer">← retour</button> · hameau ({{ d.x }}, {{ d.y }})</small>
    <h2>Pop {{ d.id }} <small class="doux">100 habitants</small></h2>

    <h3>Aujourd'hui</h3>
    <div class="ligne">
      <span>Lieu de travail</span>
      <span v-if="lieu">{{ type(lieu.type_nom) }} ({{ lieu.x }}, {{ lieu.y }})</span>
      <span v-else>chez elle</span>
    </div>
    <div class="ligne"><span>Travail fourni</span><span>{{ un(d.travail, 2) }} pop-jour <small>(moy. 30 j : {{ un(d.travail_moyen, 2) }})</small></span></div>
    <div class="note">{{ resume }}</div>

    <h3>Ce qu'elle possède et consomme</h3>
    <div class="ligne"><span>Nourriture</span><span>{{ un(d.nutrition_jours, 1) }} jours d'autonomie</span></div>
    <div class="ligne"><span>Chauffage</span><span>{{ un(d.chauffage_jours, 1) }} jours d'autonomie</span></div>
    <small class="doux">Cible : {{ un(d.cible_jours, 0) }} jours de réserve. Son stock est à elle : deux pops d'un même hameau peuvent être inégalement servies.</small>
    <table class="tab">
      <thead><tr><th>Ration</th><th>Stock</th><th>Tient</th><th>Mangé auj.</th></tr></thead>
      <tbody>
        <tr v-for="r in rations" :key="r.ressource">
          <td>{{ RESSOURCES[r.ressource] }} <small class="doux">{{ un(r.ration, 1) }}/j</small></td>
          <td>{{ un(r.stock, 1) }}</td>
          <td><span v-if="r.autonomie_jours != null" :style="{ color: couleurRatio(r.autonomie_jours / d.cible_jours) }">{{ un(r.autonomie_jours, 1) }} j</span></td>
          <td :style="{ color: couleurRatio(r.couverture) }">{{ pct(r.couverture) }}</td>
        </tr>
      </tbody>
    </table>

    <h3>Où elle pourrait travailler</h3>
    <small class="doux">Terres à moins de {{ un(store.meta.distance_max / 10, 0) }} tiles de chemin, de la plus utile à ce qui lui manque à la moins utile. Une terre ne réclame de bras que si son stock a de la place (au moins {{ un(d.seuil_deplacement, 1) }} pop-jour).</small>
    <div v-for="c in d.candidats" :key="c.lieu" class="cand" :class="{ choisi: c.statut === 'choisi' }">
      <b>{{ type(c.type_nom) }}</b> <small>({{ c.x }}, {{ c.y }}) · {{ un(c.distance, 1) }} tiles</small>
      <button class="lien" title="Voir cette terre" @click="voirLieu(c)">voir</button><br />
      <small>produit du {{ nom(c.ressource) }} · {{ classeTexte[c.classe] }}<template v-if="c.manque_jours > 0"> (manque {{ un(c.manque_jours, 1) }} j)</template>
        · réclame encore {{ un(c.demande, 2) }} pop-jour</small><br />
      <small :style="{ color: statut[c.statut].couleur }">● {{ statut[c.statut].texte }}</small>
    </div>
    <p v-if="!d.candidats.length" class="doux">Aucune terre à sa portée.</p>
    <small v-if="d.hors_portee.length" class="doux">{{ d.hors_portee.length }} terre(s) trop loin du hameau, hors de portée.</small>

    <h3>Santé et croissance</h3>
    <div class="ligne"><span>Besoins vitaux</span><span :style="{ color: couleurRatio(d.vital) }">{{ pct(d.vital) }}</span></div>
    <div class="ligne"><span>Confort</span><span :style="{ color: couleurRatio(d.confort) }">{{ pct(d.confort) }}</span></div>
    <div class="ligne"><span>Croissance</span><span>{{ d.taux_annuel >= 0 ? '+' : '' }}{{ un(d.taux_annuel, 1) }} %/an (jauge {{ un(d.jauge, 0) }} %)</span></div>
    <p class="doux"><small>À venir (champs prévus, sans effet pour l'instant) : richesse, santé, culture, strate, salaire et contrat.</small></p>
  </aside>
</template>
