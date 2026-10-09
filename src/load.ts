// Reading .masm files from disk, with the libraries beside them.

import { readFileSync, existsSync } from 'node:fs'
import { basename, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { assemble, type Program } from './asm/assembler.ts'

export const LIB_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'lib')
export const SPELL_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'spells')

/** Finds a library in the spell's own folder first, then in lib/. */
export function resolver(...dirs: string[]) {
  return (name: string) => {
    for (const dir of [...dirs, LIB_DIR]) {
      const path = join(dir, `${name}.masm`)
      if (existsSync(path)) return readFileSync(path, 'utf8')
    }
    return undefined
  }
}

export function assembleFile(path: string): Program {
  return assemble(readFileSync(path, 'utf8'), basename(path), resolver(dirname(path)))
}

export function spell(name: string): Program {
  return assembleFile(join(SPELL_DIR, `${name}.masm`))
}
