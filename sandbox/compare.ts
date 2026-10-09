// Every strategy, for each caster, side by side.
//
//   npx tsx sandbox/compare.ts [hold|throw] [ticks]

import { STRATEGIES } from './strategies.ts'
import { DEFAULT_SETUP, play, type Kind } from './scenario.ts'
import type { CasterKind } from './mind.ts'

const kind = (process.argv[2] ?? 'hold') as Kind
const ticks = Number(process.argv[3] ?? (kind === 'hold' ? 40 : 120))
const pct = (v: number) => `${(v * 100).toFixed(0).padStart(4)}%`
const num = (v: number, d = 1) => v.toFixed(d).padStart(7)

console.log(
  kind === 'hold'
    ? `Holding a fireball of ${DEFAULT_SETUP.amount} M, ${DEFAULT_SETUP.radius} m, for ${ticks} ticks`
    : `Throwing a fireball of ${DEFAULT_SETUP.amount} M at ${DEFAULT_SETUP.speed} m/tick into the wall`,
)
for (const caster of ['adept', 'master'] as CasterKind[]) {
  console.log(`\n${caster}`)
  console.log(
    kind === 'hold'
      ? '  strategy                           held  spread    mana  beats/t  pushes/t'
      : '  strategy                        let go   held   at wall    mana  beats/t',
  )
  for (const s of STRATEGIES) {
    if (kind === 'hold' && s.throwOnly) continue
    const r = play({ ...DEFAULT_SETUP, kind, strategy: s.id, caster }, ticks)
    const name = s.name.padEnd(32)
    if (kind === 'hold') {
      console.log(`  ${name} ${pct(r.held)} ${num(r.spread, 2)} ${num(r.spent)} ${num(r.beatsPerTick, 0)} ${num(r.pushesPerTick)}`)
    } else {
      const letGo = r.letGoAt < 0 ? '      -' : `${String(r.letGoAt).padStart(5)} t`
      const hit = r.hitAt < 0 ? '  never' : `${pct(r.held)} t${r.hitAt}`
      console.log(`  ${name} ${letGo} ${pct(r.heldAtLetGo)}  ${hit.padStart(9)} ${num(r.spent)} ${num(r.beatsPerTick, 0)}`)
    }
    if (r.manaError > 1e-6 || r.momentumError > 1e-6) {
      console.log(`    ledger off: mana ${r.manaError.toExponential(2)}, momentum ${r.momentumError.toExponential(2)}`)
    }
  }
}
