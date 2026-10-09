// mas: .masm → bytes.
//
//   label:   INSTR  operand, operand   ; comment
//   .local:  …                         a label scoped to the last global label
//   .use     Elements, Shapes          take in libraries (each once, after the spell)
//   .const   EARTH 3                   a name for a number, written #EARTH

import {
  IMMEDIATE_BIT,
  LOCKS,
  MANA_BIT,
  OP_BY_NAME,
  PORT_BY_NAME,
  type Op,
  type OperandKind,
} from './isa.ts'

export type SourceLine = { file: string; line: number; text: string }

export type Program = {
  bytes: Uint8Array
  /** Global and local labels (`Fireball`, `Fireball.order`) to addresses. */
  labels: Map<string, number>
  consts: Map<string, number>
  /** Where each instruction came from, by address. */
  lines: Map<number, SourceLine>
  warnings: string[]
}

export class AsmError extends Error {
  constructor(public problems: string[]) {
    super(problems.join('\n'))
  }
}

/** Finds a library's source by name (`Shapes` → the text of Shapes.masm). */
export type LibraryResolver = (name: string) => string | undefined

type Pending = {
  op: Op
  tokens: string[]
  scope: string
  where: SourceLine
  addr: number
  size: number
}

export function assemble(source: string, file: string, libraries: LibraryResolver = () => undefined): Program {
  const problems: string[] = []
  const warnings: string[] = []
  const labels = new Map<string, number>()
  const consts = new Map<string, number>()
  const pending: Pending[] = []
  const used = new Set<string>()
  const queue: { file: string; source: string }[] = [{ file, source }]
  let addr = 0

  const fail = (where: SourceLine, message: string) => problems.push(`${where.file}:${where.line}: ${message}`)

  // Pass 1: read every file, place labels, size every instruction.
  while (queue.length) {
    const unit = queue.shift()!
    let scope = ''
    unit.source.split(/\r?\n/).forEach((raw, i) => {
      const where: SourceLine = { file: unit.file, line: i + 1, text: raw.trim() }
      let text = raw.replace(/;.*$/, '').trim()
      if (!text) return

      // Labels, any number of them, at the start of the line.
      for (;;) {
        const m = /^(\.?[A-Za-z_][\w]*):\s*/.exec(text)
        if (!m) break
        const name = m[1]
        let full = name
        if (name.startsWith('.')) {
          if (!scope) fail(where, `local label ${name} before any global label`)
          full = scope + name
        } else {
          scope = name
        }
        if (labels.has(full)) fail(where, `label ${full} is defined twice`)
        labels.set(full, addr)
        text = text.slice(m[0].length)
      }
      if (!text) return

      if (text.startsWith('.')) {
        const [directive, ...rest] = text.split(/\s+/)
        const args = rest.join(' ')
        if (directive === '.use') {
          for (const lib of args.split(',').map((s) => s.trim()).filter(Boolean)) {
            if (used.has(lib)) continue
            used.add(lib)
            const src = libraries(lib)
            if (src === undefined) fail(where, `no library called ${lib}`)
            else queue.push({ file: `${lib}.masm`, source: src })
          }
        } else if (directive === '.const') {
          const m = /^([A-Za-z_]\w*)\s+(-?[\d.]+)$/.exec(args)
          if (!m) fail(where, `.const needs a name and a number`)
          else if (consts.has(m[1])) fail(where, `${m[1]} is defined twice`)
          else consts.set(m[1], Number(m[2]))
        } else {
          fail(where, `unknown directive ${directive}`)
        }
        return
      }

      const sp = text.search(/\s/)
      const mnemonic = (sp < 0 ? text : text.slice(0, sp)).toUpperCase()
      const rest = sp < 0 ? '' : text.slice(sp).trim()
      const op = OP_BY_NAME.get(mnemonic)
      if (!op) {
        fail(where, `unknown instruction ${mnemonic}`)
        return
      }
      if (op.illegal) warnings.push(`${where.file}:${where.line}: ${op.name} isn't an instruction anyone teaches`)
      const tokens = rest ? rest.split(',').map((s) => s.trim()) : []
      if (tokens.length !== op.operands.length) {
        fail(where, `${op.name} takes ${op.operands.length} operand(s), not ${tokens.length}`)
        return
      }
      let size = 1
      op.operands.forEach((kind, k) => (size += operandSize(kind, tokens[k])))
      pending.push({ op, tokens, scope, where, addr, size })
      addr += size
    })
  }

  // Pass 2: encode.
  const bytes = new Uint8Array(addr)
  const view = new DataView(bytes.buffer)
  const lines = new Map<number, SourceLine>()
  for (const p of pending) {
    lines.set(p.addr, p.where)
    let at = p.addr + 1
    let immediate = false
    p.op.operands.forEach((kind, k) => {
      const token = p.tokens[k]
      const bad = (what: string) => fail(p.where, `${p.op.name} operand ${k + 1}: ${what}, not "${token}"`)
      switch (kind) {
        case 'n': {
          const r = register(token, 'n', 32)
          if (r === undefined) bad('a number register (n0–n31)')
          else bytes[at] = r
          at += 1
          break
        }
        case 'm': {
          const r = register(token, 'm', 8)
          if (r === undefined) bad('a mana register (m0–m7)')
          else bytes[at] = MANA_BIT | r
          at += 1
          break
        }
        case 't': {
          const m = /^n(\d+):(\d+)$/i.exec(token)
          const a = m ? Number(m[1]) : NaN
          if (!m || Number(m[2]) !== a + 2 || a > 29) bad('three number registers, like n4:6')
          else bytes[at] = a
          at += 1
          break
        }
        case 'a': {
          const m = /^\[\s*n(\d+)\s*\]$/i.exec(token)
          if (!m || Number(m[1]) > 31) bad('an address in a register, like [n3]')
          else bytes[at] = Number(m[1])
          at += 1
          break
        }
        case 's': {
          if (token.startsWith('#')) {
            const v = constant(token, consts)
            if (v === undefined) bad('a number or a known #NAME')
            else view.setFloat32(at, v, true)
            immediate = true
            at += 4
          } else {
            const r = register(token, 'n', 32)
            if (r === undefined) bad('a number register or an immediate')
            else bytes[at] = r
            at += 1
          }
          break
        }
        case 'I': {
          const v = token.startsWith('#') ? constant(token, consts) : undefined
          if (v === undefined) bad('an immediate, like #2.5')
          else view.setFloat32(at, v, true)
          immediate = true
          at += 4
          break
        }
        case 'K': {
          const v = token.startsWith('#') ? constant(token, consts) : LOCKS[token.toUpperCase()]
          if (v === undefined || !Number.isInteger(v) || v < 0 || v > 255) bad('a small constant, like #3 or SHAPE')
          else bytes[at] = v
          at += 1
          break
        }
        case 'P': {
          const port = PORT_BY_NAME.get(token.toUpperCase())
          if (!port) bad('a port (AIM, HAND, SELF, AMOUNT…)')
          else bytes[at] = port.code
          at += 1
          break
        }
        case 'L': {
          const full = token.startsWith('.') ? p.scope + token : token
          const target = labels.get(full)
          if (target === undefined) bad('a label that exists')
          else view.setUint16(at, target, true)
          at += 2
          break
        }
      }
    })
    bytes[p.addr] = p.op.code | (immediate ? IMMEDIATE_BIT : 0)
    // A triple port is read by IN into a triple: check the register leaves room.
    if (p.op.name === 'IN') {
      const port = PORT_BY_NAME.get(p.tokens[1]?.toUpperCase())
      const r = /^n(\d+)(?::(\d+))?$/i.exec(p.tokens[0])
      if (port && r && port.size === 3 && Number(r[1]) > 29) fail(p.where, `${port.name} needs three registers from ${p.tokens[0]}`)
    }
  }

  if (problems.length) throw new AsmError(problems)
  return { bytes, labels, consts, lines, warnings }
}

function operandSize(kind: OperandKind, token: string): number {
  if (kind === 'L') return 2
  if (kind === 'I') return 4
  if (kind === 's') return token.startsWith('#') ? 4 : 1
  return 1
}

function register(token: string, prefix: 'n' | 'm', count: number): number | undefined {
  // `IN n4:6, AIM` names a triple; the register byte is its first.
  const m = new RegExp(`^${prefix}(\\d+)(?::(\\d+))?$`, 'i').exec(token)
  if (!m) return undefined
  const r = Number(m[1])
  if (m[2] !== undefined && Number(m[2]) !== r + 2) return undefined
  return r < count ? r : undefined
}

function constant(token: string, consts: Map<string, number>): number | undefined {
  const body = token.slice(1).trim()
  const negative = body.startsWith('-')
  const name = negative ? body.slice(1) : body
  const v = /^[\d.]+(e-?\d+)?$/i.test(name) ? Number(name) : consts.get(name)
  if (v === undefined || Number.isNaN(v)) return undefined
  return negative ? -v : v
}
