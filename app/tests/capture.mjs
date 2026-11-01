// Validation visuelle : ouvre l'application dans Chromium (sans écran) et enregistre des captures.
//   node tests/capture.mjs [dossier dist] [dossier de sortie]
import { createRequire } from 'node:module'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { extname, join, resolve } from 'node:path'
import { mkdirSync } from 'node:fs'

const { chromium } = createRequire(import.meta.url)('playwright')
const dist = resolve(process.argv[2] ?? 'dist')
const sortie = resolve(process.argv[3] ?? '/tmp/captures')
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
const pause = (v) => page.evaluate((x) => { window.__store.enPause = x }, v)
const avancer = (jours) => page.evaluate((j) => { window.__sim.avancer(j); }, jours)

export { page, attendre, pause, avancer, sortie, erreurs, navigateur, serveur }

// --- scénario ---
const cliquer = async (x, y) => {
  const p = await page.evaluate(([x, y]) => window.__carte.positionEcran(x, y), [x, y])
  await page.mouse.click(p.x, p.y)
  await attendre(250)
}
const trouver = (pred) => page.evaluate((src) => {
  const f = new Function('e', 'm', `return (${src})`)
  const e = window.__store.etat
  return f(e, window.__store.meta)
}, pred)
const photo = (nom) => page.screenshot({ path: join(sortie, nom) })
const mode = async (m) => { await page.evaluate((x) => { window.__store.mode = x }, m); await attendre(300) }
const zoomSur = async (x, y, z) => {
  await page.evaluate(([x, y, z]) => {
    const c = window.__carte, { width, height } = c.app.screen
    c.monde.scale.set(z); c.monde.position.set(width / 2 - (x + 0.5) * 256 * z, height / 2 - (y + 0.5) * 256 * z)
  }, [x, y, z])
  await attendre(300)
}

await pause(true)
await attendre(400)
await photo('1_depart.png')

// Un hameau, une terre : on les sélectionne.
const hameau = await trouver('e.hameaux.find(h => h.habitat === "hameau_textile")')
await cliquer(hameau.x, hameau.y)
await photo('2_selection_hameau.png')
const champ = await trouver('e.lieux.find(l => l.type_nom === "champ")')
await cliquer(champ.x, champ.y)
await photo('3_selection_champ.png')

// 120 ans plus tard : le village s'est étendu (chantiers, nouveaux hameaux).
await avancer(365 * 120)
await page.evaluate(() => window.__rafraichir())
await page.evaluate(() => window.__carte.centrer(window.__store.carte))
await attendre(500)
await photo('4_apres_120_ans.png')
await mode('fertilite'); await photo('5_mode_fertilite.png')
await mode('besoins'); await photo('6_mode_besoins.png')
await mode('normal')

// Un chantier en cours (on avance jusqu'à en trouver un).
for (let i = 0; i < 200 && !(await trouver('e.lieux.some(l => l.chantier)')); i++) { await avancer(30); await page.evaluate(() => window.__rafraichir()) }
const chantier = await trouver('e.lieux.find(l => l.chantier)')
if (chantier) { await zoomSur(chantier.x, chantier.y, 0.45); await cliquer(chantier.x, chantier.y); await photo('7_chantier.png') }

// Nuit, de près : les pops rentrent chez elles.
const h0 = await trouver('e.hameaux.find(h => h.capacite > 0)')
await zoomSur(h0.x, h0.y, 0.5)
await page.evaluate(() => { window.__store.vitesse = 1 / 24; window.__store.heure = 23; })
await page.evaluate(() => { window.__carte.rafraichir(window.__store.etat) })
await attendre(300)
await photo('8_nuit.png')

const etat = await page.evaluate(() => ({ jour: window.__store.etat.jour, pops: window.__store.etat.nb_pops, ev: window.__store.evenements.length, hameaux: window.__store.etat.hameaux.length }))
console.log('état :', JSON.stringify(etat), '| erreurs :', erreurs.length ? erreurs : 'aucune')
await navigateur.close()
serveur.close()
