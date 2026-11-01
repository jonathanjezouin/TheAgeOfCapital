<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { RESSOURCES, TYPES, HABITATS, limitantTexte, pct, un, etatPop, couleurRatio, dateJeu, traduire } from '../libelles.js'

const nomRes = (r) => store.meta.ressources[r]

// Ce qu'il y a sur la tile sélectionnée.
const entite = computed(() => {
  const s = store.selection, e = store.etat, g = store.carte?.grille, m = store.meta
  if (!s || !e || !g || !m) return null
  const fert = (store.fertilite[s.y * m.largeur + s.x] ?? 0) / 100
  const t = g.get(s.y * m.largeur + s.x)
  if (!t) return { genre: 'vide', ...s, fert }
  if (t.c === 'chemin') return { genre: 'chemin', ...s, fert }
  if (t.c === 'hameau') {
    const h = e.hameaux[t.m]
    const chantier = e.lieux.find((l) => l.chantier && l.x === s.x && l.y === s.y)
    return { genre: chantier && h.capacite === 0 ? 'chantier' : 'hameau', q: t.m, h, chantier: chantier?.chantier, lieu: chantier, b: chantier ? e.lieux.indexOf(chantier) : undefined, ...s, fert }
  }
  const l = e.lieux[t.m]
  return { genre: l.chantier ? 'chantier' : 'terre', b: t.m, l, chantier: l.chantier, lieu: l, ...s, fert }
})

const pops = computed(() => {
  const en = entite.value, e = store.etat
  if (!en || !e) return []
  if (en.genre === 'hameau') return e.pops.filter((p) => p[1] === en.q)
  if (en.genre === 'terre' || (en.genre === 'chantier' && en.b != null)) return e.pops.filter((p) => p[2] === en.b)
  return []
})

const lieuDe = (emploi) => (emploi >= 0 ? store.etat.lieux[emploi] : null)
const nomLieu = (l) => (l.quartier != null ? 'Atelier du hameau' : (TYPES[l.type_nom] ?? l.type_nom))
const hameauDe = (i) => store.etat.hameaux[i]

// Motif du lancement d'un chantier : on le relit dans le journal.
const motif = computed(() => {
  const en = entite.value
  if (!en || en.genre !== 'chantier') return null
  const cle = `(${en.x}, ${en.y})`
  const ev = [...store.evenements].reverse().find((x) => x[1].startsWith('Chantier lancé') && x[1].includes(cle))
  return ev ? traduire(ev[1].split(' — ')[1] ?? '') : null
})

const rations = computed(() => {
  const en = entite.value
  if (!en?.h) return []
  return store.meta.ressources
    .map((nom, r) => ({ nom, r, ration: store.meta.rations[r], couv: en.h.couverture[r], stock: en.h.stock[r] }))
    .filter((x) => x.ration > 0)
})

// Croissance annuelle d'une pop d'après la couverture du hameau (mêmes règles que le moteur).
const croissance = computed(() => {
  const h = entite.value?.h
  if (!h) return null
  if (h.vital >= 0.999) return 0.5 + 1.0 * h.confort
  return 0.5 - 8.5 * (1 - h.vital)
})
const jours = computed(() => (store.etat ? dateJeu(store.etat.jour).texte : ''))
</script>

<template>
  <aside class="infos" aria-label="Informations sur la tile sélectionnée">
    <div v-if="!entite" class="aide">
      <h2>Le village de 1750</h2>
      <p>Cliquez une tile pour voir ce qu'elle contient : <b>hameau</b>, <b>terre exploitée</b>, <b>chantier</b>, chemin, terre libre.</p>
      <p><kbd>molette</kbd> zoom · <kbd>glisser</kbd> déplacer · <kbd>espace</kbd> pause.</p>
      <p>Chaque point est une <b>pop</b> (100 habitants) :
        <span class="pastille" style="background: #86c250"></span>satisfaite
        <span class="pastille" style="background: #e0a03a"></span>insatisfaite
        <span class="pastille" style="background: #d65a4a"></span>en manque
        <span class="pastille" style="background: #4a4a48; border-color: #d9d4c4"></span>sans poste.
        En mode <i>Jour/nuit</i>, elles rentrent chez elles le soir.</p>
      <p>Le <b>journal</b> en bas dit pourquoi le village s'étend : cliquez un événement pour aller sur sa tile.</p>
    </div>

    <template v-else>
      <small>Tile ({{ entite.x }}, {{ entite.y }}) · fertilité {{ Math.round(entite.fert * 100) }} %</small>

      <!-- Terre libre / chemin -->
      <div v-if="entite.genre === 'vide'">
        <h2>Terre libre</h2>
        <p class="doux">Plaine, exploitable : un défrichement doit rester à portée d'un hameau ({{ store.meta.distance_max }} tiles de chemin au plus). Plus la terre est pauvre, moins elle produit.</p>
      </div>
      <div v-else-if="entite.genre === 'chemin'">
        <h2>Chemin de terre</h2>
        <p class="doux">Distance = nombre de tiles de chemin empruntées ; aucune pénalité, mais on ne va ni travailler ni s'approvisionner au-delà de {{ store.meta.distance_max }}.</p>
      </div>

      <!-- Hameau -->
      <div v-else-if="entite.genre === 'hameau'">
        <h2>{{ HABITATS[entite.h.habitat] }}</h2>
        <div class="ligne"><span>Pops</span><span>{{ entite.h.pops }} / {{ entite.h.capacite }} ({{ entite.h.pops * store.meta.habitants_par_pop }} hab.)</span></div>
        <div class="ligne"><span>Besoins vitaux</span><span :style="{ color: couleurRatio(entite.h.vital) }">{{ pct(entite.h.vital) }}</span></div>
        <div class="ligne"><span>Confort</span><span :style="{ color: couleurRatio(entite.h.confort) }">{{ pct(entite.h.confort) }}</span></div>
        <div class="ligne"><span>Croissance d'une pop</span><span>{{ croissance >= 0 ? '+' : '' }}{{ un(croissance, 1) }} %/an</span></div>
        <div class="note" v-if="entite.h.vital < 0.999">Besoins vitaux non couverts (nourriture ou chauffage) : la population décline.</div>
        <div class="note" v-else-if="entite.h.confort < 0.999">Vivre est assuré, mais il manque de la viande ou des vêtements : +0,5 %/an seulement.</div>
        <div class="note" v-else>Tout est couvert : +1,5 %/an. Si le hameau est plein, une pop qui attend un logement fera bâtir un hameau.</div>

        <h3>Rations du jour (reçu / requis)</h3>
        <div v-for="x in rations" :key="x.nom" class="rang">
          <span>{{ RESSOURCES[x.nom] }}</span>
          <div class="barre"><i :style="{ width: Math.min(100, x.couv * 100) + '%', background: couleurRatio(x.couv) }"></i></div>
          <span class="val">{{ pct(x.couv) }}</span>
        </div>
        <h3>Stock du hameau</h3>
        <div class="doux">
          <span v-for="x in rations" :key="x.nom" style="margin-right: 10px">{{ RESSOURCES[x.nom] }} {{ un(x.stock, 0) }}</span>
        </div>
        <h3>Pops</h3>
        <div v-for="p in pops" :key="p[0]" class="pop">
          <span class="pastille" :style="{ background: etatPop(entite.h).couleur }"></span><b>Pop {{ p[0] }}</b>
          <span class="etiquette">{{ etatPop(entite.h).texte }}</span><br />
          <small v-if="lieuDe(p[2])">travaille : {{ nomLieu(lieuDe(p[2])) }} ({{ lieuDe(p[2]).x }}, {{ lieuDe(p[2]).y }})</small>
          <small v-else>sans poste : elle consomme mais ne produit pas</small>
        </div>
        <p v-if="!pops.length" class="doux">Aucune pop n'y vit pour le moment.</p>
      </div>

      <!-- Terre exploitée -->
      <div v-else-if="entite.genre === 'terre'">
        <h2>{{ TYPES[entite.l.type_nom] ?? entite.l.type_nom }}</h2>
        <div class="ligne"><span>Production</span><span :style="{ color: couleurRatio(entite.l.prod) }">{{ pct(entite.l.prod) }}</span></div>
        <div class="barre"><i :style="{ width: entite.l.prod * 100 + '%', background: couleurRatio(entite.l.prod) }"></i></div>
        <div class="ligne"><span>Limité par</span><span>{{ limitantTexte(entite.l.limitant) }}</span></div>
        <div class="ligne"><span>Fertilité de la tile</span><span>{{ pct(entite.l.fertilite) }}</span></div>
        <div class="ligne"><span>Effectif</span><span>{{ entite.l.occ }} / {{ entite.l.postes }} pops</span></div>
        <div class="note" v-if="entite.l.limitant === 'main_oeuvre'">Postes vacants : les naissances et la réorientation mensuelle des bras les rempliront.</div>
        <div class="note" v-else-if="entite.l.limitant === 'stock_plein'">Rien à écouler : des pops pourront être réorientées vers ce qui manque ailleurs.</div>
        <h3>Stock de sortie</h3>
        <div class="doux">
          <template v-for="(q, r) in entite.l.sortie" :key="r"><span v-if="q > 0.05" style="margin-right: 10px">{{ RESSOURCES[nomRes(r)] }} {{ un(q, 0) }}</span></template>
        </div>
        <h3>Pops affectées</h3>
        <div v-for="p in pops" :key="p[0]" class="pop">
          <b>Pop {{ p[0] }}</b> <small>du hameau ({{ hameauDe(p[1]).x }}, {{ hameauDe(p[1]).y }})</small>
        </div>
        <p v-if="!pops.length" class="doux">Personne n'y travaille.</p>
      </div>

      <!-- Chantier -->
      <div v-else-if="entite.genre === 'chantier'">
        <h2>Chantier : {{ entite.chantier.cible === 'hameau' ? `bâtir un ${HABITATS[entite.chantier.type_nom].toLowerCase()}` : `défricher — ${(TYPES[entite.chantier.type_nom] ?? entite.chantier.type_nom).toLowerCase()}` }}</h2>
        <div class="ligne"><span>Avancement</span><span>{{ pct(entite.chantier.avancement) }}</span></div>
        <div class="barre"><i :style="{ width: entite.chantier.avancement * 100 + '%', background: 'var(--accent)' }"></i></div>
        <div class="ligne"><span>Équipe présente</span><span>{{ entite.chantier.presents }} / {{ entite.chantier.equipe }} pops</span></div>
        <div class="ligne"><span>Travail restant</span><span>{{ un(entite.chantier.restant_pop_jours, 0) }} pop-jours</span></div>
        <div class="ligne"><span>Limité par</span><span>{{ limitantTexte(entite.lieu.limitant) }}</span></div>
        <div class="note" v-if="motif">Pourquoi : {{ motif }}.</div>
        <div class="note" v-if="entite.lieu.limitant.startsWith('intrant')">Le chantier attend son matériau (bois) : il arrive par le réseau comme toute marchandise.</div>
        <h3>Équipe</h3>
        <div v-for="p in pops" :key="p[0]" class="pop"><b>Pop {{ p[0] }}</b> <small>du hameau ({{ hameauDe(p[1]).x }}, {{ hameauDe(p[1]).y }})</small></div>
        <p v-if="!pops.length" class="doux">Personne pour l'instant : le chantier recrutera à la prochaine saison (corvée si besoin).</p>
      </div>
      <small class="doux">{{ jours }}</small>
    </template>
  </aside>
</template>
