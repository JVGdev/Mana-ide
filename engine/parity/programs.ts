// Every .masm file, assembled by the TypeScript assembler: bytes, labels, constants, lines and listing.

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { assemble, AsmError } from '../../src/asm/assembler.ts'
import { listing } from '../../src/asm/disassembler.ts'
import { resolver } from '../../src/load.ts'

const repo = join(import.meta.dirname, '..', '..')
const out = join(repo, 'engine', 'target', 'parity')
mkdirSync(out, { recursive: true })
const programs = []
for (const dir of ['spells', 'bench', 'lib']) {
  for (const name of readdirSync(join(repo, dir)).filter((f) => f.endsWith('.masm')).sort()) {
    const source = readFileSync(join(repo, dir, name), 'utf8')
    try {
      const p = assemble(source, name, resolver(join(repo, dir)))
      programs.push({
        path: `${dir}/${name}`,
        bytes: Buffer.from(p.bytes).toString('hex'),
        labels: [...p.labels],
        consts: [...p.consts],
        lines: [...p.lines].map(([a, l]) => [a, l.file, l.line, l.text]),
        listing: listing(p.bytes, p.labels),
      })
    } catch (e) {
      if (!(e instanceof AsmError)) throw e
      programs.push({ path: `${dir}/${name}`, problems: e.problems })
    }
  }
}
// Mistakes, and the edges of the syntax: each problem has to be named the same way.
const snippets = [
  'NOP\nFLY n1', 'GATH n0, n1', 'EMIT m1, n6, n4, n13:14', 'JMP nowhere', 'FILT m1, m0, #EARTH', '.use Nothing', 'ADD n1',
  '.x: NOP', 'a: NOP\na: NOP', '.const X', '.const X 1\n.const X 2', '.const Y 1.2.3\nLDI n0, #Y', '.bogus', 'LOCK n1, SHAPE',
  'LOCK n1, #300', 'LOCK n1, #1.5', 'LOCK n1, #0', 'IN n30, AIM', 'IN n29:31, AIM', 'IN n0, NOPE', 'LD n1, [n32]', 'LD n1, n2',
  'MOV n32, n1', 'GATH m8, n1', 'PPOS n30:32, n1, n2', 'LDI n0, 3', 'LDI n0, #-.5', 'LDI n0, #1e-3', 'LDI n0, #1E3', 'LDI n0, #5.',
  'lab: lab2: NOP ; two labels\n JMP lab2', 'a: JMP .b\n.b: JMP a.b', 'add n1, #2', '  ; only a comment', 'ADD n1, #2, n3',
  'WSET n1, #8, #1', 'PUTW #256, n1', 'SEND m0, n1, n2:4, n5:7\nTICK\nHALT', 'TICK extra', 'x:\r\n NOP\r\n',
  'LDI n0, #0.1\nLDI n1, #16777217\nLDI n2, #-0\nLDI n3, #3.4e38\nLDI n4, #1e39',
]
for (const [i, src] of snippets.entries()) {
  try {
    const p = assemble(src, 'test.masm', resolver())
    programs.push({ path: `snippet:${i}`, source: src, bytes: Buffer.from(p.bytes).toString('hex'), labels: [...p.labels], consts: [...p.consts], lines: [...p.lines].map(([a, l]) => [a, l.file, l.line, l.text]), listing: listing(p.bytes, p.labels) })
  } catch (e) {
    if (!(e instanceof AsmError)) throw e
    programs.push({ path: `snippet:${i}`, source: src, problems: e.problems })
  }
}
writeFileSync(join(out, 'programs.json'), JSON.stringify(programs))
console.log(`${programs.length} programs`)
