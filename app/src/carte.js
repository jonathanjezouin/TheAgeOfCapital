// Rendu de la carte avec PixiJS : couches sol / infrastructure / bâtiments / pops, caméra et sélection.
// Vue strictement de dessus, soleil au zénith ; le jour/nuit est une teinte globale.

import 'pixi.js/unsafe-eval' // aucune génération de code à l'exécution : compatible avec une CSP stricte
import { Application, Assets, Container, Graphics, Rectangle, Sprite, Texture, Point } from 'pixi.js'
import { store } from './store.js'

export const T = 256 // pixels d'une tile dans l'atlas (unités du monde)
const RAYON_SOL = 40 // tiles d'herbe dessinées autour du centre de la carte

// Type de terre -> groupe d'images de l'atlas.
const GROUPE = {
  champ: 'bat_champ', paturage: 'bat_paturage', verger: 'bat_verger', foret_exploitee: 'bat_foret_exploitee',
  champ_lin: 'bat_champ_lin', elevage_laine: 'bat_elevage_laine',
}
// Les 8 directions d'arêtes, dans l'ordre du moteur : E, SE, S, SO, O, NO, N, NE (bit i du champ `a`).
const DIRS8 = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]]
const ECHELLE_MAISON = 1.1 // sprite de 128 px -> toit d'environ 1/3 de tile : une maison = une pop, taille exagérée (docs/03 §6)
const DECALAGE_ROUTE = 0.2 // décalage organique des routes, en fraction de tile
// Limite des terres le long du bord de leur emprise.
const BORDURES = { champ: 'haie', champ_lin: 'haie', paturage: 'cloture', elevage_laine: 'cloture' }
const LARGEUR_ROUTE = 0.12 // largeur du trait, exagérée (une piste de 5 m ferait moins d'un pixel)
const hashf = (x, y, s) => (hash(x * 7 + s, y * 13 + s * 3) % 10000) / 10000

const hash = (x, y) => (Math.imul(x + 1, 73856093) ^ Math.imul(y + 1, 19349663)) >>> 0
const hex = (r, g, b) => (r << 16) | (g << 8) | b

function hsl(h, s, l) {
  s /= 100; l /= 100
  const k = (n) => (n + h / 30) % 12
  const a = s * Math.min(l, 1 - l)
  const f = (n) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)))
  return hex(Math.round(255 * f(0)), Math.round(255 * f(8)), Math.round(255 * f(4)))
}

export class Carte {
  async init(parent, urls, meta, fertilite) {
    this.meta = meta
    this.parent = parent
    this.app = new Application()
    await this.app.init({
      resizeTo: parent, background: 0x6c7a3a, antialias: true, preference: 'webgl',
      resolution: Math.min(window.devicePixelRatio || 1, 2), autoDensity: true,
    })
    parent.appendChild(this.app.canvas)

    const atlas = await Assets.load(urls.atlasPng)
    try {
      atlas.source.autoGenerateMipmaps = true
      atlas.source.style.mipmapFilter = 'linear'
    } catch { /* mipmaps facultatifs */ }
    this.manifeste = await (await fetch(urls.atlasJson)).json()
    this.atlas = atlas
    this.textures = {}
    // Texture de terre battue du trait des routes (hors atlas, répétée).
    this.solChemin = null
    const nomSol = this.manifeste.textures?.sol_chemin
    if (nomSol) {
      try {
        this.solChemin = await Assets.load(urls.atlasJson.replace(/[^/]*$/, '') + nomSol)
        this.solChemin.source.style.addressMode = 'repeat'
      } catch { this.solChemin = null }
    }
    this.chantiersMaison = []

    this.monde = new Container()
    this.sol = new Container()
    this.terres = new Container() // terres exploitées sur N tiles, sous les routes (une route peut les traverser)
    this.infra = new Container()
    this.bat = new Container()
    this.couleurs = new Graphics() // mode « besoins »
    this.pops = new Graphics()
    this.barres = new Graphics()
    this.selectionG = new Graphics()
    this.nuit = new Graphics()
    this.monde.addChild(this.sol)
    this.fertiliteSprite = this.construireFertilite(fertilite)
    this.monde.addChild(this.fertiliteSprite, this.terres, this.infra, this.bat, this.couleurs, this.barres, this.pops, this.selectionG)
    this.app.stage.addChild(this.monde, this.nuit)

    this.construireSol()
    this.installerCamera()
    this.app.renderer.on('resize', () => this.majNuit())
    return this
  }

  // ---------- Images ----------

  texture(nom) {
    if (!this.textures[nom]) {
      const f = this.manifeste.frames[nom]
      this.textures[nom] = new Texture({ source: this.atlas.source, frame: new Rectangle(f.x, f.y, f.w, f.h) })
    }
    return this.textures[nom]
  }

  variante(groupe, h) {
    const liste = this.manifeste.groupes[groupe]
    return this.texture(liste[h % liste.length])
  }

  poser(conteneur, texture, x, y) {
    const s = new Sprite(texture)
    s.position.set(x * T, y * T)
    conteneur.addChild(s)
    return s
  }

  // ---------- Couches ----------

  construireSol() {
    const cx = this.meta.largeur >> 1, cy = this.meta.hauteur >> 1
    for (let y = cy - RAYON_SOL; y <= cy + RAYON_SOL; y++) {
      for (let x = cx - RAYON_SOL; x <= cx + RAYON_SOL; x++) {
        this.poser(this.sol, this.variante('terrain_plaine', hash(x, y)), x, y)
      }
    }
  }

  /** Carte de chaleur de la fertilité (mode « fertilité »), une image d'un pixel par tile. */
  construireFertilite(fertilite) {
    const cx = this.meta.largeur >> 1, cy = this.meta.hauteur >> 1
    const R = RAYON_SOL * 2 + 1
    const canvas = document.createElement('canvas')
    canvas.width = canvas.height = R
    const ctx = canvas.getContext('2d')
    const img = ctx.createImageData(R, R)
    for (let j = 0; j < R; j++) {
      for (let i = 0; i < R; i++) {
        const f = fertilite[(cy - RAYON_SOL + j) * this.meta.largeur + (cx - RAYON_SOL + i)] / 100
        const t = Math.max(0, Math.min(1, f / 1.2))
        const [r, g, b] = rampe(t)
        const k = (j * R + i) * 4
        img.data[k] = r; img.data[k + 1] = g; img.data[k + 2] = b; img.data[k + 3] = 255
      }
    }
    ctx.putImageData(img, 0, 0)
    const s = new Sprite(Texture.from(canvas))
    s.position.set((cx - RAYON_SOL) * T, (cy - RAYON_SOL) * T)
    s.width = s.height = R * T
    s.alpha = 0.62
    s.visible = false
    return s
  }

  /** Nouvelle carte de fertilité (nouveau village). */
  remplacerFertilite(fertilite) {
    const visible = this.fertiliteSprite.visible
    this.monde.removeChild(this.fertiliteSprite)
    this.fertiliteSprite.destroy({ texture: true })
    this.fertiliteSprite = this.construireFertilite(fertilite)
    this.fertiliteSprite.visible = visible
    this.monde.addChildAt(this.fertiliteSprite, 1)
  }

  /** (Re)construit routes, maisons et terres quand la carte change (chantier lancé ou terminé, route créée). */
  appliquerCarte(carte, etat) {
    this.carteVersion = carte.version
    this.terres.removeChildren().forEach((c) => c.destroy())
    this.infra.removeChildren().forEach((c) => c.destroy())
    this.bat.removeChildren().forEach((c) => c.destroy())
    this.chantiersMaison = []
    const W = this.meta.largeur
    const grille = new Map()
    for (const t of carte.tuiles) grille.set(t.y * W + t.x, t)
    this.grille = grille
    const bordures = new Graphics()
    this.tracerRoutes(carte, W)
    for (const t of carte.tuiles) {
      if (t.c === 'chemin') continue
      if (t.c === 'hameau') {
        this.poserMaisons(t, etat)
      } else {
        // Terre exploitée : une texture raccordable par tile de l'emprise, limites le long du bord.
        const lieu = etat.lieux[t.m]
        const base = lieu.chantier ? 'bat_chantier_champ' : GROUPE[lieu.type_nom]
        if (!base) continue
        const plein = !lieu.chantier && this.manifeste.groupes[base + '_plein']
        this.poser(this.terres, this.variante(plein ? base + '_plein' : base, hash(t.x, t.y)), t.x, t.y)
        if (!lieu.chantier) this.bordureTerre(bordures, t, lieu.type_nom, grille, W)
      }
    }
    this.terres.addChild(bordures)
  }

  /** Haie ou clôture le long des côtés de la tile qui ne touchent pas la même terre. */
  bordureTerre(g, t, type, grille, W) {
    const style = BORDURES[type]
    if (!style) return
    for (const [dx, dy] of [[0, -1], [1, 0], [0, 1], [-1, 0]]) {
      const v = grille.get((t.y + dy) * W + (t.x + dx))
      if (v && v.c === 'bat' && v.m === t.m) continue
      const x0 = (t.x + (dx === 1 ? 1 : 0)) * T, y0 = (t.y + (dy === 1 ? 1 : 0)) * T
      const [x1, y1] = dx === 0 ? [x0 + T, y0] : [x0, y0 + T]
      if (style === 'haie') {
        g.moveTo(x0, y0).lineTo(x1, y1).stroke({ width: 12, color: 0x3f5a2f, alpha: 0.92 })
        g.moveTo(x0, y0).lineTo(x1, y1).stroke({ width: 5, color: 0x6f8f45, alpha: 0.9 })
      } else {
        g.moveTo(x0, y0).lineTo(x1, y1).stroke({ width: 5, color: 0x7a5a3a })
        for (let k = 0; k <= 4; k++) {
          const px = x0 + (x1 - x0) * k / 4, py = y0 + (y1 - y0) * k / 4
          g.rect(px - 5, py - 5, 10, 10).fill(0x5e442b)
        }
      }
    }
  }

  // ---------- Routes : arêtes lissées en courbes ----------

  /** Le réseau est une couche d'arêtes (champ `a` de chaque tile). On le découpe en chaînes entre
   *  carrefours, bouts de route et lieux, puis chaque chaîne est tracée en courbe lissée (Catmull-Rom)
   *  passant par des centres de tiles légèrement décalés (décalage déterministe). */
  tracerRoutes(carte, W) {
    const noeuds = new Map()
    for (const t of carte.tuiles) {
      if (!t.a) continue
      const voisins = []
      for (let d = 0; d < 8; d++) if (t.a & (1 << d)) voisins.push((t.y + DIRS8[d][1]) * W + (t.x + DIRS8[d][0]))
      noeuds.set(t.y * W + t.x, { x: t.x, y: t.y, voisins, fixe: t.c !== 'chemin' })
    }
    const vu = new Set()
    const cle = (a, b) => (a < b ? a + ':' + b : b + ':' + a)
    const chaines = []
    const suivre = (depart, premier) => {
      const chaine = [depart]
      let prec = depart, cur = premier
      vu.add(cle(depart, premier))
      for (;;) {
        chaine.push(cur)
        const n = noeuds.get(cur)
        if (!n || n.fixe || n.voisins.length !== 2) break
        const suiv = n.voisins[0] === prec ? n.voisins[1] : n.voisins[0]
        if (vu.has(cle(cur, suiv))) break
        vu.add(cle(cur, suiv))
        prec = cur
        cur = suiv
      }
      chaines.push(chaine)
    }
    for (const [k, n] of noeuds) {
      if (n.fixe || n.voisins.length !== 2) for (const v of n.voisins) if (noeuds.has(v) && !vu.has(cle(k, v))) suivre(k, v)
    }
    for (const [k, n] of noeuds) for (const v of n.voisins) if (noeuds.has(v) && !vu.has(cle(k, v))) suivre(k, v) // boucles
    const g = new Graphics()
    const pts = (chaine) => chaine.map((k, i) => {
      const n = noeuds.get(k)
      let x = (n.x + 0.5) * T, y = (n.y + 0.5) * T
      if (!n.fixe && i > 0 && i < chaine.length - 1) {
        x += (hashf(n.x, n.y, 1) - 0.5) * 2 * DECALAGE_ROUTE * T
        y += (hashf(n.x, n.y, 2) - 0.5) * 2 * DECALAGE_ROUTE * T
      }
      return [x, y]
    })
    const courbe = (p) => {
      g.moveTo(p[0][0], p[0][1])
      for (let i = 0; i < p.length - 1; i++) {
        const p0 = p[i - 1] || p[i], p1 = p[i], p2 = p[i + 1], p3 = p[i + 2] || p2
        g.bezierCurveTo(p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6, p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6, p2[0], p2[1])
      }
    }
    const traces = chaines.map(pts)
    const larg = LARGEUR_ROUTE * T
    for (const p of traces) { courbe(p); g.stroke({ width: larg * 1.22, color: 0x8f7650, cap: 'round', join: 'round' }) }
    for (const p of traces) {
      courbe(p)
      if (this.solChemin) g.stroke({ width: larg, texture: this.solChemin, textureSpace: 'global', cap: 'round', join: 'round' })
      else g.stroke({ width: larg, color: 0xc9b184, cap: 'round', join: 'round' })
    }
    this.infra.addChild(g)
  }

  // ---------- Maisons : une par pop, le long des routes, face à la route ----------

  /** Emplacements d'une tile d'habitat : le long de chaque arête incidente, de part et d'autre,
   *  du centre vers le bord ; l'angle fait face à la route. Sans arête : couronne autour du centre. */
  emplacementsMaisons(t) {
    const cx = (t.x + 0.5) * T, cy = (t.y + 0.5) * T
    const dirs = []
    for (let d = 0; d < 8; d++) if (t.a & (1 << d)) dirs.push(d)
    const slots = []
    if (!dirs.length) {
      for (let i = 0; i < 10; i++) {
        const a = (i / 10) * 2 * Math.PI
        slots.push({ x: cx + Math.cos(a) * 0.3 * T, y: cy + Math.sin(a) * 0.3 * T, angle: Math.atan2(-Math.cos(a) * -1, -Math.sin(a)) })
      }
      return slots
    }
    for (const rang of [0.26, 0.47]) {
      for (const d of dirs) {
        const l = Math.hypot(DIRS8[d][0], DIRS8[d][1])
        const ux = DIRS8[d][0] / l, uy = DIRS8[d][1] / l
        for (const cote of [1, -1]) {
          const nx = -uy * cote, ny = ux * cote
          const vx = -nx, vy = -ny // vers la route
          const j = (hashf(t.x * 5 + d, t.y * 3 + cote, 3) - 0.5) * 0.04
          slots.push({
            x: cx + ux * rang * T + nx * (0.24 + j) * T,
            y: cy + uy * rang * T + ny * (0.24 + j) * T,
            angle: Math.atan2(-vx, vy),
          })
        }
      }
    }
    return slots
  }

  poserMaisons(t, etat) {
    const h = etat.hameaux[t.m]
    const slots = this.emplacementsMaisons(t)
    const n = h.capacite
    for (let i = 0; i < n; i++) {
      const s = slots[i % slots.length]
      const sp = new Sprite(this.variante('bat_maison_bois', hash(t.x * 31 + i, t.y * 17 + i)))
      sp.anchor.set(0.5)
      sp.scale.set(ECHELLE_MAISON)
      sp.position.set(s.x, s.y)
      sp.rotation = s.angle
      this.bat.addChild(sp)
    }
    // Chantier en cours (fondation du hameau, ou une maison de plus) : un sprite par chantier.
    const lieu = etat.lieux.find((l) => l.chantier && l.x === t.x && l.y === t.y)
    if (lieu) {
      const s = slots[n % slots.length]
      const sp = new Sprite(this.texture(this.manifeste.groupes.bat_chantier_maison[0]))
      sp.anchor.set(0.5)
      sp.scale.set(ECHELLE_MAISON)
      sp.position.set(s.x, s.y)
      sp.rotation = s.angle
      this.bat.addChild(sp)
      this.chantiersMaison.push({ sprite: sp, x: t.x, y: t.y })
    }
  }

  /** État visuel du chantier de maison selon l'avancement (sol dégagé, ossature, charpente). */
  majChantiersMaison(etat) {
    const noms = this.manifeste.groupes.bat_chantier_maison
    for (const c of this.chantiersMaison) {
      const lieu = etat.lieux.find((l) => l.chantier && l.x === c.x && l.y === c.y)
      if (!lieu) continue
      const a = lieu.chantier.avancement
      c.sprite.texture = this.texture(noms[Math.min(noms.length - 1, Math.floor(a * noms.length))])
    }
  }

  // ---------- Mise à jour dynamique ----------

  rafraichir(etat) {
    this.etat = etat
    this.majChantiersMaison(etat)
    this.dessinerChantiers(etat)
    this.dessinerPops(etat)
    this.dessinerModes(etat)
    this.dessinerSelection()
    this.fertiliteSprite.visible = store.mode === 'fertilite'
    this.majNuit()
  }

  dessinerChantiers(etat) {
    const g = this.barres
    g.clear()
    etat.lieux.forEach((l) => {
      if (!l.chantier) return
      const x = l.x * T + 36, y = l.y * T + T - 34, w = T - 72
      g.rect(x, y, w, 16).fill({ color: 0x1c1a14, alpha: 0.85 })
      g.rect(x + 2, y + 2, Math.max(0, (w - 4) * l.chantier.avancement), 12).fill(0xe8b84a)
    })
  }

  dessinerPops(etat) {
    const g = this.pops
    g.clear()
    const jour = store.heure >= 7 && store.heure < 18
    for (const [id, res, emploi, jauge] of etat.pops) {
      const hameau = etat.hameaux[res]
      const lieu = emploi >= 0 ? etat.lieux[emploi] : null
      const cible = jour && lieu ? lieu : hameau
      const h = hash(id * 7 + 3, id * 13 + 5)
      const ox = ((h & 255) / 255 - 0.5) * 150
      const oy = (((h >> 8) & 255) / 255 - 0.5) * 150
      const px = cible.x * T + T / 2 + ox
      const py = cible.y * T + T / 2 + oy
      const couleur = hameau.vital < 0.999 ? 0xd65a4a : hameau.confort < 0.999 ? 0xe0a03a : 0x86c250
      if (emploi < 0) {
        g.circle(px, py, 15).fill({ color: 0x4a4a48, alpha: 0.9 }).stroke({ width: 4, color: 0xd9d4c4 })
      } else {
        g.circle(px, py, 14).fill(couleur).stroke({ width: 4, color: 0x1f1d18 })
      }
    }
  }

  dessinerModes(etat) {
    const g = this.couleurs
    g.clear()
    if (store.mode !== 'besoins') return
    for (const h of etat.hameaux) {
      const c = h.vital < 0.999 ? hsl(8 + 40 * h.vital, 65, 45) : hsl(30 + 90 * h.confort, 62, 45)
      g.rect(h.x * T + 6, h.y * T + 6, T - 12, T - 12).fill({ color: c, alpha: 0.5 })
    }
    for (const l of etat.lieux) {
      if (l.quartier != null || (!l.chantier && l.postes === 0)) continue
      g.rect(l.x * T + 6, l.y * T + 6, T - 12, T - 12).fill({ color: hsl(8 + 112 * l.prod, 60, 42), alpha: 0.42 })
    }
  }

  dessinerSelection() {
    const g = this.selectionG
    g.clear()
    const s = store.selection
    if (!s) return
    const tuile = this.grille?.get(s.y * this.meta.largeur + s.x)
    const lieu = tuile && tuile.c === 'bat' ? this.etat?.lieux[tuile.m] : null
    if (lieu && lieu.emprise.length > 1) {
      for (const [x, y] of lieu.emprise) g.rect(x * T, y * T, T, T).fill({ color: 0xffffff, alpha: 0.1 })
    }
    g.rect(s.x * T + 4, s.y * T + 4, T - 8, T - 8).fill({ color: 0xffffff, alpha: 0.08 }).stroke({ width: 8, color: 0xfff4c2 })
  }

  /** Teinte de nuit, uniquement en mode « jour/nuit » (sinon le cycle serait un clignotement). */
  majNuit() {
    const g = this.nuit
    g.clear()
    if (store.vitesse > 0.5) return
    const h = store.heure
    const nuit = h < 5 || h >= 21 ? 1 : h < 7 ? 1 - (h - 5) / 2 : h >= 19 ? (h - 19) / 2 : 0
    if (nuit <= 0) return
    const { width, height } = this.app.screen
    g.rect(0, 0, width, height).fill({ color: 0x0b1438, alpha: 0.55 * nuit })
  }

  // ---------- Caméra et sélection ----------

  centrer(carte) {
    const xs = carte.tuiles.map((t) => t.x), ys = carte.tuiles.map((t) => t.y)
    const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs) + 1, Math.min(...ys), Math.max(...ys) + 1]
    const { width, height } = this.app.screen
    const z = Math.max(0.08, Math.min(0.6, 0.94 * Math.min(width / ((x1 - x0 + 1) * T), height / ((y1 - y0 + 1) * T))))
    this.monde.scale.set(z)
    this.monde.position.set(width / 2 - ((x0 + x1) / 2) * T * z, height / 2 - ((y0 + y1) / 2) * T * z)
  }

  /** Position écran (px CSS de la page) du centre d'une tile. */
  positionEcran(x, y) {
    const r = this.app.canvas.getBoundingClientRect()
    const p = this.monde.toGlobal(new Point((x + 0.5) * T, (y + 0.5) * T))
    return { x: r.left + p.x, y: r.top + p.y }
  }

  tuileSous(clientX, clientY) {
    const r = this.app.canvas.getBoundingClientRect()
    const p = this.monde.toLocal(new Point(clientX - r.left, clientY - r.top))
    const x = Math.floor(p.x / T), y = Math.floor(p.y / T)
    return x >= 0 && y >= 0 && x < this.meta.largeur && y < this.meta.hauteur ? { x, y } : null
  }

  installerCamera() {
    const cv = this.app.canvas
    let drag = null
    cv.addEventListener('pointerdown', (e) => {
      drag = { x: e.clientX, y: e.clientY, px: this.monde.x, py: this.monde.y, bouge: false }
      cv.setPointerCapture(e.pointerId)
    })
    cv.addEventListener('pointermove', (e) => {
      if (drag) {
        const dx = e.clientX - drag.x, dy = e.clientY - drag.y
        if (Math.abs(dx) + Math.abs(dy) > 5) drag.bouge = true
        if (drag.bouge) this.monde.position.set(drag.px + dx, drag.py + dy)
      }
      store.survol = this.tuileSous(e.clientX, e.clientY)
    })
    cv.addEventListener('pointerup', (e) => {
      if (drag && !drag.bouge) {
        store.selection = this.tuileSous(e.clientX, e.clientY)
        this.dessinerSelection()
      }
      drag = null
    })
    cv.addEventListener('pointerleave', () => { store.survol = null })
    cv.addEventListener('wheel', (e) => {
      e.preventDefault()
      const r = cv.getBoundingClientRect()
      const px = e.clientX - r.left, py = e.clientY - r.top
      const ancien = this.monde.scale.x
      const z = Math.max(0.07, Math.min(1.2, ancien * Math.exp(-e.deltaY * 0.0015)))
      this.monde.scale.set(z)
      this.monde.position.set(px - (px - this.monde.x) * (z / ancien), py - (py - this.monde.y) * (z / ancien))
    }, { passive: false })
  }

  detruire() {
    this.app.destroy(true, { children: true })
  }
}

/** Rampe de couleurs de la fertilité : terre pauvre (brun) -> ocre -> prairie -> vert profond. */
function rampe(t) {
  const pts = [[0, [150, 84, 56]], [0.3, [196, 150, 70]], [0.6, [150, 176, 70]], [0.85, [74, 150, 70]], [1, [30, 110, 76]]]
  for (let i = 1; i < pts.length; i++) {
    if (t <= pts[i][0]) {
      const [t0, c0] = pts[i - 1], [t1, c1] = pts[i]
      const k = (t - t0) / (t1 - t0)
      return c0.map((v, j) => Math.round(v + (c1[j] - v) * k))
    }
  }
  return pts[pts.length - 1][1]
}
