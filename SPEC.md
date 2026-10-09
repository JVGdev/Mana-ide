# Mana: a language, a machine and a spell tester for Ikozu's magic

*Working name: **Mana**. Files are `.mana` (spells), `.masm` (assembly) and `.mbc` (bytecode); the tools are `manac` (compiler),
`mas` (assembler) and `mvm` (the machine). Rename freely.*

Ikozu's Mana is already written like code in Quire's *Codefied Modern Formulae* notation, but nothing runs it. This project
makes it real:

1. a **machine** whose instructions are what a caster's body can actually do with mana: the Core, as close to machine code as
   it gets;
2. a **language**, in the style of the codified spells (braces for declarations, Fortran for control flow, `!` comments), that
   compiles to that machine;
3. a **spell tester**: an editor, the compiled assembly beside it, and a world where the spell is cast, watched and stepped
   through.

The examples throughout are the four spells of Ikozu: **Stone Wall**, **Fireball**, **Gust** and **Water Shield**.

---

## 1. Decisions so far

| # | Decision |
|---|---|
| D1 | The Core is rethought: no more one operation per aspect (FILTER·Fire, FILTER·Earth, SEND·Wall, LOCK·Shield…). Aspects become **operands**, and the Core is a machine with registers and instructions. |
| D2 | The Core is as close to machine code as possible: fixed mnemonics, registers, labels, jumps, and a byte encoding. |
| D3 | Mana works in **3D**. A **2D** world is a 3D world one cell thick, so every spell runs in both unchanged. |
| D4 | Keep the codified style: `Type name(params) { }` declarations, `IF (…) DO … END IF`, `WHILE (…) DO … END DO`, `.NOT.`, `!` comments, optional `CALL`. |
| D5 | Mana that isn't maintained **joins the body's natural flow** and follows it. Held mana plus the flow above the body's capacity is an **overcharge**. |
| D6 | **LOCK comes after manifest**. It locks something of the manifestation: its shape, its input, its position. |
| D7 | **Affinities** are the caster's capacity to filter and circulate each element. Filtering loses mana in proportion to the lack of affinity, and a weaker charge makes a weaker spell. |
| D8 | The tester is a practical tool, maybe the kind of spell tester Ikozu's mage-engineers would have, but not dressed up in lore. Errors are technical and name the law they break. |
| D9 | Aspect costs are left out. A spell's strength comes from how much mana it carries, not from a price list. |

---

## 2. The world

### Space

- The world is a 3D grid of **cells** (say 0.25 m each). A 2D world has depth 1.
- Each cell holds:
  - **air mana**: how much mana floats there, as an element vector (below);
  - **matter**: none, earth, water or air, and how much.
- Positions are vectors `(x, y, z)`. In 2D, `z` is always 0.
- Time moves in **ticks**. Every instruction takes one tick unless it says otherwise.

### Mana is a vector of four elements

The Law of Equality says *M = Ma + Mf + Mw + Me*, each a quarter of a particle's volume. So every amount of mana, wherever it
is, is a vector `[F, W, A, E]`:

- **raw** mana, gathered from the air, is `[¼, ¼, ¼, ¼]` of its amount;
- **filtered** mana is pure: `[0, 0, 0, E]` is earth mana;
- **residue** is what's left after filtering: raw mana missing one or more parts.

The Law of the Four becomes a type rule: only **pure** mana (one element) can be given a shape.

The Law of Conservation is a property of the machine, not a rule a spell can break. No instruction creates or destroys mana.
Every instruction moves it between places, and the tester's **ledger** always balances:

```
Σ air + Σ body + Σ registers + Σ vessels = constant
```

### The caster's body

A caster is a body with:

| Stat | Meaning |
|---|---|
| `capacity` C | How much mana the body can bear at once. |
| `flow` | The body's natural flow: mana circulating on its own. It rests at a **baseline** B. Above B, it drains back to the air at a natural rate *d* per tick. |
| `affinity[e]` | 0–100% for each element: how well the caster filters and circulates it. |
| `focus` W | How many ticks a CIRCULATE keeps mana held before it has to be done again. |
| `streams` | How many mana registers the caster can work with (2 for an apprentice, 8 for a master). |

Each tick, the machine does this:

1. **Holds run down.** A held mana register whose hold has run out (W ticks after its last CIRC) **joins the flow**. Its mana
   isn't lost: it is now the body's.
2. **The flow drains.** Flow above the baseline drains to the air around the caster, at most *d* per tick.
3. **Overcharge check.** If `flow + held registers + vessels still in hand > capacity`, the body is **overcharged**: an
   `OVERCHARGE` fault, with the excess.

The residue left after a filter doesn't need any special handling. If the spell doesn't hold it, it joins the flow and leaves
the body on its own. If the spell holds too much, or gathers too much too fast, the body overcharges.

### Filtering and affinity

`FILT` takes one element's part out of a mana register:

```
part   = source[e]                    ! at most ¼ of raw mana
out    = part × affinity[e]           ! what the caster manages to filter
loss   = part × (1 − affinity[e])     ! slips into the body's flow
source = source without its e part    ! the residue stays where it was
```

A caster with 40% earth affinity who gathers 100 M gets 10 M of earth (25 × 0.4). Another 15 M slips into their flow, and 75 M
of residue is left in the register. A Stone Wall charged with 10 M is lower and weaker than one charged with 25 M.

### Vessels

A **vessel** is anything that holds mana and can be ordered:

- a **construct**: mana given a shape, made from nothing but mana (a fireball, a shield);
- **matter** taken hold of: the earth under a Stone Wall, a pool of water.

A vessel is **in hand** from the moment it is made or taken hold of until it is manifested. While in hand, its mana counts
toward the body's load. **Manifesting** sets it loose in the world. From then on it lives on its own mana, and its orders can
be **locked**.

---

## 3. The machine (the Core)

### Registers

| Registers | Holds | Notes |
|---|---|---|
| `r0`–`r15` | numbers | Amounts, forces, counters, flags. |
| `v0`–`v7` | vectors | Positions and directions in space. |
| `m0`–`m7` | mana | Each holds an element vector. **Linear**: mana can be moved, split and joined, never copied. The number usable is the caster's `streams`. |
| `h0`–`h7` | vessel handles | 0 means no vessel. |
| `flags` | result of `CMP` | Read by the conditional jumps. |

### Ports (the caster's will and the caster's sheet)

`IN` reads a port. The caster's will is live input, read again every time. In the tester, it comes from the mouse, the
keyboard and sliders.

| Port | Type | What |
|---|---|---|
| `AIM` | v | Where the caster means it to go (`data.coordinate`). |
| `HAND` | v | The casting hand. |
| `SELF` | v | The caster's body (centre). |
| `AMOUNT` | r | How much the caster means to gather. |
| `FORCE` | r | How hard. |
| `MAINTAIN` | r | 1 while the caster keeps the spell going. |
| `AFF_F`, `AFF_W`, `AFF_A`, `AFF_E` | r | The caster's affinities. |
| `LOAD`, `CAPACITY` | r | The body's current load and capacity. |

### Operand constants

| Kind | Values |
|---|---|
| element | `RAW` 0, `FIRE` 1, `WATER` 2, `AIR` 3, `EARTH` 4 |
| property (`ORD`) | `POS` 0, `VEL` 1, `SHAPE` 2, `SIZE` 3, `GROW` 4, `ANCHOR` 5 |
| lock (`LOCK`) | `SHAPE` 0, `INPUT` 1, `POS` 2, `ALL` 7 |
| trigger (`REACT`) | `TOUCH` 0, `AFTER` 1 (ticks), `EMPTY` 2 (charge ran out) |
| shape | indices into the **shape table** that libraries compile into (`BALL`, `WALL`, `SHIELD`, `BOLT`, `BLADE`, `SPIKE`, `WAVE`…) |

### Instructions

`d` is a destination, `s` a source, `#` an immediate. **Mana** instructions are the old Core, with aspects as operands.

#### Control

| Op | Mnemonic | Does |
|---|---|---|
| `00` | `NOP` | Nothing, for one tick. |
| `01` | `HALT` | Ends the spell. |
| `02` | `FAIL #code` | Ends the spell as a failure (`Metadata.fail`). |
| `03` | `JMP label` | Jump. |
| `04` | `JZ r, label` / `JZ h, label` | Jump if zero / no vessel. |
| `05` | `JNZ r, label` | Jump if not zero. |
| `06` | `CMP a, b` | Compare, set flags. |
| `07` | `JEQ` `JNE` `JLT` `JGE label` | Jump on flags. |
| `08` | `CALL label` | Call a routine (library code). |
| `09` | `RET [h]` | Return, optionally with a vessel whose metadata is the spell's result. |
| `0A` | `TICK [#n]` | Wait one tick (or n). |

#### Data

| Op | Mnemonic | Does |
|---|---|---|
| `10` | `MOV d, s` | Copy a number, vector or handle. **Not allowed on mana registers.** |
| `11` | `LDI d, #imm` | Load an immediate. |
| `12`–`15` | `ADD` `SUB` `MUL` `DIV d, s` | Arithmetic on numbers and vectors. |
| `16` | `VEC v, rx, ry, rz` | Build a vector. |
| `17` | `IN d, PORT` | Read the caster's will or sheet. |

#### Mana

| Op | Mnemonic | Lore name | Does |
|---|---|---|---|
| `20` | `GATH m, r[, v]` | GATHER | Draw `r` M from the air around the caster (or around `v`) into `m`. Draws less if the air is thin. |
| `21` | `CIRC m` | CIRCULATE | Hold `m` for the caster's focus W ticks. |
| `22` | `FILT md, ms, elem` | FILTER | Take `elem`'s part of `ms` into `md`. Lack of affinity loses some to the flow (§2). |
| `23` | `SPLT md, ms, r` | | Move `r` M of `ms` into `md`. |
| `24` | `JOIN md, ms` | | Move all of `ms` into `md`. |
| `25` | `MEAS r, m[, elem]` | | How much `m` holds (of `elem`). Reading isn't using: `m` stays. |
| `26` | `SEND m, v[, r]` | SEND | Let `m` out at `v`, pushed with force `r`. It's raw release: wind, heat, splash. |
| `27` | `VENT m` | | Let `m` out into the air around the caster, gently. |

#### Sense

| Op | Mnemonic | Lore name | Does |
|---|---|---|---|
| `30` | `PROB r, v, what` | PROBE | How much `what` (an element of air mana, or a matter) is at `v`. A little mana goes out and comes back. |
| `31` | `FIND v, s, what` | PROBE | The nearest cell to `s` that holds `what` (`v = s` if none). |

#### Vessels

| Op | Mnemonic | Lore name | Does |
|---|---|---|---|
| `40` | `MAKE h` | | A blank construct, in hand. |
| `41` | `GRAB h, v, matter` | | Take hold of the matter at `v` (`h = 0` if there's none). In hand. |
| `42` | `INFU h, m` | | Move all of `m` into the vessel. |
| `43` | `DRAW m, h, r` | | Move `r` M out of the vessel back into `m`. |
| `44` | `ORD h, prop, s` | ORDER / POSITION | Set a property: position, velocity, shape, size… |
| `45` | `REACT h, trigger, label` | REACT | When `trigger` happens to the vessel, run `label` with the vessel in `h0`. |
| `46` | `MANI h` | SEND (manifest) | Set the vessel loose in the world. It leaves the body's load. |
| `47` | `LOCK h, what` | LOCK | Lock a property of a **manifested** vessel. |
| `48` | `RELS h` | | Dissolve the vessel: its mana returns to the air where it is. |

### Encoding

Each instruction is one opcode byte followed by its operands:

- a register: one byte, with the class in the high nibble (`0` r, `1` v, `2` m, `3` h) and the number in the low one;
- a constant (element, property, port…): one byte;
- an immediate: two bytes, little-endian, fixed point 8.8;
- a label: two bytes, the address.

```
GATH m0, r0        →  20 20 00
FILT m1, m0, EARTH →  22 21 20 04
ORD  h0, SHAPE, #1 →  44 30 02 00 01
```

The bytecode is what `mvm` runs. Whoever wants to can write `.masm` by hand. The language also has inline assembly:

```
ASM DO
  FILT m1, m0, EARTH
END ASM
```

### Faults

| Fault | When |
|---|---|
| `OVERCHARGE` | The body's load goes past its capacity (§2). |
| `THIN_AIR` | `GATH` found less mana than asked. A warning, not a stop. |
| `MOVED` | A mana register was used after it was emptied. |
| `IMPURE` | `ORD SHAPE` on a vessel whose mana isn't one element (Law of the Four). |
| `NOT_MANIFESTED` | `LOCK` before `MANI` (D6). |
| `LOCKED` | `ORD` on a property that is locked. |
| `NO_STREAM` | A mana register beyond the caster's `streams`. |

What a fault does (stop the spell, burst at the caster, or hurt and go on) is an open question (§9).

---

## 4. The language

### Shape of a file

```
from Shapes use wall            ! imports from libraries
from Elements use earth

Metadata StoneWall(Metadata data) {   ! <ReturnType> <Name>(<params>) { }
  …                                   ! a return type of `spell` means none
}
```

- Declarations use braces: spells, routines, `class`, `Shape`.
- Control flow is Fortran-style: `IF (c) DO … ELSE DO … END IF`, `WHILE (c) DO … END DO`, `.NOT.`, `.AND.`, `.OR.`.
- `!` starts a comment. `CALL` may come before a call whose result isn't used.

### Types

| Type | What | Machine |
|---|---|---|
| `num` | A number, with an optional unit (`4 M`, `2 m`, `3 ticks`). | `r` |
| `bool` | | `r` |
| `space3d` | A position or direction (also in 2D worlds). | `v` |
| `mu`, `mu<Fire>`… | Mana: raw, or pure in one element. **Linear.** | `m` |
| `ManaConstruct` | A construct. | `h` |
| `Matter` | Matter taken hold of. | `h` |
| `Reading` | What a probe found. | `r`, `v` |
| `Metadata` | The cast: what the caster wills going in, and what the spell leaves coming out. | ports / `RET` |

### Mana is linear

A `mu` value can be used once. Passing it to `filter`, `add_mana`, `infuse` or `send` **moves** it, and the compiler rejects
a second use:

```
spell.add_mana(active_mana)
other.add_mana(active_mana)     ! error: active_mana was moved into spell on line 12
```

`filter` is the exception that proves the rule. It takes its source by reference, because the residue stays in it.

### `self` and `data`

- **`self`** is the caster: `class Caster` in the Basics library. A character's own class can extend it.
- **`data`** is the caster's will, read live. `data.maintain` asked in a loop is asked again each time, so `self.update_data()`
  isn't needed any more.

| Code | Compiles to |
|---|---|
| `self.gather(n)` | `GATH` |
| `self.circulate(m)` | `CIRC` |
| `self.filter(m, earth)` | `FILT` |
| `self.send(m, at, force)` | `SEND` |
| `self.sense(at, earth)` | `PROB` / `FIND` |
| `self.grab(at, earth)` | `GRAB` |
| `self.blank_spell()` | `MAKE` |
| `x.add_mana(m)`, `x.infuse(m)` | `INFU` |
| `x.reposition(p)`, `x.set_shape(s)`, `x.throw(at, f)` | `ORD` |
| `x.add_react(t, f)` | `REACT` |
| `x.manifest()` | `MANI` |
| `x.lock(shape)`, `x.lock(input)` | `LOCK` |
| `tick` | `TICK` |
| `data.coordinate`, `data.maintain`… | `IN` |

### Libraries

Libraries are what Quire already calls libraries (`from Shapes use wall`). Each compiles into:

- **shape table** entries: `Shape wall(height = 2 m, length = 4 m, thickness = 0.5 m)`, defined in any number of dimensions;
- **routines**: assembly subroutines. A reaction such as `expand` is a routine that a `REACT` jumps to.

What a caster knows (Quire's *Knows* links, schools…) decides which libraries are in reach. That comes later, with the Quire
link.

### What the compiler checks

- Names, imports and members: `manisfest` → *ManaConstruct has no member `manisfest`: `manifest`?*
- Types: a shape needs pure mana (`mu<Earth>`, not `mu`).
- Linearity: no mana used twice.
- Order: no `lock` before `manifest`, and no order on a locked property.
- Holds: a warning when mana is used after its hold has surely run out (more than W ticks since its last circulate).
- Streams: a warning when a spell needs more mana registers at once than a caster may have.

---

## 5. The four spells

Each spell appears twice: in the language, and the assembly it compiles to. Compared with the writings in Quire:

- `Mana[:, :, :] = self.scan()` is gone, because gathering reads the air itself;
- `scan_with_eligible_mana`, `match`, `blank_influence`, `infuse`, `assert` became **one** idea: take hold of matter, infuse it,
  order it;
- residue is left alone: it isn't held, so it joins the body's flow.

### Stone Wall

The ground heaves up into a wall of rock. It infuses matter and builds no construct.

```
from Shapes use wall
from Elements use earth

Metadata StoneWall(Metadata data) {
  Matter ground = self.grab(data.coordinate, earth)   ! there has to be earth where the caster aims
  IF (.NOT. ground.found) DO
    return Metadata.fail(ground)
  END IF

  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Earth> active_mana = self.filter(mana_pool, earth)

  ground.infuse(active_mana)
  ground.set_shape(wall)
  ground.manifest()
  ground.lock(shape)

  return ground.metadata
}
```

```
StoneWall:
        IN    v0, AIM
        GRAB  h0, v0, EARTH      ; PROBE: take hold of the earth at the aim
        JZ    h0, .fail
        IN    r0, AMOUNT
        GATH  m0, r0             ; GATHER
        CIRC  m0                 ; CIRCULATE
        FILT  m1, m0, EARTH      ; FILTER: m1 = earth × affinity, the residue stays in m0
        INFU  h0, m1
        ORD   h0, SHAPE, WALL
        MANI  h0                 ; SEND: the wall rises
        LOCK  h0, SHAPE          ; LOCK: it stays a wall
        RET   h0                 ; m0 isn't held any more: it joins the flow
.fail:  FAIL  #1                 ; no earth there
```

### Fireball

A sphere of fire, gathered, shaped and thrown. It is a construct that reacts.

```
from Shapes use ball
from Reactions use touch
from Transformations use expand

Metadata Fireball(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Fire> active_mana = self.filter(mana_pool, fire)

  ManaConstruct spell = self.blank_spell()
  spell.add_mana(active_mana)
  spell.reposition(self.hand)
  spell.set_shape(ball)
  spell.throw(data.coordinate, data.force)
  spell.add_react(touch, expand)
  spell.manifest()
  spell.lock(input)                 ! cut from the caster: it lives on what it carries

  return spell.metadata
}
```

```
Fireball:
        IN    r0, AMOUNT
        GATH  m0, r0
        CIRC  m0
        FILT  m1, m0, FIRE
        MAKE  h0
        INFU  h0, m1
        IN    v0, HAND
        ORD   h0, POS, v0
        ORD   h0, SHAPE, BALL
        IN    v1, AIM
        SUB   v1, v0             ; direction: hand → aim
        IN    r1, FORCE
        MUL   v1, r1
        ORD   h0, VEL, v1
        REACT h0, TOUCH, expand  ; library routine
        MANI  h0
        LOCK  h0, INPUT
        RET   h0

expand:                          ; from Transformations, with the vessel in h0
        LDI   r0, #4
        ORD   h0, GROW, r0       ; bursts out to 4× its size
        TICK  #2
        RELS  h0                 ; and its mana returns to the air
        RET
```

### Gust

A sudden push of wind. It is a channelled spell: it runs while the caster maintains it.

```
from Elements use air

spell Gust(Metadata data) {
  mu mana_pool = self.gather(data.amount)

  WHILE (data.maintain) DO
    CALL self.circulate(mana_pool)
    mu<Air> active_mana = self.filter(mana_pool, air)
    CALL self.send(active_mana, data.coordinate, data.force)
    mana_pool += self.gather(data.amount)          ! fresh air for the next breath
    tick
  END DO
}
```

```
Gust:
        IN    r0, AMOUNT
        GATH  m0, r0
.loop:  IN    r2, MAINTAIN
        JZ    r2, .end
        CIRC  m0
        FILT  m1, m0, AIR
        IN    v0, AIM
        IN    r1, FORCE
        SEND  m1, v0, r1         ; the push of wind
        GATH  m2, r0
        JOIN  m0, m2
        TICK
        JMP   .loop
.end:   HALT                     ; the residue in m0 joins the flow
```

Every pass keeps the residue (raw mana without its air) and adds a fresh gather. If the caster gathers more than the flow can
drain, the body's load climbs, and a long Gust can **overcharge** them. That's the price of channelling.

### Water Shield

A skin of moving water around the caster. It is a construct anchored to the caster, with its shape locked.

```
from Shapes use shield

Metadata WaterShield(Metadata data) {
  mu mana_pool = self.gather(data.amount)
  CALL self.circulate(mana_pool)
  mu<Water> active_mana = self.filter(mana_pool, water)

  ManaConstruct spell = self.blank_spell()
  spell.add_mana(active_mana)
  spell.anchor(self)                ! it moves with the caster
  spell.set_shape(shield)
  spell.manifest()
  spell.lock(shape)
  spell.lock(input)

  return spell.metadata
}
```

```
WaterShield:
        IN    r0, AMOUNT
        GATH  m0, r0
        CIRC  m0
        FILT  m1, m0, WATER
        MAKE  h0
        INFU  h0, m1
        IN    v0, SELF
        ORD   h0, ANCHOR, v0
        ORD   h0, SHAPE, SHIELD
        MANI  h0
        LOCK  h0, SHAPE
        LOCK  h0, INPUT
        RET   h0
```

### What the compiler would have said about the writings in Quire

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

### Matching Quire's Programs

Each spell in Quire has a **Program** in the old Core. The compiled assembly, read back through the lore names, should cover
the same steps:

| Spell | Quire's Program | Compiled, by lore name |
|---|---|---|
| Stone Wall | GATHER, CIRCULATE, FILTER·Earth, PROBE·Earth, POSITION, SEND·Wall, LOCK·Wall | PROBE (`GRAB`), GATHER, CIRCULATE, FILTER (EARTH), ORDER (WALL), SEND (`MANI`), LOCK (SHAPE) |
| Fireball | GATHER, CIRCULATE, FILTER·Fire, POSITION, SEND·Ball, REACT·Touch·Expand, LOCK | GATHER, CIRCULATE, FILTER (FIRE), POSITION, ORDER (BALL), REACT (TOUCH), SEND, LOCK (INPUT) |
| Gust | GATHER, CIRCULATE, FILTER·Air, POSITION, SEND·Air | GATHER, CIRCULATE, FILTER (AIR), SEND (at AIM), in a loop |
| Water Shield | GATHER, CIRCULATE, FILTER·Water, POSITION, SEND·Shield, LOCK·Shield | GATHER, CIRCULATE, FILTER (WATER), POSITION (ANCHOR), ORDER (SHIELD), SEND, LOCK (SHAPE, INPUT) |

The order differs in one place: Stone Wall probes **before** it gathers, so a caster aiming at empty air doesn't gather for
nothing.

---

## 6. The spell tester

One screen, four parts:

- **Code**: the editor (CodeMirror 6), with highlighting, hover (what a name is and where it's from), completion and live
  errors.
- **Machine**: the compiled assembly, line by line beside the code, with the bytes in hex. Click a line on one side to find it
  on the other.
- **World**: the spell being cast, in 2D (a slice) first and 3D later. You see air mana as a haze, matter, and constructs
  with their shapes. Mana is drawn moving: drawn in, circling the body, filtered, set loose.
- **Caster**:
  - the body's gauge: baseline, flow, held, vessels in hand, and the capacity line where overcharge begins;
  - the registers (`m0`–`m7` as four-colour bars, `r`, `v`, `h`);
  - the ledger;
  - who is casting: pick a caster (affinities, capacity, focus, streams) and see the same spell cast by someone else.

**Controls:**
- Cast, and step one instruction or one tick at a time, with breakpoints on code or assembly lines.
- The caster's will comes from you: click to aim, hold a key to maintain, sliders for amount and force.

---

## 7. How it's built

TypeScript throughout, like Quire, so the compiler can later run inside Quire too.

```
packages/
  lang/      lexer, parser, checker, code generator: .mana → .masm
  asm/       assembler and disassembler: .masm ⇄ .mbc
  vm/        the machine and the world: runs .mbc, ticks, faults, ledger
  cli/       manac, mas, mvm
apps/
  tester/    the spell tester (Svelte 5 + Vite)
spells/      the four spells, as .mana, with the expected .masm
```

Tests (vitest) compile the four spells and compare the result with the expected assembly. They also run each spell in a test
world and check the ledger balances, the wall stands, and the fireball bursts on touch.

---

## 8. Phases

1. **Machine.** The VM and the world with no language yet: registers, every instruction, the body (holds, flow, overcharge),
   the ledger. The assembler and disassembler. The four spells hand-written in `.masm`, running in tests.
2. **Language.** Lexer, parser and checker for the codified style. A code generator to `.masm`. The four spells compile to the
   hand-written assembly.
3. **Tester, first cut.** Editor and machine view, a 2D world, the caster panel, stepping.
4. **Libraries.** Shapes, Elements, Reactions and Transformations as real libraries (shape tables and routines). Inline `ASM`.
5. **3D world**, and casters from Quire: read the Ikozu system, its spells, libraries and casters from a Quire export.
6. **Other notations** (later): render compiled spells as runes, a Manatech circuit, a Bardic score.

---

## 9. Open questions

1. **Holds.** Is the focus W (ticks a CIRCULATE lasts) a caster stat? And is `streams` (how many mana registers) one too?
2. **Overcharge.** Does the spell stop and the excess burst out at the caster? Or does the caster take harm and the spell go
   on?
3. **The body's flow.** How fast does flow above the baseline drain? Is the baseline the caster's own mana, there before any
   spell?
4. **Affinity and circulation.** Does low affinity also lose mana while it's held (a slow slip each tick), or only when
   filtering?
5. **Matter.** How much earth does a wall need for its size? Does infusing more mana make it higher, harder, or both?
6. **Runes.** Elvish *Runic Magic* is written "their own way". Could runes be the machine code itself, one glyph per opcode,
   so that elves have always written spells in assembly?
