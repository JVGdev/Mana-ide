// The .masm files: spells and libraries. In development they come from disk and save back to it. Otherwise they're the
// copies built in, and edits live in this browser.

import type { LibraryResolver } from '../../../src/asm/assembler.ts'

export type MasmFile = { path: string; text: string; saved: string }

const BUILT: Record<string, string> = {
  ...import.meta.glob('../../../spells/*.masm', { query: '?raw', import: 'default', eager: true }),
  ...import.meta.glob('../../../lib/*.masm', { query: '?raw', import: 'default', eager: true }),
}

const DRAFT = 'mana.draft:'

function readDraft(path: string): string | null {
  try {
    return localStorage.getItem(DRAFT + path)
  } catch {
    return null
  }
}

function writeDraft(path: string, text: string | null) {
  try {
    if (text === null) localStorage.removeItem(DRAFT + path)
    else localStorage.setItem(DRAFT + path, text)
  } catch {
    // no storage: edits last as long as the page
  }
}

export const baseName = (path: string) => path.slice(path.lastIndexOf('/') + 1)

/** Spells first, then libraries, each by name. */
const order = (a: { path: string }, b: { path: string }) => {
  const sa = a.path.startsWith('spells/')
  const sb = b.path.startsWith('spells/')
  return sa === sb ? a.path.localeCompare(b.path) : sa ? -1 : 1
}

class Files {
  list = $state<MasmFile[]>([])
  /** The development server is there to save to. */
  canSave = $state(false)

  async load() {
    let loaded: { path: string; text: string }[]
    try {
      const res = await fetch('./api/files')
      if (!res.ok) throw new Error(String(res.status))
      loaded = await res.json()
      this.canSave = true
    } catch {
      loaded = Object.entries(BUILT).map(([p, text]) => ({ path: p.replace(/^(\.\.\/)+/, ''), text }))
    }
    // Spells you made here and haven't saved to disk.
    const extra: { path: string; text: string }[] = []
    try {
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i)!
        const path = key.slice(DRAFT.length)
        if (key.startsWith(DRAFT) && !loaded.some((f) => f.path === path)) extra.push({ path, text: '' })
      }
    } catch {
      // nothing stored
    }
    this.list = [...loaded, ...extra]
      .map(({ path, text }) => ({ path, saved: text, text: readDraft(path) ?? text }))
      .sort(order)
  }

  get(path: string): MasmFile | undefined {
    return this.list.find((f) => f.path === path)
  }

  /** By the name a `.use` gives, or the file name the assembler records. */
  byName(name: string): MasmFile | undefined {
    const file = name.endsWith('.masm') ? name : `${name}.masm`
    return this.list.find((f) => f.path === `spells/${file}`) ?? this.list.find((f) => f.path === `lib/${file}`)
  }

  edit(path: string, text: string) {
    const f = this.get(path)
    if (!f || f.text === text) return
    f.text = text
    writeDraft(path, text === f.saved ? null : text)
  }

  dirty(path: string): boolean {
    const f = this.get(path)
    return !!f && f.text !== f.saved
  }

  async save(path: string): Promise<boolean> {
    const f = this.get(path)
    if (!f || !this.canSave) return false
    const res = await fetch(`./api/files/${encodeURIComponent(path)}`, { method: 'PUT', body: f.text })
    if (!res.ok) return false
    f.saved = f.text
    writeDraft(path, null)
    return true
  }

  /** Throws away unsaved edits. */
  revert(path: string) {
    const f = this.get(path)
    if (!f) return
    f.text = f.saved
    writeDraft(path, null)
  }

  create(path: string, text: string) {
    if (this.get(path)) return
    this.list.push({ path, text, saved: '' })
    this.list.sort(order)
    writeDraft(path, text)
  }

  /** Libraries as the assembler asks for them: the spell's own folder first, then lib/. Unsaved edits count. */
  resolver: LibraryResolver = (name) => this.byName(name)?.text
}

export const files = new Files()
