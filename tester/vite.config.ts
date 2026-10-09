// The spell tester. In development it reads and saves the .masm files in lib/ and spells/ directly;
// built, it carries copies of them and keeps your edits in the browser.

import { defineConfig, type Plugin } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath } from 'node:url'
import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, normalize } from 'node:path'

const root = fileURLToPath(new URL('.', import.meta.url))
const repo = join(root, '..')
const DIRS = ['spells', 'lib']

/** Only .masm files directly in spells/ or lib/. */
function safe(path: string): string | undefined {
  const clean = normalize(path).replace(/\\/g, '/')
  const [dir, name, ...rest] = clean.split('/')
  if (rest.length || !DIRS.includes(dir) || !/^[A-Za-z0-9_-]+\.masm$/.test(name ?? '')) return undefined
  return join(repo, dir, name)
}

function files(): Plugin {
  return {
    name: 'mana-files',
    configureServer(server) {
      server.middlewares.use('/api/files', (req, res) => {
        if (req.method === 'GET') {
          const out = DIRS.flatMap((dir) =>
            readdirSync(join(repo, dir))
              .filter((f) => f.endsWith('.masm'))
              .map((f) => ({ path: `${dir}/${f}`, text: readFileSync(join(repo, dir, f), 'utf8') })),
          )
          res.setHeader('content-type', 'application/json')
          res.end(JSON.stringify(out))
          return
        }
        if (req.method === 'PUT') {
          const path = safe(decodeURIComponent((req.url ?? '').replace(/^\//, '')))
          if (!path) {
            res.statusCode = 400
            res.end('only .masm files in spells/ or lib/')
            return
          }
          let body = ''
          req.setEncoding('utf8')
          req.on('data', (chunk) => (body += chunk))
          req.on('end', () => {
            writeFileSync(path, body)
            res.end('saved')
          })
          return
        }
        res.statusCode = 405
        res.end()
      })
    },
  }
}

export default defineConfig({
  root,
  base: './',
  plugins: [svelte(), files()],
  server: { port: 5175, fs: { allow: [repo] } },
  build: { outDir: 'dist', emptyOutDir: true, chunkSizeWarningLimit: 1500 },
})
