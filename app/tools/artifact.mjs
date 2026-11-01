// Prépare la page pour un Artifact : un fragment HTML (sans <html>/<head>/<body>) avec JS et CSS inclus,
// plus les fichiers voisins (moteur WebAssembly, atlas). Sortie : dist/artifact/
//   node tools/artifact.mjs
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from 'node:fs'

const src = readFileSync('dist/index.html', 'utf8')
const unique = (re, nom) => {
  const m = src.match(re)
  if (!m) throw new Error(`${nom} introuvable dans dist/index.html (lancez d'abord « npm run build »)`)
  return m[1]
}
const titre = unique(/<title>([^<]*)<\/title>/, 'titre')
const css = unique(/<style[^>]*>([\s\S]*?)<\/style>/, 'style')
const js = unique(/<script type="module"[^>]*>([\s\S]*?)<\/script>/, 'script')

const fragment = `<title>${titre}</title>
<style>${css}</style>
<div id="app"></div>
<script type="module">${js}</script>
`
mkdirSync('dist/artifact/assets', { recursive: true })
writeFileSync('dist/artifact/index.html', fragment)
copyFileSync('public/sim_wasm.wasm', 'dist/artifact/sim_wasm.wasm')
for (const f of ['atlas.png', 'atlas.json']) copyFileSync(`public/assets/${f}`, `dist/artifact/assets/${f}`)
console.log(`dist/artifact/index.html : ${(fragment.length / 1024).toFixed(0)} Ko`)
