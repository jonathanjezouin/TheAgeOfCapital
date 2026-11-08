// Carte PixiJS : sol de plaine, ferme, champs, forêts ; états lisibles, sélection et survol des tiles.
// Une tile = 256 unités du monde. Les sprites sont créés une fois ; seuls teintes et surcouches changent.
import 'pixi.js/unsafe-eval'
import { Application, Assets, Container, Graphics, Rectangle, Sprite, Texture } from 'pixi.js'

const T = 256
const hash = (x, y) => (Math.imul(x + 9, 73856093) ^ Math.imul(y + 9, 19349663)) >>> 0

export class Carte {
  async init(parent, urlPng, urlJson, onChoix) {
    this.onChoix = onChoix
    this.app = new Application()
    await this.app.init({ resizeTo: parent, background: 0x4f5c2b, antialias: true, resolution: Math.min(devicePixelRatio || 1, 2), autoDensity: true })
    parent.appendChild(this.app.canvas)
    const atlas = await Assets.load(urlPng)
    const man = await (await fetch(urlJson)).json()
    this.tex = (nom) => {
      const f = man.frames[nom]
      return new Texture({ source: atlas.source, frame: new Rectangle(f.x, f.y, f.w, f.h) })
    }
    this.monde = new Container()
    this.app.stage.addChild(this.monde)
    this.sol = new Container()
    this.objets = new Container()
    this.surcouche = new Graphics()
    this.monde.addChild(this.sol, this.objets, this.surcouche)
    this.app.renderer.on('resize', () => this.cadrer())

    this.app.stage.eventMode = 'static'
    this.app.stage.hitArea = this.app.screen
    this.app.stage.on('pointermove', (e) => { const i = this.tileSous(e.global); if (i !== this.survol) { this.survol = i; this.redessiner() } })
    this.app.stage.on('pointerleave', () => { this.survol = null; this.redessiner() })
    this.app.stage.on('pointerdown', (e) => { const i = this.tileSous(e.global); if (i != null) this.onChoix(i) })
    this.survol = null
    this.choix = null
  }

  tileSous(global) {
    if (!this.dims) return null
    const p = this.monde.toLocal(global)
    const x = Math.floor(p.x / T), y = Math.floor(p.y / T)
    if (x < 0 || y < 0 || x >= this.dims.l || y >= this.dims.h) return null
    return y * this.dims.l + x
  }

  /** Crée les sprites (une seule fois par monde). */
  construire(etat) {
    this.sol.removeChildren()
    this.objets.removeChildren()
    this.dims = { l: etat.largeur, h: etat.hauteur }
    this.sprites = []
    for (let y = -1; y <= etat.hauteur; y++) {
      for (let x = -1; x <= etat.largeur; x++) {
        const s = new Sprite(this.tex(`terrain_plaine_0${hash(x, y) % 10}`))
        s.position.set(x * T, y * T)
        this.sol.addChild(s)
      }
    }
    for (const t of etat.tiles) {
      let s = null
      if (t.type === 'champ') s = new Sprite(this.tex('bat_champ_0'))
      else if (t.type === 'foret') s = new Sprite(this.tex('bat_foret_exploitee_0'))
      else if (t.type === 'ferme') {
        s = new Sprite(this.tex('bat_maison_bois_0'))
        s.anchor.set(0.5); s.scale.set(2); s.position.set((t.x + 0.5) * T, (t.y + 0.5) * T)
      }
      if (s) {
        if (t.type !== 'ferme') s.position.set(t.x * T, t.y * T)
        this.objets.addChild(s)
      }
      this.sprites.push(s)
    }
    this.cadrer()
  }

  cadrer() {
    if (!this.dims) return
    const { l, h } = this.dims
    const w = this.app.screen.width, hh = this.app.screen.height
    const k = Math.min(w / ((l + 0.6) * T), hh / ((h + 0.6) * T))
    this.monde.scale.set(k)
    this.monde.position.set((w - l * T * k) / 2, (hh - h * T * k) / 2)
  }

  mettreAJour(etat, choix) {
    if (!this.dims || this.dims.l !== etat.largeur) this.construire(etat)
    this.etat = etat
    this.choix = choix
    const mur = etat.date.mois === 7 || etat.date.mois === 8
    etat.tiles.forEach((t, i) => {
      const s = this.sprites[i]
      if (!s) return
      if (t.type === 'champ') {
        // Nu / labouré : terre brune (dessinée en surcouche) ; semé : pousse verte ; mûr : doré.
        s.visible = t.phase === 'seme'
        s.tint = 0xcfe89a
      } else if (t.type === 'foret') {
        s.alpha = 0.45 + 0.55 * (t.disponible / 3)
      }
    })
    this.redessiner()
  }

  redessiner() {
    const e = this.etat
    if (!e) return
    const g = this.surcouche
    g.clear()
    const mur = e.date.mois === 7 || e.date.mois === 8
    e.tiles.forEach((t, i) => {
      const x = t.x * T, y = t.y * T
      if (t.type === 'champ') {
        if (t.phase !== 'seme') {
          g.rect(x + 12, y + 12, T - 24, T - 24).fill({ color: t.phase === 'nu' ? 0xb59868 : 0x7a5434, alpha: 0.95 })
          if (t.phase === 'laboure') for (let k = 1; k < 8; k++) g.moveTo(x + 12, y + 12 + k * 28).lineTo(x + T - 12, y + 12 + k * 28).stroke({ color: 0x4e3320, width: 4, alpha: 0.7 })
        }
        if (t.phase === 'seme' && mur) g.rect(x + 12, y + 12, T - 24, T - 24).fill({ color: 0xf2c94c, alpha: 0.5 })
        if (t.travail > 0) this.barre(g, x, y, t.travail / t.travail_requis, 0xffffff)
        if (t.grain > 0) this.tas(g, x + T - 56, y + 56, 0xf6d24a, t.grain)
      } else if (t.type === 'foret') {
        if (t.travail > 0) this.barre(g, x, y, t.travail / t.travail_requis, 0xffffff)
        if (t.coupe > 0) this.tas(g, x + T - 56, y + 56, 0x9b6a3c, t.coupe)
      }
    })
    // Trajet de la pop vers sa tile de travail
    const pop = e.pop
    const f = e.tiles.find((t) => t.type === 'ferme')
    const dest = e.tiles[pop.tile]
    if (dest && f && dest !== f && pop.activite !== 'aucune') {
      g.moveTo((f.x + 0.5) * T, (f.y + 0.5) * T).lineTo((dest.x + 0.5) * T, (dest.y + 0.5) * T).stroke({ color: 0xffffff, width: 6, alpha: 0.35 })
    }
    // Pop
    if (dest && pop.activite !== 'aucune') {
      const px = (dest.x + 0.5) * T + (dest.type === 'ferme' ? 70 : 0)
      const py = (dest.y + 0.5) * T + (dest.type === 'ferme' ? 70 : 0)
      g.circle(px, py, 26).fill({ color: 0xc0392b }).stroke({ color: 0xffffff, width: 6 })
    }
    // Survol et sélection
    for (const [idx, couleur, ep] of [[this.survol, 0xffffff, 5], [this.choix, 0xffd34a, 10]]) {
      if (idx == null || !e.tiles[idx]) continue
      const t = e.tiles[idx]
      g.rect(t.x * T + ep / 2, t.y * T + ep / 2, T - ep, T - ep).stroke({ color: couleur, width: ep, alpha: idx === this.choix ? 1 : 0.6 })
    }
  }

  barre(g, x, y, frac, couleur) {
    g.rect(x + 24, y + T - 40, T - 48, 20).fill({ color: 0x000000, alpha: 0.5 })
    g.rect(x + 24, y + T - 40, (T - 48) * Math.min(1, frac), 20).fill({ color: couleur })
  }

  tas(g, x, y, couleur, qte) {
    const r = 16 + Math.min(20, qte * 6)
    g.circle(x, y, r).fill({ color: couleur }).stroke({ color: 0x000000, width: 4, alpha: 0.6 })
  }
}
