// The bench (PLAN step R5): every variant, for an adept and a master, measured as the TypeScript bench measures it.
// engine/mana/tests/parity_bench.rs runs the same ones.

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { VARIANTS, orderSize, runVariant } from '../../src/bench.ts'

const out = join(import.meta.dirname, '..', 'target', 'parity')
mkdirSync(out, { recursive: true })
const results = []
for (const v of VARIANTS)
  for (const c of ['adept', 'master'] as const)
    for (const layout of [0, 2, 4]) results.push({ ...runVariant(v, c, 2, layout), ms: 0, order: orderSize(v) })
for (const id of ['HoldCling', 'ThrowCohere']) {
  const v = VARIANTS.find((x) => x.id === id)!
  results.push({ ...runVariant(v, 'adept', 3, 0), ms: 0, order: orderSize(v) })
}
writeFileSync(join(out, 'bench.json'), JSON.stringify(results))
console.log(`${results.length} bench runs`)
