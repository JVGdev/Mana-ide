// The world's state, as the parity checks compare it: particles, bonds, bodies, impulses and heat in full, and the grids
// as hashes of their exact bits.

import type { World } from '../../src/vm/world.ts'

const view = new DataView(new ArrayBuffer(8))

/** FNV-1a over 32-bit words of each number's exact bits. */
export function hash(xs: ArrayLike<number>): string {
  let h = 2166136261
  for (let i = 0; i < xs.length; i++) {
    view.setFloat64(0, xs[i], true)
    h = Math.imul(h ^ view.getUint32(0, true), 16777619) >>> 0
    h = Math.imul(h ^ view.getUint32(4, true), 16777619) >>> 0
  }
  return h.toString(16).padStart(8, '0')
}

const flat = (cells: number[][]) => cells.flat()

export function state(w: World, full = true) {
  const particles = w.particles.map((p) => [
      p.id, ...p.pos, ...p.vel, ...p.free, ...p.carried, p.weave, p.order ? p.order.addr : -1, p.yaw, p.pushedAt, p.touchedAt,
      p.rho, p.felt, ...p.grad, ...p.nvel, p.rhoM, p.mass, ...p.regs, ...p.stamp,
    ])
  const bonds = w.bonds.map((b) => [b.a.id, b.b.id, b.rest])
  return {
    tick: w.tick,
    // Big worlds are compared by hash: the same numbers, in the same order, to the last bit.
    particles: full ? particles : hash(particles.flat()),
    bonds: full ? bonds : hash(bonds.flat()),
    bodies: w.bodies.map((b) => [b.id, ...b.pos, ...b.vel, b.mass]),
    impulse: [...w.impulse.gravity, ...w.impulse.walls, ...w.impulse.outside],
    heat: Object.entries(w.heat),
    air: hash(flat(w.air)),
    airVel: hash(w.airVel),
    matter: hash(flat(w.matter)),
  }
}
