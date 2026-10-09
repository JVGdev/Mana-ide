// Where a cast's thought went: by routine, and by line. What to look at first when making a spell faster.

import type { Program, SourceLine } from './asm/assembler.ts'
import type { Cast } from './vm/sim.ts'

export type RoutineCost = { name: string; beats: number; share: number }
export type LineCost = { addr: number; source?: SourceLine; runs: number; beats: number; share: number }
export type Profile = { beats: number; routines: RoutineCost[]; lines: LineCost[] }

/** The routine an address is in: the last global label at or before it. */
function routines(program: Program): (addr: number) => string {
  const globals = [...program.labels].filter(([name]) => !name.includes('.')).sort((a, b) => a[1] - b[1])
  return (addr) => {
    let name = '?'
    for (const [n, a] of globals) if (a <= addr) name = n
    return name
  }
}

export function profile(cast: Cast): Profile {
  const routineOf = routines(cast.program)
  const byRoutine = new Map<string, number>()
  const lines: LineCost[] = []
  for (const [addr, { runs, beats }] of cast.profile) {
    const name = routineOf(addr)
    byRoutine.set(name, (byRoutine.get(name) ?? 0) + beats)
    lines.push({ addr, source: cast.program.lines.get(addr), runs, beats, share: beats / cast.beats })
  }
  return {
    beats: cast.beats,
    routines: [...byRoutine]
      .map(([name, beats]) => ({ name, beats, share: beats / cast.beats }))
      .sort((a, b) => b.beats - a.beats),
    lines: lines.sort((a, b) => b.beats - a.beats),
  }
}

/** A plain-text report: the routines, then the costliest lines. */
export function report(cast: Cast, top = 12): string {
  const p = profile(cast)
  const pct = (x: number) => `${(x * 100).toFixed(1).padStart(5)}%`
  const out = [`${cast.name}: ${p.beats} beats over ${(cast.endedAt ?? 0) - cast.startedAt + 1} ticks`, '', 'routines:']
  for (const r of p.routines) out.push(`  ${String(r.beats).padStart(7)}  ${pct(r.share)}  ${r.name}`)
  out.push('', 'lines:')
  for (const l of p.lines.slice(0, top)) {
    const where = l.source ? `${l.source.file}:${l.source.line}`.padEnd(22) : ''.padEnd(22)
    out.push(`  ${String(l.beats).padStart(7)}  ${pct(l.share)}  ×${String(l.runs).padEnd(6)} ${where} ${l.source?.text.trim() ?? ''}`)
  }
  return out.join('\n')
}
