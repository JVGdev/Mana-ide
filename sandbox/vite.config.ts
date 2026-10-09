// The mana fluid sandbox, as a page: npm run sandbox.

import { defineConfig } from 'vite'
import { fileURLToPath } from 'node:url'

export default defineConfig({
  root: fileURLToPath(new URL('.', import.meta.url)),
  base: './',
  server: { port: 5176 },
  build: { outDir: 'dist', emptyOutDir: true },
})
