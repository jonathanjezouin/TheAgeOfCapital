// Rendu du terrain (docs/07 §3) : une image pour toute la carte, peinte pixel par pixel, puis les cours d'eau en courbes.
// Pas d'atlas : plaine, plaine riche, rocailleuse, marécageuse et forêt ont chacune une texture procédurale lisible.
// Les bordures entre terrains sont déformées par un bruit (frontières organiques), les rivières lissées entre tiles.

export const TERRAINS = ['plaine', 'plaine_riche', 'plaine_rocailleuse', 'plaine_marecageuse', 'foret']
export const EAUX = ['', 'ruisseau', 'riviere', 'fleuve']
export const PX = 12 // pixels de l'image par tile

const hash = (x, y, s = 0) => {
  let h = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263) + Math.imul(s | 0, 2147483647)) | 0
  h = Math.imul(h ^ (h >>> 13), 1274126177)
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296
}
/** Bruit de valeur lissé en (x, y) (unités de la grille), valeur dans [0, 1). */
function bruit(x, y, s = 0) {
  const x0 = Math.floor(x), y0 = Math.floor(y)
  const fx = x - x0, fy = y - y0
  const sx = fx * fx * (3 - 2 * fx), sy = fy * fy * (3 - 2 * fy)
  const a = hash(x0, y0, s), b = hash(x0 + 1, y0, s), c = hash(x0, y0 + 1, s), d = hash(x0 + 1, y0 + 1, s)
  return (a + (b - a) * sx) * (1 - sy) + (c + (d - c) * sx) * sy
}

const mix = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]

const COUL = {
  plaine: [160, 172, 94],
  plaineClair: [170, 180, 102],
  riche: [122, 158, 74],
  richeSombre: [108, 146, 66],
  rocaille: [178, 168, 122],
  caillou: [138, 134, 120],
  marais: [96, 124, 92],
  maraisEau: [78, 124, 142],
  roseau: [80, 104, 66],
  foret: [62, 100, 54],
  foretClair: [82, 124, 66],
  foretSombre: [40, 74, 42],
  berge: [108, 134, 112],
  eau: [[84, 146, 196], [64, 126, 188], [48, 108, 176]],
}

/** Couleur du pixel (px, py) de l'image, pour une tile de terrain `t`. */
function couleurTerrain(t, px, py) {
  const grain = (hash(px, py, 5) - 0.5) * 7
  const lent = bruit(px / 40, py / 40, 11) - 0.5
  let c
  switch (t) {
    case 1: // plaine riche : vert plus profond, sillons très discrets
      c = mix(COUL.riche, COUL.richeSombre, bruit(px / 22, py / 22, 3))
      break
    case 2: { // rocailleuse : sol ocre clair parsemé de cailloux
      c = mix(COUL.rocaille, [160, 150, 108], bruit(px / 18, py / 18, 4))
      const h = hash(px >> 1, py >> 1, 9)
      if (h > 0.93) c = mix(c, COUL.caillou, 0.75)
      break
    }
    case 3: { // marécageuse : herbe détrempée, flaques et roseaux
      const e = bruit(px / 9, py / 9, 6)
      c = e > 0.62 ? mix(COUL.marais, COUL.maraisEau, Math.min(1, (e - 0.62) * 7)) : mix(COUL.marais, COUL.roseau, bruit(px / 14, py / 14, 7))
      break
    }
    case 4: { // forêt : couronnes d'arbres en taches claires et sombres
      const a = bruit(px / 5.2, py / 5.2, 8), b = bruit(px / 2.6, py / 2.6, 12)
      c = a > 0.5 ? mix(COUL.foret, COUL.foretClair, (a - 0.5) * 2.2 + (b - 0.5) * 0.6) : mix(COUL.foret, COUL.foretSombre, (0.5 - a) * 2.4)
      break
    }
    default: // plaine
      c = mix(COUL.plaine, COUL.plaineClair, bruit(px / 26, py / 26, 2))
  }
  const k = t === 4 ? 0.4 : 1
  return [c[0] + grain * k + lent * 14, c[1] + grain * k + lent * 14, c[2] + grain * k + lent * 12]
}

/** Peint le rectangle de pixels [px0, px1) × [py0, py1) du sol (sans les cours d'eau). */
function peindreRect(ctx, donnees, largeur, hauteur, px0, py0, px1, py1) {
  const w = px1 - px0, h = py1 - py0
  const img = ctx.createImageData(w, h)
  const d = img.data
  const terrainDe = (x, y) => donnees[(Math.min(hauteur - 1, Math.max(0, y)) * largeur + Math.min(largeur - 1, Math.max(0, x))) * 2]
  for (let py = py0; py < py1; py++) {
    for (let px = px0; px < px1; px++) {
      // Bordure organique : on lit la tile à une position déformée par un bruit à deux échelles.
      const u = px / PX, v = py / PX
      const wu = u + (bruit(u * 2.1, v * 2.1, 21) - 0.5) * 0.9 + (bruit(u * 6, v * 6, 22) - 0.5) * 0.25
      const wv = v + (bruit(u * 2.1, v * 2.1, 23) - 0.5) * 0.9 + (bruit(u * 6, v * 6, 24) - 0.5) * 0.25
      const t = terrainDe(Math.floor(wu), Math.floor(wv))
      const [r, g, b] = couleurTerrain(t, px, py)
      const k = ((py - py0) * w + (px - px0)) * 4
      d[k] = r; d[k + 1] = g; d[k + 2] = b; d[k + 3] = 255
    }
  }
  ctx.putImageData(img, px0, py0)
}

/**
 * Construit les images du terrain de toute la carte : le sol (canvas opaque) et les cours d'eau (canvas transparent,
 * posé au-dessus). Séparés pour que le sol puisse être repeint tile par tile quand un terrain est converti.
 * @param {Uint8Array} donnees 2 octets par tile : terrain, puis eau (bits 0-1) et sens de l'aval (bits 2-4)
 */
export function construireTerrain(donnees, largeur, hauteur) {
  const W = largeur * PX, H = hauteur * PX
  const sol = document.createElement('canvas')
  sol.width = W
  sol.height = H
  const ctx = sol.getContext('2d')
  peindreRect(ctx, donnees, largeur, hauteur, 0, 0, W, H)
  const eau = document.createElement('canvas')
  eau.width = W
  eau.height = H
  tracerEau(eau.getContext('2d'), donnees, largeur, hauteur)
  return { sol, eau }
}

/** Repeint le sol autour des tiles dont le terrain a changé (défrichement, drainage). Renvoie leur nombre. */
export function repeindreTuiles(sol, anciennes, donnees, largeur, hauteur) {
  const ctx = sol.getContext('2d')
  const faits = new Set()
  let n = 0
  for (let i = 0; i < largeur * hauteur; i++) {
    if (anciennes[i * 2] === donnees[i * 2]) continue
    n++
    const x = i % largeur, y = Math.floor(i / largeur)
    // Marge d'une tile : la bordure déformée déborde de la tile.
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) faits.add((y + dy) * 100000 + (x + dx))
  }
  for (const c of faits) {
    const y = Math.floor(c / 100000), x = c % 100000
    if (x < 0 || y < 0 || x >= largeur || y >= hauteur) continue
    peindreRect(ctx, donnees, largeur, hauteur, x * PX, y * PX, (x + 1) * PX, (y + 1) * PX)
  }
  return n
}

/** Cours d'eau : pour chaque tile, une courbe lissée du milieu de l'arête amont au milieu de l'arête aval. */
function tracerEau(ctx, donnees, largeur, hauteur) {
  const DIR = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]]
  const eauDe = (i) => donnees[i * 2 + 1] & 3
  const avalDe = (i) => { const c = (donnees[i * 2 + 1] >> 2) & 7; return c === 0 ? -1 : (c - 1) * 2 }
  const n = largeur * hauteur
  // Centre décalé de chaque tile d'eau : méandre léger, identique pour toutes les courbes qui s'y rejoignent.
  const centre = (i) => {
    const x = i % largeur, y = Math.floor(i / largeur)
    return [(x + 0.5 + (hash(x, y, 31) - 0.5) * 0.5) * PX, (y + 0.5 + (hash(x, y, 32) - 0.5) * 0.5) * PX]
  }
  const aval = new Int32Array(n).fill(-1)
  const amont = Array.from({ length: n }, () => [])
  for (let i = 0; i < n; i++) {
    if (!eauDe(i)) continue
    const d = avalDe(i)
    if (d < 0) continue
    const j = (Math.floor(i / largeur) + DIR[d][1]) * largeur + (i % largeur) + DIR[d][0]
    if (j >= 0 && j < n && eauDe(j)) { aval[i] = j; amont[j].push(i) }
  }
  const milieu = (a, b) => { const [ax, ay] = centre(a), [bx, by] = centre(b); return [(ax + bx) / 2, (ay + by) / 2] }
  const LARG = [0, 2.6, 5.4, 9.6]
  const courbes = []
  for (let i = 0; i < n; i++) {
    const g = eauDe(i)
    if (!g) continue
    const c = centre(i)
    // fin : milieu de l'arête aval, ou prolongement hors de la tile à l'embouchure
    let fin
    if (aval[i] >= 0) fin = milieu(i, aval[i])
    else { const d = avalDe(i); fin = d >= 0 ? [c[0] + DIR[d][0] * PX * 0.9, c[1] + DIR[d][1] * PX * 0.9] : c }
    const debuts = amont[i].length ? amont[i].map((p) => milieu(p, i)) : [c]
    for (const debut of debuts) courbes.push({ debut, c, fin, g })
  }
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  const passe = (largeurDe, couleurDe) => {
    for (const { debut, c, fin, g } of courbes) {
      ctx.strokeStyle = couleurDe(g)
      ctx.lineWidth = largeurDe(g)
      ctx.beginPath()
      ctx.moveTo(debut[0], debut[1])
      ctx.quadraticCurveTo(c[0], c[1], fin[0], fin[1])
      ctx.stroke()
    }
  }
  const rgb = (c, a = 1) => `rgba(${c[0] | 0},${c[1] | 0},${c[2] | 0},${a})`
  passe((g) => LARG[g] + 2.6, () => rgb(COUL.berge, 0.55)) // berges
  passe((g) => LARG[g], (g) => rgb(COUL.eau[g - 1]))
  passe((g) => LARG[g] * 0.35, () => 'rgba(190,225,240,0.35)') // reflet central
}

export const nomTerrain = (t) => TERRAINS[t] ?? 'plaine'
