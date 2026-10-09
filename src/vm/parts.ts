// Mana is four parts (the Law of Equality). Every amount of it, free or condensed, is four numbers.
// The machine only knows them as 0–3; the Elements library calls them fire, water, air and earth.

export type Parts = [number, number, number, number]

export const zero = (): Parts => [0, 0, 0, 0]

export const total = (p: Parts): number => p[0] + p[1] + p[2] + p[3]

/** Adds `b` into `a`. */
export function add(a: Parts, b: Parts): Parts {
  a[0] += b[0]
  a[1] += b[1]
  a[2] += b[2]
  a[3] += b[3]
  return a
}

/** Moves up to `amount` out of `from`, keeping its proportions, and returns what was moved. */
export function take(from: Parts, amount: number): Parts {
  const t = total(from)
  if (t <= 0 || amount <= 0) return zero()
  if (amount >= t) {
    const all: Parts = [from[0], from[1], from[2], from[3]]
    from.fill(0)
    return all
  }
  const f = amount / t
  const out: Parts = [from[0] * f, from[1] * f, from[2] * f, from[3] * f]
  from[0] -= out[0]
  from[1] -= out[1]
  from[2] -= out[2]
  from[3] -= out[3]
  return out
}

/** Moves a share (0–1) of each part out of `from`. */
export function share(from: Parts, f: number): Parts {
  return take(from, total(from) * Math.max(0, Math.min(1, f)))
}

/** Which part a mix is mostly made of. */
export function dominant(p: Parts): number {
  let best = 0
  for (let k = 1; k < 4; k++) if (p[k] > p[best]) best = k
  return best
}
