// A 2D slice of the world as text, for the terminal.
//
//   #  earth            H  earth a weave holds      @  the caster
//   :  a little earth   W  water a weave holds      &  another body
//   ~  water            *  fire mana in a weave     .  loose mana
//   ^  flame            o  water mana in a weave    =  air mana in a weave
//                       +  earth mana in a weave

import type { Sim } from './vm/sim.ts'
import { dominant, total } from './vm/parts.ts'
import { EARTH, FIRE, WATER } from './vm/world.ts'

export function render(sim: Sim, z?: number): string {
  const w = sim.world
  const slice = z ?? Math.floor((sim.casters[0]?.body.pos[2] ?? 0) / w.cell)
  const grid: string[][] = Array.from({ length: w.h }, () => Array(w.w).fill(' '))
  const put = (x: number, y: number, ch: string) => {
    if (x >= 0 && y >= 0 && x < w.w && y < w.h) grid[w.h - 1 - y][x] = ch
  }

  for (let y = 0; y < w.h; y++)
    for (let x = 0; x < w.w; x++) {
      const m = w.matter[w.index(x, y, slice)]
      const t = total(m)
      if (t < 5) continue
      const k = dominant(m)
      put(x, y, k === EARTH ? (t >= 40 ? '#' : ':') : k === WATER ? '~' : k === FIRE ? '^' : ' ')
    }
  // Mana: the matter weaves hold, cell by cell, then the mana itself.
  const held = new Map<number, number[]>()
  for (const p of w.particles) {
    const i = w.cellOf(p.pos)
    if (i < 0 || w.coords(i)[2] !== slice) continue
    const e = held.get(i) ?? [0, 0, 0, 0, 0, 0, 0, 0]
    for (let k = 0; k < 4; k++) {
      e[k] += p.carried[k]
      e[4 + k] += p.weave ? p.free[k] : 0
    }
    held.set(i, e)
    if (!p.weave) {
      const [x, y] = w.coords(i)
      put(x, y, '.')
    }
  }
  for (const [i, e] of held) {
    const [x, y] = w.coords(i)
    const carried = e.slice(0, 4) as [number, number, number, number]
    const free = e.slice(4) as [number, number, number, number]
    if (total(carried) >= 5) put(x, y, dominant(carried) === WATER ? 'W' : 'H')
    else if (total(free) > 0.01) put(x, y, ['*', 'o', '=', '+'][dominant(free)])
  }
  for (const b of w.bodies) {
    const [x, y, zz] = w.coords(w.clampedCellOf(b.pos))
    if (zz !== slice) continue
    const ch = sim.casters.some((c) => c.body === b) ? '@' : '&'
    put(x, y, ch)
    put(x, y + 1, ch)
    put(x, y - 1, ch)
  }
  const edge = '+' + '-'.repeat(w.w) + '+'
  return [edge, ...grid.map((row) => '|' + row.join('') + '|'), edge].join('\n')
}
