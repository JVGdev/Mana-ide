#!/usr/bin/env -S npx tsx
// bench: every way of holding and throwing a ball of fire, on the real machine, side by side (src/bench.ts).
//
//   npm run bench                     2D, an adept and a master, five layouts each
//   npm run bench -- --3d             3D too (slow)
//   npm run bench -- --only Throw     variants whose name has this in it
//   npm run bench -- --json out.json  every result, for a report

import { writeFileSync } from 'node:fs'
import { CASTERS, LAYOUTS, VARIANTS, orderSize, runVariant, summarise, type CasterName, type Result } from '../bench.ts'

const args = process.argv.slice(2)
const flag = (name: string) => (args.includes(name) ? args[args.indexOf(name) + 1] : undefined)
const only = flag('--only')
const dims: (2 | 3)[] = args.includes('--3d') ? [2, 3] : [2]
const variants = VARIANTS.filter((v) => !only || v.id.includes(only))

const all: Result[] = []
const pct = (x: number) => `${(100 * x).toFixed(0)}%`.padStart(5)
const num = (x: number, d = 2) => x.toFixed(d).padStart(6)

for (const d of dims)
  for (const kind of ['hold', 'throw'] as const) {
    const vs = variants.filter((v) => v.kind === kind)
    if (!vs.length) continue
    console.log(`\n${kind === 'hold' ? 'Holding a ball still' : 'Throwing a ball at the pillar'}, ${d}D (mean of ${LAYOUTS.length} layouts)\n`)
    console.log(
      kind === 'hold'
        ? 'variant          caster  order ingrain  kept  spread  hand M  order M  burned  beats/tick'
        : 'variant          caster  order ingrain arrived  ticks  togeth  kept  spread  hand M  order M  burned',
    )
    for (const v of vs) {
      const size = orderSize(v)
      for (const c of Object.keys(CASTERS) as CasterName[]) {
        const rs = LAYOUTS.map((l) => runVariant(v, c, d, l))
        all.push(...rs)
        const s = summarise(rs)
        const head = `${v.id.padEnd(16)} ${c.padEnd(7)} ${String(size || '-').padStart(5)} ${num(s.ingrain, 1)}`
        console.log(
          kind === 'hold'
            ? `${head} ${pct(s.kept)}  ${num(s.spread)}  ${num(s.push)}   ${num(s.kick)}  ${num(s.burn)}   ${num(s.handBeats, 0)}`
            : `${head}  ${pct(s.arrived)}  ${num(s.ticks, 1)}  ${pct(s.together)}  ${pct(s.kept)}  ${num(s.spread)}  ${num(s.push)}   ${num(s.kick)}  ${num(s.burn)}`,
        )
      }
    }
  }

const out = flag('--json')
if (out) writeFileSync(out, JSON.stringify({ variants: VARIANTS.map((v) => ({ ...v, order: orderSize(v) })), results: all }, null, 1))
