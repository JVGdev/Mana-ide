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
  for (const p of w.loose) {
    const [x, y, zz] = w.coords(w.clampedCellOf(p.pos))
    if (zz === slice) put(x, y, '.')
  }
  for (const weave of sim.weaves.values())
    for (const c of weave.cells) {
      const i = w.cellOf(weave.worldPos(c))
      if (i < 0) continue
      const [x, y, zz] = w.coords(i)
      if (zz !== slice) continue
      if (total(c.carried) >= 0.2) put(x, y, dominant(c.carried) === WATER ? 'W' : 'H')
      else if (total(c.free) > 0.01) put(x, y, ['*', 'o', '=', '+'][dominant(c.free)])
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
