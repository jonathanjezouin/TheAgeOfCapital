import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { viteSingleFile } from 'vite-plugin-singlefile'

// Un seul fichier HTML (JS et CSS inclus) ; l'atlas de tiles et le moteur WebAssembly restent des
// fichiers voisins, chargés par chemins relatifs : la page marche dans un dossier, un Artifact ou Electron.
export default defineConfig({
  base: './',
  plugins: [vue(), viteSingleFile()],
  build: { target: 'es2022', chunkSizeWarningLimit: 4000 },
})
