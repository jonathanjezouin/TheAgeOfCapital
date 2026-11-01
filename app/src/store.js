import { reactive } from 'vue'

// État partagé entre l'interface (Vue) et la carte (PixiJS).
export const store = reactive({
  pret: false,
  erreur: null,
  meta: null,
  etat: null,
  carte: null,
  fertilite: null,
  evenements: [], // tous les événements reçus : [jour, texte]
  // Vitesse en jours de simulation par seconde réelle ; 1/24 = « jour/nuit » (un jour dure 24 s).
  vitesse: 10,
  enPause: false,
  mode: 'normal', // normal | fertilite | besoins
  selection: null, // { x, y }
  survol: null,
  heure: 12,
  fraction: 0,
  graine: 1,
  hameaux: 10,
})

export const VITESSES = [
  { jps: 1 / 24, label: 'Jour/nuit', titre: 'Un jour dure 24 s : on voit la nuit tomber et les pops rentrer chez elles' },
  { jps: 1, label: '×1', titre: '1 jour par seconde' },
  { jps: 10, label: '×10', titre: '10 jours par seconde' },
  { jps: 100, label: '×100', titre: '100 jours par seconde' },
  { jps: 1000, label: '×1000', titre: '1 000 jours par seconde (environ 3 ans)' },
]
