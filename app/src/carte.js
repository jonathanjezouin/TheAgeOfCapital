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
const N = 1, E = 2, S = 4, O = 8
const DIRS = [[N, 0, -1, S], [E, 1, 0, O], [S, 0, 1, N], [O, -1, 0, E]]

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

    this.monde = new Container()
    this.sol = new Container()
    this.infra = new Container()
    this.bat = new Container()
    this.couleurs = new Graphics() // mode « besoins »
    this.pops = new Graphics()
    this.barres = new Graphics()
    this.selectionG = new Graphics()
    this.nuit = new Graphics()
    this.monde.addChild(this.sol)
    this.fertiliteSprite = this.construireFertilite(fertilite)
    this.monde.addChild(this.fertiliteSprite, this.infra, this.bat, this.couleurs, this.barres, this.pops, this.selectionG)
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

  /** (Re)construit chemins et bâtiments quand la carte change (chantier lancé ou terminé, route créée). */
  appliquerCarte(carte, etat) {
    this.carteVersion = carte.version
    this.infra.removeChildren().forEach((c) => c.destroy())
    this.bat.removeChildren().forEach((c) => c.destroy())
    const W = this.meta.largeur
    const grille = new Map()
    for (const t of carte.tuiles) grille.set(t.y * W + t.x, t)
    this.grille = grille
    const chemins = this.manifeste.groupes.infra_chemin_terre

    // Masque de raccordement d'une tile traversable : voisins réellement joints.
    const masqueVoisins = (t) => {
      let m = 0
      for (const [bit, dx, dy, opp] of DIRS) {
        const v = grille.get((t.y + dy) * W + (t.x + dx))
        if (!v) continue
        if ((v.c === 'chemin' && (v.m & opp)) || v.c === 'hameau') m |= bit
      }
      return m
    }

    for (const t of carte.tuiles) {
      const h = hash(t.x, t.y)
      if (t.c === 'chemin') {
        if (t.m) this.poser(this.infra, this.texture(chemins[String(t.m)]), t.x, t.y)
      } else if (t.c === 'hameau') {
        const m = masqueVoisins(t)
        if (m) this.poser(this.infra, this.texture(chemins[String(m)]), t.x, t.y)
        const hameau = etat.hameaux[t.m]
        const groupe = hameau.capacite === 0 ? 'bat_chantier_hameau'
          : hameau.habitat === 'hameau_textile' ? 'bat_hameau_textile' : 'bat_hameau'
        this.poser(this.bat, this.variante(groupe, h), t.x, t.y)
      } else {
        const lieu = etat.lieux[t.m]
        const groupe = lieu.chantier ? 'bat_chantier_champ' : GROUPE[lieu.type_nom]
        if (groupe) this.poser(this.bat, this.variante(groupe, h), t.x, t.y)
      }
    }
  }

  // ---------- Mise à jour dynamique ----------

  rafraichir(etat) {
    this.etat = etat
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
    etat.hameaux.forEach((h) => {
      if (h.capacite !== 0) return
      const lieu = etat.lieux.find((l) => l.chantier && l.x === h.x && l.y === h.y)
      if (!lieu) return
      const x = h.x * T + 36, y = h.y * T + T - 34, w = T - 72
      g.rect(x, y, w, 16).fill({ color: 0x1c1a14, alpha: 0.85 })
      g.rect(x + 2, y + 2, Math.max(0, (w - 4) * lieu.chantier.avancement), 12).fill(0xe8b84a)
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
