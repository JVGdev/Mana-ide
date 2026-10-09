#!/usr/bin/env -S npx tsx
// mas: assemble a spell. Prints the listing; writes the bytes with -o.
//
//   mas spells/Fireball.masm
//   mas spells/Fireball.masm -o Fireball.mbc

import { writeFileSync } from 'node:fs'
import { AsmError } from '../asm/assembler.ts'
import { listing } from '../asm/disassembler.ts'
import { assembleFile } from '../load.ts'

const args = process.argv.slice(2)
const file = args.find((a) => !a.startsWith('-') && args[args.indexOf(a) - 1] !== '-o')
const out = args.includes('-o') ? args[args.indexOf('-o') + 1] : undefined
if (!file) {
  console.error('usage: mas <spell.masm> [-o out.mbc]')
  process.exit(2)
}

try {
  const program = assembleFile(file)
  for (const w of program.warnings) console.error(`warning: ${w}`)
  console.log(listing(program.bytes, program.labels))
  console.log(`\n${program.bytes.length} bytes`)
  if (out) writeFileSync(out, program.bytes)
} catch (e) {
  if (e instanceof AsmError) {
    for (const p of e.problems) console.error(p)
    process.exit(1)
  }
  throw e
}
