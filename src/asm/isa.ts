// The instruction set: one table that the assembler, the disassembler and the machine all read.
//
// Operand kinds:
//   n  a number register (n0–n31)          one byte, top bit 0
//   m  a mana register (m0–m7)             one byte, top bit 1
//   s  a number register or an immediate   a register byte, or four bytes (float32) when it's an immediate;
//                                          only ever the last operand, and an immediate sets the opcode's top bit
//   t  a triple: the first of three number registers
//   a  an address held in a register, written [n]
//   L  a label                             two bytes, the address
//   P  a port                              one byte
//   K  a small constant (#k, SHAPE…)       one byte
//   I  an immediate                        four bytes (float32)

export type OperandKind = 'n' | 'm' | 's' | 't' | 'a' | 'L' | 'P' | 'K' | 'I'

export type Op = {
  code: number
  name: string
  operands: OperandKind[]
  /** Beats of thought it takes. */
  beats: number
  /** Only inside a weave's order. */
  order?: boolean
}

const MIND = 1
const BODY = 4

export const OPS: Op[] = [
  // Mind: control
  { code: 0x00, name: 'NOP', operands: [], beats: MIND },
  { code: 0x01, name: 'HALT', operands: [], beats: MIND },
  { code: 0x02, name: 'FAIL', operands: ['K'], beats: MIND },
  { code: 0x03, name: 'TICK', operands: [], beats: MIND },
  { code: 0x07, name: 'RET', operands: [], beats: MIND },
  { code: 0x08, name: 'JMP', operands: ['L'], beats: MIND },
  { code: 0x09, name: 'JEQ', operands: ['L'], beats: MIND },
  { code: 0x0a, name: 'JNE', operands: ['L'], beats: MIND },
  { code: 0x0b, name: 'JLT', operands: ['L'], beats: MIND },
  { code: 0x0c, name: 'JLE', operands: ['L'], beats: MIND },
  { code: 0x0d, name: 'JGT', operands: ['L'], beats: MIND },
  { code: 0x0e, name: 'JGE', operands: ['L'], beats: MIND },
  { code: 0x0f, name: 'CALL', operands: ['L'], beats: MIND },
  // Mind: numbers
  { code: 0x10, name: 'LDI', operands: ['n', 'I'], beats: MIND },
  { code: 0x11, name: 'MOV', operands: ['n', 's'], beats: MIND },
  { code: 0x12, name: 'ADD', operands: ['n', 's'], beats: MIND },
  { code: 0x13, name: 'SUB', operands: ['n', 's'], beats: MIND },
  { code: 0x14, name: 'MUL', operands: ['n', 's'], beats: MIND },
  { code: 0x15, name: 'DIV', operands: ['n', 's'], beats: MIND },
  { code: 0x16, name: 'MOD', operands: ['n', 's'], beats: MIND },
  { code: 0x17, name: 'NEG', operands: ['n'], beats: MIND },
  { code: 0x18, name: 'ABS', operands: ['n'], beats: MIND },
  { code: 0x19, name: 'SQRT', operands: ['n'], beats: MIND },
  { code: 0x1a, name: 'FLOOR', operands: ['n'], beats: MIND },
  { code: 0x1b, name: 'ROUND', operands: ['n'], beats: MIND },
  { code: 0x1c, name: 'SIN', operands: ['n'], beats: MIND },
  { code: 0x1d, name: 'COS', operands: ['n'], beats: MIND },
  { code: 0x1e, name: 'TAN', operands: ['n'], beats: MIND },
  { code: 0x1f, name: 'ATAN', operands: ['n'], beats: MIND },
  { code: 0x20, name: 'ATN2', operands: ['n', 's'], beats: MIND },
  { code: 0x21, name: 'MIN', operands: ['n', 's'], beats: MIND },
  { code: 0x22, name: 'MAX', operands: ['n', 's'], beats: MIND },
  { code: 0x23, name: 'CMP', operands: ['n', 's'], beats: MIND },
  { code: 0x24, name: 'PUSH', operands: ['n'], beats: MIND },
  { code: 0x25, name: 'POP', operands: ['n'], beats: MIND },
  { code: 0x26, name: 'LD', operands: ['n', 'a'], beats: MIND },
  { code: 0x27, name: 'ST', operands: ['n', 'a'], beats: MIND },
  { code: 0x28, name: 'IN', operands: ['n', 'P'], beats: MIND },
  // Body
  { code: 0x30, name: 'GATH', operands: ['m', 's'], beats: BODY },
  { code: 0x31, name: 'CIRC', operands: ['m'], beats: BODY },
  { code: 0x32, name: 'FILT', operands: ['m', 'm', 's'], beats: BODY },
  { code: 0x33, name: 'SPLT', operands: ['m', 'm', 's'], beats: BODY },
  { code: 0x34, name: 'JOIN', operands: ['m', 'm'], beats: BODY },
  { code: 0x35, name: 'MEAS', operands: ['n', 'm'], beats: BODY },
  { code: 0x36, name: 'PART', operands: ['n', 'm', 's'], beats: BODY },
  { code: 0x37, name: 'VENT', operands: ['m'], beats: BODY },
  // Reach
  { code: 0x40, name: 'PROB', operands: ['n', 't', 's'], beats: BODY },
  { code: 0x41, name: 'AIRM', operands: ['n', 't', 's'], beats: BODY },
  { code: 0x42, name: 'SEND', operands: ['m', 'n', 't', 't'], beats: BODY },
  // Weave
  { code: 0x50, name: 'WEAV', operands: ['n', 't'], beats: BODY },
  { code: 0x51, name: 'TURN', operands: ['n', 't'], beats: BODY },
  { code: 0x52, name: 'EMIT', operands: ['m', 'n', 'n', 't'], beats: BODY },
  { code: 0x53, name: 'WSET', operands: ['n', 'K', 's'], beats: BODY },
  { code: 0x54, name: 'WGET', operands: ['n', 'n', 'K'], beats: BODY },
  { code: 0x55, name: 'ORDR', operands: ['n', 'L'], beats: BODY },
  { code: 0x56, name: 'MANI', operands: ['n'], beats: BODY },
  { code: 0x57, name: 'LOCK', operands: ['n', 'K'], beats: BODY },
  { code: 0x58, name: 'RELS', operands: ['n'], beats: BODY },
  // Order: only inside a weave's order
  { code: 0x60, name: 'MOVE', operands: ['t'], beats: MIND, order: true },
  { code: 0x61, name: 'TUCH', operands: ['n'], beats: MIND, order: true },
  { code: 0x62, name: 'GETW', operands: ['n', 'K'], beats: MIND, order: true },
  { code: 0x63, name: 'PUTW', operands: ['K', 's'], beats: MIND, order: true },
  { code: 0x64, name: 'DISS', operands: [], beats: MIND, order: true },
  { code: 0x65, name: 'CNDS', operands: ['s'], beats: MIND, order: true },
]

export const IMMEDIATE_BIT = 0x80

export const OP_BY_NAME = new Map(OPS.map((op) => [op.name, op]))
export const OP_BY_CODE = new Map(OPS.map((op) => [op.code, op]))

/** Ports, with how many numbers each reads. */
export const PORTS: { name: string; code: number; size: number; order?: boolean }[] = [
  { name: 'AIM', code: 0x00, size: 3 },
  { name: 'HAND', code: 0x01, size: 3 },
  { name: 'SELF', code: 0x02, size: 3 },
  { name: 'AMOUNT', code: 0x03, size: 1 },
  { name: 'FORCE', code: 0x04, size: 1 },
  { name: 'MAINTAIN', code: 0x05, size: 1 },
  { name: 'CELL', code: 0x06, size: 1 },
  { name: 'DEPTH', code: 0x07, size: 1 },
  { name: 'LOAD', code: 0x08, size: 1 },
  { name: 'CAPACITY', code: 0x09, size: 1 },
  { name: 'ORIGIN', code: 0x0a, size: 3, order: true },
  { name: 'MAKER', code: 0x0b, size: 3, order: true },
]
export const PORT_BY_NAME = new Map(PORTS.map((p) => [p.name, p]))
export const PORT_BY_CODE = new Map(PORTS.map((p) => [p.code, p]))

/** Names a K operand can be written as. */
export const LOCKS: Record<string, number> = { SHAPE: 0, INPUT: 1, ORDER: 2 }
export const LOCK_NAMES = ['SHAPE', 'INPUT', 'ORDER']

export const MANA_BIT = 0x80
