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
const vitaux = computed(() => d.value.rations.filter((r) => r.vital))
const confort = computed(() => d.value.rations.filter((r) => r.confort && !r.vital))
// Humeur : lue sur la couverture des besoins vitaux puis du confort.
const humeur = computed(() => {
  const x = d.value
  if (x.vital < 0.999) return { texte: 'en manque', couleur: 'var(--mauvais, #d65a4a)', phrase: 'Ses besoins vitaux ne sont pas couverts : la famille s\'inquiète, la population décline.' }
  if (x.confort < 0.999) return { texte: 'insatisfaite', couleur: 'var(--moyen)', phrase: 'Elle vit, mais le confort manque : la population ne croît que lentement.' }
  return { texte: 'satisfaite', couleur: 'var(--bon)', phrase: 'Tout est couvert : la population croît.' }
})
const impacts = computed(() => {
  const x = d.value, l = []
  const manques = (c) => x.rations.filter((r) => r[c] && r.manque_jour > 0.05).map((r) => nom(r.ressource))
  const mv = manques('vital'), mc = manques('confort')
  if (x.vital < 0.999) l.push(`Il manque ${mv.join(', ') || 'de quoi vivre'} : mortalité accrue (${un(x.taux_annuel, 1)} %/an).`)
  if (x.vital >= 0.999 && x.confort < 0.999) l.push(`Il manque ${mc.join(', ')} : croissance réduite à ${un(x.taux_annuel, 1)} %/an au lieu de +1,5 %.`)
  if (x.vital >= 0.999 && x.confort >= 0.999) l.push(`Tout est couvert : croissance +${un(x.taux_annuel, 1)} %/an. Quand la jauge est pleine (${un(x.jauge, 0)} %), un enfant naît : il faut un logement.`)
  if (x.motif === 'besoin') l.push('Ce qui lui manque la pousse vers un lieu de travail qui le produit.')
  return l
})
const fermer = () => { store.selectionPop = null }
const voirLieu = (c) => { store.selectionPop = null; store.selection = { x: c.x, y: c.y } }
</script>

<template>
  <aside class="infos" aria-label="Informations sur une pop">
    <small><button class="lien" @click="fermer">← retour</button> · hameau ({{ d.x }}, {{ d.y }})</small>
    <h2>Pop {{ d.id }} <small class="doux">100 habitants</small></h2>

    <h3>Qui est-elle</h3>
    <div class="ligne"><span>Statut</span><span>{{ d.statut === 'serf' ? 'serve' : 'libre' }} <small class="doux">· ne décide ni des règles ni des constructions</small></span></div>
    <div class="ligne"><span>Seigneur</span><span>{{ d.seigneur }} <small class="doux">· champart {{ pct(d.champart) }} de la récolte</small></span></div>
    <div class="ligne"><span>État d'esprit</span><span :style="{ color: humeur.couleur }"><b>{{ humeur.texte }}</b></span></div>
    <div class="note">{{ humeur.phrase }}</div>

    <h3>Aujourd'hui</h3>
    <div class="ligne">
      <span>Lieu de travail</span>
      <span v-if="lieu">{{ type(lieu.type_nom) }} ({{ lieu.x }}, {{ lieu.y }})</span>
      <span v-else>chez elle</span>
    </div>
    <div class="ligne"><span>Travail fourni</span><span>{{ un(d.travail, 2) }} pop-jour <small>(moy. 30 j : {{ un(d.travail_moyen, 2) }})</small></span></div>
    <div class="note">{{ resume }}</div>

    <h3>Ce qu'elle possède</h3>
    <div class="ligne"><span>Nourriture en réserve</span><span>{{ un(d.nutrition_jours, 1) }} jours</span></div>
    <div class="ligne"><span>Chauffage en réserve</span><span>{{ un(d.chauffage_jours, 1) }} jours</span></div>
    <small class="doux">Elle vise {{ un(d.cible_jours, 0) }} jours de réserve. Le stock est à elle : deux pops d'un même hameau peuvent être inégalement servies.</small>

    <h3>Pour vivre (besoins vitaux)</h3>
    <table class="tab">
      <thead><tr><th>Il lui faut</th><th>Par jour</th><th>Stock</th><th>Couvert</th></tr></thead>
      <tbody>
        <tr v-for="r in vitaux" :key="r.ressource">
          <td>{{ RESSOURCES[r.ressource] }}</td><td>{{ un(r.ration, 1) }}</td><td>{{ un(r.stock, 1) }} <small class="doux">({{ r.autonomie_jours != null ? un(r.autonomie_jours, 0) + ' j' : '—' }})</small></td>
          <td :style="{ color: couleurRatio(r.couverture) }">{{ pct(r.couverture) }}</td>
        </tr>
      </tbody>
    </table>
    <small class="doux">Nourriture : le grain, la viande et les fruits sont interchangeables (5 unités nutritives par jour). Couverture vitale : {{ pct(d.vital) }}.</small>

    <h3>Pour son confort</h3>
    <table class="tab">
      <thead><tr><th>Il lui faut</th><th>Par jour</th><th>Stock</th><th>Couvert</th></tr></thead>
      <tbody>
        <tr v-for="r in confort" :key="r.ressource">
          <td>{{ RESSOURCES[r.ressource] }}</td><td>{{ un(r.ration, 1) }}</td><td>{{ un(r.stock, 1) }}</td>
          <td :style="{ color: couleurRatio(r.couverture) }">{{ pct(r.couverture) }}</td>
        </tr>
        <tr v-if="rations.find((r) => r.ressource === 'viande')"><td colspan="4"><small class="doux">La viande compte aussi pour le confort (une part de viande dans l'alimentation).</small></td></tr>
      </tbody>
    </table>
    <small class="doux">Couverture du confort : {{ pct(d.confort) }}.</small>

    <h3>Ce que cela change</h3>
    <div class="note" v-for="(t, i) in impacts" :key="i">{{ t }}</div>

    <h3>Où elle pourrait travailler</h3>
    <div class="ligne"><span>Accès</span><span>{{ d.candidats.length }} lieu(x) à portée · {{ d.hors_portee.length }} hors de portée</span></div>
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
