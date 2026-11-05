// Carte PixiJS : sol de plaine, un champ, une maison. Vue de dessus, une tile = 256 unités du monde.
import 'pixi.js/unsafe-eval'
import { Application, Assets, Container, Rectangle, Sprite, Texture } from 'pixi.js'

const T = 256
const MARGE = 1 // tiles vides autour du monde

export class Carte {
  async init(parent, urlAtlasPng, urlAtlasJson) {
    this.app = new Application()
    await this.app.init({ resizeTo: parent, background: 0x6c7a3a, antialias: true, resolution: Math.min(devicePixelRatio || 1, 2), autoDensity: true })
    parent.appendChild(this.app.canvas)
    const atlas = await Assets.load(urlAtlasPng)
    const man = await (await fetch(urlAtlasJson)).json()
    this.tex = (nom) => {
      const f = man.frames[nom]
      return new Texture({ source: atlas.source, frame: new Rectangle(f.x, f.y, f.w, f.h) })
    }
    this.monde = new Container()
    this.app.stage.addChild(this.monde)
    this.app.renderer.on('resize', () => this.cadrer())
  }

  /** (Re)construit la scène à partir d'un instantané. */
  dessiner(etat) {
    this.etat = etat
    this.monde.removeChildren()
    const hauteur = Math.ceil(etat.tiles.length / etat.largeur)
    this.dims = { l: etat.largeur, h: hauteur }
    for (let y = -MARGE; y < hauteur + MARGE; y++) {
      for (let x = -MARGE; x < etat.largeur + MARGE; x++) {
        const s = new Sprite(this.tex(`terrain_plaine_0${((Math.imul(x + 9, 73856093) ^ Math.imul(y + 9, 19349663)) >>> 0) % 10}`))
        s.position.set(x * T, y * T)
        this.monde.addChild(s)
      }
    }
    for (const t of etat.tiles) {
      if (t.type === 'champ') {
        const s = new Sprite(this.tex('bat_champ_0'))
        s.position.set(t.x * T, t.y * T)
        this.monde.addChild(s)
      } else if (t.type === 'habitation') {
        const s = new Sprite(this.tex('bat_maison_bois_0'))
        s.anchor.set(0.5)
        s.scale.set(2) // 128 px -> 256 : la maison occupe la tile
        s.position.set((t.x + 0.5) * T, (t.y + 0.5) * T)
        this.monde.addChild(s)
      }
    }
    this.cadrer()
  }

  cadrer() {
    if (!this.dims) return
    const { l, h } = this.dims
    const w = this.app.screen.width, hh = this.app.screen.height
    const k = Math.min(w / ((l + 2) * T), hh / ((h + 2) * T))
    this.monde.scale.set(k)
    this.monde.position.set((w - l * T * k) / 2, (hh - h * T * k) / 2)
  }
}
