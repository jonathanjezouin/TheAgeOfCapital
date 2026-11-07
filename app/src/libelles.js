// Libellés et formats partagés par l'interface.
export const MOIS = ['janvier', 'février', 'mars', 'avril', 'mai', 'juin', 'juillet', 'août', 'septembre', 'octobre', 'novembre', 'décembre']

export const ACTIVITES = {
  aucune: { nom: 'Pas encore commencé', couleur: '#777', icone: '·' },
  repos: { nom: 'Repos dominical', couleur: '#8a8f99', icone: '✝' },
  labour: { nom: 'Labour', couleur: '#a9794a', icone: '⛏' },
  semis: { nom: 'Semis', couleur: '#8fb04a', icone: '🌱' },
  recolte: { nom: 'Récolte', couleur: '#e0b43a', icone: '🌾' },
  transport_grain: { nom: 'Transport du grain', couleur: '#c98f3c', icone: '🎒' },
  coupe: { nom: 'Coupe du bois', couleur: '#3f8f5a', icone: '🪓' },
  transport_bois: { nom: 'Transport du bois', couleur: '#2f7a7a', icone: '🪵' },
  vetement: { nom: 'Couture', couleur: '#8d5fb0', icone: '🧵' },
  chomage: { nom: 'Chômage supplémentaire', couleur: '#aab0a0', icone: '…' },
}

// Ordre d'affichage des postes de la ligne « jours » d'un bilan.
export const POSTES = ['labour', 'semis', 'recolte', 'transport_grain', 'coupe', 'transport_bois', 'vetement', 'chomage', 'repos']

export const PHASES = {
  nu: { nom: 'Nu (à labourer)', fenetre: 'octobre-novembre' },
  laboure: { nom: 'Labouré (à semer)', fenetre: 'mars-avril' },
  seme: { nom: 'Semé (à récolter)', fenetre: 'juillet-août' },
}

export const unites = (q, Q, d = 2) => {
  const v = q / Q
  return (Math.abs(v - Math.round(v)) < 1e-9 ? String(Math.round(v)) : v.toFixed(d)).replace('.', ',')
}

export const dateCourte = (jour) => {
  // jour = numéro de jour depuis le 1er janvier 1750 (années de 365 jours)
  const an = 1750 + Math.floor(jour / 365)
  const jda = jour % 365
  const longs = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
  let m = 0, r = jda
  while (r >= longs[m]) { r -= longs[m]; m++ }
  return `${r + 1} ${MOIS[m].slice(0, 4)}${MOIS[m].length > 4 ? '.' : ''} ${an}`
}
