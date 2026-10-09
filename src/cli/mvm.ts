#!/usr/bin/env -S npx tsx
// mvm: cast a spell in a test world and watch it in the terminal.
//
//   mvm spells/StoneWall.masm                 the scene named after the spell, in 2D
//   mvm spells/Fireball.masm --ticks 30 --every 3
//   mvm spells/Gust.masm --scene Gust --3d    (3D runs; the terminal shows one slice)

import { basename } from 'node:path'
import { AsmError } from '../asm/assembler.ts'
import { assembleFile } from '../load.ts'
import { render } from '../render.ts'
import { SCENES } from '../scenes.ts'

const args = process.argv.slice(2)
const flag = (name: string) => (args.includes(name) ? args[args.indexOf(name) + 1] : undefined)
const file = args.find((a, i) => !a.startsWith('-') && !args[i - 1]?.startsWith('--'))
if (!file) {
  console.error('usage: mvm <spell.masm> [--scene StoneWall|Fireball|Gust|WaterShield] [--3d] [--ticks n] [--every n] [--maintain n]')
  process.exit(2)
}

let program
try {
  program = assembleFile(file)
} catch (e) {
  if (e instanceof AsmError) {
    for (const p of e.problems) console.error(p)
    process.exit(1)
  }
  throw e
}

const sceneName = (flag('--scene') ?? basename(file, '.masm')) as keyof typeof SCENES
const make = SCENES[sceneName]
if (!make) {
  console.error(`no scene called ${sceneName}: try ${Object.keys(SCENES).join(', ')}`)
  process.exit(2)
}
const scene = make(args.includes('--3d') ? 3 : 2)
const { sim, caster } = scene
const ticks = Number(flag('--ticks') ?? 40)
const every = Number(flag('--every') ?? 5)
const maintain = Number(flag('--maintain') ?? 12)

const before = sim.ledger().total
sim.cast(caster, program)
for (let t = 0; t <= ticks; t++) {
  if (t % every === 0 || t === ticks) {
    console.log(`tick ${sim.tick}   load ${caster.held().toFixed(0)} / ${caster.capacity.toFixed(0)} M   harm ${caster.harm.toFixed(0)}`)
    console.log(render(sim))
  }
  if (t === maintain) caster.will.maintain = false
  sim.step()
}

console.log('\nevents:')
for (const e of sim.events) console.log(`  ${String(e.tick).padStart(4)}  ${e.kind.padEnd(10)} ${e.weave ? `weave ${e.weave}  ` : ''}${e.detail ?? ''}`)
const l = sim.ledger()
console.log(`\nledger: free ${l.free.toFixed(2)} + condensed ${l.condensed.toFixed(2)} = ${l.total.toFixed(2)} M  (was ${before.toFixed(2)})`)
