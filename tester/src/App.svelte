<script lang="ts">
  import { onMount } from 'svelte'
  import Editor from './lib/Editor.svelte'
  import World from './lib/World.svelte'
  import Mind from './lib/panels/Mind.svelte'
  import Body from './lib/panels/Body.svelte'
  import Weaves from './lib/panels/Weaves.svelte'
  import Profile from './lib/panels/Profile.svelte'
  import Events from './lib/panels/Events.svelte'
  import Caster from './lib/panels/Caster.svelte'
  import Reference from './lib/panels/Reference.svelte'
  import Bytes from './lib/panels/Bytes.svelte'
  import Order from './lib/panels/Order.svelte'
  import { files, baseName } from './lib/files.svelte.ts'
  import { session, SCENE_NAMES, PANELS, type SceneName } from './lib/session.svelte.ts'
  import { num, PART_COLORS, PART_NAMES } from './lib/format.ts'
  import { SCENES } from '../../src/scenes.ts'
  import { PHYSICS } from '../../src/vm/physics.ts'

  let ready = $state(false)
  let note = $state('')

  onMount(async () => {
    await files.load()
    session.loadScene('Fireball')
    session.lint()
    ready = true
  })

  /** The bench's library, beside its spells. */
  const isLibrary = (path: string) => path.startsWith('lib/') || path === 'bench/Bench.masm'
  const spells = $derived(files.list.filter((f) => f.path.startsWith('spells/')))
  const benchSpells = $derived(files.list.filter((f) => f.path.startsWith('bench/') && !isLibrary(f.path)))
  const libraries = $derived(files.list.filter((f) => isLibrary(f.path)))
  const openFile = $derived(files.get(session.open))
  /** The libraries the spell uses, from its `.use` lines. */
  const uses = $derived.by(() => {
    const text = files.get(session.spell)?.text ?? ''
    const names = [...text.matchAll(/^\s*\.use\s+([^;!\n]+)/gim)].flatMap((m) => m[1].split(',').map((n) => n.trim()))
    return [...new Set(names)].map((n) => files.byName(n)).filter((f) => !!f)
  })

  const openName = $derived(baseName(session.open))
  const here = $derived(session.here())
  /** The order instruction shown in the Order panel, when that's open. */
  const orderHere = $derived.by(() => {
    if (session.panel !== 'order') return undefined
    const step = session.orderTrace()?.steps[session.orderSel.step]
    return step && session.program?.lines.get(step.addr)
  })
  const breakLines = $derived(
    session.breakpoints.filter((b) => b.startsWith(`${openName}:`)).map((b) => Number(b.slice(openName.length + 1))),
  )
  const beats = $derived.by(() => {
    void session.version
    const out = new Map<number, number>()
    const cast = session.cast
    if (!cast) return out
    for (const [addr, { beats }] of cast.profile) {
      const l = cast.program.lines.get(addr)
      if (l?.file === openName) out.set(l.line, (out.get(l.line) ?? 0) + beats)
    }
    return out
  })
  const problemsHere = $derived(session.problems.filter((p) => p.file === openName))

  let lintTimer: ReturnType<typeof setTimeout>
  function edited(text: string) {
    files.edit(session.open, text)
    clearTimeout(lintTimer)
    lintTimer = setTimeout(() => session.lint(), 250)
  }

  function flash(message: string) {
    note = message
    setTimeout(() => note === message && (note = ''), 2500)
  }

  async function save() {
    if (!files.canSave) return flash('Saved in this browser. Run the tester with npm run tester to save to disk.')
    flash((await files.save(session.open)) ? `Saved ${session.open}` : `Couldn't save ${session.open}`)
  }

  function chooseSpell(path: string) {
    session.spell = path
    session.open = path
    const name = baseName(path).replace(/\.masm$/, '')
    // The bench's spells: a ball held in front of the caster, or thrown at the Fireball's pillar. Its orders that only
    // feel are run where orders aren't told where their centre is.
    const bench = path.startsWith('bench/')
    PHYSICS.orderKnowsCentre = !(bench && name.endsWith('Feel'))
    const scene = bench ? (name.startsWith('Hold') ? 'Hold' : 'Fireball') : name
    if (scene in SCENES) session.loadScene(scene as SceneName)
    else session.reset()
    session.lint()
  }

  async function newSpell() {
    const name = prompt('Name the spell (one word, like StoneWall):')?.trim()
    if (!name) return
    if (!/^[A-Z][A-Za-z0-9]*$/.test(name)) return flash('A spell is named with one word, starting with a capital.')
    const path = `spells/${name}.masm`
    if (files.get(path)) return flash(`${name} already exists.`)
    files.create(
      path,
      `; ${name}: what it does.
        .use  Elements, Basics, Shapes

${name}:
        IN    n17, AMOUNT
        GATH  m0, n17             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #FIRE       ; FILTER
        IN    n1:3, HAND
        WEAV  n16, n1:3           ; POSITION: a weave at the hand
        MOV   n4, n16
        LDI   n0, #0.5
        CALL  ball                ; ORDER: the mana laid out as a ball
        MANI  n16                 ; SEND
        MOV   n0, n16
        HALT
`,
    )
    if (files.canSave) await files.save(path)
    chooseSpell(path)
  }

  function cast() {
    if (!session.castSpell()) {
      flash('It doesn’t assemble yet: see the problems under the editor.')
      return
    }
    session.panel = 'mind'
    session.goHere()
  }

  function keys(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault()
      cast()
    } else if (e.key === 'F10' && e.shiftKey) {
      e.preventDefault()
      session.stepTick()
    } else if (e.key === 'F10') {
      e.preventDefault()
      session.stepInstruction()
    } else if (e.key === 'F8') {
      e.preventDefault()
      session.playing ? session.pause() : session.play()
    }
  }

  const tick = $derived.by(() => {
    void session.version
    return session.sim?.tick ?? 0
  })
  const midTick = $derived.by(() => {
    void session.version
    return session.sim?.midTick ?? false
  })
</script>

<svelte:window onkeydown={keys} />

<div class="app">
  <header>
    <div class="brand"><span class="sigil"></span>Mana <span class="muted">tester</span></div>

    <div class="group">
      <select value={session.spell} onchange={(e) => chooseSpell(e.currentTarget.value)} aria-label="Spell">
        <optgroup label="Spells">
          {#each spells as f}<option value={f.path}>{baseName(f.path).replace('.masm', '')}{files.dirty(f.path) ? ' •' : ''}</option>{/each}
        </optgroup>
        <optgroup label="Bench">
          {#each benchSpells as f}<option value={f.path}>{baseName(f.path).replace('.masm', '')}{files.dirty(f.path) ? ' •' : ''}</option>{/each}
        </optgroup>
      </select>
      <button onclick={newSpell} title="A new spell">New</button>
    </div>

    <div class="group">
      <select
        value={session.scene}
        onchange={(e) => session.loadScene(e.currentTarget.value as SceneName)}
        aria-label="World"
        title="The world to cast in"
      >
        {#each SCENE_NAMES as s}<option value={s}>{s === 'Field' ? 'Empty field' : `${s}'s world`}</option>{/each}
      </select>
      <div class="seg" role="group" aria-label="Dimensions">
        <button class:on={session.dims === 2} onclick={() => session.loadScene(session.scene, 2)}>2D</button>
        <button class:on={session.dims === 3} onclick={() => session.loadScene(session.scene, 3)}>3D</button>
      </div>
    </div>

    <div class="group transport">
      <button class="primary" onclick={cast} title="Cast: a fresh world, and the spell (Ctrl+Enter)">Cast</button>
      <button onclick={() => (session.playing ? session.pause() : session.play())} title="Play or pause (F8)">
        {session.playing ? 'Pause' : 'Play'}
      </button>
      <button onclick={() => session.stepInstruction()} title="One instruction (F10)">Step</button>
      <button onclick={() => session.stepTick()} title="The rest of this tick, or one tick (Shift+F10)">Tick</button>
      <select bind:value={session.tps} aria-label="Speed" title="Ticks a second while playing">
        {#each [1, 2, 4, 8, 15, 30, 60] as n}<option value={n}>{n}/s</option>{/each}
      </select>
      <span class="tick mono" title={midTick ? 'partway through this tick: the world moves when it ends' : ''}>
        tick {tick}{#if midTick}<span class="mid">thinking</span>{/if}
      </span>
    </div>
  </header>

  {#if ready}
    <main>
      <section class="code">
        <div class="tabs">
          {#each [files.get(session.spell), ...uses].filter((f) => !!f) as f}
            <button class:on={session.open === f.path} onclick={() => (session.open = f.path)}>
              {baseName(f.path).replace('.masm', '')}{files.dirty(f.path) ? ' •' : ''}
            </button>
          {/each}
          <select
            class="more"
            value=""
            onchange={(e) => {
              if (e.currentTarget.value) session.open = e.currentTarget.value
              e.currentTarget.value = ''
            }}
            aria-label="Open a library"
          >
            <option value="">Libraries…</option>
            {#each libraries as f}<option value={f.path}>{baseName(f.path)}</option>{/each}
          </select>
          <span class="spacer"></span>
          {#if files.dirty(session.open)}
            <button class="quiet" onclick={() => files.revert(session.open)} title="Throw away the changes">Revert</button>
          {/if}
          <button class="quiet" onclick={save} disabled={!files.dirty(session.open)} title="Save (Ctrl+S)">Save</button>
        </div>
        <div class="editor-wrap">
          {#if openFile}
            <Editor
              path={session.open}
              text={openFile.text}
              onchange={edited}
              onsave={save}
              current={here && here.file === openName ? here.line : undefined}
              orderLine={orderHere && orderHere.file === openName ? orderHere.line : undefined}
              breakpoints={breakLines}
              ontoggle={(line) => session.toggleBreakpoint(openName, line)}
              {beats}
              problems={problemsHere}
              live={{
                register: (name) => {
                  const cast = session.cast
                  if (!cast) return undefined
                  if (name.startsWith('m')) {
                    const r = cast.caster.regs[Number(name.slice(1))]
                    return r ? r.parts.map((v, k) => `${PART_NAMES[k]} ${num(v)}`).join(', ') : undefined
                  }
                  const [a, b] = name.slice(1).split(':').map(Number)
                  return Array.from(cast.frame.n.slice(a, (b ?? a) + 1), (v) => num(v, 4)).join(', ')
                },
              }}
              labels={() => [...(session.program?.labels.keys() ?? [])].filter((l) => !l.includes('.'))}
              consts={() => [...(session.program?.consts.keys() ?? [])]}
              goto={session.goto}
            />
          {/if}
        </div>
        <div class="problems" class:clear={!session.problems.length}>
          {#if session.problems.length}
            {#each session.problems as p}
              <button class="problem" onclick={() => session.jump(p.file, p.line)}>
                <span class="mono">{p.file}:{p.line}</span>
                {p.message}
              </button>
            {/each}
          {:else}
            <span>Assembles cleanly.</span>
            <span class="muted">Click the gutter to set a breakpoint. Hover an instruction for what it does.</span>
          {/if}
        </div>
      </section>

      <section class="run">
        <div class="world-card">
          <World />
          <div class="legend">
            {#each PART_NAMES as n, k}<span><i style="background: {PART_COLORS[k]}"></i>{n}</span>{/each}
            <span><i class="aim"></i>aim</span>
            <label class="follow"><input type="checkbox" bind:checked={session.follow} /> follow the mind in the editor</label>
          </div>
        </div>
        <nav class="panel-tabs">
          {#each PANELS as t}<button class:on={session.panel === t} onclick={() => (session.panel = t)}>{t}</button>{/each}
        </nav>
        <div class="panel">
          {#if session.panel === 'mind'}<Mind />
          {:else if session.panel === 'body'}<Body />
          {:else if session.panel === 'weaves'}<Weaves />
          {:else if session.panel === 'order'}<Order />
          {:else if session.panel === 'profile'}<Profile onjump={(f, l) => session.jump(f, l)} />
          {:else if session.panel === 'events'}<Events />
          {:else if session.panel === 'caster'}<Caster />
          {:else if session.panel === 'bytes'}<Bytes onjump={(f, l) => session.jump(f, l)} />
          {:else}<Reference />{/if}
        </div>
      </section>
    </main>
  {:else}
    <p class="loading">Reading the spells…</p>
  {/if}

  {#if note}<div class="toast">{note}</div>{/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100dvh;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 18px;
    align-items: center;
    padding: 8px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .brand {
    display: flex;
    gap: 8px;
    align-items: center;
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .brand .muted {
    font-family: var(--font-ui);
    font-weight: 400;
    font-size: 14px;
  }
  .sigil {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2.5px solid var(--accent);
    box-shadow: inset 0 0 0 3px var(--panel), inset 0 0 0 8px var(--accent);
  }
  .group {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .transport {
    margin-left: auto;
  }
  .tick {
    min-width: 72px;
    color: var(--text-2);
    font-size: 12.5px;
  }
  .mid {
    margin-left: 6px;
    font-size: 11px;
    color: var(--accent);
  }
  .seg {
    display: flex;
  }
  .seg button {
    border-radius: 0;
  }
  .seg button:first-child {
    border-radius: var(--radius-xs) 0 0 var(--radius-xs);
  }
  .seg button:last-child {
    border-radius: 0 var(--radius-xs) var(--radius-xs) 0;
    border-left: none;
  }
  .seg button.on,
  .tabs button.on,
  .panel-tabs button.on {
    background: var(--raised);
    color: var(--text);
    border-color: var(--line-2);
  }
  main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(380px, 1fr) minmax(420px, 1.1fr);
  }
  .code,
  .run {
    min-width: 0;
  }
  .code {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--line);
  }
  .tabs {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 6px 8px;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }
  .tabs button,
  .panel-tabs button {
    border-color: transparent;
    background: none;
    color: var(--text-2);
    white-space: nowrap;
  }
  .tabs .more {
    max-width: 130px;
  }
  .spacer {
    flex: 1;
  }
  .editor-wrap {
    flex: 1;
    min-height: 0;
  }
  .problems {
    max-height: 120px;
    overflow: auto;
    padding: 6px 10px;
    border-top: 1px solid var(--line);
    background: var(--danger-soft);
    font-size: 12.5px;
  }
  .problems.clear {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    background: var(--panel);
    color: var(--ok);
  }
  .problem {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    color: var(--text);
    padding: 2px 0;
    min-height: 0;
  }
  .problem .mono {
    color: var(--danger);
    margin-right: 8px;
  }
  .run {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .world-card {
    padding: 10px 12px 6px;
    border-bottom: 1px solid var(--line);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    align-items: center;
    font-size: 12px;
    color: var(--muted);
    padding-top: 2px;
  }
  .legend i {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    margin-right: 5px;
    vertical-align: -1px;
  }
  .legend i.aim {
    border-radius: 50%;
    border: 1.5px solid var(--danger);
    width: 7px;
    height: 7px;
  }
  .follow {
    margin-left: auto;
    display: flex;
    gap: 5px;
    align-items: center;
  }
  .panel-tabs {
    display: flex;
    gap: 2px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }
  .panel-tabs button {
    text-transform: capitalize;
  }
  .panel {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 12px 14px 24px;
  }
  .loading {
    padding: 40px;
    color: var(--muted);
  }
  .toast {
    position: fixed;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 14px;
    border-radius: var(--radius-sm);
    background: var(--pop);
    border: 1px solid var(--line-2);
    box-shadow: var(--shadow);
    font-size: 13px;
    max-width: calc(100vw - 32px);
  }

  @media (max-width: 900px) {
    .app {
      height: auto;
      min-height: 100dvh;
    }
    main {
      grid-template-columns: minmax(0, 1fr);
    }
    header {
      padding: 8px 12px;
    }
    .code {
      border-right: none;
      border-bottom: 1px solid var(--line);
    }
    .editor-wrap {
      height: 55vh;
      flex: none;
    }
    .transport {
      margin-left: 0;
    }
    .panel {
      overflow: visible;
    }
  }
</style>
