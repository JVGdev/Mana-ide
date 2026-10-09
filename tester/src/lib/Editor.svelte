<script lang="ts">
  import { onMount } from 'svelte'
  import { basicSetup } from 'codemirror'
  import { EditorState, StateEffect, StateField, RangeSet, type Extension } from '@codemirror/state'
  import { EditorView, Decoration, GutterMarker, gutter, keymap, type DecorationSet } from '@codemirror/view'
  import { indentWithTab } from '@codemirror/commands'
  import { setDiagnostics, lintGutter, type Diagnostic } from '@codemirror/lint'
  import { masm, type Live } from './masm.ts'

  type Props = {
    path: string
    text: string
    onchange: (text: string) => void
    onsave?: () => void
    /** The line the mind runs next, in this file. */
    current?: number
    breakpoints: number[]
    ontoggle: (line: number) => void
    /** Beats spent on each line of this file. */
    beats: Map<number, number>
    problems: { line: number; message: string }[]
    live: Live
    labels: () => string[]
    consts: () => string[]
    /** A line to scroll to and put the cursor on; `seq` changes each time. */
    goto: { line: number; seq: number }
  }
  let { path, text, onchange, onsave, current, breakpoints, ontoggle, beats, problems, live, labels, consts, goto }: Props =
    $props()

  let host: HTMLDivElement
  let view: EditorView | undefined
  const states = new Map<string, EditorState>()
  let shown = ''

  // The line the mind is on.
  const setCurrent = StateEffect.define<number | null>()
  const currentField = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(deco, tr) {
      deco = deco.map(tr.changes)
      for (const e of tr.effects)
        if (e.is(setCurrent)) {
          if (e.value === null || e.value > tr.state.doc.lines) deco = Decoration.none
          else {
            const line = tr.state.doc.line(e.value)
            deco = Decoration.set([Decoration.line({ class: 'cm-here' }).range(line.from)])
          }
        }
      return deco
    },
    provide: (f) => EditorView.decorations.from(f),
  })

  // Breakpoints, in their own gutter.
  class Dot extends GutterMarker {
    toDOM() {
      const d = document.createElement('span')
      d.className = 'cm-bp'
      return d
    }
  }
  const dot = new Dot()
  const setBreaks = StateEffect.define<number[]>()
  const breakField = StateField.define<RangeSet<GutterMarker>>({
    create: () => RangeSet.empty,
    update(set, tr) {
      set = set.map(tr.changes)
      for (const e of tr.effects)
        if (e.is(setBreaks))
          set = RangeSet.of(
            e.value.filter((l) => l <= tr.state.doc.lines).sort((a, b) => a - b).map((l) => dot.range(tr.state.doc.line(l).from)),
          )
      return set
    },
  })
  const breakGutter = gutter({
    class: 'cm-bp-gutter',
    markers: (v) => v.state.field(breakField),
    initialSpacer: () => dot,
    domEventHandlers: {
      mousedown(v, block) {
        ontoggle(v.state.doc.lineAt(block.from).number)
        return true
      },
    },
  })

  // Beats per line: where the thought went.
  class Cost extends GutterMarker {
    beats: number
    heat: number
    constructor(beats: number, heat: number) {
      super()
      this.beats = beats
      this.heat = heat
    }
    eq(o: Cost) {
      return o.beats === this.beats && o.heat === this.heat
    }
    toDOM() {
      const d = document.createElement('span')
      d.className = 'cm-cost'
      d.textContent = this.beats >= 10000 ? `${(this.beats / 1000).toFixed(0)}k` : this.beats >= 1000 ? `${(this.beats / 1000).toFixed(1)}k` : String(this.beats)
      d.style.setProperty('--heat', String(this.heat))
      return d
    }
  }
  const setBeats = StateEffect.define<Map<number, number>>()
  const beatsField = StateField.define<RangeSet<GutterMarker>>({
    create: () => RangeSet.empty,
    update(set, tr) {
      set = set.map(tr.changes)
      for (const e of tr.effects)
        if (e.is(setBeats)) {
          const max = Math.max(1, ...e.value.values())
          set = RangeSet.of(
            [...e.value]
              .filter(([l]) => l <= tr.state.doc.lines)
              .sort((a, b) => a[0] - b[0])
              .map(([l, b]) => new Cost(b, Math.sqrt(b / max)).range(tr.state.doc.line(l).from)),
          )
        }
      return set
    },
  })
  const beatsGutter = gutter({
    class: 'cm-cost-gutter',
    markers: (v) => v.state.field(beatsField),
    initialSpacer: () => new Cost(9999, 0),
  })

  function extensions(): Extension[] {
    return [
      breakGutter,
      beatsGutter,
      basicSetup,
      lintGutter(),
      keymap.of([
        indentWithTab,
        {
          key: 'Mod-s',
          preventDefault: true,
          run: () => {
            onsave?.()
            return true
          },
        },
      ]),
      masm(live, labels, consts),
      currentField,
      breakField,
      beatsField,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) onchange(u.state.doc.toString())
      }),
    ]
  }

  function stateFor(p: string, t: string) {
    const cached = states.get(p)
    if (cached && cached.doc.toString() === t) return cached
    return EditorState.create({ doc: t, extensions: extensions() })
  }

  function show() {
    if (!view) return
    if (shown !== path) {
      if (shown) states.set(shown, view.state)
      view.setState(stateFor(path, text))
      shown = path
    } else if (view.state.doc.toString() !== text) {
      // Changed from outside the editor (reverted, say).
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } })
    }
  }

  onMount(() => {
    view = new EditorView({ parent: host, state: stateFor(path, text) })
    shown = path
    return () => view?.destroy()
  })

  $effect(() => {
    void path
    void text
    show()
    if (!view) return
    const doc = view.state.doc
    const diagnostics: Diagnostic[] = problems
      .filter((p) => p.line >= 1 && p.line <= doc.lines)
      .map((p) => {
        const l = doc.line(p.line)
        return { from: l.from, to: l.to, severity: 'error', message: p.message }
      })
    view.dispatch(
      setDiagnostics(view.state, diagnostics),
      { effects: [setCurrent.of(current ?? null), setBreaks.of(breakpoints), setBeats.of(beats)] },
    )
  })

  let seen = 0
  $effect(() => {
    const { line, seq } = goto
    void path
    if (!view || seq === seen || line < 1 || line > view.state.doc.lines) return
    seen = seq
    const l = view.state.doc.line(line)
    view.dispatch({ selection: { anchor: l.from }, effects: EditorView.scrollIntoView(l.from, { y: 'center' }) })
  })
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
  .editor :global(.cm-editor) {
    height: 100%;
    background: var(--bg-2);
    color: var(--text);
    font-size: 13px;
  }
  .editor :global(.cm-scroller) {
    font-family: var(--font-mono);
    line-height: 1.55;
  }
  .editor :global(.cm-gutters) {
    background: var(--bg-2);
    border-right: 1px solid var(--line);
    color: var(--muted);
  }
  .editor :global(.cm-activeLineGutter),
  .editor :global(.cm-activeLine) {
    background: rgba(255, 255, 255, 0.025);
  }
  .editor :global(.cm-cursor) {
    border-left-color: var(--accent);
  }
  .editor :global(.cm-selectionBackground),
  .editor :global(.cm-focused .cm-selectionBackground) {
    background: var(--accent-soft) !important;
  }
  .editor :global(.cm-here) {
    background: var(--accent-soft) !important;
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .editor :global(.cm-bp-gutter) {
    width: 16px;
    cursor: pointer;
  }
  .editor :global(.cm-bp-gutter .cm-gutterElement) {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .editor :global(.cm-bp) {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--danger);
  }
  .editor :global(.cm-cost-gutter) {
    min-width: 38px;
    text-align: right;
  }
  .editor :global(.cm-cost) {
    display: inline-block;
    padding: 0 4px;
    border-radius: 3px;
    font-size: 11px;
    color: var(--text-2);
    background: rgba(229, 138, 114, calc(var(--heat) * 0.55));
  }
  .editor :global(.cm-tooltip) {
    background: var(--pop);
    border: 1px solid var(--line-2);
    color: var(--text);
    border-radius: var(--radius-xs);
  }
  .editor :global(.cm-tooltip-autocomplete ul li[aria-selected]) {
    background: var(--accent-soft);
    color: var(--text);
  }
  .editor :global(.masm-tip) {
    padding: 6px 9px;
    max-width: 380px;
    font-family: var(--font-ui);
    font-size: 13px;
    line-height: 1.45;
  }
  .editor :global(.masm-tip b) {
    font-family: var(--font-mono);
    color: var(--accent);
  }
  .editor :global(.tip-meta) {
    color: var(--muted);
    font-size: 12px;
  }
  .editor :global(.cm-panels) {
    background: var(--panel-2);
    color: var(--text);
  }
</style>
