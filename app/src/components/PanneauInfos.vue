<script setup>
import { computed } from 'vue'
import { store } from '../store.js'
import { TERRAINS_INFO, EAUX_INFO, RESSOURCES, TYPES, HABITATS, limitantTexte, pct, un, etatPop, couleurRatio, dateJeu, traduire } from '../libelles.js'

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

// Terrain et eau de la tile sélectionnée (2 octets par tile, voir terrain.js).
const sol = computed(() => {
  const s = store.selection, m = store.meta
  if (!s || !m || !store.terrain) return null
  const i = (s.y * m.largeur + s.x) * 2
  return { ...TERRAINS_INFO[store.terrain[i]], eau: EAUX_INFO[store.terrain[i + 1] & 3] }
})
const sansBatir = computed(() => !!sol.value && (sol.value.eau !== '' || ['Forêt', 'Plaine marécageuse'].includes(sol.value.nom)))

const pops = computed(() => {
  const en = entite.value, e = store.etat
  if (!en || !e) return []
  if (en.genre === 'hameau') return e.pops.filter((p) => p.residence === en.q)
  if (en.genre === 'terre' || (en.genre === 'chantier' && en.b != null)) return e.pops.filter((p) => p.emploi === en.b)
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
        (contour clair : reste chez elle aujourd'hui). <b>Cliquez une pop</b> pour voir où elle travaille, pourquoi, et ce qu'elle possède.
        En mode <i>Jour/nuit</i>, elles marchent sur les routes.</p>
      <p>Mode <b>Flux</b> : suivre une marchandise de sa production à sa consommation.</p>
      <p>Le <b>journal</b> en bas dit pourquoi le village s'étend : cliquez un événement pour aller sur sa tile.</p>
    </div>

    <template v-else>
      <small>Tile ({{ entite.x }}, {{ entite.y }}) · {{ sol?.eau || sol?.nom }} · fertilité {{ Math.round(entite.fert * 100) }} %</small>

      <!-- Terre libre / chemin -->
      <div v-if="entite.genre === 'vide'">
        <h2>{{ sol?.eau ? sol.eau : sansBatir ? sol.nom : 'Terre libre' }}</h2>
        <p v-if="sol?.eau" class="doux">Cours d'eau : il coupe le territoire, on ne le franchit pas encore (ponts à venir). Un point d'eau attire l'habitat.</p>
        <p v-else-if="sansBatir" class="doux">{{ sol.texte }} Pas encore constructible : il faudra défricher ou drainer.</p>
        <p v-else class="doux"><b>{{ sol?.nom }}</b>. {{ sol?.texte }} Un défrichement doit rester à portée d'un hameau ({{ store.meta.distance_max }} tiles de chemin au plus). Plus la terre est pauvre, moins elle produit.</p>
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
        <h3>Stock des pops du hameau</h3>
        <div class="doux">
          <span v-for="x in rations" :key="x.nom" style="margin-right: 10px">{{ RESSOURCES[x.nom] }} {{ un(x.stock, 0) }}</span>
        </div>
        <h3>Pops</h3>
        <div v-for="p in pops" :key="p.id" class="pop">
          <span class="pastille" :style="{ background: etatPop(p).couleur }"></span>
          <button class="lien" title="Inspecter cette pop" @click="store.selectionPop = p.id"><b>Pop {{ p.id }}</b></button>
          <span class="etiquette">{{ etatPop(p).texte }}</span><br />
          <small v-if="lieuDe(p.emploi)">aujourd'hui : {{ nomLieu(lieuDe(p.emploi)) }} ({{ lieuDe(p.emploi).x }}, {{ lieuDe(p.emploi).y }})</small>
          <small v-else>aujourd'hui : chez elle</small>
        </div>
        <p v-if="!pops.length" class="doux">Aucune pop n'y vit pour le moment.</p>
      </div>

      <!-- Terre exploitée -->
      <div v-else-if="entite.genre === 'terre'">
        <h2>{{ TYPES[entite.l.type_nom] ?? entite.l.type_nom }}</h2>
        <div class="ligne"><span>Production</span><span :style="{ color: couleurRatio(entite.l.prod) }">{{ pct(entite.l.prod) }}</span></div>
        <div class="barre"><i :style="{ width: entite.l.prod * 100 + '%', background: couleurRatio(entite.l.prod) }"></i></div>
        <div class="ligne"><span>Limité par</span><span>{{ limitantTexte(entite.l.limitant) }}</span></div>
        <div class="ligne"><span>Surface</span><span>{{ entite.l.emprise.length }} tiles ({{ un(entite.l.emprise.length * 6.25, 0) }} ha)</span></div>
        <div class="ligne"><span>Fertilité moyenne</span><span>{{ pct(entite.l.fertilite) }}</span></div>
        <div class="ligne"><span>Travail reçu (moy. 30 j) / requis</span><span>{{ un(entite.l.recu, 2) }} / {{ un(entite.l.postes, 2) }} pops</span></div>
        <div class="ligne"><span>Reçu aujourd'hui · encore réclamé</span><span>{{ un(entite.l.recu_jour, 2) }} · {{ un(entite.l.demande, 2) }} pop-jour</span></div>
        <div class="note" v-if="entite.l.limitant === 'main_oeuvre'">Cette terre réclame du travail que personne n'est venu faire : les pops vont d'abord vers ce qui leur manque pour vivre.</div>
        <div class="note" v-else-if="entite.l.limitant === 'stock_plein'">Rien à écouler : le stock de sortie est presque plein, personne n'a besoin de venir.</div>
        <h3>Stock de sortie</h3>
        <div class="doux">
          <template v-for="(q, r) in entite.l.sortie" :key="r"><span v-if="q > 0.05" style="margin-right: 10px">{{ RESSOURCES[nomRes(r)] }} {{ un(q, 0) }}</span></template>
        </div>
        <h3>Pops venues y travailler aujourd'hui</h3>
        <div v-for="p in pops" :key="p.id" class="pop">
          <button class="lien" @click="store.selectionPop = p.id"><b>Pop {{ p.id }}</b></button> <small>du hameau ({{ hameauDe(p.residence).x }}, {{ hameauDe(p.residence).y }}) · {{ un(p.travail, 2) }} pop-jour</small>
        </div>
        <p v-if="!pops.length" class="doux">Personne n'est venu aujourd'hui.</p>
      </div>

      <!-- Chantier -->
      <div v-else-if="entite.genre === 'chantier'">
        <h2>Chantier : {{ entite.chantier.cible === 'maison' ? `une maison de plus (${HABITATS[entite.chantier.type_nom].toLowerCase()})` : entite.chantier.cible === 'hameau' ? `bâtir un ${HABITATS[entite.chantier.type_nom].toLowerCase()}` : `défricher — ${(TYPES[entite.chantier.type_nom] ?? entite.chantier.type_nom).toLowerCase()}` }}</h2>
        <div class="ligne"><span>Avancement</span><span>{{ pct(entite.chantier.avancement) }}</span></div>
        <div class="barre"><i :style="{ width: entite.chantier.avancement * 100 + '%', background: 'var(--accent)' }"></i></div>
        <div class="ligne"><span>Équipe présente</span><span>{{ entite.chantier.presents }} / {{ entite.chantier.equipe }} pops</span></div>
        <div class="ligne"><span>Travail restant</span><span>{{ un(entite.chantier.restant_pop_jours, 0) }} pop-jours</span></div>
        <div class="ligne"><span>Limité par</span><span>{{ limitantTexte(entite.lieu.limitant) }}</span></div>
        <div class="note" v-if="motif">Pourquoi : {{ motif }}.</div>
        <div class="note" v-if="entite.lieu.limitant.startsWith('intrant')">Le chantier attend son matériau (bois) : il arrive par le réseau comme toute marchandise.</div>
        <h3>Équipe</h3>
        <div v-for="p in pops" :key="p.id" class="pop"><button class="lien" @click="store.selectionPop = p.id"><b>Pop {{ p.id }}</b></button> <small>du hameau ({{ hameauDe(p.residence).x }}, {{ hameauDe(p.residence).y }})</small></div>
        <p v-if="!pops.length" class="doux">Personne pour l'instant : le chantier recrutera à la prochaine saison (corvée si besoin).</p>
      </div>
      <small class="doux">{{ jours }}</small>
    </template>
  </aside>
</template>
