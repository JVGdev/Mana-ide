// The sandbox as one self-contained HTML file, its script inlined, to open anywhere or publish.
//
//   npx tsx sandbox/bundle.ts [out.html]

import { build } from 'esbuild'
import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const here = (p: string) => fileURLToPath(new URL(p, import.meta.url))
const out = process.argv[2] ?? here('./dist/sandbox.html')

const js = await build({
  entryPoints: [here('./main.ts')],
  bundle: true,
  format: 'iife',
  target: 'es2022',
  minify: true,
  write: false,
})
const script = js.outputFiles[0].text.replace(/<\/script/gi, '<\\/script')
const html = readFileSync(here('./index.html'), 'utf8').replace(
  '<script type="module" src="./main.ts"></script>',
  () => `<script>${script}</script>`,
)
writeFileSync(out, html)
console.log(`${out}: ${(html.length / 1024).toFixed(0)} KB`)
