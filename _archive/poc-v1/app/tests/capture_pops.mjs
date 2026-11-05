// Validation visuelle des pops : inspecteur, marche sur les routes, mode « Flux ».
//   node tests/capture_pops.mjs [dossier dist] [dossier de sortie]
import { createRequire } from 'node:module'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join, resolve } from 'node:path'
import { mkdirSync } from 'node:fs'

const { chromium } = createRequire(import.meta.url)('playwright')
const dist = resolve(process.argv[2] ?? 'dist')
const sortie = resolve(process.argv[3] ?? '/tmp/captures-pops')
mkdirSync(sortie, { recursive: true })

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.png': 'image/png', '.json': 'application/json', '.wasm': 'application/wasm', '.css': 'text/css' }
const serveur = createServer(async (req, rep) => {
  try {
    const chemin = join(dist, decodeURIComponent(req.url.split('?')[0]).replace(/^\/$/, '/index.html'))
    rep.writeHead(200, { 'content-type': TYPES[extname(chemin)] ?? 'application/octet-stream' })
    rep.end(await readFile(chemin))
  } catch { rep.writeHead(404); rep.end() }
}).listen(0)
const port = serveur.address().port

const navigateur = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--no-sandbox'] })
const page = await navigateur.newPage({ viewport: { width: 1440, height: 860 } })
const erreurs = []
page.on('pageerror', (e) => erreurs.push('pageerror: ' + e.message))
page.on('console', (m) => { if (m.type() === 'error') erreurs.push('console: ' + m.text()) })
await page.goto(`http://localhost:${port}/`)
await page.waitForFunction(() => window.__store?.pret === true || window.__store?.erreur, null, { timeout: 60000 })
const attendre = (ms) => page.waitForTimeout(ms)
const photo = (nom) => page.screenshot({ path: join(sortie, nom) })
const evalu = (f, a) => page.evaluate(f, a)

await evalu(() => { window.__store.enPause = true })
await evalu(() => window.__sim.avancer(365 * 12))
await evalu(() => window.__rafraichir())
await evalu(() => window.__carte.centrer(window.__store.carte))
await attendre(500)
await photo('1_vue_generale.png')

// Heure du jour : le matin, des pops marchent sur les routes.
const regle = async (h) => {
  await evalu((h) => { const s = window.__store; s.vitesse = 1 / 24; s.heure = h; window.__carte.rafraichir(s.etat) }, h)
  await attendre(250)
}
// Zoom sur un hameau qui a des pops.
const hameau = await evalu(() => window.__store.etat.hameaux.filter((h) => h.pops > 0).sort((a, b) => b.pops - a.pops)[0])
const zoomSur = async (x, y, z) => {
  await evalu(([x, y, z]) => {
    const c = window.__carte, { width, height } = c.app.screen
    c.monde.scale.set(z); c.monde.position.set(width / 2 - (x + 0.5) * 256 * z, height / 2 - (y + 0.5) * 256 * z)
  }, [x, y, z])
  await attendre(300)
}
await zoomSur(hameau.x, hameau.y, 0.32)
for (const h of [3, 6.8, 7.4, 12, 17.2, 17.9]) {
  await regle(h)
  await photo(`2_heure_${String(h).replace('.', '_')}.png`)
}

// Inspecteur d'une pop qui travaille.
await regle(12)
const pop = await evalu(() => window.__store.etat.pops.find((p) => p.emploi >= 0 && p.motif === 'besoin'))
console.log('pop choisie', JSON.stringify(pop))
await evalu((id) => { window.__store.selectionPop = id }, pop.id)
await attendre(400)
await evalu(() => window.__carte.centrer(window.__store.carte))
await attendre(400)
await photo('3_inspecteur_pop.png')
const detail = await evalu(() => window.__store.popDetail)
console.log('candidats', detail.candidats.map((c) => `${c.type_nom}@${c.x},${c.y} ${c.statut} ${c.classe}`).join(' | '))

// Mode « Flux » sur le grain, puis sur le bois.
await evalu(() => { const s = window.__store; s.selectionPop = null; s.selection = null; s.mode = 'ressource'; s.ressource = 0 })
await attendre(500)
await photo('4_flux_grain.png')
await evalu(() => { window.__store.ressource = 3 })
await attendre(500)
await photo('5_flux_bois.png')
console.log('erreurs :', erreurs.length ? erreurs.join('\n') : 'aucune')
await navigateur.close()
serveur.close()
