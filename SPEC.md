# Mana: a language, a machine and a spell tester for Ikozu's magic

*Working name: **Mana**. Files are `.mana` (spells), `.masm` (assembly) and `.mbc` (bytecode); the tools are `manac` (compiler),
`mas` (assembler) and `mvm` (the machine). Rename freely.*

Ikozu's Mana is already written like code in Quire's *Codefied Modern Formulae* notation, but nothing runs it. This project
makes it real:

1. a **machine** that is a caster: a **mind** that computes with numbers and a **body** that moves mana. Its instructions are
   all a caster can do, and nothing more;
2. **libraries** written for that machine: the elements, the shapes, the reactions. A ball is shells out to a radius, each laid out
   ring by ring around its axis with `sin`, `cos` and the circumference, 2πr. A shield is one such shell;
3. a **language**, in the style of the codified spells, that compiles to the machine and calls the libraries;
4. a **spell tester**: an editor, the assembly beside it, and a world where the spell is cast, watched and stepped through.

The examples throughout are the four spells of Ikozu: **Stone Wall**, **Fireball**, **Gust** and **Water Shield**.

---

## 1. Decisions so far

| # | Decision |
|---|---|
| D1 | The old Core (one operation per aspect: FILTER·Fire, SEND·Wall, LOCK·Shield…) is replaced by a machine. |
| D2 | The machine is as close to machine code as possible. It knows **nothing** of elements, shapes or reactions: those are **libraries**, written in its assembly. |
| D3 | The machine runs on a person. The **mind** has number registers. The **body** has mana registers. Registers hold numbers or mana, nothing else: weaves, positions and elements are numbers. |
| D4 | Mana works in **3D**. A **2D** world is a 3D world one cell deep. Libraries read the world's depth and lay out a disc where they'd lay out a ball. |
| D5 | Keep the codified style for the language: `Type name(params) { }` declarations, `IF (…) DO … END IF`, `WHILE (…) DO … END DO`, `.NOT.`, `!` comments, optional `CALL`. |
| D6 | Mana that isn't maintained **joins the body's natural flow** and follows it. If held mana plus the flow goes over the body's capacity, the caster is **overcharged**: they are harmed. Nothing else in the machine depends on it. |
| D7 | **LOCK comes after manifest.** It locks something of the manifestation: its shape, its input, its order. |
| D8 | **Affinities** set how much is lost when filtering, and only then. Less affinity means less filtered mana, so a weaker spell. |
| D9 | How long a hold lasts (**focus**), how many mana registers there are (**streams**) and how fast the flow drains (**drain**) are all stats of the caster's body. |
| D10 | Aspect costs are left out. A spell's strength is the mana it carries. |
| D12 | **Mana is what the world is made of.** Everything physical is mana, condensed. Earth is condensed earth mana, and a plant is condensed water and earth. |
| D13 | A spell can **influence** an element already in the world (move the ground, push the air) or **make** it from its own mana (condense water out of water mana). |
| D14 | A weave set loose **slowly loses** its mana back into the air. A wall stands while its mana holds the earth, then crumbles. *(Now: there's no fixed leak. What a weave loses is what holding it costs: its order burns its mana as it thinks, D27.)* |
| D15 | Every stat of a caster, body and mind, comes from **genetics**, their **condition** right now, and **training**. |
| D16 | Matter can be freed back into free mana, but only through **a flaw**: no instruction does it. `CNDS` checks that an amount fits the room a cell has, and never that it's above nothing. Condensing less than nothing runs backwards (§5, *The flaw*). |
| D17 | **Burning doesn't free anything.** What fire burns is still matter, and so is the fire. |
| D18 | **Spells can be optimized like real algorithms.** Thought costs what it costs a real processor: adding is quick, dividing is slow, a sine is slower. The same spell written better casts faster. The libraries are first drafts, to be made better by whoever writes spells. |
| D11 | The tester is a practical tool, maybe the kind Ikozu's mage-engineers would have, but not dressed up in lore. |
| D19 | **The physics is as real as we can make it.** Spells get better by using the shortcuts reality gives, so the more real the world, the better the spells that can be written for it (§11). |
| D20 | **Mana is chemistry, Energy is physics.** Mana is what things are. Energy is how things happen: force, motion, electricity. Each is conserved on its own, and mana moves Energy only indirectly. Fire mana is the substance of heat; the motion that heat is, is Energy. |
| D21 | **Free mana is a fluid of particles.** It has pressure, and spreads unless something holds it. Particles that share a velocity travel together. |
| D22 | **Pushing mana costs mana.** It is poured onto a particle, and the poured mana goes loose where it was poured. A push changes a particle's speed only so much per tick: a caster speeds mana up by keeping the push going. *(How much it costs: D29.)* |
| D23 | **A construct is held by pushing it.** The caster keeps its particles in by pushing them back, as many and as often as their mind allows. A particle that gets out of the caster's field leaves the weave. Locking a shape is a loop in a library, not an instruction. |
| D24 | **Orders are reactions ingrained in mana.** Each particle carries its order. An order can spend its own particle's mana to push it. |
| D25 | **Orders don't spread to other mana**, except through a second flaw (§11, *The second flaw*). |
| D26 | **A weave in hand is held still.** Its mana counts toward the body's load because the body holds it: it stays where it was laid until `MANI` lets it go. |
| D28 | **Things weigh.** A tick is 1/30 s, and things fall at 9.8 m/s². Matter held by mana weighs what that much matter weighs (a full cell of earth, 25 kg), and the mana holding it has to carry it. What rests on the ground is held up by the ground. |
| D29 | **A push costs the kinetic energy it adds** (D22, made exact): mana poured onto a particle turns into its motion. Speeding up costs more the faster it's already going. Slowing down costs nothing: what's taken out of the motion is heat. Holding something up against its weight costs nothing; lifting it costs its weight times the height. |
| D30 | **Earth held by mana is rock.** When a weave holding earth is let go of, its particles are bound to their neighbours, and keep their shape. Rock cracks where it's bent too far or made to hold too much, and comes apart as its mana lets go of the earth. |
| D31 | **An order only feels.** It knows its own particle (its mana, its speed, whether it touches something), the mana around it (how dense, which way it thickens, how it moves), its weave's age, and what its caster wrote into the weave's registers. It isn't told where it is, where its weave's centre is, or where its maker is. What it has to know beyond that, its caster works out and writes in: once for the whole weave, or tick by tick while they keep it up. |
| D27 | **Knowing costs.** An order is ingrained one particle at a time, at a beat for every instruction it could run, and every beat it thinks burns its particle's mana. A short order is cheap to ingrain and cheap to keep; an order that senses more costs more. |

---

## 2. Layers

```
  spells        Fireball, Stone Wall…           .mana, or .masm by hand
  ─────────────────────────────────────────────
  language      the codified style               compiles to assembly, calls libraries
  ─────────────────────────────────────────────
  libraries     Elements, Shapes, Reactions,     assembly: loops, sin, cos, sqrt
                Transformations, Basics
  ─────────────────────────────────────────────
  machine       mind + body                      ~60 instructions, numbers and mana
  ─────────────────────────────────────────────
  world         cells, air mana, matter          physics: not programmable
```

Every layer can be read and stepped through in the tester, down to the bytes.

---

## 3. The world

### Space and time

- The world is a 3D grid of **cells** (0.25 m by default; the `CELL` port says). A 2D world is one cell deep (`DEPTH` = 1).
- Positions are three numbers `(x, y, z)` in metres, with **y up**. In 2D, `z` is 0.
- Time moves in **ticks**.

### Mana is four parts

The Law of Equality says *M = Mf + Mw + Ma + Me*, each a quarter of a particle. Every amount of mana, anywhere, is four
amounts, one per part. The machine numbers the parts **0, 1, 2, 3**. Calling part 3 "earth" is what the Elements library does.

- **Raw** mana, gathered from the air: a quarter in each part.
- **Pure** mana: one part only. Filtering makes it.
- **Residue**: raw mana with one or more parts taken out.

Mana answers to only four names (the Law of the Four), so the machine has no part 4. Filtering by anything else fails.

### Free and condensed

Mana is what the world is made of, and it is always in one of two states:

- **Free** mana flows. It's in the air, in a body, in a register, in a weave, or loose after a spell.
- **Condensed** mana is **matter**. Each cell's matter is four amounts too, one per part, and what it *is* comes from that mix.

The Law of Conservation covers both. No instruction creates or destroys mana; it only moves it and condenses it. The tester's
**ledger** always balances:

```
Σ free (air, flows, registers, weaves, loose)  +  Σ condensed (all the matter in the world)  =  constant
```

### Matter

How matter behaves comes from its parts. Each part brings its own properties, and a mix behaves as their blend:

| Part | Brings | Alone it is |
|---|---|---|
| Fire | heat, rises, spreads thin into warmth | flame |
| Water | flows, fills what's below it | water |
| Air | light, fills what's empty, is pushed by moving air mana | air |
| Earth | heavy, holds together, piles up | stone, soil, sand |

So **earth and water** together flow slowly and hold some shape: mud, or with the right order, a plant. Names like *mud* and
*plant* aren't physics. They belong to the Elements library and to lore. The world itself runs as a grid of cells following
these properties: earth falls and piles, water flows, air fills, flame rises and fades.

### What mana does to matter (the physics)

These rules belong to the world, not to the machine:

1. **Pure free mana binds matter of its own part** in the cell it's in (*influence*): a weave's earth mana takes hold of the
   earth there, water mana of the water. How much it can hold depends on how much mana is there. Too little, and some matter
   is left behind.
2. **Bound matter moves with its mana, and weighs.** It adds its mass to the particle holding it (`matterMass`: a full cell
   of earth is 25 kg, of water 15.6 kg), so the particle is heavier to push and falls with it. Earth mana that moves up
   carries its earth with it, paying to lift it, and the ground it left is empty. Matter that is no longer bound follows its
   nature again: lifted earth falls.
3. **Mana can condense** (*make*): an order can turn some of its particle's free mana into matter of the same parts (`CNDS`).
   Condensed matter is real. It stays when the weave is gone. Nothing natural frees it again: burning only changes what
   matter is mixed with fire, and a flame thins out into warmth that is still matter. Freeing it is possible, but only through a
   flaw in condensing (§5, *The flaw*).
4. **Free mana is a fluid of particles** (§11). It presses on itself and spreads, drags the air it moves through and is
   dragged by it (**wind**), and strikes the bodies it runs into. Loose mana that has slowed to the speed of the air around it
   settles into it.
5. **Matter blocks matter.** Mana holding matter can't move into a cell without room for it, unless that room is taken by
   its own weave's matter, and coming down it lands on any solid matter, however loose. Free mana stops against solid
   matter. What rests on the ground is held up by it, and its friction keeps it from sliding. Being against matter, or a
   body, is a **touch**.
6. **Earth held by mana is rock** (D30). A weave's earth particles are bound to their neighbours when it's let go of. A
   bond keeps its length, and breaks if it's bent too far, made to hold more than it can, or either particle stops holding
   earth. Water held by mana is water: it holds together, and can't be squeezed past full.
7. **Holding costs.** A weave doesn't leak by itself any more. What holds it together is its caster's pushes or its own
   order, and an order burns its mana as it thinks (D27). Less mana binds less matter, so a Stone Wall slowly crumbles as its
   earth falls free.

Every rule has numbers to tune: how much matter 1 M binds, how hard each part presses, what a push costs, how much an order
burns, and so on. They live in one table (`src/vm/physics.ts`).

**Earth holds together.** A cell of solid earth with solid earth beside it stays where it is, even over a hole, so the ground
around a Stone Wall's trench doesn't pour in like sand. Loose earth (less than solid, or with nothing beside it) falls and
piles.

---

## 4. The caster

The machine is a person, and its limits are their stats. Every stat, body or mind, is made of three things:

```
stat = genetics × condition + training
```

- **Genetics** is what they were born with: a people, a bloodline, a gift.
- **Condition** is how they are right now, from 0 to 1. Tired, hurt or drunk lowers it. Overcharge harm lowers it too, which
  is where harm ends up.
- **Training** is what they've learned or drilled: a mage who has trained their streams works more of them than they were
  born with.

The tester shows all three, so the same caster can be tried rested, exhausted or after ten years of study.

### The body

| Stat | Meaning |
|---|---|
| `capacity` | The most mana the body can bear. Past it, **overcharge**: the caster is harmed, and the tester counts how much. |
| `baseline` | The body's own flow, before any spell. |
| `drain` | How much flow above the baseline leaves the body each tick, back into the air. |
| `affinity[0..3]` | 0–100% per part: how much of that part filtering keeps. |
| `focus` | How many ticks a CIRCULATE holds a mana register. |
| `streams` | How many mana registers (`m0`… up to `m7`) the caster can use. |
| `reach` | How far from the body, in metres, the caster can push mana or ingrain an order into it. An adept reaches 4 m. |

### The mind

| Stat | Meaning |
|---|---|
| `speed` | **Beats** of thought per tick. |
| `registers` | How many number registers the mind has (`n0`… up to `n31`). A child might think with 8. |
| `memory` | How many numbers the mind's memory holds (up to 256), and how deep its stack goes. |
| `conditioning` | Per spell: how many times the caster has cast it. |

A mind with fewer than 32 registers can't run a routine that needs more. A library says what it needs, so a child's mind can't
hold `Shapes.ball` as written, but a simpler ball could be written for one.

**The Law of Conditioning.** A tick holds `speed × (1 + c)` beats, where `c` is the conditioning for the spell being cast.
- Mind instructions take 1 beat, except slow math: `DIV`, `MOD` and `SQRT` take 4, and `SIN`, `COS`, `TAN`, `ATAN` and
  `ATN2` take 8.
- Body and reach instructions take 4 beats.

So the usual tricks pay off: compute a sine once outside a loop instead of every time through it, multiply by `1/x` instead
of dividing by `x`, use a shape's symmetry. The tester's profiler (`mvm --profile`) shows which routines and lines the beats
went to.
- `TICK` ends the tick.

So a spell that takes 12 ticks the first time takes about 1 tick after a lot of practice. That's `t / (1 + c)`, and it falls
out of the machine without being written anywhere.

### Each tick

1. **The mind thinks:** it runs instructions until the tick's beats run out or a `TICK`.
2. **Holds run down.** A mana register whose hold has run out (`focus` ticks after its last CIRC) **joins the flow**.
3. **The flow drains:** whatever is above the baseline leaves, at most `drain` per tick, into the air around the caster.
4. **Overcharge:** if `flow + held mana + weaves still in hand > capacity`, the excess is counted as harm.
5. **Weaves hold:** every weave takes hold of the matter its free mana can bind (and lets go of what it no longer can).
6. **Orders run:** every ingrained particle of a weave set loose runs its order, and pays for it (§5).
7. **The world moves:** the mana (weight, pressure, holding together, rock, the air, the ground, what it runs into),
   particles at rest together merging and thin ones splitting, particles that strayed past their weave's field, loose mana
   settling, pushed bodies, falling and flowing matter, the air flowing. A weave in hand stays still (D26).

Holding comes before the orders, so a Stone Wall has its earth in hand before its first rise.

---

## 5. The machine

### Registers

| | Registers | Holds |
|---|---|---|
| Mind | `n0`–`n31` | Numbers (floating point). Also positions, weaves, element parts: everything that isn't mana. Only the first `registers` exist. |
| Body | `m0`–`m7` | Mana: four parts each, and how long it is still held. **Linear**: mana is moved, split and joined, never copied. Only the first `streams` exist. |
| Mind | `flags` | The result of the last `CMP`. |

**Triples.** Where an instruction takes a position or a direction, it names the first of three registers. `n4:6` is `n4, n5,
n6` read as `(x, y, z)`.

**Calling convention.** Arguments go in `n0`–`n15`, and a routine may change any of them. `n16`–`n31` belong to the caller and
a routine keeps them. A routine that lays out mana takes it in **`m1`**, so a caster with only two streams can still use it.

### Weaves

A **weave** is mana laid out in the world as a thing: a fireball, a wall, a shield. A weave is a number, its id, like a file
handle. Each weave has:

- an **origin**, and a **frame** (x right, y up, z forward) that `TURN` rotates about the vertical. Positions given to `EMIT`
  are in the weave's frame, from its origin. A library can lay a ball out around `(0, 0, 0)` without knowing where it is.
- **particles**: its mana, each particle with the matter it binds (§11). `PCNT` counts them and `PPOS`/`PVEL` sense one,
  by its number;
- a **field**: how far around its centre it reaches (`HOLD`). A particle that strays further leaves the weave;
- **registers** `w0`–`w7`, numbers its order can read and write (a velocity, a phase);
- an **order**: a routine (`ORDR`) that each particle it's ingrained into (`INGR`) runs, every tick, once the weave is set
  loose.

A weave is **in hand** from `WEAV` until `MANI`: its mana counts toward the body's load, and stays where it was laid (D26).
`MANI` sets it loose. From then on its mana moves, and is measured from its centre, which moves with it. `LOCK` can fix its
input (no more mana goes into it) or its order (it can't be given another one). A shape isn't locked: it's held, by the
caster's pushes (`SHOV`) or by its own order (`KICK`). A weave whose mana has all gone, or that came apart, can still be named
by its caster: there's nothing in it.

### The order: mana running code

ORDER, in the old Core, was *give an order to the mana particles*. Here it is literal. A weave's order is an assembly routine
that **each particle it's ingrained into runs every tick**, like a tiny mind inside the mana. A particle thinks with
`n0`–`n15` of its own and has no mana registers. When it starts, its registers hold:

| Register | |
|---|---|
| `n0:2` | 0. *(They used to hold where the particle is from its weave's centre: D31.)* |
| `n3` | How much free mana it holds. |
| `n4` | The weave's age, in ticks since it was set loose. |

An order can do arithmetic and jumps, read its weave's registers and the ports below, and use the **order** instructions
(`KICK`, `TUCH`, `GETW`, `PUTW`, `DISS`, `CNDS`, `DENS`, `GRAD`, `NVEL`). It ends with `RET`. Every beat it thinks burns
some of its particle's mana into the air (`orderBurn`). An order that thinks more than 64 beats in one tick **frays**: the
weave comes apart, and its mana goes loose.

What an order knows is what it reads (D31): its mana and its weave's age for free, the weave's registers, and, at a price in
beats, its own speed (`VEL`), whether it touches something (`TUCH`), and what it feels around it (`DENS`, `GRAD`, `NVEL`).
It isn't told where it is. Anything more it needs, its caster writes into the weave's registers: the Stone Wall's caster
works out a schedule for it once, and the Water Shield's tells it how to move every tick it keeps it up.

### Ports

`IN` reads the caster's will and senses. The will is live: it comes from the person casting (in the tester: the mouse, the
keyboard, sliders).

| Port | Size | What |
|---|---|---|
| `AIM` | 3 | Where the caster means it to go. |
| `HAND` | 3 | The casting hand. |
| `SELF` | 3 | The caster's body. |
| `AMOUNT` | 1 | How much the caster means to gather. |
| `FORCE` | 1 | How hard. |
| `MAINTAIN` | 1 | 1 while the caster keeps the spell going. |
| `CELL`, `DEPTH` | 1 | The world's cell size, and its depth in cells (1 = 2D). |
| `LOAD`, `CAPACITY` | 1 | The body's load now, and its capacity. |
| `REACH` | 1 | How far from the body the caster can push. |
| `VEL` | 3 | *In an order:* how this particle moves, in the weave's frame. |

### Instructions

`d` is a destination register, `s` a source register or an immediate (`#3.5`), `n:3` a triple, `L` a label.

#### Mind

| Op | Mnemonic | Does |
|---|---|---|
| `00` | `NOP` | |
| `01` | `HALT` | The spell ends. Its result is the weave in `n0`, if any. |
| `02` | `FAIL #code` | The spell ends, failed. |
| `03` | `TICK` | End this tick's thinking. |
| `08` | `JMP L` | |
| `09`–`0E` | `JEQ` `JNE` `JLT` `JLE` `JGT` `JGE L` | Jump on the flags. |
| `0F` | `CALL L` / `RET` (`07`) | |
| `10` | `LDI d, #imm` | |
| `11` | `MOV d, s` | Numbers only: **no instruction copies mana.** |
| `12`–`16` | `ADD` `SUB` `MUL` `DIV` `MOD d, s` | `d = d op s`. Dividing by 0 gives 0. |
| `17`–`1B` | `NEG` `ABS` `SQRT` `FLOOR` `ROUND d` | |
| `1C`–`1F` | `SIN` `COS` `TAN` `ATAN d` | Radians. |
| `20` | `ATN2 d, s` | `d = atan2(d, s)` |
| `21`–`22` | `MIN` `MAX d, s` | |
| `23` | `CMP a, s` | Set the flags. |
| `24`–`25` | `PUSH` `POP n` | The mind's stack. |
| `26`–`27` | `LD d, [n]` / `ST s, [n]` | The mind's memory: 256 numbers. |
| `28` | `IN d, PORT` | Read a port (into a triple, for size-3 ports). |

#### Body

| Op | Mnemonic | Lore | Does |
|---|---|---|---|
| `30` | `GATH m, s` | GATHER | Draw `s` M of air mana from around the body into `m`, raw. Draws less if the air is thin. |
| `31` | `CIRC m` | CIRCULATE | Hold `m` for `focus` ticks. |
| `32` | `FILT md, ms, s` | FILTER | Move part `s` (0–3) of `ms` into `md`: `md += part × affinity[s]`. The rest of the part, `part × (1 − affinity[s])`, slips into the flow. The other parts stay in `ms`. |
| `33` | `SPLT md, ms, s` | | Move `s` M of `ms` into `md`, keeping its proportions. |
| `34` | `JOIN md, ms` | | Move all of `ms` into `md`. |
| `35` | `MEAS d, m` | | How much `m` holds. Feeling it doesn't spend it. |
| `36` | `PART d, m, s` | | How much of part `s` `m` holds. |
| `37` | `VENT m` | | Let all of `m` out, gently, around the body. |

#### Reach

| Op | Mnemonic | Lore | Does |
|---|---|---|---|
| `40` | `PROB d, n:3, s` | PROBE | How much matter of part `s` is in the cell at `n:3`. |
| `41` | `AIRM d, n:3, s` | PROBE | How much air mana of part `s` floats at `n:3`. |
| `42` | `SEND m, n, n:3, n:3` | SEND | Let `n` M of `m` out at a position, with a velocity, loose. The speed is paid for from what's sent, by the kinetic energy it carries: a share goes into the air there. |
| `43` | `WPOS n:3, w` | | Where the weave's centre is, in the world. |
| `44` | `WVEL n:3, w` | | How the weave's mana moves on average, in its frame. |

#### Weave

| Op | Mnemonic | Lore | Does |
|---|---|---|---|
| `50` | `WEAV d, n:3` | | Begin a weave with its origin at `n:3`. Its id goes into `d`. In hand. |
| `51` | `TURN w, n:3` | POSITION | Turn the weave's frame so forward points along `n:3` (about the vertical). |
| `52` | `EMIT m, n, w, n:3` | | Pour `n` M of `m` into the weave as particles, at `n:3` in the weave's frame, within the cell there. Gives what there is if `m` holds less, and nothing at a point outside the world. |
| `53` | `WSET w, #k, s` / `WGET d, w, #k` (`54`) | | Write and read a weave's registers. |
| `55` | `ORDR w, L` | ORDER | Give the weave its order: the routine at `L`. |
| `56` | `MANI w` | SEND | Set the weave loose. It leaves the body's load, its mana is free to move, and its ingrained particles run their order. |
| `57` | `LOCK w, INPUT \| ORDER` | LOCK | Lock it. Only after `MANI`. |
| `58` | `RELS w` | | Let the weave go: its mana goes loose where it is. |
| `59` | `PCNT d, w` | | How many particles the weave holds. |
| `5A` | `PPOS n:3, w, s` | PROBE | Where particle `s` is, from the weave's origin, in its frame. |
| `5B` | `PVEL n:3, w, s` | PROBE | How particle `s` moves, in the weave's frame. |
| `5C` | `SHOV m, w, n, n:3` | PUSH | Push particle `n`: change its velocity by `n:3`, at most `pushRate` a tick. It costs the kinetic energy it adds, at `pushEnergy` for each M, from `m`, poured into the air there (D29). Slowing a particle costs nothing. Only within reach. |
| `5D` | `INGR w, s` | ORDER | Ingrain the weave's order into particle `s`. 4 beats, and one more for every instruction the order could run. Only within reach. |
| `5E` | `HOLD w, s` | | The weave's field: `s` metres around its centre. |

#### Order (only inside an order)

| Op | Mnemonic | Does |
|---|---|---|
| `60` | `KICK n:3` | Push this particle: change its velocity by `n:3`, in the weave's frame, paid from its own mana (never more than half of it at once), by the kinetic energy it adds (D29). 4 beats. |
| `61` | `TUCH d` | `d = 1` if this particle is against matter, or a body, that isn't its maker's. |
| `62` | `GETW d, #k` / `PUTW #k, s` (`63`) | Read and write this weave's registers. |
| `64` | `DISS` | The whole weave comes apart. Its mana goes loose where it is, and forgets its orders. |
| `65` | `CNDS s` | Condense `s` M of this particle's free mana into matter of the same parts (*make*). The particle carries it, bound by whatever free mana is left. Only as much as its cell has room for. |
| `66` | `DENS d` | How dense the mana around this particle is (M/m³). 4 beats. |
| `67` | `GRAD n:3` | Which way, and how steeply, the mana around it thickens. 4 beats. |
| `68` | `NVEL n:3` | How its neighbours move, on average. 4 beats. |

#### The flaw

Every instruction that takes an amount of mana treats an amount below nothing as nothing: `GATH m0, #-50` gathers nothing.
Every instruction except one.

`CNDS` asks one question of its amount: does the cell have room for it? It never asks whether the amount is above nothing,
because nobody thought to condense less than nothing. Below nothing, condensing runs backwards. The matter the particle holds
comes apart into free mana of the same parts, as much of it as asked:

```
unmake: CNDS  #-10                ; 10 M of the matter this weave binds, back to free mana
        RET
```

It only frees what the weave holds, so a weave has to take hold of the matter first: an earth weave in the ground, a water
weave in a lake. The Law of Conservation still holds; no mana is made, it just stops being matter.

It is not an instruction. There is no opcode for it, nothing in the assembler knows about it, and nothing in a library uses
it. A tester shows it only the way it shows anything: the ledger's condensed mana goes down. Whoever finds it finds it by
reading what the body does, not what it's taught to do.

### Encoding

One opcode byte, then the operands:

- a register: one byte, with `0` in the top bit for `n0`–`n31` and `1` for `m0`–`m7`;
- a triple: its first register;
- a port, or `INPUT`/`ORDER`: one byte;
- a label: two bytes, the address;
- an immediate: four bytes (float32, little-endian). An instruction whose last operand is an immediate sets the opcode's top
  bit.

```
GATH m0, n17               →  30 80 11
FILT m1, m0, #3            →  B2 81 80 00 00 40 40
EMIT m1, n6, n4, n13:15    →  52 81 06 04 0D
```

### Faults

| Fault | When |
|---|---|
| `NO_STREAM` | A mana register beyond the caster's `streams`. |
| `NO_ROOM` | A number register beyond the mind's `registers`, or the stack or memory past its size. |
| `NO_NAME` | `FILT` by a part other than 0–3. Mana doesn't answer (the Law of the Four). |
| `NOT_LOOSE` | `LOCK` before `MANI`. |
| `LOCKED` | `EMIT` into a weave with locked input, or `ORDR` on one with a locked order. |
| `FRAYED` | An order ran too long in one tick. The weave comes apart. |
| `NOT_YOURS` | A weave id that isn't one of this caster's. |
| `NO_ORDER` | `INGR` into a weave that hasn't been given an order. |
| `ORDER_ONLY` | An order's instruction (`KICK`, `TUCH`…) in a mind. |
| `NOT_IN_ORDER` | A body, reach or weave instruction inside an order. The weave frays. |
| `BAD_PORT` | `VEL` read by a mind, or a will port read by an order. |

Overcharge is not a fault (D6). It's harm, counted by the tester.

---

## 6. The libraries

Libraries are written in assembly, or in the language once it exists. They are Quire's libraries: what a caster knows decides
which they can use. The machine has no shapes or elements. Change `Shapes.ball` and every fireball changes. Someone's own ball,
`Correni.Shapes`, can be rounder. The listings here are the files in `lib/`.

### Elements

```
; Elements: the four names mana answers to (the Law of the Four).
; The machine only knows parts 0–3. This is where they get their names.
        .const FIRE   0
        .const WATER  1
        .const AIR    2
        .const EARTH  3
```

Compound aspects, like Quire's Plant (born from Water and Earth), are routines here that filter two parts and join them.
Mana mixed like that, condensed, is matter of both parts. The Elements library is also where mixes get their names: earth and
water condensed is *mud* or *plant*, depending on how they're ordered.

### Basics

How to ingrain an order, how to throw a weave by pushing it, and how to keep a weave with its caster: the caster tells it
how to move every tick (`keep`), and its order moves that way (`follow`). The order can't see its maker (D31).

```
; Basics: what every caster of the system knows.

; toward: a velocity from n0:2 to n3:5 at speed n6.  Out: n3:5
toward: SUB   n3, n0
        SUB   n4, n1
        SUB   n5, n2
        MOV   n7, n3
        MUL   n7, n3
        MOV   n8, n4
        MUL   n8, n4
        ADD   n7, n8
        MOV   n8, n5
        MUL   n8, n5
        ADD   n7, n8
        SQRT  n7                  ; the distance
        DIV   n6, n7              ; speed per metre of it
        MUL   n3, n6
        MUL   n4, n6
        MUL   n5, n6
        RET

; ingrain: the weave's order (ORDR) into every one of its particles, one by one. Each takes a beat for every instruction
; the order could run, so a long order takes a while to ingrain. It keeps holding m0 as it goes, for what comes after.
;   in: n4 weave
ingrain: PCNT n5, n4
        LDI   n6, #0
.next:  CMP   n6, n5
        JGE   .done
        CIRC  m0
        INGR  n4, n6
        ADD   n6, #1
        JMP   .next
.done:  RET

; throw: push every particle of a weave toward a velocity, round and round, until the weave moves at it or leaves the
; caster's reach. Each push pays for itself from m0, and can only speed a particle up so much a tick.
;   in: n0:2 the velocity (in the weave's frame), n4 weave
throw:  IN    n15, REACH
        MUL   n15, n15            ; reach², to compare without a square root
.pass:  CIRC  m0                  ; keep holding what it pays with
        WPOS  n8:10, n4
        IN    n11:13, SELF
        SUB   n8, n11
        SUB   n9, n12
        SUB   n10, n13
        MUL   n8, n8
        MUL   n9, n9
        MUL   n10, n10
        ADD   n8, n9
        ADD   n8, n10
        CMP   n8, n15
        JGT   .done               ; out of reach: it's on its own
        WVEL  n8:10, n4
        SUB   n8, n0
        SUB   n9, n1
        SUB   n10, n2
        MUL   n8, n8
        MUL   n9, n9
        MUL   n10, n10
        ADD   n8, n9
        ADD   n8, n10             ; how far from the velocity it still is, squared
        MOV   n11, n0
        MUL   n11, n0
        MOV   n12, n1
        MUL   n12, n1
        ADD   n11, n12
        MOV   n12, n2
        MUL   n12, n2
        ADD   n11, n12
        MUL   n11, #0.0025        ; (5% of the speed)²
        CMP   n8, n11
        JLE   .done               ; fast enough
        PCNT  n5, n4
        CMP   n5, #0
        JEQ   .done               ; nothing left of it to push
        LDI   n6, #0
.p:     PVEL  n8:10, n4, n6
        MOV   n11, n0
        SUB   n11, n8
        MOV   n12, n1
        SUB   n12, n9
        MOV   n13, n2
        SUB   n13, n10
        SHOV  m0, n4, n6, n11:13
        ADD   n6, #1
        CMP   n6, n5
        JLT   .p
        JMP   .pass
.done:  RET

; follow (an order): each particle pushes itself toward the velocity in w5–w7 (in its weave's frame), paying with
; itself, and does nothing when it's near enough to it. It doesn't know where it is, or where anything else is: its
; caster has to tell it how to move (keep), and while nobody does, it holds still where it is.
follow: GETW  n5, #5
        GETW  n6, #6
        GETW  n7, #7              ; the speed it's told
        IN    n8:10, VEL
        SUB   n5, n8
        SUB   n6, n9
        SUB   n7, n10             ; how far from it
        MOV   n8, n5
        ABS   n8
        MOV   n9, n6
        ABS   n9
        ADD   n8, n9
        MOV   n9, n7
        ABS   n9
        ADD   n8, n9
        CMP   n8, #0.01
        JLT   .done
        KICK  n5:7
.done:  RET

; keep: for as long as its caster maintains it, keeps a loose weave where it was from them when this began. Each tick, it
; tells its order (follow) the velocity that would bring it there in two ticks, in w5–w7. Then it tells it to stay still.
;   in: n4 weave (loose, not turned)
keep:   WPOS  n8:10, n4
        IN    n11:13, SELF
        SUB   n8, n11
        SUB   n9, n12
        SUB   n10, n13            ; where it is from its caster
.tick:  IN    n5, MAINTAIN
        CMP   n5, #0
        JEQ   .stop
        IN    n11:13, SELF
        ADD   n11, n8
        ADD   n12, n9
        ADD   n13, n10            ; where it should be
        WPOS  n14:16, n4
        SUB   n11, n14
        SUB   n12, n15
        SUB   n13, n16
        MUL   n11, #0.5
        MUL   n12, #0.5
        MUL   n13, #0.5           ; half the way there each tick
        WSET  n4, #5, n11
        WSET  n4, #6, n12
        WSET  n4, #7, n13
        TICK
        JMP   .tick
.stop:  WSET  n4, #5, #0
        WSET  n4, #6, #0
        WSET  n4, #7, #0
        RET
```

`throw` pushes every particle in turn, so it costs about 20 beats a particle a pass. An adept throwing a 72-particle fireball
gets round it once every five ticks, and the ball leaves their reach before it's as fast as they meant. A master gets round
it every tick.

### Shapes

A shape lays the mana in `m1` out in the weave, around its origin. It's plain geometry. A big shape takes a mind many
ticks to lay out, longer than a hold lasts, so each shape re-`CIRC`s its mana as it goes. Without that, the mana slips into
the body's flow halfway through, and half a wall is laid out with nothing. The weave is in hand while it's laid out, so
what's laid stays where it's laid (D26).

The ball is built the way you'd draw one by hand. Take the radius, and go out from the centre in shells. Each shell is
rings around the up axis, from its top to its bottom: a ring at angle φ down the shell has radius `ρ sin φ` and height
`ρ cos φ`, and as many points as its circumference, `2πs`, has steps. Each point is `(s cos θ, y, s sin θ)`. In 2D the page
cuts every ring at two points, `θ = 0` and `π`, and the same walk draws a disc.

That's thorough, and slow: a sine and a cosine for every point. An adept's mind takes about 48 ticks to lay out a 3D ball of
0.5 m. Fireball doesn't use it any more: it pours its fire into one point and lets the fire's own pressure fill out the ball
(§7). `ball` is still the way to lay out mana that doesn't spread by itself.

```
; ball: the mana in m1, spread through a ball around the weave's origin. A ball is shells, from the centre out to the
; radius. Each shell is rings around the up axis, from its top to its bottom, and each ring is points along its
; circumference, 2πs. In 2D the page cuts each ring at two points: a disc.
; It walks the ball twice: once to count the points, once to give each its share.
;   in: n0 radius (m), n4 weave
ball:   IN    n5, CELL
        MUL   n5, #0.5            ; h: points half a cell apart, so that no cell is missed
        IN    n2, DEPTH           ; 1 in 2D
        LDI   n1, #1              ; the centre is a point
        LDI   n6, #0              ; nothing to give yet: the first walk only counts, into n1
        CALL  .walk
        MEAS  n6, m1
        DIV   n6, n1              ; n6 = mana per point
        LDI   n13, #0
        LDI   n14, #0
        LDI   n15, #0
        EMIT  m1, n6, n4, n13:15  ; the centre: a shell of radius 0
        CALL  .walk
        MEAS  n6, m1              ; what the rounding left
        LDI   n13, #0
        LDI   n14, #0
        LDI   n15, #0
        EMIT  m1, n6, n4, n13:15  ; goes to the centre
        RET
.walk:  MOV   n7, n5              ; ρ: the first shell, one step out
.shell: LDI   n8, #0              ; φ: from the top of the shell down
.lat:   CIRC  m1                  ; keep holding it, ring by ring: a big ball takes a while
        MOV   n9, n8
        SIN   n9
        MUL   n9, n7              ; s = ρ sinφ: this ring's radius
        MOV   n10, n8
        COS   n10
        MUL   n10, n7             ; y = ρ cosφ: its height
        MOV   n12, n9
        MUL   n12, #6.28319
        DIV   n12, n5
        ROUND n12                 ; points: the ring's circumference, 2πs, in steps of h
        MAX   n12, #1
        CMP   n2, #1
        JNE   .count
        MIN   n12, #2             ; 2D: the two points where the ring crosses the page
.count: CMP   n6, #0
        JNE   .ring
        ADD   n1, n12             ; counting: just add them up
        JMP   .down
.ring:  LDI   n3, #6.28319
        DIV   n3, n12             ; dθ
        LDI   n11, #0             ; θ: around the axis
.point: MOV   n13, n11
        COS   n13
        MUL   n13, n9             ; x = s cosθ
        MOV   n14, n10            ; y
        MOV   n15, n11
        SIN   n15
        MUL   n15, n9             ; z = s sinθ: 0 in 2D, where θ is 0 or π
        EMIT  m1, n6, n4, n13:15
        ADD   n11, n3
        SUB   n12, #1
        CMP   n12, #0
        JGT   .point
.down:  MOV   n3, n5
        DIV   n3, n7              ; dφ: one step of arc down the shell
        ADD   n8, n3
        CMP   n8, #3.14160
        JLE   .lat
        ADD   n7, n5              ; the next shell out
        CMP   n7, n0
        JLE   .shell
        RET
```

```
; shield: the mana in m1, as a shell around the weave's origin
;   in: n0 radius (m), n4 weave
shield: IN    n5, CELL
        MOV   n6, n5
        DIV   n6, n0              ; dθ: one cell of arc
        MEAS  n7, m1
        IN    n8, DEPTH
        CMP   n8, #1
        JEQ   .ring
        MOV   n9, n0              ; 3D: a sphere, about 4π r² / cell² points
        MUL   n9, n0
        MUL   n9, #12.5664
        DIV   n9, n5
        DIV   n9, n5
        DIV   n7, n9              ; mana per point
        LDI   n10, #0             ; φ, from the top down
.phi:   CIRC  m1                  ; keep holding it, ring by ring
        MOV   n12, n10
        SIN   n12                 ; sin φ
        LDI   n11, #0             ; θ, around
.theta: MOV   n13, n11
        COS   n13
        MUL   n13, n12
        MUL   n13, n0             ; x = r sinφ cosθ
        MOV   n14, n10
        COS   n14
        MUL   n14, n0             ; y = r cosφ
        MOV   n15, n11
        SIN   n15
        MUL   n15, n12
        MUL   n15, n0             ; z = r sinφ sinθ
        EMIT  m1, n7, n4, n13:15
        ADD   n11, n6
        CMP   n11, #6.28319
        JLT   .theta
        ADD   n10, n6
        CMP   n10, #3.14159
        JLE   .phi
        JMP   .rest
.ring:  MOV   n9, n0              ; 2D: a circle, about 2π r / cell points
        MUL   n9, #6.28319
        DIV   n9, n5
        DIV   n7, n9
        LDI   n11, #0
        LDI   n15, #0             ; z = 0
.arc:   MOV   n13, n11
        COS   n13
        MUL   n13, n0             ; x = r cosθ
        MOV   n14, n11
        SIN   n14
        MUL   n14, n0             ; y = r sinθ
        EMIT  m1, n7, n4, n13:15
        ADD   n11, n6
        CMP   n11, #6.28319
        JLT   .arc
.rest:  MEAS  n7, m1              ; what's left goes to the last point
        EMIT  m1, n7, n4, n13:15
        RET
```

This shield is uneven: near the poles, θ steps by the same angle on a smaller circle, so points crowd together, and the mana
runs out before the bottom of the shell. That's the kind of thing a better library version fixes. It's also the kind of thing
a mage could be known for.

```
; wall: a block of the ground under the weave's origin, given the order to rise (ingrain it, then set it loose)
;   in: n0 height, n1 length, n2 thickness (m), n4 weave (on the ground, turned to face out)
wall:   IN    n5, CELL
        DIV   n0, n5
        FLOOR n0                  ; H, in cells
        DIV   n1, n5
        FLOOR n1                  ; L
        DIV   n2, n5
        FLOOR n2                  ; T
        IN    n3, DEPTH
        CMP   n3, #1
        JNE   .size
        LDI   n1, #1              ; 2D: its length runs into the page, one cell
.size:  MOV   n6, n0
        MUL   n6, n1
        MUL   n6, n2              ; cells to lift
        MEAS  n7, m1
        DIV   n7, n6              ; mana per cell
        MOV   n10, n1
        SUB   n10, #1
        DIV   n10, #2
        FLOOR n10                 ; ⌊(L − 1) / 2⌋, to centre the length on whole cells
        MOV   n11, n2
        SUB   n11, #1
        DIV   n11, #2
        FLOOR n11                 ; ⌊(T − 1) / 2⌋, to centre the thickness
        LDI   n9, #0              ; d: 0 … H−1, down into the ground
.d:     CIRC  m1                  ; keep holding it, layer by layer
        LDI   n8, #0              ; t: across
.t:     LDI   n3, #0              ; a: along
.a:     MOV   n13, n3
        SUB   n13, n10
        MUL   n13, n5             ; x = (a − ⌊(L−1)/2⌋) · cell
        MOV   n14, n9
        NEG   n14
        MUL   n14, n5             ; y = −d · cell
        MOV   n15, n8
        SUB   n15, n11
        MUL   n15, n5             ; z = (t − ⌊(T−1)/2⌋) · cell
        EMIT  m1, n7, n4, n13:15
        ADD   n3, #1
        CMP   n3, n1
        JLT   .a
        ADD   n8, #1
        CMP   n8, n2
        JLT   .t
        ADD   n9, #1
        CMP   n9, n0
        JLT   .d
        MUL   n0, n5              ; H, back in metres
        ADD   n0, #0.05           ; and a little more, to clear the ground
        MOV   n3, n0
        DIV   n3, #0.1
        ADD   n3, #0.499
        ROUND n3                  ; ticks to climb, at no more than 0.1 m a tick
        WSET  n4, #0, n3
        DIV   n0, n3
        WSET  n4, #1, n0          ; how fast: it climbs exactly H + 0.05 m
        MUL   n2, n5              ; how far to step back: its thickness, onto the ground in front of its trench
        MOV   n6, n2
        DIV   n6, #0.05
        ADD   n6, #0.499
        ROUND n6                  ; ticks to step, at no more than 0.05 m a tick
        DIV   n2, n6
        WSET  n4, #4, n2          ; how fast
        ADD   n6, n3
        WSET  n4, #3, n6          ; when it's there
        ORDR  n4, rise
        RET

; rise (an order): the wall climbs out of the ground, steps back toward its maker, and comes down on solid ground in
; front of the trench it came from, where it stands on its own. It can't see where it is: it goes by the clock its caster
; worked out (wall), and by feeling its own speed. Phases, in w2, for whoever watches: 0 climbing, 1 stepping, 2 standing.
;   Climbing, its first w0 ticks: up at w1 m a tick.
;   Stepping, until tick w3: back at w4 m a tick.
;   Each tick it kicks itself to the speed it wants, and a little faster than that, by half of what it falls in a tick
;   (9.8 m/s² is 0.011 m/tick²), so that it holds itself up as it goes.
;   Standing: it stops dead, lets go of itself and drops the last few centimetres onto the ground, which holds it up from
;   then on. Its rock holds its shape. Its order thinks three beats a tick, which is all it costs to keep.
rise:   GETW  n5, #2
        CMP   n5, #2
        JEQ   .done               ; standing: nothing to do
        IN    n8:10, VEL
        LDI   n11, #0
        SUB   n11, n8             ; nothing sideways
        LDI   n12, #0
        LDI   n13, #0
        SUB   n13, n10            ; nor forward or back
        GETW  n6, #0
        CMP   n4, n6
        JGE   .step
        GETW  n12, #1             ; climbing: up
        JMP   .kick
.step:  GETW  n6, #3
        CMP   n4, n6
        JGE   .stand
        PUTW  #2, #1
        GETW  n7, #4
        SUB   n13, n7             ; stepping: back, toward its maker
.kick:  ADD   n12, #0.0055        ; and half of what it'll fall this tick
        SUB   n12, n9
        KICK  n11:13
.done:  RET
.stand: PUTW  #2, #2              ; there: stop, and let go
        SUB   n12, n9
        KICK  n11:13
        RET
```

The wall isn't made of the spell's mana. It's the ground, lifted: the earth mana *influences* the earth around each particle,
binding it and carrying it up, and the ground it came from is left as a trench. A caster with poor earth affinity binds less
earth, and the wall rises full of holes. Climbing costs the earth mana some of itself, lifting 375 kg of earth 2 m (about
10 M) and thinking about it (more), so it lets go of some earth on the way up; that earth falls to the bottom of the trench.
Once it stands on the ground, its order thinks three beats a tick, which costs little, but not nothing: the wall slowly
crumbles as its order burns its mana away, and its rock comes apart as its mana lets go of the earth.

Size matters in 3D. The 2D wall is 16 cells of earth; the 3D one, 4 m long, is 256. Binding a full cell takes 5 M of earth
mana, so the 3D wall needs about 8000 M gathered: far past an adept's capacity of 600. It's a master's spell.

Holding a shape together is pushing: by its caster (Holding), or by an order in its particles that feels where the mana
thins (Orders: `cling`, `cohere`).

### Reactions

```
; Reactions: what a particle can notice.

; touch: n5 = 1 if this particle is against something that isn't its own
touch:  TUCH  n5
        RET
```

### Transformations

```
; Transformations: what a weave can become.

; burst (an order): each particle spends some of itself flying 0.2 m/tick out, toward where the mana around it thins
; (against GRAD), for 3 ticks from w4; then the weave lets go. One in the very middle feels no way out, and stays.
burst:  GETW  n5, #4
        MOV   n6, n4
        SUB   n6, n5              ; ticks since it began
        CMP   n6, #3
        JGE   .gone
        GRAD  n6:8
        MOV   n9, n6
        MUL   n9, n6
        MOV   n10, n7
        MUL   n10, n7
        ADD   n9, n10
        MOV   n10, n8
        MUL   n10, n8
        ADD   n9, n10
        CMP   n9, #0
        JEQ   .done               ; in the very middle: nowhere is out
        SQRT  n9
        LDI   n10, #-0.2
        DIV   n10, n9
        MUL   n6, n10
        MUL   n7, n10
        MUL   n8, n10
        KICK  n6:8                ; out, away from the thicker mana: the flare
.done:  RET
.gone:  DISS                      ; its mana goes loose
        RET

; condense (an order): on the first tick, half of each particle's mana condenses into matter (make)
condense:
        CMP   n4, #0
        JNE   .done
        MOV   n5, n3
        MUL   n5, #0.5
        CNDS  n5                  ; the other half stays free and holds it
.done:  RET
```

`burst` is an explosion paid for by the fireball itself: each particle spends some of its own mana flying out. Nothing in the
machine knows what an explosion is.

---

### Orders

Ways for mana to move itself, knowing only what it feels and what it's told (D31). The bench (§11) throws a fireball with
each. `heading` is told one angle and flies along it; `steer` is told an angle and a speed and feels its own speed to keep
to them. `cling` and `cohere` hold a weave together: where the mana around a particle is thinner than its caster said the
edge is (`edge`), it kicks itself back toward thicker mana; `cohere` only while it's drifting away from its neighbours. An
order that works something out once (a sine and a cosine) keeps it in the weave's registers for every particle after.

```
; heading: flies along one angle, w1 (up from forward, in radians), speeding up 0.04 m/tick every tick for its first 8
; ticks. It knows nothing else: not how fast it's going, not where the rest of its weave is. The sine and cosine are
; worked out on its first tick and kept in w5 and w6 for every particle after: a weave can remember for its mana.
heading: CMP  n4, #8
        JGE   .done
        CALL  angle
        MUL   n7, #0.04
        MUL   n8, #0.04
        LDI   n6, #0
        KICK  n6:8
.done:  RET

; angle: the sine and cosine of w1, into n7 and n8. Worked out once and kept in w5 and w6 (a cosine of 0 means not yet),
; or given there by the caster, who can work it out once for the whole weave.
angle:  GETW  n8, #6
        CMP   n8, #0
        JNE   .known
        GETW  n7, #1
        MOV   n8, n7
        SIN   n7
        COS   n8
        PUTW  #5, n7
        PUTW  #6, n8
        RET
.known: GETW  n7, #5
        RET

; steer: flies along w1 at w7 m/tick. It feels its own speed (IN VEL) and kicks itself toward the speed it wants, so it
; also cancels whatever else moves it: the spread of its own pressure, the drag of the air.
steer:  CALL  angle
        GETW  n9, #7
        MUL   n7, n9
        MUL   n8, n9              ; the velocity it wants: (0, n7, n8)
        IN    n9:11, VEL
        LDI   n6, #0
        SUB   n6, n9
        SUB   n7, n10
        SUB   n8, n11             ; what it lacks
        MOV   n9, n6
        ABS   n9
        MOV   n10, n7
        ABS   n10
        ADD   n9, n10
        MOV   n10, n8
        ABS   n10
        ADD   n9, n10
        CMP   n9, #0.01
        JLT   .done               ; near enough: think no more
        KICK  n6:8
.done:  RET

; cling: holds a weave together by feel alone. Where the mana around it is thinner than w3 (M/m³), a particle is at
; the weave's edge, and kicks itself back toward thicker mana, harder the steeper it thins: ∇ρ/ρ, times 0.01 m²/tick.
; That's about ten times what its own gas's pressure pushes it out with, there. Short, so it's quick to ingrain, and
; rough: what it pulls back, it doesn't slow, so the edge sways. A particle with nothing near enough to feel can't find
; its way back.
cling:  DENS  n5
        GETW  n9, #3
        CMP   n5, n9
        JGE   .done               ; thick enough: it's inside
        GRAD  n6:8                ; toward thicker mana
        LDI   n9, #0.01
        DIV   n9, n5
        MUL   n6, n9
        MUL   n7, n9
        MUL   n8, n9
        KICK  n6:8
.done:  RET

; cohere: cling, done carefully: it kicks only while it's drifting away from its neighbours (VEL − NVEL, along ∇ρ). So
; it spends nothing on a particle already coming back, and a thrown ball holds together however fast it flies.
cohere: DENS  n5
        GETW  n9, #3
        CMP   n5, n9
        JGE   .done               ; thick enough: it's inside
        GRAD  n6:8                ; toward thicker mana
        IN    n11:13, VEL
        NVEL  n0:2
        SUB   n11, n0
        SUB   n12, n1
        SUB   n13, n2             ; its speed against its neighbours'
        MUL   n11, n6
        MUL   n12, n7
        MUL   n13, n8
        ADD   n11, n12
        ADD   n11, n13
        CMP   n11, #0
        JGT   .done               ; already coming back in
        LDI   n9, #0.01
        DIV   n9, n5
        MUL   n6, n9
        MUL   n7, n9
        MUL   n8, n9
        KICK  n6:8
.done:  RET

; edge: how thick the mana is at the edge of a ball it fills, for cling and cohere: 3/4 of it spread evenly through a
; ball of radius n0 (in 2D, a disc one cell deep), into w3. The caster works it out once; the particles are told it.
;   in: n0 radius (m), n7 its mana, n4 weave
edge:   MOV   n9, n0
        MUL   n9, n0              ; r²
        IN    n8, DEPTH
        CMP   n8, #1
        JNE   .ball
        IN    n8, CELL
        MUL   n9, n8
        MUL   n9, #3.14159        ; π r² · cell
        JMP   .div
.ball:  MUL   n9, n0
        MUL   n9, #4.18879        ; 4/3 π r³
.div:   MOV   n8, n7
        MUL   n8, #0.75
        DIV   n8, n9
        WSET  n4, #3, n8
        RET
```

### Holding

Ways for a caster to hold a weave together by hand: every particle, every other one, or a quick look and then only the
surface. Each pushes a straying particle back in, and pays only for the speed that adds.

```
; inward: particle n6 of weave n4, if it's past n14 metres from the centre and not already coming back, is pushed back
; in: faster the further it's strayed. Inside, it costs a look and a compare. n18:20 is the weave's velocity.
inward: PPOS  n8:10, n4, n6
        MOV   n11, n8
        MUL   n11, n8
        MOV   n12, n9
        MUL   n12, n9
        ADD   n11, n12
        MOV   n12, n10
        MUL   n12, n10
        ADD   n11, n12            ; r²
        MOV   n12, n14
        MUL   n12, n14
        CMP   n11, n12
        JLE   .done               ; inside
        SQRT  n11
        LDI   n12, #1
        DIV   n12, n11
        MUL   n8, n12
        MUL   n9, n12
        MUL   n10, n12            ; out, one metre long
        PVEL  n1:3, n4, n6
        SUB   n1, n18
        SUB   n2, n19
        SUB   n3, n20
        MUL   n1, n8
        MUL   n2, n9
        MUL   n3, n10
        ADD   n1, n2
        ADD   n1, n3              ; how fast it's moving out
        SUB   n11, n14
        MUL   n11, #-0.2          ; how fast it should: back in
        SUB   n11, n1
        CMP   n11, #0
        JGE   .done               ; coming back fast enough
        MUL   n8, n11
        MUL   n9, n11
        MUL   n10, n11
        SHOV  m0, n4, n6, n8:10
.done:  RET

; hold: one pass over every particle.
hold:   WVEL  n18:20, n4
        PCNT  n5, n4
        LDI   n6, #0
.next:  CMP   n6, n5
        JGE   .done
        CALL  inward
        ADD   n6, #1
        JMP   .next
.done:  RET

; holdOther: one pass over every other particle: the even ones, then next time the odd ones (n21 says which). Their
; neighbours pass the push on.
holdOther: WVEL n18:20, n4
        PCNT  n5, n4
        MOV   n6, n21
.next:  CMP   n6, n5
        JGE   .done
        CALL  inward
        ADD   n6, #2
        JMP   .next
.done:  LDI   n6, #1
        SUB   n6, n21
        MOV   n21, n6
        RET

; holdSurface: a quick look at where every particle is, noting in memory the ones in the outer part of the ball (past
; 0.6 of the radius), then four passes over only those. The inside is held by its skin. Up to 200 are noted.
holdSurface: PCNT n5, n4
        LDI   n6, #0
        LDI   n7, #0              ; how many noted
        MOV   n21, n14
        MUL   n21, #0.6
        MUL   n21, n21            ; (0.6 R)²
.look:  CMP   n6, n5
        JGE   .push
        PPOS  n8:10, n4, n6
        MUL   n8, n8
        MUL   n9, n9
        MUL   n10, n10
        ADD   n8, n9
        ADD   n8, n10
        CMP   n8, n21
        JLE   .in
        CMP   n7, #200
        JGE   .in
        ST    n6, [n7]
        ADD   n7, #1
.in:    ADD   n6, #1
        JMP   .look
.push:  LDI   n21, #4             ; passes left
.pass:  WVEL  n18:20, n4
        LDI   n5, #0
.each:  CMP   n5, n7
        JGE   .round
        LD    n6, [n5]
        CALL  inward
        ADD   n5, #1
        JMP   .each
.round: SUB   n21, #1
        CMP   n21, #0
        JGT   .pass
        RET
```

## 7. The four spells

Each spell is written in assembly, with the old Core's lore names in the comments. After it comes the same spell in the
language, which compiles to roughly the same thing (§8). Every spell begins `.use Elements, Basics, Shapes, Reactions,
Transformations`, or whichever it needs. The listings are the files in `spells/`.

### Stone Wall

The ground heaves up into a wall of rock.

```
StoneWall:
        IN    n16:18, AIM         ; a point on the ground
        PROB  n0, n16:18, #EARTH  ; PROBE: is there earth there?
        CMP   n0, #0
        JEQ   .fail
        IN    n19, AMOUNT
        GATH  m0, n19             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #EARTH      ; FILTER: earth × affinity; the residue stays in m0
        WEAV  n20, n16:18         ; a weave on the ground at the aim
        HOLD  n20, #3             ; a field big enough for the whole wall
        IN    n0:2, SELF
        MOV   n3, n16
        MOV   n4, n17
        MOV   n5, n18
        SUB   n3, n0
        SUB   n4, n1
        SUB   n5, n2
        TURN  n20, n3:5           ; POSITION: facing away from the caster
        MOV   n4, n20
        LDI   n0, #2              ; 2 m high
        LDI   n1, #4              ; 4 m long
        LDI   n2, #0.5            ; 0.5 m thick
        CALL  wall                ; the ground laid out, and its order to rise
        MOV   n4, n20
        CALL  ingrain             ; ORDER: into every particle, before any of it moves
        MANI  n20                 ; SEND: it rises
        MOV   n0, n20
        HALT                      ; m0 isn't held any more: it joins the flow
.fail:  FAIL  #1                  ; no earth there
```

```
from Shapes use wall
from Basics use ingrain
from Elements use earth

Metadata StoneWall(Metadata data) {
  IF (.NOT. self.probe(data.coordinate, earth)) DO
    return Metadata.fail(data)
  END IF

  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Earth> active_mana = self.filter(mana_pool, earth)

  ManaConstruct ground = self.weave(data.coordinate)
  ground.hold(3 m)
  ground.face_away(self)
  ground.lay(active_mana, wall(2 m, 4 m, 0.5 m))
  ground.ingrain()
  ground.manifest()

  return ground.metadata
}
```

The whole wall has to be in reach to be ingrained: its foot is 2 m down. Every particle is ingrained before any of it moves,
or the top would rise and tear away from the rest.

A wall that rose out of its trench and stayed there would have nothing under it: it would have to hold itself up forever,
thinking every tick, and burning its mana to do it. So it rises, steps back toward its caster by its own thickness, and
comes down on the solid ground in front of its trench. Its rock holds its shape, the ground holds it up, and its order goes
quiet. The trench is a ditch in front of it.

### Fireball

Fire poured into a ball, held together by its own order, thrown by pushing, and burst on touch.

```
; Fireball: fire poured into a ball, held together by its own order, thrown by pushing, and burst on touch.
        .use  Elements, Basics, Reactions, Transformations, Orders

Fireball:
        IN    n17, AMOUNT
        GATH  m0, n17             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #FIRE       ; FILTER: the fire; the rest stays in m0, to push it with
        IN    n1:3, HAND
        WEAV  n16, n1:3           ; POSITION: a weave at the hand
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        MEAS  n7, m1
        EMIT  m1, n7, n16, n4:6   ; all the fire, poured into one point, held still in the hand
        HOLD  n16, #1.5           ; the field: how far it may spread and still be this weave
        MOV   n4, n16
        LDI   n0, #0.5
        CALL  edge                ; how thin its mana is at 0.5 m out: where its order holds it
        ORDR  n16, .order
        MOV   n4, n16
        CALL  ingrain             ; ORDER: into every particle, one by one, while the hand holds it
        MANI  n16                 ; let go: its pressure fills out the ball, and its order catches it at the radius
        IN    n0:2, HAND
        IN    n3:5, AIM
        IN    n6, FORCE
        CALL  toward
        MOV   n0, n3
        MOV   n1, n4
        MOV   n2, n5
        MOV   n4, n16
        CALL  throw               ; SEND: pushed toward the aim until it's fast enough, or out of reach
        LOCK  n16, INPUT          ; LOCK: cut from the caster
        MOV   n0, n16
        HALT

.order: GETW  n5, #0              ; the phase: 0 holding together, 1 bursting
        CMP   n5, #0
        JNE   .burst
        CALL  touch               ; n5 = 1 if it's against something
        CMP   n5, #0
        JNE   .hit
        CALL  cling               ; back toward thicker mana if it strays past its edge
        RET
.hit:   PUTW  #0, n5              ; the whole weave bursts from the next tick
        PUTW  #4, n4              ; counting from now
        RET
.burst: CALL  burst               ; REACT: it spends itself flying apart
        RET
```

```
from Basics use ingrain, throw
from Orders use cling, edge
from Reactions use touch
from Transformations use burst

Metadata Fireball(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Fire> active_mana = self.filter(mana_pool, fire)

  ManaConstruct spell = self.weave(self.hand)
  spell.pour(active_mana)
  spell.hold(1.5 m)
  spell.order(cling(edge(0.5 m)), touch -> burst)
  spell.ingrain()
  spell.manifest()
  spell.throw(data.coordinate, data.force, paid_from: mana_pool)
  spell.lock(input)

  return spell.metadata
}
```

This is the idea the physics started from: the caster doesn't lay the ball out point by point. They pour all the fire into
one point in their hand, ingrain the order while the hand holds it still, and let go. The fire's own pressure fills out the
ball, and `cling` catches it where the mana is as thin as at 0.5 m out: the caster works that density out (`edge`) and
writes it into the weave, since the order can't know where the radius is. Laying it out with `ball` took an adept 48 ticks in 3D; pouring takes a few
instructions, and ingraining about 12 ticks. The residue in `m0`, the parts that weren't fire, pays for the throw.

Once it's out of reach, nothing but its order holds it. In the test world an adept's fireball flies 8 m to the pillar with
all its particles together, having spent about a sixth of its mana on the way, and bursts against it.

### Gust

A sudden push of wind, for as long as the caster keeps it up. It makes no weave: it sends loose air mana, which drags the air
it moves through and strikes whoever is in its way.

```
; Gust: a sudden push of wind, for as long as the caster keeps it up.
        .use  Elements, Basics

Gust:
        IN    n17, AMOUNT
        GATH  m0, n17             ; GATHER
.loop:  IN    n18, MAINTAIN
        CMP   n18, #0
        JEQ   .end
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #AIR        ; FILTER
        IN    n0:2, HAND
        IN    n3:5, AIM
        IN    n6, FORCE
        CALL  toward              ; n3:5 = the push
        MEAS  n6, m1
        SEND  m1, n6, n0:2, n3:5  ; SEND: all of it, from the hand. Its speed is paid from it.
        GATH  m1, n17
        JOIN  m0, m1              ; a fresh breath for the next pass
        CIRC  m0                  ; held as one: the fresh breath wasn't
        TICK
        JMP   .loop
.end:   HALT                      ; the residue joins the flow
```

```
from Elements use air

spell Gust(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  WHILE (data.maintain) DO
    CALL self.circulate(mana_pool)
    mu<Air> active_mana = self.filter(mana_pool, air)
    CALL self.send(active_mana, self.hand, data.coordinate, data.force)
    mana_pool += self.gather(data.amount)
    tick
  END DO
}
```

Each pass keeps the residue and adds a fresh gather. If the body drains slower than the caster gathers, its load climbs, and a
long Gust overcharges. It needs only two streams. Its speed is paid for from what it sends: at 0.6 m/tick, about a quarter.

### Water Shield

A skin of water around the caster that turns blades and flame.

```
WaterShield:
        IN    n16, AMOUNT
        GATH  m0, n16             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #WATER      ; FILTER
        IN    n1:3, SELF
        WEAV  n17, n1:3           ; POSITION: a weave around the caster
        HOLD  n17, #2             ; a field wider than the shell
        MOV   n4, n17
        LDI   n0, #1.2
        CALL  shield              ; a shell of water mana, 1.2 m
        ORDR  n17, .order
        MOV   n4, n17
        CALL  ingrain             ; ORDER
        MANI  n17                 ; SEND
        LOCK  n17, INPUT          ; LOCK: nothing more goes into it
        MOV   n4, n17
        CALL  keep                ; MAINTAIN: it goes where its caster goes, for as long as they keep it up
        MOV   n0, n17
        HALT

.order: CALL  condense            ; MAKE: half its mana becomes water, held by the other half
        CALL  follow              ; and it moves as it's told, paying with itself
        RET
```

```
from Shapes use shield
from Basics use follow, keep, ingrain
from Transformations use condense

Metadata WaterShield(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Water> active_mana = self.filter(mana_pool, water)

  ManaConstruct spell = self.weave(self.position)
  spell.hold(2 m)
  spell.lay(active_mana, shield(1.2 m))
  spell.order(condense, follow)
  spell.ingrain()
  spell.manifest()
  spell.lock(input)
  spell.keep()                      // while maintained: tells it how to move to stay with the caster

  return spell.metadata
}
```

This shield *makes* its water instead of finding it. The water is real, and held up by the mana left free. Every tick its
order burns a little of that mana, and every step its caster takes costs it the push to follow, so it holds less and less
water. It follows only while its caster keeps it up, telling it each tick how to move; let be, it holds still where it is. When its mana is gone, the water falls in a splash at the caster's feet. A shield cast beside a river could
*influence* the river's water instead, and keep all of its mana free to hold it.

### What compiling the writings in Quire would say

The language will be strict where the old writings were loose:

```
WaterShield:7   error    circulate() needs the mana to circulate: mana_pool (line 6)?
WaterShield:14  error    Metadata has no `coordinates`: `coordinate`?
WaterShield:16  error    lock_shape() comes after manifest() (line 18)
WaterShield:18  error    ManaConstruct has no member `manisfest`: `manifest`?
Fireball:14     error    `ManaConstruct = …` declares nothing: it needs a name (`ManaConstruct spell = …`)
Fireball:19     error    lock() comes after manifest() (line 21)
Gust:4          error    `mana_pool` has no type: `mu mana_pool`?
Gust:7          error    `active_mana` has no type: `mu<Air> active_mana`?
```

---

## 8. The language

The language is written in the codified style. Its compiler does what a person writing the assembly does by hand:

- gives registers to variables (mind for numbers, body for `mu`), keeping them out of `n0`–`n15` across calls;
- lays out calls by the calling convention;
- turns `add_react(touch, expand)` into an order: a phase in `w0`, with a jump on it;
- checks what the machine can't, before it runs:
  - names and members;
  - types (a shape needs pure mana: `mu<Earth>`, not `mu`);
  - linearity (no mana used twice);
  - that `lock` comes after `manifest`;
  - holds that will surely have run out;
  - how many streams the spell needs at once.

| Type | Lives in |
|---|---|
| `num`, `bool` | `n` |
| `space3d` | three `n` |
| `mu`, `mu<Fire>` | `m`. Linear: passing it on moves it. `filter` takes its source by reference, because the residue stays. |
| `ManaConstruct` | `n` (a weave id) |
| `Metadata` | The will going in (ports), the weave coming out. |

The language is designed after the machine is built, and §7's versions are a sketch of it.

---

## 9. The spell tester

A browser app (Svelte 5, Vite, CodeMirror 6) in `tester/`. `npm run tester` opens it; in development it reads and saves
the `.masm` files in `spells/` and `lib/` directly.

- **Code**: the spell and the libraries it uses, one tab each.
  - Highlighting, completion, and a hover that says what an instruction does and what it costs, or what a register
    holds right now.
  - Errors from the assembler as you type, on their lines.
  - Breakpoints in the gutter, and a second gutter with the beats each line has cost so far.
  - The line the mind runs next is lit, and the editor follows it into whichever library it's in.
- **Running it**: Cast (a fresh world, and the spell), Play at a chosen number of ticks a second, Step one instruction,
  Tick to the end of the tick. Stepping can stop partway through a tick: the mind has thought, but the world hasn't
  moved yet.
- **World**: a 2D view, or a slice of a 3D world. Matter by its dominant part, every particle of mana (bright in a weave, dim
  when loose, white the tick it's pushed), the matter weaves hold, thin and thick air, wind, bodies, the hand and the aim.
  Click to aim; hover a cell to read it.
- **Panels**:
  - *Mind*: what it runs next, beats left this tick, flags, `n0`–`n31` (the ones that changed are lit; ones this mind
    doesn't have are dimmed), return addresses, the stack and memory.
  - *Body*: load against capacity, flow, drain, focus, harm, condition, and `m0`–`m7` as four coloured parts, each with
    how long it's still held or that it's slipping.
  - *Weaves*: each weave's state, locks, order, its particles and how many are ingrained, its field, mana, the matter it
    holds, and `w0`–`w7`.
  - *Profile*: beats by routine and the costliest lines (D18).
  - *Events* and the ledger.
  - *Caster*: child, adept or master; every stat as genetics and training, with what it comes to now; condition; how
    many times they've cast this spell before; and the will (amount, force, maintain), which a running spell reads live.
  - *Bytes*: the assembled program, with where the mind is.
  - *Reference*: every instruction and port.

- *Order*: any particle of any weave, chosen by number or by clicking it in the world, and its order from the last tick,
  instruction by instruction: step forward and back, its 16 registers at each step, the weave's registers, how it kicked,
  the mana its thinking burned, and its beats against the 64 it has. A breakpoint in order code (a weave's `.order`, or a
  library routine it calls) pauses the run after the tick a particle hit it, on that particle and that instruction. The
  machine records this only when asked (`Sim.traceOrders`), since every particle of every weave is recorded every tick.

---

## 10. How it's built

TypeScript throughout, like Quire, so it can run inside Quire later.

```
src/
  asm/       isa.ts (the instruction table), assembler.ts, disassembler.ts
  vm/        physics.ts (the numbers), parts.ts, world.ts, caster.ts, weave.ts, sim.ts (the machine and the tick),
             fluid.ts (mana's particles), air.ts (the air), energy.ts (the Energy ledger)
  cli/       mas.ts, mvm.ts, bench.ts
  scenes.ts  test worlds for the four spells and the bench, in 2D and 3D
  bench.ts   the bench: every way of holding and throwing, measured
  render.ts  a slice of the world as text
  profile.ts where a cast's beats went
tester/      the spell tester: Svelte 5, Vite, CodeMirror 6
lib/         Elements, Basics, Shapes, Reactions, Transformations, Orders, Holding, in .masm
spells/      StoneWall, Fireball, Gust, WaterShield, in .masm
bench/       the bench's spells
scripts/     listings.ts: SPEC's code listings, from the .masm files
test/        the assembler, the machine, the physics, the Energy ledger, the four spells, the bench
```

```
npm install
npm test                                   the tests
npm run tester                             the spell tester, at http://localhost:5175
npx tsx src/cli/mas.ts spells/Fireball.masm  the listing: addresses, bytes, instructions
npx tsx src/cli/mvm.ts spells/StoneWall.masm cast it in its test world, in the terminal
npx tsx src/cli/mvm.ts spells/Gust.masm --ticks 40 --maintain 30
npm run bench                              every way of holding and throwing, side by side
npm run listings                           SPEC's code listings, brought up to date
```

### Phases

1. **The machine.** *Built.*
   - The world (2D and 3D), the caster, the tick.
   - Every instruction. Orders running per cell.
   - The ledger, the assembler and disassembler.
   - The libraries and the four spells in `.masm`.
   - Tests: the wall stands and leaves a trench, then crumbles back into it; the fireball bursts on touch; the shield follows
     its maker and falls in a splash; Gust pushes and, kept up too long, overcharges; the ledger balances every tick.
2. **The tester, first cut.** *Built.* Editing with live errors, breakpoints, stepping by instruction or tick, the mind,
   the body, weaves, stepping through any particle's order, the profile, the ledger, the caster and their will, and the world
   in 2D or a 3D slice.
3. **Mana physics** (§11). *Built:* mana is particles with pressure, moved by pushes and orders that pay in mana.
   Then, by `PLAN.md`:
   - **The bench**, which replaced the sandbox: every way of holding and throwing, on the machine. *Built.*
   - **Weight, rock and energy-priced pushes** (D28–D30). *Built.*
   - **Merging and splitting**, and with them the second flaw. *Built.*
   - **Air that flows.** *Built.*
   - **The Energy ledger.** *Built.*
   - **What an order knows** (D31): it only feels, and what else it needs its caster writes into its weave. *Built.*
4. **The language**, compiling to what phase 1 runs by hand.
5. Casters, spells and libraries read from Quire.
6. **Other notations** (later): runes, circuits and scores.

---

## 11. Mana physics

*The machine used to run on simple rules: mana stayed where it was emitted, `MOVE` moved a cell by however far it asked, and
`LOCK SHAPE` held a shape for free. This section replaced them with real physics (D19–D27). It was tried first in a sandbox,
then built into the machine: §3–§7 describe the machine as it is now, and the bench measures it. What isn't built yet says so.*

### Why real

Reality is full of shortcuts: pressure fills a vessel without anyone placing each drop, a hammer passes its swing to a nail,
a sphere holds the most for the least skin. A world that runs on real physics hands those shortcuts to whoever writes
spells. It also refuses to give anything away: mana and Energy are both conserved, so a better spell never comes from
nothing. It comes from thinking less, wasting less, and using what the world already does.

### Mana and Energy

There are two ledgers, and each balances on its own:

```
Mana     Σ free (air, particles, registers)  +  Σ condensed (matter)   =  constant
Energy   Σ motion  +  Σ heat  +  Σ electricity  +  …                    =  constant
```

Mana is chemistry: what a thing is made of. Energy is physics: force, motion, heat as motion, electricity. Mana can't take
hold of Energy directly. It only moves it indirectly, the way pouring mana onto a particle sets it moving (*Pushing*).
Fire mana is the substance of heat: what burns, what is hot. The heat itself, the motion in it, is Energy.

A push turns mana into motion (D29), so mana pays for the Energy it puts in; nothing else pays yet (open question 5). The
machine keeps two more ledgers that physics needs.

**Momentum.** Everything inside the world pushes on everything else equally and oppositely (particle and particle, particle
and air, air and air, particle and body), so the world's momentum changes only by what comes from outside it, and the tests
check that every tick (`World.momentumError`).

**Energy** (`src/vm/energy.ts`, `Sim.keepEnergy`). The world holds Energy as motion (of particles, the air and bodies), as
height (weight lifted, of particles and of the ground's matter), and stored: in mana's gas pressed together, in matter
packed past full, in what coheres pulled apart, and in the air pressed or drawn thin. Each is measured from the air at rest,
so mana that moves into or out of the air as dense as it is brings nothing with it. Motion becomes **heat** wherever two
things even out their speeds: in the mana's thickness, the air dragging and its own thickness, landing on the ground and
sliding on it, rock keeping its shape, particles merging, mana settling into the air. That heat is counted where it
happens, to the joule, and kept by how it was made (`World.heat`). Casters and orders put Energy in with every push and
kick. So, every tick:

```
held now + heat  =  held at the start + what casters and orders put in + what the numbers got wrong
```

The last term is real, and is counted, not hidden in the heat: each step that should keep Energy is measured before and
after, and what it got wrong is kept by step. It's the price of stepping time rather than flowing it. For a Fireball, a
Gust or a Water Shield it's under 2% of the Energy that moves through. A Stone Wall, gathering 500 M at once, thins the air
around its caster so hard that the air's refilling gets about 8% wrong.

The density mana's pressure works from is summed with the same kernel its pushes follow (the spiky kernel), so that its
pressure is exactly the pull of the energy its gas stores, and the ledger can count it.

### Particles

Free mana that is moving or held is **particles**. Real mana particles are far smaller than atoms; a simulated particle
stands for a crowd of them, the way a fluid simulation's particles do. There are enough of them for mana to behave as a
fluid: about one every half cell, so a 3D fireball of 0.5 m is a few hundred.

Each particle has:

- a position and a velocity;
- its mana, four amounts, one per part;
- the weave holding it, if any;
- its order, if any.

Particles push on their neighbours with **pressure**: mana packed denser than it rests spreads out. Every push between two
particles is equal and opposite, so momentum is conserved. Mana is conserved because particles are counted.

**Air mana** stays a grid. A particle that slows down and belongs to no weave settles into the grid and loses its order;
`GATH` draws from the grid. A particle moving through the air drags on the air mana around it and is dragged by it, both
ways. That drag is wind.

The air is a gas (`src/vm/air.ts`). It presses from dense to thin, at its own speed of sound (`airSound`, 18 m/s: slower
than real air's 340, to keep the steps few, and still fast beside its winds). It carries itself along, and its mana and its
momentum with it, from cell to cell. Solid cells and the world's edge are closed: it flows around them, and what it pushes
on them is momentum given to the world. Its thickness (`airViscosity`) evens out its speed between neighbours and holds it
still against the ground. So a gust travels on as a jet once it's let go, a fireball leaves a wake and pushes air ahead of
it, wind turns up and over a pillar, and the hole a caster gathers from fills back in from around it. Air that has all but
stopped stops, and air at rest, as dense as the rest of it, costs nothing to run.

When particles come to rest beside each other, they merge, to keep their number down: closer than `mergeRange`, moving
within `mergeSpeed` of each other, and together no more than `maxMote`. The new particle sits at their centre of mass with
their summed mana, matter and momentum, and keeps the bigger one's weave and order (the same size, the older one's). A
particle of two motes or more that has spread thin (more of the density it feels is its own than its neighbours') splits in
two, side by side. Both halves keep its weave and order. Nothing in a caster's hand merges or splits, and neither does rock.

What a particle feels (`DENS`, `GRAD`, `NVEL`, and the test for spreading thin) is weighed with a flat kernel
(Epanechnikov's, `h² − r²`), not the peaked one the pressure is summed with: a neighbour most of a smoothing length away
counts a fair share of what the particle itself does. With the peaked kernel, a particle a smoothing length from the next
felt mostly itself, and couldn't tell the inside of a ball from its edge.

So a fireball held together by its order merges as it flies: its particles share a velocity, and the 72 it was poured as
become about 45. A merged particle runs its order once where two ran it before, and burns half as much to think.

### What each part brings

Each part has its own numbers, in `physics.ts`, by part number. The machine still knows no element names (D2).

| Part | Pressure | Weight | Holds together | So |
|---|---|---|---|---|
| 0 (fire) | high | rises | barely | spreads fast and rises. Easy to pour, hard to hold. |
| 1 (water) | low | heavy | some | flows down. A shell of it sags unless held up. |
| 2 (air) | high | light | no | fills what's empty. The easiest to pour. |
| 3 (earth) | very low | heavy | strongly | barely spreads. It has to be laid out by hand. |

*Built:* each part's pressure, thickness, weight (`fall`: fire rises a little, air floats, water and earth fall) and how it
holds together (`cohesion`, a pull between neighbours, strongest at half the smoothing length). Matter held by mana adds its
weight and its mass (`matterMass`), can't be packed past full (`matterStiffness`), and pulls on its neighbours as water
does (`matterCohesion`). Earth held by a weave becomes rock when the weave is let go of (D30): each particle is bound to
its neighbours within `bondRange`, and the bonds hold their length, a dozen passes a step, so that the ground's support
reaches up through a wall. The ground holds up what rests on it, with friction.

An adept's fireball is 72 particles of a quarter of an M each (`mote`).


### Pushing

A push pours mana onto a particle and changes its velocity, up to a limit per tick (`pushRate`). The poured mana turns into
the particle's motion: a push costs the **kinetic energy it adds**, measured against the ground, at `pushEnergy` for each M
(D29). One M is 900 J. So:

- Speeding something up from rest costs ½mv². Speeding it up further costs more for the same change, the faster it
  already goes: the same 0.05 m/tick costs a 0.3 m/tick fireball thirteen times what it costs one at rest.
- Slowing something down costs nothing. What's taken out of its motion becomes heat.
- Holding something up against its weight costs nothing: each tick the push only takes back the speed it gained falling.
  Lifting it costs its weight times the height: lifting a 2 m Stone Wall of 375 kg of earth costs about 10 M.
- Pushing sideways across a motion costs only what the sideways speed adds.

The poured mana goes **loose where it was poured**. Nothing is lost, but a construct pushed for a long time sits in a haze
of spent mana.

So a fireball isn't thrown in one instruction. The caster pushes it along the aim tick after tick, holding it together
while it speeds up. A heavier ball takes longer to get going.

### Holding

A weave is the particles in its caster's **field**. The caster keeps the field like a hold (`CIRC`), and it reaches only
so far from their body. A particle outside the field leaves the weave. Its order stays with it: it is still that mana.

Pressure pushes a held construct apart all the time. The caster keeps it together by pushing its particles back in,
spending beats on each one. A push that only slows a straying particle costs no mana (D29); one that sends it back faster
than it strayed costs a little. So what holding costs is thought, and which particles the caster thinks about is the skill:

| How | What it costs |
|---|---|
| Every particle, inward | Every particle, every tick. |
| Every other particle | About half. Its neighbours pass the push on. |
| Only the surface | Grows with the area, not the volume. The inside is held by its skin. |
| Only the ones moving out | Sensing first costs beats, and saves pushes. |
| A hard push every few ticks | The construct breathes, and leaks a little between pushes. |

The bench measures these (`lib/Holding.masm`, §6).

A bigger mind holds a bigger construct, because it pushes more per tick. A practised spell (the Law of Conditioning) holds
more cheaply. A sphere is the cheapest shape to hold, because it has the least surface for what it holds.

### Released mana

A construct the caster lets go of, or throws past their field, is held by nothing. Its particles share one velocity, so in
its own frame the ball stands still, and only its own pressure pulls it apart. It holds together for about its radius over
how fast it spreads, and travels as far as its speed carries it in that time. Faster goes further, and costs more to throw.
Denser hits harder, and comes apart sooner. The air strips its front as it flies.

When it hits something, its front stops and its back keeps coming: it piles up, packs denser, and splashes out. A burst on
impact needs no code.

### Orders

An order is a reaction or a phenomenon ingrained in mana: each particle carries it and runs it every tick, within its
beats (§5). An order can sense the particle and its neighbourhood, touch, condense, and **push its own particle, paying
with that particle's own mana**. So:

- *Burst on touch:* each particle spends some of itself to fly outward. The explosion is the fireball's own mana, spent.
- *Hold itself:* each particle at the edge, where it feels the mana thin, pushes itself back toward thicker mana. The
  construct stays together, and shrinks as it pays. A weave's leak (D14) is no longer a fixed rate: it is the price of
  holding.

A strong order holds tighter or bursts harder, and burns through its mana sooner.

**What an order costs.** Nothing about an order is free:

- *Ingraining it* costs the caster beats, particle by particle, as many as the order is long. A 120-particle fireball with a
  24-beat order takes an adept about ten ticks, and the ball isn't held while it's being ingrained.
- *Running it* burns the particle's own mana, a little for every beat it thinks. An order that decides quickly when there's
  nothing to do (most particles, most ticks) lasts much longer than one that works everything out every time.
- *Pushing* is paid from the particle, as before.

So a well-written order is short, and quick to say "nothing to do": D18 again, inside the mana.

### The second flaw

Orders don't spread. A particle that settles into the air leaves its order behind, and mana that was never ordered is
never given one. Except for one case.

When two particles merge, the new particle keeps the order of the bigger one. That rule was written to keep the number of
particles down, and nobody asked whose particles they were. A big ordered particle that comes to rest against someone
else's mana takes it over: loose mana, the mana of another weave, another caster's fireball. With enough mana packed into
one place, an order spreads through whatever it merges with, like a chemical reaction running through a substance.

It's held back by the same rule that made it: two particles merge only if together they're no more than `maxMote`, so an
ordered particle of 0.75 M can take in one mote and no more, until it splits. A particle of only one mote, ordered, ties with
the mote beside it, and the older one wins. Every takeover is logged as `taken`: which weave took in whose mana, and whether
it gave it its order.

Like the first flaw, it isn't an instruction, nothing in the libraries uses it, and the tester only shows what it does.

### Built

1. A 2D sandbox, outside the machine: a ball of particles with pressure, and a caster with beats and mana pushing it by each
   strategy in *Holding*. The bench (below) has taken its place.
2. Particles in the world, beside the air grid, which now moves (`src/vm/fluid.ts`). The ledger counts them, and a second
   ledger counts momentum.
3. The instructions in §5: sensing and pushing particles (`PCNT`, `PPOS`, `PVEL`, `SHOV`, `WPOS`, `WVEL`), ingraining
   (`INGR`), the field (`HOLD`), and in orders `KICK`, `DENS`, `GRAD`, `NVEL` and `VEL`. `MOVE` and `LOCK SHAPE` are gone.
4. The libraries and the four spells, rewritten (§6, §7).
5. The bench (below).
6. Weight, matter's mass, the ground's support and friction, water that holds together and can't be squeezed, rock, and
   pushes priced by the energy they add (D28–D30).
7. Particles merging and splitting, and the second flaw.
8. The air as a gas that flows (`src/vm/air.ts`).
9. The Energy ledger (`src/vm/energy.ts`).

Things the machine found that the sandbox couldn't:

- **Pouring needs the hand.** Fire poured into one point bursts outward at about 0.1 m a tick. An adept ingrains an order
  into one particle at a time, and the last ones had flown a metre before they got it. Holding a weave in hand still (D26)
  is what makes pouring work: ingrain at leisure, then let go.
- **Reach decides what you can ingrain.** A Stone Wall 4.5 m away has its foot 2 m underground, out of a 3 m reach: only its
  top got the order, rose, and tore away. An adept reaches 4 m now, and the test wall is raised 4 m away.
- **A rising wall drops earth into its own path.** Climbing costs mana, less mana holds less earth, and dropped earth filled
  the trench below the rows still climbing and blocked them. A weave's own matter doesn't block it, as before; others' does.
- **A wall that weighs can't hang over its own trench.** Lifted out of the ground, it has nothing under it: holding it up
  costs nothing in mana (D29), but its order has to think every tick to do it, and burns. So it steps back onto the solid
  ground in front of the trench and stands there, its order quiet. The trench becomes a ditch in front of it.
- **Rock needs bracing.** Bound only to the neighbours straight beside them, a 3D wall's particles fold like a stack of
  cards: nothing resists shear. Bound to their diagonal neighbours too (`bondRange` 0.2 m), it stands.
- **Rock has to be held from the ground up.** Bonds that keep their length are solved a pass at a time; solved in any order,
  the ground's support climbs a 2 m wall too slowly, and it sags. Solved lowest first, with the ground in the same passes,
  it stands in one.
- **Air that's too soft piles up.** With mana's own gas stiffness, the air's speed of sound was 1 m/s, and wind piled up
  against a pillar instead of going over it. The air has its own, faster (`airSound`).
- **Stepping waves the wrong way makes them grow.** Moving the air with its old speeds while the pressure pushed it made
  every disturbance grow into a storm. Pushed first, then moved, they die away.
- **Holding is thinking, not mana.** With pushes priced by energy, pushing a straying particle back in mostly slows it:
  holding a ball by hand costs almost no mana, and what limits a caster is how fast they think, as the sandbox found.
- **Merging is an optimisation nobody wrote.** A fireball held by its order merges as it flies (72 particles to about 45),
  and a merged particle thinks once where two did: the ball burns about 40% less.

### How the spells do now

The targets, and where the four spells stand against them (2D, an adept), with ticks of 1/30 s:

| Spell | Target | Now |
|---|---|---|
| Fireball | Leaves the hand within a second, reaches a pillar 9 m away with most of its mana | Let go after 0.4 s, hits after 0.9 s more, 95% of its mana together |
| Stone Wall | Rises in seconds, stands on its own for half a minute or more | Rises and stands in 4 s, holds half its earth for 38 s, then crumbles |
| Water Shield | Holds its water around its caster for several seconds | Makes 11.5 M of water, holds half of it for 7 s |
| Gust | Knocks someone back | Pushes a 60 kg body back 1.4 m |

The numbers in `physics.ts` were left where they are: every spell meets its target.

### The bench

`npm run bench` holds a ball of fire still for 40 ticks, and throws one at a pillar 9 m away, in every way below, on the
machine itself. It runs each one for an adept and a master, on five layouts each (the ball gathered from 100 to 140 M).
Its spells are in `bench/`, and the orders and the ways of holding they use are libraries: `lib/Orders.masm` and
`lib/Holding.masm`.

| Spell | Who holds it | How |
|---|---|---|
| HoldNothing | nobody | its own pressure takes it apart |
| HoldEvery, HoldOther, HoldSurface | the hand | every particle; every other one; a look, then only the outer ones |
| HoldCling, HoldCohere | its order | where it feels the mana thinner than it's told the edge is, it kicks itself toward thicker mana; `cohere` only while drifting away from its neighbours |
| ThrowHand, ThrowHandHeld | the hand | pushed up to speed; and held by hand while in reach |
| ThrowCling, ThrowCohere | its order | thrown by hand, held by its order |
| ThrowHeading | its order | told one angle, it speeds itself along it, and nothing holds it |
| ThrowSteer, ThrowSteerCling | its order | told an angle and a speed, it kicks itself toward that velocity; and clings too |

What it found, on the machine with weight, rock, flowing air and merging (2D and 3D, 300 runs):

- **An order told only an angle moves the ball, and loses it.** `heading` flies, but nothing holds the ball: it spreads
  into the ground and bursts there. None of 20 throws reached the pillar.
- **An order that feels its own speed is the fastest throw.** `steer` kicks each particle toward the velocity it's told,
  which also cancels the ball's spreading. It reaches the pillar in 19 ticks against 26–28 for an adept's hand, because
  reach doesn't limit it, and pays for the speed out of the ball: all of it arrives, with about 80% of its mana.
- **Feeling does as well as knowing, thrown.** Held by `feel` (its neighbourhood, and how thin the edge is, told by the
  caster), 93–97% of a thrown ball arrives, as with `pull`, which knows the centre. Held still, `feel` keeps 93% of a 2D
  ball, but only about 40% of a 3D one: its edge test is too coarse for a 3D ball's neighbours. So knowing the centre is a
  convenience, not a power (question 6).
- **Length is time.** An adept ingrains a 19-instruction order into a fireball in 7 ticks, a 43 in 13, a 77 in 21.
- **Long orders fray.** Steering and pulling in the same order, and working out its direction (a sine and a cosine) on
  its first tick, comes to more than 64 beats: the weave frays the tick it's let go. The caster works the direction out
  instead, once, and the order is told it (w5, w6). Orders that work something out once also keep it in the weave's
  registers for every particle after.
- **Holding is thinking.** Pushing a straying particle back in mostly slows it, which costs nothing (D29): holding a ball
  by hand spends almost no mana. An adept can't hold a ball by hand and throw it too: they don't get round fast enough,
  and most of it strays. A master can.
- **Every other particle does better than every particle**, for an adept holding still: 98–99% kept against 91–92%, for the
  same beats. The neighbours pass the push on.

The report of these runs, with every number, is published as a page: *Mana Order Bench*.

**Since D31** orders only feel. The bench's orders that knew the centre (`pull`, and `cohere` as it was) are gone; `cling`
and `cohere` hold by feel, and the findings above that compare them with knowing the centre are from before. What the bench
finds now is under *Orders that only feel*, below.

The sandbox's findings (2D, its own physics). The first group follows from the physics, and should hold whatever the
numbers are tuned to:

- **Unheld, a fireball comes apart.** 120 M laid out in 0.5 m keeps a quarter of its mana in the weave after 40 ticks, and
  doubles its spread.
- **Reach limits a throw.** Pushes speed a ball up only so fast, and it leaves the caster's reach before it's up to
  speed: a master's 0.4 m/tick throw goes at 0.29.
- **You can't throw fire by pushing its back.** Pressure carries a push forward only at about the speed mana spreads
  (0.03 m/tick), far slower than a throw: the back packs in and the front lags. Stiff mana, like earth, should carry it.
- **Only an order holds a ball once it's thrown.** Out of reach, nobody can push it. A master's ball held by its order
  reaches the wall with 77% of its mana together; held by hand until it leaves their reach, 43% at best.
- **The air drags.** A thrown ball sets the air behind it moving, and thicker air slows it: with three times the world's
  air, a master's throw takes nearly twice as long to reach the wall.
- **The air doesn't crush a fireball.** Pressure here grows with density, as in a gas, so the fire mana already in the air
  presses the same inside the ball and outside it. It cancels. A ball pushes out by its own density, however thin it is
  beside the air. (An earlier version of this section said otherwise.)

The second group depends on prices that are still guesses:

- **By hand, the mind is the limit.** Every hand strategy uses all the beats it has, and spends little mana (4–6 M to hold
  a fireball for 40 ticks). An adept pushes about 8 particles a tick, a master about 40, and a master's ball stays tighter.
  A lower push yield would make mana the limit instead.
- **Every other particle is not cheaper by itself.** A visit costs the same whichever particle it's on, so pushing every
  other one only changes which get pushed. For an adept at rest, looking first and pushing only the surface did best, by a
  modest margin. If pushing cost more than sensing, every other one could win.
- **At rest, an order trades mana for thought.** It holds tighter than hand (1.13 against 1.30 for an adept) and leaves
  the caster's mind free once it's ingrained, but costs three to five times the mana (about 20 M over 40 ticks, from the
  ball itself).
- **An order by feel needs a dense ball.** An order that knows only its neighbourhood holds as well as one that knows the
  centre when a master ingrains it quickly. An adept takes ten ticks to ingrain it, the ball thins meanwhile, every
  particle feels it's at the edge, and they all burn themselves pushing: 17% reaches the wall, against 42% for an order
  that knows the centre.

The sandbox was 2D, with about 120 particles to a fireball, and is gone: the bench runs the same comparisons on the
machine.

---

## 12. Open questions

1. **The numbers.** How much matter 1 M binds, how fast weaves leak, how training grows stats. They start as guesses in
   `src/vm/physics.ts`, to be tuned in the tester.
2. **Runes as machine code.** Elvish *Runic Magic* is written "their own way", and a glyph already means a step. Glyphs could
   be **opcodes**, and the marks around them (the lattice, its families) the **operands**. A carved ring would then be a
   program the elves have always read straight, with no language in between. This needs its own design pass.
3. **Who found the flaw?** A flaw nobody teaches still has a history: who first condensed less than nothing, what it cost
   them, and who keeps it quiet. That's lore for Quire.
4. **The field.** Is a caster's field a ball around the weave's origin, or the particles they're keeping up with? Which body
   stat sets how far it reaches, and does it weaken with distance?
5. **Energy's price.** A push costs mana, by the kinetic energy it adds (D29), and the Energy ledger now counts every joule
   (§11). Whether a caster's own Energy (stamina, `condition`) pays too, and whether heat should warm anything (fire mana is
   what's hot; heat is the motion), is left for you: the ledger shows how much there'd be to pay. A Fireball's throw is
   about 1 kJ, lifting a Stone Wall about 9 kJ.
6. **How far does a particle feel?** *(Question 6, what an order knows, is closed: D31.)* A particle feels the mana within
   the smoothing length, 0.25 m. A 3D fireball of 0.5 m is only two of those across, with about two particles in each, so a
   few particles bunched at its edge feel as thick as its middle, and an order can't tell where the edge is: in 3D, holding
   a still ball by feel keeps little more than nothing does. Letting particles feel further than they press (say 0.5 m,
   worked out once a tick, only for orders) would fix it, at some cost in speed. So would more, smaller particles.
