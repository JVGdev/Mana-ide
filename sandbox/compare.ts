// Every strategy, for each caster, side by side. Each is run on several layouts of the same ball (seeds), and shows the
// mean, with the lowest and highest in brackets where they differ by much.
//
//   npx tsx sandbox/compare.ts [hold|throw] [ticks] [seeds]

import { STRATEGIES } from './strategies.ts'
import { DEFAULT_SETUP, play, type Kind, type Result } from './scenario.ts'
import type { CasterKind } from './mind.ts'

const kind = (process.argv[2] ?? 'hold') as Kind
const ticks = Number(process.argv[3] ?? (kind === 'hold' ? 40 : 160))
const seeds = Number(process.argv[4] ?? 5)

function stat(rs: Result[], f: (r: Result) => number, show: (v: number) => string, width: number): string {
  const vs = rs.map(f)
  const mean = vs.reduce((a, b) => a + b, 0) / vs.length
  const lo = Math.min(...vs)
  const hi = Math.max(...vs)
  const range = hi - lo > Math.abs(mean) * 0.1 + 1e-9 ? ` (${show(lo)}–${show(hi)})` : ''
  return `${show(mean)}${range}`.padStart(width)
}
const pct = (v: number) => `${(v * 100).toFixed(0)}%`
const fix = (d: number) => (v: number) => v.toFixed(d)

console.log(
  kind === 'hold'
    ? `Holding a fireball of ${DEFAULT_SETUP.amount} M, ${DEFAULT_SETUP.radius} m, for ${ticks} ticks, ${seeds} layouts`
    : `Throwing a fireball of ${DEFAULT_SETUP.amount} M at ${DEFAULT_SETUP.speed} m/tick into the wall, ${seeds} layouts`,
)
for (const caster of ['adept', 'master'] as CasterKind[]) {
  console.log(`\n${caster}`)
  console.log(
    kind === 'hold'
      ? `  ${'strategy'.padEnd(36)}${'spread'.padStart(18)}${'held'.padStart(16)}${'mana'.padStart(20)}${'beats/t'.padStart(9)}`
      : `  ${'strategy'.padEnd(36)}${'let go'.padStart(16)}${'at wall'.padStart(18)}${'mana'.padStart(8)}`,
  )
  for (const s of STRATEGIES) {
    if (kind === 'hold' && s.throwOnly) continue
    const rs: Result[] = []
    for (let seed = 1; seed <= seeds; seed++) rs.push(play({ ...DEFAULT_SETUP, kind, strategy: s.id, caster, seed }, ticks))
    const name = s.name.padEnd(36)
    if (kind === 'hold') {
      console.log(
        `  ${name}${stat(rs, (r) => r.spread, fix(2), 18)}${stat(rs, (r) => r.held, pct, 16)}${stat(rs, (r) => r.spent, fix(1), 20)}${stat(rs, (r) => r.beatsPerTick, fix(0), 9)}`,
      )
    } else {
      console.log(
        `  ${name}${stat(rs, (r) => r.letGoAt, fix(0), 16)}${stat(rs, (r) => r.held, pct, 18)}${stat(rs, (r) => r.spent, fix(1), 8)}`,
      )
    }
    const bad = rs.find((r) => r.manaError > 1e-6 || r.momentumError > 1e-6)
    if (bad) console.log(`    ledger off: mana ${bad.manaError.toExponential(2)}, momentum ${bad.momentumError.toExponential(2)}`)
  }
}
