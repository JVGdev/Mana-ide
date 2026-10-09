// Brings the code listings in SPEC.md up to date with lib/, spells/ and bench/: npm run listings.
//
// A listing is a fenced block whose labels are routines of one .masm file. It's replaced by those routines as the file
// has them now, each with the comment above it. A listing whose first line is a file's first line starts with the file's
// head (its comment and .use lines).

import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const root = join(import.meta.dirname, '..')
const files = ['lib', 'spells', 'bench'].flatMap((d) => readdirSync(join(root, d)).filter((f) => f.endsWith('.masm')).map((f) => join(root, d, f)))

type Routine = { name: string; text: string }
type File = { head: string; routines: Routine[] }

const label = /^([A-Za-z_][A-Za-z0-9_]*):/

/** A file as its head and its routines: each from the comment above its label to the next one's comment. */
function parse(text: string): File {
  const lines = text.replace(/\n+$/, '').split('\n')
  const starts: { name: string; at: number }[] = []
  for (let i = 0; i < lines.length; i++) {
    const m = label.exec(lines[i])
    if (!m) continue
    let at = i
    while (at > 0 && lines[at - 1].startsWith(';')) at--
    starts.push({ name: m[1], at })
  }
  const head = lines.slice(0, starts[0]?.at ?? lines.length).join('\n').replace(/\n+$/, '')
  const routines = starts.map((s, k) => ({
    name: s.name,
    text: lines.slice(s.at, starts[k + 1]?.at ?? lines.length).join('\n').replace(/\n+$/, ''),
  }))
  return { head, routines }
}

const parsed = files.map((f) => ({ path: f, ...parse(readFileSync(f, 'utf8')) }))

const spec = readFileSync(join(root, 'SPEC.md'), 'utf8')
let changed = 0
const out = spec.replace(/```\n([\s\S]*?)```/g, (whole, body: string) => {
  const first = body.split('\n')[0]
  // A head's first line is a comment; an old listing may have lost its semicolon, and read like a label.
  const isHead = (f: File) => !!f.head && [first, `; ${first}`].includes(f.head.split('\n')[0])
  const names = body
    .split('\n')
    .map((l, i) => (i === 0 && parsed.some(isHead) ? undefined : label.exec(l)?.[1]))
    .filter((n): n is string => !!n)
  if (!names.length) return whole
  const file = parsed.find((f) => names.every((n) => f.routines.some((r) => r.name === n)))
  if (!file) return whole
  const withHead = isHead(file)
  const text = [withHead ? file.head : '', ...file.routines.filter((r) => names.includes(r.name)).map((r) => r.text)].filter(Boolean).join('\n\n')
  const next = '```\n' + text + '\n```'
  if (next !== whole) changed++
  return next
})
writeFileSync(join(root, 'SPEC.md'), out)
console.log(`${changed} listing${changed === 1 ? '' : 's'} brought up to date`)
