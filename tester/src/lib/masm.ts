// .masm for CodeMirror: colours, completion and hover.

import { StreamLanguage, HighlightStyle, syntaxHighlighting, LanguageSupport } from '@codemirror/language'
import { tags as t } from '@lezer/highlight'
import { autocompletion, type CompletionContext, type Completion } from '@codemirror/autocomplete'
import { hoverTooltip, type Tooltip } from '@codemirror/view'
import { OPS, PORTS, LOCK_NAMES, OP_BY_NAME, PORT_BY_NAME } from '../../../src/asm/isa.ts'
import { OP_DOCS, PORT_DOCS } from '../../../src/asm/docs.ts'

const MNEMONICS = new Set(OPS.map((o) => o.name))
const PORT_NAMES = new Set(PORTS.map((p) => p.name))
const LOCKS = new Set(LOCK_NAMES)

type State = { start: boolean }

const masmStream = StreamLanguage.define<State>({
  name: 'masm',
  startState: () => ({ start: true }),
  token(stream, state) {
    if (stream.sol()) state.start = true
    if (stream.eatSpace()) return null
    if (stream.match(/^[;!].*/)) return 'comment'
    if (stream.match(/^\.(use|const)\b/i)) return 'meta'
    // A label where it's defined: `name:` or `.name:`
    if (state.start && stream.match(/^\.?[A-Za-z_][\w.]*:/)) {
      state.start = false
      return 'labelName'
    }
    state.start = false
    if (stream.match(/^#-?[\d.]+(e-?\d+)?/i)) return 'number'
    if (stream.match(/^#[A-Za-z_]\w*/)) return 'constant'
    if (stream.match(/^-?\d[\d.]*/)) return 'number'
    if (stream.match(/^m[0-7]\b/)) return 'special'
    if (stream.match(/^n\d+(:\d+)?\b/)) return 'variableName'
    const word = stream.match(/^\.?[A-Za-z_][\w.]*/) as RegExpMatchArray | null
    if (word) {
      const w = word[0]
      if (MNEMONICS.has(w.toUpperCase())) return 'keyword'
      if (PORT_NAMES.has(w) || LOCKS.has(w)) return 'atom'
      return 'labelName'
    }
    if (stream.match(/^[,[\]]/)) return 'punctuation'
    stream.next()
    return null
  },
  tokenTable: {
    special: t.special(t.variableName),
    constant: t.constant(t.name),
  },
  languageData: { commentTokens: { line: ';' } },
})

const style = HighlightStyle.define([
  { tag: t.comment, color: 'var(--muted)', fontStyle: 'italic' },
  { tag: t.keyword, color: 'var(--accent)', fontWeight: '600' },
  { tag: t.meta, color: 'var(--special)' },
  { tag: t.labelName, color: 'var(--info)' },
  { tag: t.number, color: 'var(--ok)' },
  { tag: t.constant(t.name), color: 'var(--ok)' },
  { tag: t.variableName, color: 'var(--text)' },
  { tag: t.special(t.variableName), color: 'var(--mana)', fontWeight: '600' },
  { tag: t.atom, color: 'var(--special)' },
  { tag: t.punctuation, color: 'var(--text-2)' },
])

/** What the hover can ask about the running machine. */
export type Live = { register?: (name: string) => string | undefined }

function wordAt(text: string, pos: number): { from: number; to: number; word: string } | null {
  let from = pos
  let to = pos
  while (from > 0 && /[\w.#:]/.test(text[from - 1])) from--
  while (to < text.length && /[\w.#:]/.test(text[to])) to++
  if (from === to) return null
  return { from, to, word: text.slice(from, to) }
}

function hover(live: Live) {
  return hoverTooltip((view, pos): Tooltip | null => {
    const line = view.state.doc.lineAt(pos)
    const found = wordAt(line.text, pos - line.from)
    if (!found) return null
    const word = found.word.replace(/,$/, '')
    let html = ''
    const op = OP_BY_NAME.get(word.toUpperCase())
    if (op && OP_DOCS[op.name]) {
      const d = OP_DOCS[op.name]
      html = `<b>${d.syntax}</b> <span class="tip-meta">${op.beats} beat${op.beats === 1 ? '' : 's'}${d.step ? ` · ${d.step}` : ''}${op.order ? ' · in an order' : ''}</span><br>${d.doc}`
    } else if (PORT_BY_NAME.has(word)) {
      html = `<b>${word}</b> <span class="tip-meta">port</span><br>${PORT_DOCS[word] ?? ''}`
    } else if (/^[nm]\d+(:\d+)?$/.test(word)) {
      const v = live.register?.(word)
      if (v === undefined) return null
      html = `<b>${word}</b> <span class="tip-meta">now</span><br>${v}`
    } else return null
    return {
      pos: line.from + found.from,
      end: line.from + found.to,
      above: true,
      create() {
        const dom = document.createElement('div')
        dom.className = 'masm-tip'
        dom.innerHTML = html
        return { dom }
      },
    }
  })
}

function completions(labels: () => string[], consts: () => string[]) {
  const ops: Completion[] = OPS.map((o) => ({
    label: o.name,
    type: 'keyword',
    detail: OP_DOCS[o.name]?.syntax.slice(o.name.length).trim(),
    info: OP_DOCS[o.name]?.doc,
  }))
  const ports: Completion[] = PORTS.map((p) => ({ label: p.name, type: 'constant', info: PORT_DOCS[p.name] }))
  const locks: Completion[] = LOCK_NAMES.map((l) => ({ label: l, type: 'constant' }))
  return (ctx: CompletionContext) => {
    const word = ctx.matchBefore(/[#.]?[\w.]*/)
    if (!word || (word.from === word.to && !ctx.explicit)) return null
    const before = ctx.state.doc.lineAt(ctx.pos).text.slice(0, word.from - ctx.state.doc.lineAt(ctx.pos).from)
    const atMnemonic = /^\s*(\.?[\w.]+:)?\s*$/.test(before)
    if (word.text.startsWith('#'))
      return { from: word.from + 1, options: consts().map((c) => ({ label: c, type: 'constant' })) }
    if (atMnemonic) return { from: word.from, options: ops }
    return {
      from: word.from,
      options: [...ports, ...locks, ...labels().map((l) => ({ label: l, type: 'function' }))],
    }
  }
}

export function masm(live: Live, labels: () => string[], consts: () => string[]) {
  return [
    new LanguageSupport(masmStream),
    syntaxHighlighting(style),
    autocompletion({ override: [completions(labels, consts)] }),
    hover(live),
  ]
}
