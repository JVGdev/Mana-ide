import { describe, expect, it } from 'vitest'
import { assemble, AsmError } from '../src/asm/assembler.ts'
import { decodeAll, format, listing } from '../src/asm/disassembler.ts'
import { resolver } from '../src/load.ts'

const hex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).toUpperCase().padStart(2, '0')).join(' ')
const asm = (src: string) => assemble(src, 'test.masm', resolver())

describe('encoding', () => {
  it('encodes the examples in the spec', () => {
    expect(hex(asm('GATH m0, n17').bytes)).toBe('30 80 11')
    expect(hex(asm('FILT m1, m0, #3').bytes)).toBe('B2 81 80 00 00 40 40')
    expect(hex(asm('EMIT m1, n6, n4, n13:15').bytes)).toBe('52 81 06 04 0D')
  })

  it('sets the top bit only when the last operand is an immediate', () => {
    expect(asm('ADD n1, n2').bytes[0]).toBe(0x12)
    expect(asm('ADD n1, #2').bytes[0]).toBe(0x92)
  })

  it('reads constants from libraries it uses', () => {
    const p = asm('.use Elements\n FILT m1, m0, #EARTH')
    expect(hex(p.bytes)).toBe('B2 81 80 00 00 40 40')
    expect(p.consts.get('FIRE')).toBe(0)
  })

  it('scopes local labels to the global label before them', () => {
    const p = asm('a:  JMP .x\n.x: NOP\nb:  JMP .x\n.x: HALT')
    expect(p.labels.get('a.x')).toBe(3)
    expect(p.labels.get('b.x')).toBe(7)
    const view = new DataView(p.bytes.buffer)
    expect(view.getUint16(5, true)).toBe(7)
  })

  it('takes in each library once, after the spell', () => {
    const p = asm('.use Basics, Basics\nspell: HALT')
    expect(p.labels.get('spell')).toBe(0)
    expect(p.labels.get('toward')).toBe(1)
  })
})

describe('decoding', () => {
  it('reads back what it wrote', () => {
    const src = `
start:  IN    n16:18, AIM
        LDI   n0, #2.5
        PROB  n0, n16:18, #3
        LOCK  n20, SHAPE
        PUTW  #4, n4
        LD    n1, [n2]
        JMP   start`
    const p = asm(src)
    const lines = decodeAll(p.bytes).map((i) => format(i, new Map([[0, 'start']])))
    expect(lines).toEqual([
      'IN    n16:18, AIM',
      'LDI   n0, #2.5',
      'PROB  n0, n16:18, #3',
      'LOCK  n20, SHAPE',
      'PUTW  #4, n4',
      'LD    n1, [n2]',
      'JMP   start',
    ])
  })

  it('lists the four spells without a hitch', async () => {
    const { spell } = await import('../src/load.ts')
    for (const name of ['StoneWall', 'Fireball', 'Gust', 'WaterShield']) {
      const p = spell(name)
      expect(listing(p.bytes, p.labels)).toContain(`${name}:`)
    }
  })
})

describe('mistakes', () => {
  const problems = (src: string) => {
    try {
      asm(src)
    } catch (e) {
      if (e instanceof AsmError) return e.problems
      throw e
    }
    return []
  }

  it('names the line and what was wrong', () => {
    expect(problems('NOP\nFLY n1')).toEqual(['test.masm:2: unknown instruction FLY'])
    expect(problems('GATH n0, n1')[0]).toMatch(/operand 1: a mana register/)
    expect(problems('EMIT m1, n6, n4, n13:14')[0]).toMatch(/three number registers/)
    expect(problems('JMP nowhere')[0]).toMatch(/a label that exists/)
    expect(problems('FILT m1, m0, #EARTH')[0]).toMatch(/a known #NAME/)
    expect(problems('.use Nothing')[0]).toMatch(/no library called Nothing/)
    expect(problems('ADD n1')[0]).toMatch(/takes 2 operand/)
  })
})
