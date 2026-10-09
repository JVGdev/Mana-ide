// Numbers for people.

export function num(v: number, digits = 3): string {
  if (!Number.isFinite(v)) return String(v)
  if (v === 0) return '0'
  const a = Math.abs(v)
  if (a >= 10000) return v.toFixed(0)
  if (a >= 100) return v.toFixed(1).replace(/\.0$/, '')
  return Number(v.toFixed(digits)).toString()
}

export const PART_NAMES = ['fire', 'water', 'air', 'earth'] as const
export const PART_COLORS = ['#ff8a3d', '#4fa3ff', '#d9ecff', '#d2a865'] as const
