// Libellés français, dates et petites aides d'affichage.

export const RESSOURCES = {
  grain: 'Grain', viande: 'Viande', fruit: 'Fruits', bois: 'Bois',
  lin: 'Lin', laine: 'Laine', tissu: 'Tissu', vetements: 'Vêtements',
}

export const TYPES = {
  champ: 'Champ de céréales', paturage: 'Pâturage', verger: 'Verger', foret_exploitee: 'Forêt exploitée',
  champ_lin: 'Champ de lin', elevage_laine: 'Élevage de moutons', atelier_textile: 'Atelier textile',
  chantier: 'Chantier',
}

export const HABITATS = { hameau: 'Hameau', hameau_textile: 'Hameau textile' }

const MOIS = ['janvier', 'février', 'mars', 'avril', 'mai', 'juin', 'juillet', 'août', 'septembre', 'octobre', 'novembre', 'décembre']
const JOURS_MOIS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]

/** Date de jeu : le jour 0 est le 1er janvier 1750 (année de 365 jours, sans bissextile). */
export function dateJeu(jour) {
  const an = 1750 + Math.floor(jour / 365)
  let d = Math.floor(jour % 365)
  let m = 0
  while (d >= JOURS_MOIS[m]) { d -= JOURS_MOIS[m]; m++ }
  return { an, texte: `${d + 1} ${MOIS[m]} ${an}` }
}

const MOTS = {
  hameau_textile: 'hameau textile', elevage_laine: 'élevage de moutons', foret_exploitee: 'forêt exploitée',
  champ_lin: 'champ de lin', atelier_textile: 'atelier textile', paturage: 'pâturage', vetements: 'vêtements',
}

/** Rend lisible un texte du journal : identifiants techniques -> français, accords au singulier. */
export function traduire(texte) {
  return texte
    .replace(/\b(hameau_textile|elevage_laine|foret_exploitee|champ_lin|atelier_textile|paturage|vetements)\b/g, (m) => MOTS[m])
    .replace(/\b1 pops attendent\b/, '1 pop attend')
    .replace(/\b1 pops\b/g, '1 pop')
}

export function limitantTexte(l) {
  if (!l) return 'tourne à plein régime'
  if (l === 'main_oeuvre') return 'manque de bras'
  if (l === 'stock_plein') return 'stock plein : rien à écouler'
  if (l.startsWith('intrant:')) return `manque de ${(RESSOURCES[l.slice(8)] ?? l.slice(8)).toLowerCase()}`
  return l
}

export const pct = (x) => `${Math.round(x * 100)} %`
export const un = (x, d = 1) => (Math.round(x * 10 ** d) / 10 ** d).toLocaleString('fr-FR')

/** État d'une pop d'après la couverture de son hameau. */
export function etatPop(hameau) {
  if (hameau.vital < 0.999) return { cle: 'manque', texte: 'en manque', couleur: '#d65a4a' }
  if (hameau.confort < 0.999) return { cle: 'insatisfaite', texte: 'insatisfaite', couleur: '#e0a03a' }
  return { cle: 'satisfaite', texte: 'satisfaite', couleur: '#7ab648' }
}

/** Couleur d'un ratio 0..1 : rouge -> orange -> vert. */
export function couleurRatio(r) {
  const x = Math.max(0, Math.min(1, r))
  const h = 8 + 112 * x
  return `hsl(${h} 62% 46%)`
}
