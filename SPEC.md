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
| D14 | A weave set loose **slowly leaks** its mana back into the air. A wall stands while its mana holds the earth, then crumbles. |
| D15 | Every stat of a caster, body and mind, comes from **genetics**, their **condition** right now, and **training**. |
| D16 | Matter can be freed back into free mana, but only through **a flaw**: no instruction does it. `CNDS` checks that an amount fits the room a cell has, and never that it's above nothing. Condensing less than nothing runs backwards (§5, *The flaw*). |
| D17 | **Burning doesn't free anything.** What fire burns is still matter, and so is the fire. |
| D11 | The tester is a practical tool, maybe the kind Ikozu's mage-engineers would have, but not dressed up in lore. |

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

1. **Pure free mana binds matter of its own part** in the cell it's in (*influence*): earth mana takes hold of the earth there,
   water mana of the water. How much it can hold depends on how much mana is there. Too little, and some matter is left behind.
2. **Bound matter moves with its mana**, and is held up by it. A cell of earth mana that moves up carries its earth with it,
   and the ground it left is empty. Matter that is no longer bound follows its nature again: lifted earth falls.
3. **Mana can condense** (*make*): a weave's order can turn some of a cell's free mana into matter of the same parts (`CNDS`).
   Condensed matter is real. It stays when the weave is gone. Nothing natural frees it again: burning only changes what
   matter is mixed with fire, and a flame thins out into warmth that is still matter. Freeing it is possible, but only through a
   flaw in condensing (§5, *The flaw*).
4. **Loose mana** (sent, or let go) keeps its velocity, slows down, and spreads back into the air. Moving air mana pushes air:
   **wind**, which pushes whatever is light enough.
5. **Matter blocks matter.** A cell can't move into a cell holding matter that isn't its own. Running into it is a **touch**.
6. **Weaves leak.** Every tick, a weave set loose loses a little of its free mana to the air. Less mana binds less matter, so
   a Stone Wall slowly crumbles as its earth falls free. A weave whose input isn't locked can be fed by its caster (`EMIT`) to
   keep it standing.

Every rule has numbers to tune: how much matter 1 M binds, how fast weaves leak, how fast flame spreads, and so on. They
live in one table (`src/vm/physics.ts`).

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
- Mind instructions take 1 beat.
- Body and reach instructions take 4 beats.
- `TICK` ends the tick.

So a spell that takes 12 ticks the first time takes about 1 tick after a lot of practice. That's `t / (1 + c)`, and it falls
out of the machine without being written anywhere.

### Each tick

1. **The mind thinks:** it runs instructions until the tick's beats run out or a `TICK`.
2. **Holds run down.** A mana register whose hold has run out (`focus` ticks after its last CIRC) **joins the flow**.
3. **The flow drains:** whatever is above the baseline leaves, at most `drain` per tick, into the air around the caster.
4. **Overcharge:** if `flow + held mana + weaves still in hand > capacity`, the excess is counted as harm.
5. **Weaves hold:** each weave set loose leaks a little, then every weave takes hold of the matter its free mana can bind
   (and lets go of what it no longer can).
6. **Weaves set loose run their orders** (§5), and move.
7. **The world moves:** loose mana and wind, pushed bodies, falling and flowing matter, air mana evening out.

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
- **cells**: the mana it holds, cell by cell, and the matter that mana binds;
- **registers** `w0`–`w7`, numbers its order can read and write (a velocity, a phase);
- an **order**: a routine every cell runs, every tick, once the weave is manifested.

A weave is **in hand** from `WEAV` until `MANI`, and its mana counts toward the body's load. `MANI` sets it loose. From then on
it runs its order on its own mana, and `LOCK` can fix its shape (its cells move together, as one body), its input (no more
mana goes into it) or its order (it can't be given another one).

### The order: mana running code

ORDER, in the old Core, was *give an order to the mana particles*. Here it is literal. A weave's order is an assembly routine
that **each of its cells runs every tick**, like a tiny mind inside the mana. A cell thinks with `n0`–`n15` of its own and
has no mana registers. When it starts, a cell's registers hold:

| Register | |
|---|---|
| `n0:2` | The cell's position from the weave's origin. |
| `n3` | How much mana the cell holds. |
| `n4` | The weave's age, in ticks since it was manifested. |

An order can do arithmetic and jumps, read its weave's registers and the ports below, and use the **order** instructions
(`MOVE`, `TUCH`, `GETW`, `PUTW`, `DISS`). It ends with `RET`. An order that runs more than 64 instructions in one tick
**frays**: the weave comes apart, and its mana goes loose.

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
| `ORIGIN` | 3 | *In an order:* where the weave's origin is now. |
| `MAKER` | 3 | *In an order:* where the weave's caster is now. |

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
| `42` | `SEND m, n, n:3, n:3` | SEND | Let `n` M of `m` out at a position, with a velocity, loose. |

#### Weave

| Op | Mnemonic | Lore | Does |
|---|---|---|---|
| `50` | `WEAV d, n:3` | | Begin a weave with its origin at `n:3`. Its id goes into `d`. In hand. |
| `51` | `TURN w, n:3` | POSITION | Turn the weave's frame so forward points along `n:3` (about the vertical). |
| `52` | `EMIT m, n, w, n:3` | | Move `n` M of `m` into the weave, at `n:3` in the weave's frame. Gives what there is if `m` holds less, and nothing at a point outside the world. |
| `53` | `WSET w, #k, s` / `WGET d, w, #k` (`54`) | | Write and read a weave's registers. |
| `55` | `ORDR w, L` | ORDER | Give the weave its order: the routine at `L`. |
| `56` | `MANI w` | SEND | Set the weave loose. It leaves the body's load and starts running its order. |
| `57` | `LOCK w, SHAPE \| INPUT \| ORDER` | LOCK | Lock it. Only after `MANI`. |
| `58` | `RELS w` | | Let the weave go: its mana goes loose where it is. |

#### Order (only inside an order)

| Op | Mnemonic | Does |
|---|---|---|
| `60` | `MOVE n:3` | Move this cell by `n:3` this tick, in the weave's frame, with the matter it binds. With `LOCK SHAPE`, the weave moves as one, by the average of its cells' moves. |
| `61` | `TUCH d` | `d = 1` if this cell is against matter, or a body, that isn't its own or its maker's. |
| `62` | `GETW d, #k` / `PUTW #k, s` (`63`) | Read and write this weave's registers. |
| `64` | `DISS` | The whole weave comes apart. Its mana goes loose where it is. |
| `65` | `CNDS s` | Condense `s` M of this cell's free mana into matter of the same parts (*make*). The matter stays in the cell, bound by whatever free mana is left. Only as much as the cell has room for. |

#### The flaw

Every instruction that takes an amount of mana treats an amount below nothing as nothing: `GATH m0, #-50` gathers nothing.
Every instruction except one.

`CNDS` asks one question of its amount: does the cell have room for it? It never asks whether the amount is above nothing,
because nobody thought to condense less than nothing. Below nothing, condensing runs backwards. The matter the cell holds
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
- a port, or `SHAPE`/`INPUT`/`ORDER`: one byte;
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
| `ORDER_ONLY` | An order's instruction (`MOVE`, `TUCH`…) in a mind. |
| `NOT_IN_ORDER` | A body, reach or weave instruction inside an order. The weave frays. |
| `BAD_PORT` | `ORIGIN` or `MAKER` read by a mind, or a will port read by an order. |

Overcharge is not a fault (D6). It's harm, counted by the tester.

---

## 6. The libraries

Libraries are written in assembly, or in the language once it exists. They are Quire's libraries: what a caster knows decides
which they can use. The machine has no shapes or elements. Change `Shapes.ball` and every fireball changes. Someone's own ball,
`Correni.Shapes`, can be rounder.

### Elements

```
; Elements: the four names mana answers to (the Law of the Four)
        .const FIRE   0
        .const WATER  1
        .const AIR    2
        .const EARTH  3
```

Compound aspects, like Quire's Plant (born from Water and Earth), are routines here that filter two parts and join them.
Mana mixed like that, condensed, is matter of both parts. The Elements library is also where mixes get their names: earth and
water condensed is *mud* or *plant*, depending on how they're ordered.

### Basics

```
; toward: a velocity from n0:2 to n3:5 at speed n6. Out: n3:5
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

; fly (an order): every cell moves by the weave's velocity, kept in w1–w3
fly:    GETW  n0, #1
        GETW  n1, #2
        GETW  n2, #3
        MOVE  n0:2
        RET

; anchor (an order): the weave follows its maker
anchor: IN    n5:7, MAKER
        IN    n8:10, ORIGIN
        SUB   n5, n8
        SUB   n6, n9
        SUB   n7, n10
        MOVE  n5:7
        RET
```

### Shapes

A shape lays the mana in `m1` out in the weave, around its origin. It's plain geometry. A big shape takes a mind many
ticks to lay out, longer than a hold lasts, so each shape re-`CIRC`s its mana as it goes. Without that, the mana slips into
the body's flow halfway through, and half a wall is laid out with nothing.

The ball is built the way you'd draw one by hand. Take the radius, and go out from the centre in shells. Each shell is
rings around the up axis, from its top to its bottom: a ring at angle φ down the shell has radius `ρ sin φ` and height
`ρ cos φ`, and as many points as its circumference, `2πs`, has steps. Each point is `(s cos θ, y, s sin θ)`. In 2D the page
cuts every ring at two points, `θ = 0` and `π`, and the same walk draws a disc.

Points land in cells, and a cell takes whatever points fall in it. Spaced a whole cell apart, the points miss cells and the
ball has holes, so they're spaced half a cell apart. To give every point the same share, the ball walks itself twice: once
to count its points, once to lay them out.

That's thorough, and slow. An adept's mind takes about 26 ticks to lay out a 3D ball of 0.5 m, where a 2D one takes 10. A
fireball the caster has thrown a few times is quicker (the Law of Conditioning). A faster ball is a different ball:
one that tests every cell of a cube against `x² + y² + z² ≤ r²` takes 8 ticks, but it isn't drawn around an axis.

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

This shield is uneven: near the poles, θ steps by the same angle on a smaller circle, so points crowd together. That's the
kind of thing a better library version fixes. It's also the kind of thing a mage could be known for.

```
; wall: a block of the ground under the weave's origin, given the order to rise
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
        DIV   n10, #2             ; (L − 1) / 2, to centre the length
        MOV   n11, n2
        SUB   n11, #1
        DIV   n11, #2             ; (T − 1) / 2, to centre the thickness
        LDI   n9, #0              ; d: 0 … H−1, down into the ground
.d:     CIRC  m1                  ; keep holding it, layer by layer
        LDI   n8, #0              ; t: across
.t:     LDI   n3, #0              ; a: along
.a:     MOV   n13, n3
        SUB   n13, n10
        MUL   n13, n5             ; x = (a − (L−1)/2) · cell
        MOV   n14, n9
        NEG   n14
        MUL   n14, n5             ; y = −d · cell
        MOV   n15, n8
        SUB   n15, n11
        MUL   n15, n5             ; z = (t − (T−1)/2) · cell
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
        WSET  n4, #0, n0          ; how far to rise, for the order
        ORDR  n4, rise
        RET

; rise (an order): each cell climbs one cell a tick, until the weave has risen w0 metres
rise:   IN    n5, CELL
        MOV   n6, n4
        MUL   n6, n5              ; risen so far
        GETW  n7, #0
        CMP   n6, n7
        JGE   .done
        LDI   n0, #0
        MOV   n1, n5
        LDI   n2, #0
        MOVE  n0:2                ; up one cell, carrying its earth
.done:  RET
```

The wall isn't made of the spell's mana. It's the ground, lifted: the earth mana *influences* the earth in each cell, binding
it and carrying it up, and the ground it came from is left as a trench. A caster with poor earth affinity puts less mana in
each cell, binds less earth, and the wall rises full of holes. Once set loose, the weave leaks. As its mana thins, it holds
less earth, and the wall crumbles back into the trench it came from.

Size matters in 3D. The 2D wall is 16 cells of earth; the 3D one, 4 m long, is 256. Binding a full cell takes 5 M of earth
mana, so the 3D wall needs about 8000 M gathered: far past an adept's capacity of 600. It's a master's spell.

### Reactions

```
; touch: n5 = 1 if this cell is against something that isn't its own
touch:  TUCH  n5
        RET
```

### Transformations

```
; condense (an order): on the first tick, half of each cell's mana condenses into matter (make)
condense:
        CMP   n4, #0
        JNE   .done
        MOV   n5, n3
        MUL   n5, #0.5
        CNDS  n5                  ; the other half stays free and holds it
.done:  RET
```

```
; expand (an order): each cell flies out from the origin, twice as far each tick.
; After 3 ticks the weave lets go. It reads when it began from w4.
expand: GETW  n5, #4
        MOV   n6, n4
        SUB   n6, n5              ; ticks since it began
        CMP   n6, #3
        JGE   .gone
        MOVE  n0:2                ; out along its own position: twice as far
        RET
.gone:  DISS                      ; its mana goes loose: the flare
        RET
```

---

## 7. The four spells

Each spell is written in assembly, with the old Core's lore names in the comments. After it comes the same spell in the
language, which compiles to roughly the same thing (§8). Every spell begins `.use Elements, Basics, Shapes, Reactions,
Transformations`, or whichever it needs.

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
        CALL  wall                ; ORDER: the ground is laid out to rise
        MANI  n20                 ; SEND: it rises
        LOCK  n20, SHAPE          ; LOCK: it stays a wall
        MOV   n0, n20
        HALT                      ; m0 isn't held any more: it joins the flow
.fail:  FAIL  #1                  ; no earth there
```

```
from Shapes use wall
from Elements use earth

Metadata StoneWall(Metadata data) {
  IF (.NOT. self.probe(data.coordinate, earth)) DO
    return Metadata.fail(data)
  END IF

  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Earth> active_mana = self.filter(mana_pool, earth)

  ManaConstruct ground = self.weave(data.coordinate)
  ground.face_away(self)
  ground.lay(active_mana, wall(2 m, 4 m, 0.5 m))
  ground.manifest()
  ground.lock(shape)

  return ground.metadata
}
```

### Fireball

A sphere of fire, gathered, shaped and thrown. On touch, it bursts.

```
Fireball:
        IN    n17, AMOUNT
        GATH  m0, n17             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #FIRE       ; FILTER
        IN    n1:3, HAND
        WEAV  n16, n1:3           ; POSITION: a weave at the hand
        MOV   n4, n16
        LDI   n0, #0.5
        CALL  ball                ; ORDER: the fire laid out as a ball, 0.5 m
        IN    n0:2, HAND
        IN    n3:5, AIM
        IN    n6, FORCE
        CALL  toward              ; n3:5 = its velocity
        WSET  n16, #1, n3
        WSET  n16, #2, n4
        WSET  n16, #3, n5
        ORDR  n16, .order         ; REACT: fly, and on touch, expand
        MANI  n16                 ; SEND: it leaves the hand
        LOCK  n16, INPUT          ; LOCK: cut from the caster
        MOV   n0, n16
        HALT

.order: GETW  n5, #0              ; w0, the phase: 0 flying, 1 bursting
        CMP   n5, #0
        JNE   .burst
        CALL  fly
        CALL  touch               ; n5 = 1 if this cell touched something
        CMP   n5, #0
        JEQ   .end
        PUTW  #0, n5              ; the whole weave bursts from the next tick
        PUTW  #4, n4              ; counting from now
.end:   RET
.burst: CALL  expand
        RET
```

```
from Shapes use ball
from Reactions use touch
from Transformations use expand

Metadata Fireball(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Fire> active_mana = self.filter(mana_pool, fire)

  ManaConstruct spell = self.weave(self.hand)
  spell.lay(active_mana, ball(0.5 m))
  spell.throw(data.coordinate, data.force)
  spell.add_react(touch, expand)
  spell.manifest()
  spell.lock(input)

  return spell.metadata
}
```

The shape is **not locked**. `expand` moves each cell out along its own position, so the ball has to be free to come apart.

### Gust

A sudden push of wind, for as long as the caster keeps it up. It makes no weave: it sends loose air mana, and moving air mana
is wind.

```
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
        SEND  m1, n6, n0:2, n3:5  ; SEND: all of it, from the hand
        GATH  m1, n17
        JOIN  m0, m1              ; a fresh breath for the next pass
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
long Gust overcharges. It needs only two streams.

### Water Shield

A skin of moving water around the caster that turns blades and flame.

```
WaterShield:
        IN    n16, AMOUNT
        GATH  m0, n16             ; GATHER
        CIRC  m0                  ; CIRCULATE
        FILT  m1, m0, #WATER      ; FILTER
        IN    n1:3, SELF
        WEAV  n17, n1:3           ; POSITION: a weave around the caster
        MOV   n4, n17
        LDI   n0, #1.2
        CALL  shield              ; ORDER: a shell of water, 1.2 m
        ORDR  n17, .order
        MANI  n17                 ; SEND
        LOCK  n17, SHAPE          ; LOCK: it stays a shell
        LOCK  n17, INPUT
        MOV   n0, n17
        HALT

.order: CALL  condense            ; MAKE: half its mana becomes water, held by the other half
        CALL  anchor              ; and it follows its maker
        RET
```

```
from Shapes use shield
from Basics use anchor
from Transformations use condense

Metadata WaterShield(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Water> active_mana = self.filter(mana_pool, water)

  ManaConstruct spell = self.weave(self.position)
  spell.lay(active_mana, shield(1.2 m))
  spell.order(condense, anchor)
  spell.manifest()
  spell.lock(shape)
  spell.lock(input)

  return spell.metadata
}
```

This shield *makes* its water instead of finding it. The water is real: as the weave leaks and its free mana thins, it holds
less of it, and the shield sags and falls apart in a splash at the caster's feet. A shield cast beside a river could
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

One screen, four parts:

- **Code**: the editor (CodeMirror 6), with highlighting, hover, completion and live errors.
- **Machine**: the assembly, line for line with the code, and the bytes in hex.
  - registers: the mind's `n` and the body's `m`, each `m` as four coloured parts with its hold running down;
  - the stack;
  - each weave's registers;
  - step by instruction, by tick, or into a weave's order for one cell.
- **World**: the spell being cast, in 2D first and 3D later. You see air mana as a haze, matter, weaves cell by cell, loose
  mana and wind.
- **Caster**:
  - the body's gauge: baseline, flow, held mana, weaves in hand, and the capacity line;
  - the harm from overcharge;
  - the ledger;
  - who is casting: their stats, and conditioning per spell.

Your will drives the cast: click to aim, hold a key to maintain, sliders for amount and force.

---

## 10. How it's built

TypeScript throughout, like Quire, so it can run inside Quire later.

```
src/
  asm/       isa.ts (the instruction table), assembler.ts, disassembler.ts
  vm/        physics.ts (the numbers), parts.ts, world.ts, caster.ts, weave.ts, sim.ts (the machine and the tick)
  cli/       mas.ts, mvm.ts
  scenes.ts  test worlds for the four spells, in 2D and 3D
  render.ts  a slice of the world as text
lib/         Elements, Basics, Shapes, Reactions, Transformations, in .masm
spells/      StoneWall, Fireball, Gust, WaterShield, in .masm
test/        the assembler, the machine, the four spells
```

```
npm install
npm test                                   the tests
npx tsx src/cli/mas.ts spells/Fireball.masm  the listing: addresses, bytes, instructions
npx tsx src/cli/mvm.ts spells/StoneWall.masm cast it in its test world, in the terminal
npx tsx src/cli/mvm.ts spells/Gust.masm --ticks 40 --maintain 30
```

### Phases

1. **The machine.** *Built.*
   - The world (2D and 3D), the caster, the tick.
   - Every instruction. Orders running per cell.
   - The ledger, the assembler and disassembler.
   - The libraries and the four spells in `.masm`.
   - Tests: the wall stands and leaves a trench, then crumbles back into it; the fireball bursts on touch; the shield follows
     its maker and falls in a splash; Gust pushes and, kept up too long, overcharges; the ledger balances every tick.
2. **The tester, first cut.** Assembly editing, stepping, registers, a 2D world, the caster panel.
3. **The language**, compiling to what phase 1 runs by hand.
4. Casters, spells and libraries read from Quire.
5. **Other notations** (later): runes, circuits and scores.

---

## 11. Open questions

1. **The numbers.** How much matter 1 M binds, how fast weaves leak, how training grows stats. They start as guesses in
   `src/vm/physics.ts`, to be tuned in the tester.
2. **Runes as machine code.** Elvish *Runic Magic* is written "their own way", and a glyph already means a step. Glyphs could
   be **opcodes**, and the marks around them (the lattice, its families) the **operands**. A carved ring would then be a
   program the elves have always read straight, with no language in between. This needs its own design pass.
3. **Who found the flaw?** A flaw nobody teaches still has a history: who first condensed less than nothing, what it cost
   them, and who keeps it quiet. That's lore for Quire.
