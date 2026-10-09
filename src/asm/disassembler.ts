// Bytes → instructions. The machine runs what this decodes, and the tester prints it.

import { IMMEDIATE_BIT, LOCK_NAMES, MANA_BIT, OP_BY_CODE, PORT_BY_CODE, type Op } from './isa.ts'

export type Instr = {
  addr: number
  size: number
  op: Op
  /** One number per operand: a register index, an address, a port or constant code, or an immediate's value. */
  args: number[]
  /** The last operand is an immediate, not a register. */
  immediate: boolean
}

export class DecodeError extends Error {}

export function decode(bytes: Uint8Array, addr: number): Instr {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const first = bytes[addr]
  if (first === undefined) throw new DecodeError(`no instruction at ${hex(addr, 4)}`)
  const immediate = (first & IMMEDIATE_BIT) !== 0
  const op = OP_BY_CODE.get(first & ~IMMEDIATE_BIT)
  if (!op) throw new DecodeError(`no instruction ${hex(first, 2)} at ${hex(addr, 4)}`)
  const args: number[] = []
  let at = addr + 1
  op.operands.forEach((kind, k) => {
    const last = k === op.operands.length - 1
    if (kind === 'L') {
      args.push(view.getUint16(at, true))
      at += 2
    } else if (kind === 'I' || (kind === 's' && last && immediate)) {
      args.push(view.getFloat32(at, true))
      at += 4
    } else if (kind === 'm') {
      args.push(bytes[at] & ~MANA_BIT)
      at += 1
    } else {
      args.push(bytes[at])
      at += 1
    }
  })
  return { addr, size: at - addr, op, args, immediate }
}

/** Decodes a whole program, in order. */
export function decodeAll(bytes: Uint8Array): Instr[] {
  const out: Instr[] = []
  for (let addr = 0; addr < bytes.length; ) {
    const instr = decode(bytes, addr)
    out.push(instr)
    addr += instr.size
  }
  return out
}

/** One instruction as text. Labels, when given, name jump targets. */
export function format(instr: Instr, labels?: Map<number, string>): string {
  const parts = instr.op.operands.map((kind, k) => {
    const v = instr.args[k]
    const last = k === instr.op.operands.length - 1
    switch (kind) {
      case 'n': {
        // IN reads a size-3 port into a triple.
        const port = instr.op.name === 'IN' ? PORT_BY_CODE.get(instr.args[1]) : undefined
        return port?.size === 3 ? `n${v}:${v + 2}` : `n${v}`
      }
      case 'm':
        return `m${v}`
      case 't':
        return `n${v}:${v + 2}`
      case 'a':
        return `[n${v}]`
      case 's':
        return last && instr.immediate ? `#${num(v)}` : `n${v}`
      case 'I':
        return `#${num(v)}`
      case 'K':
        return instr.op.name === 'LOCK' ? (LOCK_NAMES[v] ?? `#${v}`) : `#${v}`
      case 'P':
        return PORT_BY_CODE.get(v)?.name ?? `?${v}`
      case 'L': {
        const name = labels?.get(v)
        return name ? (name.includes('.') ? name.slice(name.indexOf('.')) : name) : hex(v, 4)
      }
    }
  })
  return parts.length ? `${instr.op.name.padEnd(5)} ${parts.join(', ')}` : instr.op.name
}

/** A listing: address, bytes, instruction. */
export function listing(bytes: Uint8Array, labels?: Map<string, number>): string {
  const byAddr = new Map<number, string>()
  for (const [name, addr] of labels ?? []) {
    // Prefer the global name when two labels share an address.
    const had = byAddr.get(addr)
    if (!had || (had.includes('.') && !name.includes('.'))) byAddr.set(addr, name)
  }
  const out: string[] = []
  for (const instr of decodeAll(bytes)) {
    const label = byAddr.get(instr.addr)
    if (label && !label.includes('.')) out.push(`${label}:`)
    const raw = Array.from(bytes.subarray(instr.addr, instr.addr + instr.size), (b) => hex(b, 2)).join(' ')
    const local = label?.includes('.') ? label.slice(label.indexOf('.')) + ':' : ''
    out.push(`${hex(instr.addr, 4)}  ${raw.padEnd(23)} ${local.padEnd(9)}${format(instr, byAddr)}`)
  }
  return out.join('\n')
}

function hex(v: number, width: number): string {
  return v.toString(16).toUpperCase().padStart(width, '0')
}

function num(v: number): string {
  return String(Math.round(v * 1e5) / 1e5)
}
