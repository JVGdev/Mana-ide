# Plan: real matter, real air, real heat

*Before anything here: the main objective (SPEC §0) comes first. Every feature of a spell comes from Ikozu's physics. The
last plan (now in git history) is built, and so is the audit's list (SPEC D32–D39). This plan is what that work turned up
(SPEC §0, What's left), shaped by your answers of 9 October.*

*Speed doesn't constrain any of it: realism comes first. The engine moves to Rust before step 2 (step R), so that steps 2–4
are built once, in the engine that keeps them. Each step is still specified in SPEC as physics, and the tests go with it.*

```
0 Land organic-physics
1 Mass is mana ──► R The engine in Rust ──► 2 One matter ──► 3 Real air ──► 4 Heat
5 The mind's reach   (independent; after R)
6 Bodies pay for gathering   (small; after R)
                                                          all ─► 7 Spells, bench, SPEC
```

1 comes first because every number after it depends on the weights. R comes next because 2 replaces the whole matter
engine, and building it in TypeScript first would mean building it twice. 2 is the biggest physics step, and 3 and 4 build
on it: real air is a gas made of matter, and heat lives in matter, mana and air alike. 5 can be built any time after R.

## Decided (9 October)

| # | Question | Your answer | What it means for the physics |
|---|---|---|---|
| A | How heavy is mana? | Each part has its own weight: fire the lightest, then air, then water, earth the heaviest. | Mass is mana, by part: 1 M of each part weighs its own amount, free or condensed, and matter is heavy because it's packed. The order is set; how far apart the parts are is tuning. Starting numbers in step 1. |
| B | Is the open air real air? | Yes. | The atmosphere is air matter, a gas, with free mana in it (step 3). |
| C | What makes a fire hot? | Fire mana is more easily turned into heat. Heat, like light, is Energy. | A mind can transform mana into heat as well as into motion (D32). How easily depends on the part: fire into heat most easily. Light is Energy too: hot things glow, and carry heat away as they do (step 4). |
| D | How does influence reach past the aura? | The aura's touch is easy. Further, a mage has to work: unaided, he works out the space's geometry in his mind and pinpoints x, y, z. Probing the space with raw free mana, which sends knowledge back, is one way to do it better. | The aura is the body's outline. Past it, every act costs the mind the work of finding where it is, and sensing what's there needs something to carry the knowledge back (step 5). |
| E | A pull request? | Commit, no PR; work on `main` from now on. | Done: `main` has the organic physics. |
| F | Change the engine's language? | Yes: port it to Rust, before anything else. | Step R. The machine, the world, the assembler and the tools go to Rust: native for the tests, the bench and the CLI, and WebAssembly for the tester. The physics doesn't change on the way (SPEC D41). |
| G | Soil or rock under a Stone Wall? | Both can be there. A mage grabs rock if there is some, and compresses soil first if there isn't; that's spell logic. | The physics lets earth's packing decide what it is (step 2). How a spell uses it is the author's. |

**And the follow-ups:**

| # | Question | Your answer | What it means for the physics |
|---|---|---|---|
| C1 | Does each part turn into some Energy most easily? | Air turns more easily into motion (wind blades, anything that just pushes forward) and into sound. Fire into heat. Water and earth: nothing known. | A table, by part and by kind of Energy (motion, heat, sound, light), of how easily a mind turns it: less strain a joule. Air: motion and sound. Fire: heat. Water and earth: like the rest, until you find out. Air mana also drags less on the air around it (your "less friction"), which is a number too, by part (step 3). |
| C2 | Can a mind make light directly? | It could, but then it has to follow light's own rules. | Light is built as real light (step 4), and a mind can turn mana into it like any other Energy. |
| D1 | How does a probe's knowledge get back? | At the mana's speed: the probe has to come back. An order that speeds it up, paying mana, brings it back sooner. | Knowledge rides in mana. A probe's particles keep what they felt in their registers, and the caster reads it when they touch the aura again (step 5). |
| D2 | Can a far point be worked out wrong? | Yes. Fine-tuning and cross-checking a position is a good mage's work. | Past the aura, what the caster senses is off, more the further it is. Looking again, or checking against something known, narrows it (step 5). |

---

## 0. Land what's built *(done)*

- `claude/organic-physics` is committed and on `main`. From here on, everything is committed straight to `main`.

## 1. Mass is mana *(done: SPEC D40)*

*Built as below, with two changes: earth packs full at soil's density (1,600 kg/m³) until step 2 tells soil from rock, or
the ground's soil packs under a wall and the wall sinks into itself; and the Stone Wall is raised by hand, since real air
can't hold up rock. What it found, and hands on:*

- *to step 2: a wall's base is as rough as its particles' scatter, so it tips unless its order holds it (it holds half
  its earth for 8 s, short of its target); rock's bonds fight what rests on them, which the Energy ledger shows; matter let
  go of stops dead;*
- *to step 3: a breath of mana weighs grams and can't knock anyone over until bodies feel the air it drives. The Gust's
  knockback test waits for it.*

- **Each part weighs its own, free or condensed** (A). Matter weighs exactly what the mana it was made of did:
  `matterMass` goes, and condensing and unmaking keep mass and momentum exactly. The `matter` impulse goes too.
- **The weights.** Free mana is thin, so the counts people think in can stay: a mage gathers 120 M, and a cell of air holds
  40 M. Raw mana in the air then weighs about what real air does if 1 M of it weighs half a gram on average. Starting
  numbers, in the order you gave, for you to tune: **fire 0.3 g, air 0.45 g, water 0.55 g, earth 0.7 g** a M.
  - In raw air, fire mana rises at about two-thirds of g, and air mana rises slightly.
  - Water mana sinks slowly, and earth mana at about a third of g.
  - A fireball of 18 M weighs about 5 g: it harms by heat, not by weight, and throwing it doesn't shove its caster.
  - Matter is thousands of M a cell. A cell of rock (2,600 kg/m³) holds about 58,000 M of earth, and a cell of water
    about 28,000 M.
- **How much room matter takes:** each part's density packed full (earth like rock, 2,600 kg/m³; water 1,000) replaces the
  one `cellMatter` for all. Gases have no fixed packing (3).
- **Every number re-anchored to real ones**, through the chosen scale: the air's mana, the ground, how much mana it takes to
  hold matter, `pushEnergy`, `orderBurn`, stiffnesses, body masses. A table in SPEC: each number, the real value it comes
  from, and why.
- **The example spells keep working** by changing their scenes' amounts (`AMOUNT`), not their code. Holding a wall's
  worth of earth now takes far more mana per M of earth, or a far stronger hold per M of mana: that's step 2's force.
- **Tests:** condensing keeps momentum and mass exactly; a cell of earth weighs what earth does; both ledgers balance.

## R. The engine in Rust *(large; before step 2)*

*Your call of 9 October (F): port the engine first, so steps 2–4 are built once, in the engine that keeps them.*

**What moves, and what stays.**

- **To Rust**, a Cargo workspace in `engine/`: everything that isn't the tester's screen.
  - The instruction set, the assembler, the disassembler and the docs of each instruction. One table, read by the
    assembler and the machine alike, so the two can't drift apart.
  - The machine and the world: all of `src/vm`.
  - The scenes, the bench, the text render and the profile.
  - `mas`, `mvm` and `bench`, as native programs.
  - The tests.
- **Stays TypeScript:** the tester (Svelte and CodeMirror) and `scripts/listings.ts`. The tester runs the engine compiled
  to WebAssembly, so it stays a web page, and the engine can still run inside Quire later: that was why it was TypeScript
  (SPEC §10).
- **Doesn't change:** the `.masm` libraries, spells and bench.

**The rule of the port: the same machine, not a better one.**

- Same physics, same numbers, the same order of operations, the same events and faults.
- No fixes to the physics, no tuning, no new features, no speed-ups that change a result. What's found wrong on the way is
  written down here for later, not fixed during the port.
- The TypeScript engine is the reference until the port is done, and neither engine changes physics meanwhile.

**How we know it's the same.**

- **Assembling:** every spell, library and bench file assembles to the same bytes, labels and source lines. Exactly.
- **Traces:** the TypeScript engine writes down the whole world every tick, for each scene: particles, air, matter, bonds,
  bodies, casters, casts, weaves, events, and both ledgers. The Rust engine has to match it.
  - Adding and multiplying are the same IEEE arithmetic in both, so those match to the last bit.
  - Sine, cosine, exp, log, hypot and powers can differ in their last bit between JavaScript and Rust, and a difference that
    small grows over many ticks. So a trace has to match exactly for the first ticks, and closely while the two stay
    together. Where they part, the tests decide.
- **The tests:** all 101 are ported one by one, with the same names, the same checks and the same numbers, and the mana and
  momentum ledgers are still checked every tick. A check that passes in one engine and not the other because a number sits
  on an edge is looked into and brought to you, never quietly loosened.
- **The bench** prints the same table, within what those last-bit differences explain.

**How it's laid out in Rust.**

- Particles live in one list and are named by their id, not by reference: weaves, bonds and the tester hold ids. The same
  goes for bodies, casters, weaves and the program an order runs from.
- Everything the TypeScript walks through in insertion order (its `Map` and `Set`) keeps that order in Rust. The order
  changes how floating-point sums round, and so the results.
- An immediate in the bytecode stays a 32-bit float, as `Math.fround` makes it now.
- No `unsafe`, and no threads yet: the port is single-threaded, like the engine it copies.

**The steps.** Each one is committed on `main` when its checks pass.

1. **Toolchain and skeleton.** The `engine/` workspace: `mana` (the library), `mana-cli` (the programs) and `mana-wasm` (the
   tester's binding). npm scripts that call cargo, and a WebAssembly build. It needs three packages you install:
   `sudo pacman -S rust-wasm wasm-bindgen wasm-pack`.
2. **The instruction set.** isa, the assembler, the disassembler, the docs. The assembler's tests pass, and every `.masm`
   file assembles to the same bytes.
3. **The world without minds.** parts, physics, world, the air, the fluid, the Energy ledger. The physics and Energy tests
   that need no caster pass, and traces of falling, pouring, merging and the air match.
4. **The machine.** The caster, the weave and the machine: minds, bodies, orders, both flaws, both ledgers. The machine's,
   the spells' and the Energy tests pass, and traces of the four spells match.
5. **The tools.** The scenes, the text render, the profile, `mas`, `mvm` and `bench`. The bench's tests pass, and its
   table matches.
6. **The tester on WebAssembly.** `session.svelte.ts` drives the machine through the binding. The world view reads the
   world as arrays, and each panel reads what it shows from the machine. Everything works as it did: casting, playing,
   stepping by instruction and by tick, breakpoints, each particle's order step by step, picking particles, the Energy
   panel, 3D slices, saving files.
7. **Retire the TypeScript engine.** It stays in git history. Then measure the speed: the tests, the four spells in 2D and
   3D, and the bench, against what TypeScript took. Bring SPEC (§10, D41), README and CLAUDE.md up to date. What the
   speed turns out to be decides how stiff step 2's matter can be: real, or a softer stand-in named in §0.

## 2. One matter *(large)*

All matter obeys one mechanics, held by mana or not: the ground, loose earth, water, rock a wall is made of. The cell
rules (`settleMatter`, "earth holds together") and the rigid "carried" matter go.

- **The model:** matter as material points over the grid (the material point method). It's the standard way to have
  water, sand, clay, snow and rock in one simulation, and they push on each other because it's all one grid.
- **Each part behaves as its material** (and a mix as their blend):
  - **water**: a liquid. Nearly incompressible, thin, with surface tension: it sticks to itself and flows. A whip of it holds
    together and bends to its speed and the air on its own; drops bead, streams hold, puddles spread.
  - **earth**: a solid that's cohesive and plastic. Push it past its strength and it takes the new shape and keeps it
    (molding); pull it apart and it cracks. Loose earth is granular: it slides, piles, and stands at its angle of repose.
  - **air matter, flame**: gases (3).
  - **mixes**: mud flows slowly and holds some shape.
- **Mana holds matter by force**, not rigidly (*influence*): free mana pulls the matter in its cell along with it, as hard
  as the mana there can. Matter pulled harder than that tears away and follows its own nature. Holding something heavy
  strains its mana; lifting it is pushing it.
- **Rock is earth's own cohesion**, not bonds a weave makes. Earth packed denser holds harder: loose soil, packed soil
  and rock are one material at different packings, and a world can hold any of them (G). A wall raised by hand, positioned and molded holds because
  earth holds (your answer 1). How to raise it stays spell logic.
- **Matter blocks matter, held or not.** A fireball stops against a Stone Wall's rock. Free mana can't pass through solid
  matter, and in a cell with a little matter (a water film) the two push on each other: the film is pushed, the mana is
  turned.
- **Energy:** stored elastically when matter bends, turned to heat when it yields, cracks, flows or rubs.
- **Tests:**
  - a drop falls, lands, and spreads into a puddle of the right volume;
  - a stream of water pulled at one end stays whole, and one of air mana doesn't;
  - sand piles at its angle of repose;
  - clay bent past its strength stays bent, and a rock beam cracks;
  - a fireball stops against held rock;
  - mass, momentum and Energy balance every tick.

## 3. Real air *(large)*

- **The atmosphere is air matter, a gas, with free mana in it**, both on the grid. Every gas obeys the ideal gas law,
  `p = ρ·R·T` for each part (isothermal until 4).
- **One pressure for everything in a cell:** particles of free mana take up room in the gas, and every phase feels the same
  pressure, `−V∇p`. Buoyancy and the push of a wind's pressure come out of it exactly. This replaces the Archimedes-at-rest
  shortcut (D38).
- **Bodies and matter feel the air:** drag, buoyancy, wind. A Gust's mana drives a real gust of air.
- **Real speed of sound** (340 m/s), since speed is no constraint: `airSound` goes. Sound is the air's pressure and motion
  rippling, and is Energy like any other: the ledger counts it. A mind can make it (step 4).
- **Each part drags on the air by its own amount** (`airDrag`, by part): air mana slips through the air most easily, so a
  blade of it keeps its speed longest (C1). Your guess, so a number to tune.
- **Tests:**
  - a balloon of light gas rises, and rock doesn't care;
  - a gust of mana makes a wind that blows past where its mana stopped;
  - a pressure wave crosses the world at 340 m/s;
  - the ledgers balance.

## 4. Heat *(large)*

- **Everything has a temperature:** each material point, particle and cell of gas. Heat is Energy held there, not a tally
  by cause: `World.heat` becomes a place.
- **Every loss turns into heat where it happens:** drag, friction, impact, thickness, molding, mixing, braking.
- **Heat moves:**
  - by conduction, at each material's own rate (rock slow, water faster);
  - by convection, carried with what moves;
  - by radiation: hot things glow and cool.
- **Hot gas expands:** the ideal gas law with temperature. A flame rises because it's hot, and so does the air over it.
- **Matter changes phase:**
  - water freezes, melts and boils, with latent heat;
  - rock melts into lava;
  - burning (fuel in matter giving off heat) needs its own lore, and waits for it.
- **Turning mana into heat, sound and light** (C, C1, C2): a mind transforms mana into any Energy, not only motion (D32).
  Heat goes straight into what the mana touches; sound is the air pushed back and forth; light leaves where the mana is.
  They're new acts for the body and for orders: what a mind can do, not something a spell was given. Each part turns into
  each kind with its own ease, less strain a joule: air into motion and sound, fire into heat, the rest alike. That's a
  table in `physics.ts`, by part and kind, and air's ease into motion counts for pushes and kicks too.
- **Light, as real light:**
  - it goes in straight lines, at the speed of light, which is instant at the world's scale;
  - it spreads out as it goes, thinning with the square of the distance;
  - matter takes it in, lets it through, or throws it back, by what it's made of: earth takes it in, water and air mostly
    let it through, and water's surface throws some back;
  - what takes it in is warmed by it, and Energy is kept exactly: what leaves one place arrives at another, or is still on
    its way;
  - hot things give it off by their temperature (σT⁴), redder when cooler and whiter when hotter, and cool as they do;
  - it pushes, but so little that nothing in the world will feel it.
- **Felt and harmful:**
  - an order can feel how hot its particle is (a new sense: what a body feels by touch);
  - a caster feels heat on their body;
  - bodies are harmed past what they can bear, by heat or by cold.
- **The Energy ledger** keeps heat where it is: held now equals held at the start, plus what minds and bodies put in, plus
  what the numbers got wrong.
- **Tests:**
  - a hot spot spreads, at the rate of what it's in;
  - a puddle boils away under enough heat, and freezes in cold;
  - flame rises, and hot air rises with it;
  - a body beside a fire takes harm;
  - Energy balances every tick.

## 5. The mind's reach *(medium; after R)*

- **The aura is the body's outline.** What touches it, the caster senses and acts on at once, at no extra cost: a fireball
  poured into the hand is easy to hold and aim.
- **Past the aura, the mind has to find the place.** It acts exactly where it means to: the soul carries it there and
  back. But it knows only roughly where things are. Every sense past the aura (a particle's place and speed, a weave's
  middle, what's at a point) comes back off by an amount that grows with the distance (D2). Each look costs beats, and
  looking again, or checking against something already known, narrows it. That's working out the space's geometry: a
  careless mage pushes the wrong particle, or pours into the wrong place, and a good one cross-checks. Each look errs on
  its own, worked out from the tick, the place and which look it is, so the same cast always goes the same way. The 4 m
  `reach` stat goes, and what limits a mage is what their mind can afford.
- **Probing** (D1): raw free mana (unfiltered) let out with an order keeps what it feels in its registers. It doesn't know
  where it is (D31), but it can count its own way by its speed: so many ticks at so much, this way. What it learned
  reaches its caster only when it comes back and touches their aura, and the caster reads it there (`WGET`). It comes
  back at its own speed, and faster if its order pays to speed it up. It's a way to know, built from the physics, not a
  rule: other ways can be found by playing with it.
- **Fixes D33 and D34**, which put the aura out to 4 m. A push still goes off the body: the soul carries it there and back.
- **Tests:**
  - pouring at the hand costs nothing extra;
  - a particle 3 m away is sensed further off than one 1 m away, and looking at it four times narrows it about by half;
  - a probe sent out and brought back tells its caster what it touched, and it can't before it's back;
  - the same cast, run twice, goes the same way.

## 6. Bodies pay for gathering and pouring *(small; after R)*

- Drawing mana in thins the air, and pouring it into a point presses it together: the exact work of each strains the mind,
  as pushes do. It's small by nature, a few kJ a cast, because every race's body is made for it (your answer 3).
- The ledger's `bodies` term goes: everything that enters comes through a mind.

## 7. Spells, bench and SPEC *(medium)*

- **The four example spells keep working** with the least change. If the Stone Wall can't rise as its order is written once
  rock blocks, it's raised by hand, positioned and molded, as you described.
- **The bench** runs again, with what it finds written down.
- **SPEC:**
  - §0 brought up to date;
  - the decisions as D40 onward;
  - §3–§5 and §11 rewritten from the code;
  - a table of every number and the real value it comes from.

## Risks

- **2 is a new engine for matter.** Material points, plasticity and fracture are well understood, but they're a lot to build
  and to get exact in the ledgers. It replaces the code under every spell, so most physics tests will be rewritten, not
  kept.
- **Coupling** mana particles, matter points and the gas grid, equally and oppositely, is where momentum and Energy will
  leak first. The ledgers catch it the same day.
- **Building it twice.** Settled by step R: the engine moves to Rust first, and 2–4 are built there.
- **The port parts from its reference.** Bit-for-bit agreement can't last past the first ticks where sine, cosine, exp or
  log round differently. From there the tests carry the weight, and a test that sits on an edge has to be looked into, not
  loosened.
- **The tester reads a lot of the machine.** Its panels look at casts, weaves, particles, traces and ledgers directly.
  Giving them all that through WebAssembly is the largest piece of step R that has no reference to match.
- **WebAssembly runs on one thread in the browser.** Native runs (tests, bench, `mvm`) can use every core later; the
  tester gets that only with more build work.
