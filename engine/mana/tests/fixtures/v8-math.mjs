// V8's answers to the math the engine uses, as raw bits: `node v8-math.mjs > v8-math.txt`. tests/v8_math.rs checks that
// the engine's math (src/js) gives the same, to the last bit.
//
// Run it with an official Node build (nodejs.org), or anything built for plain x86-64. A Node compiled for a newer CPU
// (CachyOS's x86-64-v4 packages, say) lets the C compiler fuse V8's multiply-adds, and its sines and logs come out a bit
// different from a browser's.
const view = new DataView(new ArrayBuffer(8))
const bits = (x) => {
  view.setFloat64(0, x)
  return view.getBigUint64(0).toString(16).padStart(16, '0')
}
let seed = 20261009
const rnd = () => {
  seed = (seed * 1103515245 + 12345) % 2147483648
  return seed / 2147483648
}
// Ordinary sizes, angles, particle ids, huge arguments, and the edges.
const kinds = [
  () => (rnd() - 0.5) * 10 ** (rnd() * 8 - 3),
  () => (rnd() - 0.5) * 4 * Math.PI,
  () => Math.floor(rnd() * 2e6),
  () => (rnd() - 0.5) * 10 ** (rnd() * 300),
  () => (rnd() - 0.5) * 10 ** (-rnd() * 300),
]
const special = [0, -0, 1, -1, 0.5, Math.PI / 2, Math.PI, Math.PI / 4, 1e-300, 5e-324, 1e300, Infinity, -Infinity, NaN, 709.78, -745.1, 2 ** 19 * Math.PI]
const lines = []
const one = (f, x) => lines.push(`${f} ${bits(x)} ${bits(Math[f](x))}`)
const two = (f, x, y) => lines.push(`${f} ${bits(x)} ${bits(y)} ${bits(f === 'pow' ? x ** y : Math[f](x, y))}`)
const N = 3000
for (let i = 0; i < N; i++) {
  const x = kinds[i % kinds.length]()
  const y = kinds[(i * 7 + 3) % kinds.length]()
  for (const f of ['sin', 'cos', 'tan', 'atan']) one(f, x)
  two('atan2', x, y)
  one('exp', x / 1000)
  one('exp', -rnd() * 10)
  one('log', Math.abs(x))
  for (const k of [2, 3, 4, 5, 6]) two('pow', x, k)
  two('pow', Math.abs(x), y / 1000)
  two('hypot', x, y)
  lines.push(`hypot3 ${bits(x)} ${bits(y)} ${bits(x / 3)} ${bits(Math.hypot(x, y, x / 3))}`)
}
for (const x of special) {
  for (const f of ['sin', 'cos', 'tan', 'atan', 'exp', 'log']) one(f, x)
  for (const y of special) {
    two('atan2', x, y)
    two('hypot', x, y)
  }
}
console.log(lines.join('\n'))
