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

## 0. The main objective

**Every feature of a spell comes from Ikozu's physics.** This is the project's first rule. Every other decision serves it,
and where one conflicts with it, this one wins.

What happens in the machine has three sources, and only three:

- **The mage's mind**: what it computes. That's the assembly, and thinking costs what it costs a real processor (D18).
- **The mage's body**: what a living caster can do to mana (gather it, hold it, pour it, push it, sense it), within the
  limits of their body: reach, power, capacity, how fast they think.
- **The world's physics**: everything mana and matter do by themselves.

Nothing else. No instruction, port or rule exists to make a particular spell work. A ball holds together, a fireball
bursts, a wall stands and a shield follows its caster only because the physics lets them, and a mage worked out how.

- **The physics is real science**, as close to it as it can be, and logical in Ikozu's world. Conservation holds exactly
  (mana, momentum, Energy). Every force has its equal and opposite pair. Nothing acts at a distance unless something
  carries it there.
- **When a simple spell is too hard to make, tune the world, not the spell.** Change the physics' numbers until the
  world feels right for the story. Never add a special case.
- **Making the spells is the author's job.** The libraries and spells here show how the system works. They aren't tuned
  or balanced, and they aren't the last word on any spell.

The check for any part of the machine, old or new: *is this something the world does, or something a spell was given?*

### Where the machine doesn't meet it yet

An audit of the code against this objective (after D31) found sixteen places. They were fixed on
`claude/organic-physics` (D32–D39), and what each became is below. The work turned up a few more, at the end: those are
what's left.

**Broke a law of the world. Fixed:**

1. **A kick had nothing to push against.** Now a particle pushes off what's around it: the ground or other solid
   matter it's against, on the side it pushes away from, or else the air it's in, which takes the push back (D33). In
   empty space it can't push at all.
2. **A caster's push recoiled on nobody.** Now `SHOV` goes off the caster's body, through their reach, and `SEND`
   throws mana out of it: the body is pushed back, and their feet on the ground hold them as far as friction lets
   them (D33). A 5 g fireball hardly moves its thrower; a 400 kg wall moved by hand pulls its caster along, unless they
   move it gently.
3. **A push's Energy came from nowhere.** Now a mind transforms it out of mana, as hard as its power allows, and strains
   for it; past its capacity it's harmed (the Law of Transformation, D32). The mana isn't used up. The Energy ledger
   counts what minds put in, push by push.
4. **The weave's registers were telepathy.** Now each particle keeps its own copy, written by its own order or by a
   caster within reach, and passes it on to those touching it; the newest write wins (D34).
5. **A weave died all at once.** Now a particle that lets go (`DISS`) or frays goes alone. Its neighbours go on, and a
   burst spreads only as fast as the word of it passes by touch (D34).
6. **Belonging was measured from the centre.** Now a particle belongs to its weave while it carries the order, and,
   until it's given one, while its caster can reach it. `HOLD` and the field are gone (D34).
7. **A mage acted anywhere.** Now every body and reach instruction works only within reach: a caster feels, counts and
   pushes only the particles they can reach (D34).

**Shortcuts that physics should produce. Now it does:**

8. **Touch knew who owned what.** Now `TUCH` says whether something stopped or struck the particle when the world last
   moved, matter, the ground or a body, anyone's, and mana strikes its maker's body as it would anyone's (D35).
9. **Rock formed when the caster let go.** Now earth held by mana becomes rock wherever it's packed as full as solid
   ground and still, whoever's it is, in hand or not (D36).
10. **Fire rose by a fixed share of gravity.** Now each part of free mana has its own mass, everything weighs its mass
    times g, and the air holds up a parcel of mana by what the air it pushes aside weighs. Fire is the lightest, so it
    rises; the air weighs too, and settles thicker near the ground (D38).
11. **A push changed speed by at most 0.1 m/tick.** Now a push gets as much as the mind's power gives it (an order's
    power is its particle's mana's). A mind that pushes particles one at a time has to choose how much each gets: the
    library's `throw` gives each the same 0.1 m/tick a pass, so the ball doesn't tear (D32).
12. **A weave in hand was frozen.** Now it presses on what moves around it, and is pressed by it; the hand holds it
    still, and the body behind the hand feels what that takes (D37).

**Stand-ins, named:**

13. **Flames spread by a fixed share.** Now flame spreads as any gas does, from more to less, evening out with its
    neighbours (`flame_spread`). By heat, as the audit asked, it doesn't yet: heat is counted, not a field (left below).
14. **A pushed body lost 30% of its speed a tick.** Now the ground's friction slows it by μg, as its weight presses on
    it (D39).
15. **`DENS`, `GRAD` and `NVEL` read the simulation's sums.** *Kept, and why.* They are sums over the particle's
    neighbours within the smoothing length, which is the particle's own size: what's touching it. They're the pressure
    it's under, how that pressure differs across it, and how what's around it flows past: what a body can feel by touch,
    not knowledge from a distance. What's left is question 6, how finely it can feel.
16. **Slow loose mana joins the air grid** (`loose_rest`). *Kept, and why.* It's a change of representation, not physics:
    the same mana, as a share of the air instead of a particle, exact in mana, momentum and Energy (a parcel of mana is
    worth, in Energy, what the same mana is worth as air: §11, *Mana and Energy*).

**What's left.** Found while fixing the above, and not fixed yet:

- **Matter is softer than it is.** Sound crosses earth and water at 150 m/s (`matter_sound`), not the kilometres a
  second it does in rock: stepping real stiffness would take thirty times more steps. Rock bends a little under load, like
  very stiff rubber, and a column of it stands some metres before it buckles; at 50 m/s a 3.5 m column buckled (D42).
- **Thin matter doesn't touch mana.** Free mana strikes matter that fills its cell (D46), but passes through a cell
  that's less than a third full as if it were empty: a film of water doesn't turn the mana going through it (PLAN step 2
  asked for it).
- **What bodies do to the air comes from outside.** Gathering thins the air around the body, and pouring mana into a
  point presses it together. That Energy is booked as `bodies` in the ledger, not drawn from the mind. It's small: a few
  kJ a cast. With real air it's a real pressure wave too: gathering 100 M sends the air rushing back in at about 1.7 m/s
  (PLAN step 6).
- **Air seeps through the ground at once.** Matter holds no air of its own. Where matter moves and leaves room no nearby
  air can fill (a hollow it leaves sealed in the ground), air comes in from the world beyond as if through the soil's
  pores, at once, and air it squeezes out with nowhere to go seeps away the same way (D49). Real soil lets air through by
  how permeable it is (Darcy's law): sand in a second, clay far slower, rock hardly at all. *Organic:* air in matter's
  pores, flowing through them.
- **Mana doesn't grip gases.** Mana grips the matter of its own part (D43), and the open air is air matter now (D47), but
  a weave's air mana doesn't take hold of the air in its cells, nor fire mana of flame: only earth and water are gripped.
  Gripped, a weave of air mana would carry its air along with it.
- **The air is far thicker than real air.** How the air evens out its speed with its neighbours (`air_viscosity`, a
  fifth of the difference a tick) is some 25,000 times real air's viscosity at this size: a stand-in for the eddies
  smaller than a cell. It slows a rising parcel of warm gas, a jet and a wake.
- **Faint ripples aren't followed.** Air slower than 1.5 cm/s and within half a percent of rest's thickness counts as at
  rest, and is skipped (D47): a sound fainter than about 10 Pa goes no further. It's a change of how the air is kept, like
  matter that sleeps (D44), but it isn't exact: what a faint ripple carried stays where it was.
- **The hand's grip has no limit.** It holds any weave in it still, however hard the world pushes on it. *Organic:* a
  limit on what the body can hold.
- **Heat isn't anywhere.** It's counted, by how it came about, but it doesn't warm a place, and nothing feels it. So the
  air keeps one temperature (`temperature`, 20 °C), and sound crosses it at the isothermal 280 m/s, not the 343 m/s of
  air that warms as it's pressed (PLAN step 4).

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
| D19 | **The physics is as real as we can make it.** Spells get better by using the shortcuts reality gives, so the more real the world, the better the spells that can be written for it (§11). Every feature of a spell comes from it: this is the main objective (§0). |
| D20 | **Mana is chemistry, Energy is physics.** Mana is what things are. Energy is how things happen: force, motion, electricity. Each is conserved on its own, and mana moves Energy only indirectly. Fire mana is the substance of heat; the motion that heat is, is Energy. |
| D21 | **Free mana is a fluid of particles.** It has pressure, and spreads unless something holds it. Particles that share a velocity travel together. |
| D22 | **Pushing mana costs mana.** It is poured onto a particle, and the poured mana goes loose where it was poured. *(How much it costs: D29. Where its Energy comes from: D32. How hard it can push: the mind's power, D32; it used to be a fixed 0.1 m/tick.)* |
| D23 | **A construct is held by pushing it.** The caster keeps its particles in by pushing them back, as many and as often as their mind allows. Locking a shape is a loop in a library, not an instruction. *(What belongs to a weave: D34.)* |
| D24 | **Orders are reactions ingrained in mana.** Each particle carries its order. An order can spend its own particle's mana to push it. |
| D25 | **Orders don't spread to other mana**, except through a second flaw (§11, *The second flaw*). |
| D26 | **A weave in hand is held still.** Its mana counts toward the body's load because the body holds it: it stays where it was laid until `MANI` lets it go. *(How: D37.)* |
| D28 | **Things weigh.** A tick is 1/30 s, and things fall at 9.8 m/s². Matter held by mana weighs what that much matter weighs (a full cell of earth, 25 kg), and the mana holding it has to carry it. What rests on the ground is held up by the ground. |
| D29 | **A push costs the kinetic energy it adds** (D22, made exact), to what's pushed and to what it's pushed off (D33), at `push_energy` a M of mana poured. Speeding up costs more the faster it's already going. Slowing down costs nothing: what's taken out of the motion is heat. Holding something up against its weight costs nothing on the ground; in the air, it costs the downdraft it makes. Lifting it costs its weight times the height. |
| D30 | **Earth held by mana is rock.** Its particles are bound to their neighbours, and keep their shape. Rock cracks where it's bent too far or made to hold too much, and comes apart as its mana lets go of the earth. *(When it forms: D36.)* |
| D31 | **An order only feels.** It knows its own particle (its mana, its speed, whether it touches something), the mana around it (how dense, which way it thickens, how it moves), its weave's age, and what its caster wrote into the weave's registers. It isn't told where it is, where its weave's centre is, or where its maker is. What it has to know beyond that, its caster works out and writes in: once for the whole weave, or tick by tick while they keep it up. |
| D32 | **The Law of Transformation.** A mind can transform mana into Energy: that's what a push is. The mana isn't used up (mana has no spent state); it goes loose, still mana, and could be used again. What limits it is the mind. Each mind can transform only so much a tick (`power`, a mind stat, D15), and it strains for every joule; the strain eases as it rests (`recovery`), and past its mental capacity (`capacity`) transforming more harms it, lowering its condition: madness, counted. So in principle the same mana could drive one phenomenon after another without end, but holding a mind to it is beyond anyone, alone. An order is a mind too: its power is its particle's mana's (`order_power` a M), and its kicks pour its own mana. Closes question 5. |
| D33 | **Every push pushes something back.** A caster's push (`SHOV`) goes off their body, through their reach, and `SEND` throws mana out of it: the body is pushed back, across the ground, and their feet hold it as far as friction lets them. An order's kick goes off what's around its particle: the ground or other matter it's against on the side it pushes away from, or else the air it's in. The energy of the push (D29) counts both. |
| D34 | **A weave is its particles, and each is on its own.** Each particle keeps its own copy of its weave's registers and passes it on to those touching it, `relay` times a tick; the newest write wins. A caster writes into what they touch. A particle that lets go (`DISS`) or frays goes alone. A particle belongs to its weave while it carries the order, and, until it's given one, while its caster can reach it. Everything a caster's body does or senses is within their reach: they feel, count and push only the particles they can reach. `HOLD` and the field are gone; closes question 4. |
| D35 | **Touch is a force felt.** `TUCH` says whether something stopped or struck the particle when the world last moved: matter, the ground, a body, anyone's. Mana strikes its maker's body as it strikes anyone's; the casting hand is at arm's length (0.6 m) so that what's poured there doesn't. |
| D36 | **Rock forms where earth is packed and still.** Earth held by mana, in a cell as full of earth as solid ground, binds to its neighbours that hardly move against it. Whose mana it is doesn't matter, nor whether it's in hand. |
| D37 | **The hand holds what's in it by force.** A weave in hand presses on the mana and air around it, and they on it; the hand holds it still, bears its weight, and the body behind the hand feels the rest. |
| D38 | **Free mana is a gas, and the air holds it up.** Each part of free mana has its own mass (`mana_mass`, D40), and everything weighs its mass times g. The air weighs too, and at rest it's thicker low down. A parcel of mana pushes aside its own M's worth of air, and the air holds it up by what that weighs (Archimedes): fire, the lightest, rises; earth mana sinks a little; matter is what's heavy. Where there's no air, everything falls alike. |
| D39 | **Friction and flame.** A pushed body slides until the ground's friction stops it (μg, its weight pressing on the ground). Flame spreads as a gas does, from more to less, evening out with its neighbours. |
| D40 | **Mass is mana, by part.** Each part of mana weighs its own amount a M, free or condensed: fire 0.3 g, air 0.45 g, water 0.55 g, earth 0.7 g (the order is the author's; the numbers are tuning). Raw mana weighs half a gram a M, so a cell of air at 40 M is as heavy as real air. Matter is heavy because it's packed: a full cell of earth holds 35,700 M (25 kg, as dense as packed soil), and of water 28,400 M. Condensing and unmaking keep mass and momentum exactly. Matter takes room by its density, and a cell is full when its matter fills it. 1 M of free mana holds 5 kg of matter (`bind`; 20 kg since D43). Gases press by the ideal gas law, by part. |
| D41 | **The engine is Rust.** The machine, the world, the assembler and the tools are written in Rust: native for the tests, the bench and the CLI, and compiled to WebAssembly for the tester (and for Quire, later). The move changed nothing in the world: the same physics, numbers and order of operations, matched against the TypeScript engine to the last bit, tick by tick, before it was retired (PLAN step R). So the engine computes with JavaScript's numbers: V8's math (fdlibm), its rounding, and how it writes a number. |
| D42 | **Matter is one material, as points.** All matter, held or not, is material points over the world's grid (MLS-MPM, two points a cell each way, the grid's own cells): the ground, a wall, a puddle. Earth is an elastic-plastic solid (Hencky strain, Drucker–Prager friction at 35°, cohesion 8 kPa); pulled apart past its cohesion it cracks and loosens, crushed past 150 kPa it packs denser. How packed it is decides what it is: packing hardens it by e^(12·(1−J_p)), so loose soil, soil and rock (2,600 kg/m³) are one material. Water is a liquid that can't be squeezed and holds a little tension (200 Pa). Matter is stiffened by the speed of sound in it, a stand-in at 150 m/s (§0). Gases (flame, air matter) stay in cells, at rest, until PLAN step 3. |
| D43 | **Mana grips matter by force.** Free mana of a part pulls the matter of that part in its cell toward its own speed, and is pulled back equally, as hard as `bind` kilograms weigh at 9.81 m/s² a M of it, whatever the world's gravity: 1 M holds 20 kg up (the author's choice, raised from 5 so that a Stone Wall's mana can tear its earth out of the ground). Past that, the matter slips. Each particle grips the matter around it on its own, so mana spreading inside what it holds is held back too. A hand grips as if it had no end of mass, and the body behind it takes the pull. Matter moves first each tick, then the mana with what it holds (`cling`), so mana stays in what it grips. A push on a held weave moves only as much of its matter as the grip can in a tick. |
| D44 | **Matter at rest sleeps.** Points that have barely moved for half a second (15 ticks) sleep: they're not stepped, and they hold up what's on them as the ground does, taking its momentum. Anything that would move one faster than 2 mm a tick wakes it, as does mana in its cell. It's a way of not computing what doesn't change, exact in mass and momentum. |
| D45 | **Matter's Energy is counted where it goes.** What its stretching stores is Energy (`strain`). What it loses as heat: giving way (plastic work: the stress it gave way at, times how far), and striking (each step changes motion at once, and loses half the mass times the change squared, as an impact does; the motion lost moving it to the grid and back too). What a step gets wrong is left as the ledger's error: under 3% of what moves through a spell. |
| D46 | **Mana strikes matter as it moves.** Free mana moving into a cell full of matter faster than that matter moves strikes it: the two come to one speed along the way it was going, the matter takes the impulse (sleeping matter, as the ground), and what the impact loses is heat. Matter moving away as fast stops nothing: mana inside matter moves along with it, and mana at its face waits there. A fireball shoves the wall it hits, and a Stone Wall's mana rises inside its rising earth. Water holds together where mana holds it, not by its own surface tension, which is too weak to matter at this size (the author's answer: a spell keeps its water together). |
| D47 | **The air is real air.** The open air is air matter, a gas: condensed air mana as dense as real air (1.2 kg/m³), with free mana in it, 40 M a cell as before. It weighs both, the author's answer: the air is condensed air mana, and the mana in its space weighs too, so it's 2.5 kg/m³ at about 2 atm. Every gas presses by the ideal gas law, p = n·R·T, counting M: every M of any part is as many moles (`moles_per_m`, as many as 0.45 g of real air holds), at the world's one temperature (`temperature`, 20 °C) until heat has a place. So the lighter parts make a lighter gas at the same pressure, and rise. A cell's air (its gases and the free mana in them) moves as one, and sound crosses it at √(R·T/M), 280 m/s: `air_sound` is gone. Faces carry the air as sound meeting there would (a pressure difference drives air through), and the air steps finely enough not to overshoot. |
| D48 | **Past the world's edge there's more air, at rest.** The world is a window onto a bigger world (the author's answer): its sides and its sky are open, its floor isn't. Sound and wind go out through them, and still air, as the world's air is on average, comes in where it's drawn. What crosses, mana, momentum and Energy, is counted as gone beyond the world, so both ledgers still balance: `ManaLedger::beyond`, `EnergyNow::beyond`, `Impulses::beyond`. Mana blown off the edge is gone from the scene; a gather's hole fills back from beyond. |
| D49 | **Everything in a cell feels the air's pressure, by the room it takes.** Earth and water take their room, a body its own volume (its mass over `body_density`), and a parcel of free mana as much as as many M of the air there (Dalton). The air presses by its own M over the room left to it, and each thing in the cell is pushed by its share of what the cell's faces push: −V∇p. The push of the air at rest, its weight's worth, is buoyancy, for mana (its `lift`, replacing Archimedes in air at rest, D38) and matter alike; what's different from rest is a wind's pressure. Closed faces push the matter beyond them. As mana and matter move, the air gives way at once (sound is far faster): the air where they come in goes where they left, and what can't trade places nearby seeps through the ground to and from beyond (a stand-in, §0). Sleeping matter holds still against a push no harder than its friction (μ·m·g). |
| D50 | **The air drags what it flows past.** Bodies and matter in the air feel ½·ρ·C·A·u² (`drag_coefficient`, about 1 for a person), equally and oppositely, step by step within the air's own steps: a body by its box's face, a lump of matter by about the square of its size. A body stands, so the ground takes what's up and down, and its friction holds it before it moves: a wind it can hold against moves it not at all. Free mana drags by its own amount, by part (`air_drag`): air mana slips through the air most easily (the author's guess, C1). |
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
| Fire | heat, rises (free fire mana is the lightest), spreads thin into warmth | flame |
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
2. **Held matter is pulled along by its mana, as hard as the mana can** (D43). Pure free mana pulls the matter of its own
   part in its cell toward its own speed, and is pulled back as hard: 1 M holds up 20 kg (`bind`). Matter
   weighs what the mana it was made of did (D40): a full cell of earth is 25 kg, of water 15.6 kg. Earth mana that moves
   up pulls its earth up, and paying for the push is paying to lift it. Matter pulled harder than its mana can hold
   slips, and follows its own nature.
3. **Mana can condense** (*make*): an order can turn some of its particle's free mana into matter of the same parts (`CNDS`).
   Condensed matter is real. It stays when the weave is gone. Nothing natural frees it again: burning only changes what
   matter is mixed with fire, and a flame thins out into warmth that is still matter. Freeing it is possible, but only through a
   flaw in condensing (§5, *The flaw*).
4. **Free mana is a fluid of particles** (§11), a gas. It presses on itself and spreads, drags the air it moves through
   and is dragged by it (**wind**), and strikes the bodies it runs into, its maker's as much as anyone's. It weighs its
   mass, and takes room in the air, as much as as many M of the air: the air's pressure pushes it by that room, and at
   rest holds it up by what the air in that room weighs (D49): fire mana, the lightest, rises. Loose mana that has
   slowed to the speed of the air around it settles into it.
5. **Matter blocks matter, held or not.** All matter is one material on one grid (D42): what's in the way pushes back.
   Free mana strikes solid matter it runs into, and pushes it (D46), unless it's matter its own weave holds. What rests on the ground is held up by it,
   and its friction keeps it from sliding. Being stopped or struck by matter, or a body, is a **touch** (D35).
6. **Earth is as strong as it's packed** (D42). Earth bends under a load and springs back; pushed past its strength it
   takes the new shape and keeps it (molding); pulled apart past its cohesion it cracks. Loose earth slides and piles at
   its angle of repose. Packed denser it's stronger, up to rock. Mana doesn't make rock; it can pack earth into it. Water
   flows, can't be squeezed, and holds together a little.
7. **Holding costs.** A weave doesn't leak by itself any more. What holds it together is its caster's pushes or its own
   order, and an order burns its mana as it thinks (D27). Less mana grips less hard, so a Stone Wall's mana lets go of
   its earth as it burns.
8. **The air is real air** (D47): air matter, with free mana in it, a gas that presses by the ideal gas law and weighs
   both. Past the world's edges there's more of it, at rest (D48). Everything in a cell feels its pressure by the room it
   takes, and what it flows past feels its drag (D49, D50): a gale blows a person back.

Every rule has numbers to tune: how much matter 1 M binds, how hard each part presses, what a push costs, how much an order
burns, and so on. They live in one table (`engine/mana/src/vm/physics.rs`).

**Earth holds together** by its own cohesion: a ledge of rock holds where one of soil cracks off.

---

## 4. The caster

The machine is a person, and its limits are their stats. Every stat, body or mind, is made of three things:

```
stat = genetics × condition + training
```

- **Genetics** is what they were born with: a people, a bloodline, a gift.
- **Condition** is how they are right now, from 0 to 1. Tired, hurt or drunk lowers it. Overcharge harm lowers the body's,
  and strain past the mind's capacity lowers the mind's: that's where harm ends up.
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
| `reach` | How far from the body, in metres, it can do or sense anything: gather, pour, push, probe, ingrain, write a weave's registers, feel its particles (D34). An adept reaches 4 m. It's the body's own: a push through it pushes the body back (D33). |

### The mind

| Stat | Meaning |
|---|---|
| `speed` | **Beats** of thought per tick. |
| `registers` | How many number registers the mind has (`n0`… up to `n31`). A child might think with 8. |
| `memory` | How many numbers the mind's memory holds (up to 256), and how deep its stack goes. |
| `conditioning` | Per spell: how many times the caster has cast it. |
| `power` | The most Energy it can transform out of mana in one tick (D32): how hard it can push, in kg·(m/tick)², 900 J each. An adept's is 0.5 (450 J a tick). |
| `capacity` | Its mental capacity: how much strain it bears. Every joule it transforms strains it; past its capacity, transforming more harms it (madness), and the tester counts how much. An adept's is 60 (54 kJ). |
| `recovery` | How much strain eases each tick. An adept's is 0.02 (18 J a tick: a full capacity in about 90 s). |

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
4. **Overcharge:** if `flow + held mana + weaves still in hand > capacity`, the excess is counted as harm. **Strain** eases
   by `recovery`; past the mind's `capacity`, the excess is counted as madness.
5. **Weaves hold:** every weave takes hold of the matter its free mana can bind (and lets go of what it no longer can).
6. **Orders run:** each particle passes its copy of its weave's registers on to those touching it, then every ingrained
   particle of a weave set loose runs its order, and pays for it (§5).
7. **The world moves:** the mana (weight, buoyancy, pressure, holding together, rock, the air, the ground, what it runs
   into), particles at rest together merging and thin ones splitting, particles with no order out of their caster's
   reach leaving their weave, loose mana settling, pushed bodies sliding, rock forming where earth is packed and still,
   falling and flowing matter, the air flowing. A weave in hand is held still by the hand, which bears it (D37).

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
  Each particle is told which way its frame faces when it's poured, ingrained or touched by a `TURN`: its order kicks and
  feels in it.
- **particles**: its mana, each particle with the matter it binds (§11). A particle belongs to the weave while it carries
  its order, and, until it's given one, while its caster can reach it (D34). The caster feels only those within reach:
  `PCNT` counts them, and `PPOS`/`PVEL` sense one, by its number among them;
- **registers** `w0`–`w7`, numbers its order can read and write (a velocity, a phase). Each particle keeps its own copy:
  its order reads and writes that copy, and passes it on to the particles touching it, `relay` times a tick, the newest
  write winning. A caster writes into what they touch (`WSET`), and the mana they pour carries what they've written;
- an **order**: a routine (`ORDR`) that each particle it's ingrained into (`INGR`) runs, every tick, once the weave is set
  loose.

A weave is **in hand** from `WEAV` until `MANI`: its mana counts toward the body's load, and the hand holds it where it
was laid, bearing its weight and what presses on it (D26, D37). `MANI` sets it loose. From then on its mana moves. `LOCK`
can fix its input (no more mana goes into it) or its order (it can't be given another one). A shape isn't locked: it's
held, by the caster's pushes (`SHOV`) or by its own order (`KICK`). A weave whose mana has all gone can still be named by
its caster: there's nothing in it.

### The order: mana running code

ORDER, in the old Core, was *give an order to the mana particles*. Here it is literal. A weave's order is an assembly routine
that **each particle it's ingrained into runs every tick**, like a tiny mind inside the mana. A particle thinks with
`n0`–`n15` of its own and has no mana registers. When it starts, its registers hold:

| Register | |
|---|---|
| `n0:2` | 0. *(They used to hold where the particle is from its weave's centre: D31.)* |
| `n3` | How much free mana it holds. |
| `n4` | The weave's age, in ticks since it was set loose. |

An order can do arithmetic and jumps, read its copy of its weave's registers and the ports below, and use the **order**
instructions (`KICK`, `TUCH`, `GETW`, `PUTW`, `DISS`, `CNDS`, `DENS`, `GRAD`, `NVEL`). It ends with `RET`. Every beat it
thinks burns some of its particle's mana into the air (`order_burn`). An order that thinks more than 64 beats in one tick
**frays**: that particle goes loose, and forgets its order. The rest of its weave goes on. An order is a mind (D32): it
transforms its particle's own mana into the Energy of its kicks, at most `order_power` for each M it holds, a tick.

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
| `40` | `PROB d, n:3, s` | PROBE | How much matter of part `s` is in the cell at `n:3`. 0 out of reach. |
| `41` | `AIRM d, n:3, s` | PROBE | How much air mana of part `s` floats at `n:3`. 0 out of reach. |
| `42` | `SEND m, n, n:3, n:3` | SEND | Throw `n` M of `m` out at a position within reach, with a velocity, loose. The body is pushed back by it (D33). The mind transforms a share of what's thrown into the Energy the throw takes, no more than its power has left (D32): that share goes loose into the air there. |
| `43` | `WPOS n:3, w` | | The middle of what the caster feels of the weave (its particles within reach), in the world. 0 if they feel none. |
| `44` | `WVEL n:3, w` | | How what the caster feels of the weave moves on average, in its frame. |

#### Weave

| Op | Mnemonic | Lore | Does |
|---|---|---|---|
| `50` | `WEAV d, n:3` | | Begin a weave with its origin at `n:3`. Its id goes into `d`. In hand. |
| `51` | `TURN w, n:3` | POSITION | Turn the weave's frame so forward points along `n:3` (about the vertical). |
| `52` | `EMIT m, n, w, n:3` | | Pour `n` M of `m` into the weave as particles, at `n:3` in the weave's frame (from its origin in hand, from the middle of what's felt of it once loose), within the cell there. They carry what's been written into the weave. Gives what there is if `m` holds less, and nothing out of reach or outside the world. |
| `53` | `WSET w, #k, s` / `WGET d, w, #k` (`54`) | | Write register `k` into what the caster touches of the weave (the rest hears of it by touch); read the newest copy among what they touch, or what they wrote last. |
| `55` | `ORDR w, L` | ORDER | Give the weave its order: the routine at `L`. |
| `56` | `MANI w` | SEND | Set the weave loose. It leaves the body's load, its mana is free to move, and its ingrained particles run their order. |
| `57` | `LOCK w, INPUT \| ORDER` | LOCK | Lock it. Only after `MANI`. |
| `58` | `RELS w` | | Let go of what the caster touches of the weave: it goes loose where it is, and forgets its order. What's out of reach and ordered keeps going. |
| `59` | `PCNT d, w` | | How many of the weave's particles the caster feels: those within reach, counted 0, 1, 2… in the order they were laid. |
| `5A` | `PPOS n:3, w, s` | PROBE | Where felt particle `s` is, from the middle of what's felt of the weave, in its frame. |
| `5B` | `PVEL n:3, w, s` | PROBE | How felt particle `s` moves, in the weave's frame. |
| `5C` | `SHOV m, w, n, n:3` | PUSH | Push felt particle `n` off the body (D33): change its velocity by `n:3`. It costs the kinetic energy it adds to the particle and the body, at `push_energy` for each M, from `m`, poured into the air there (D29), and gets as much as the mind's power has left this tick (D32). Slowing a particle costs nothing. |
| `5D` | `INGR w, s` | ORDER | Ingrain the weave's order into felt particle `s`, and tell it which way the weave faces. 4 beats, and one more for every instruction the order could run. |

#### Order (only inside an order)

| Op | Mnemonic | Does |
|---|---|---|
| `60` | `KICK n:3` | Push this particle off what's around it (D33): the ground or other solid matter it's against on the side it pushes away from, or else the air it's in. Change its velocity by `n:3`, in its weave's frame, paid from its own mana by the kinetic energy it adds to both (D29), up to its power (D32). 4 beats. |
| `61` | `TUCH d` | `d = 1` if something stopped or struck this particle when the world last moved: matter, the ground, a body, anyone's (D35). |
| `62` | `GETW d, #k` / `PUTW #k, s` (`63`) | Read and write this particle's copy of its weave's registers. A write counts from the next tick, and passes on by touch. |
| `64` | `DISS` | This particle lets go: it goes loose where it is, and forgets its order. |
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
| `FRAYED` | An order ran too long in one tick. That particle goes loose. |
| `NOT_YOURS` | A weave id that isn't one of this caster's. |
| `NO_ORDER` | `INGR` into a weave that hasn't been given an order. |
| `ORDER_ONLY` | An order's instruction (`KICK`, `TUCH`…) in a mind. |
| `NOT_IN_ORDER` | A body, reach or weave instruction inside an order. That particle frays. |
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
; caster's reach. Each pass works out once what the weave as a whole still lacks, at most 0.1 m/tick of it, and gives
; every particle that same push, one after another, as fast as the mind can: a ball whose first particles were thrown
; far ahead of its last would tear apart. Each push pays for itself from m0.
;   in: n0:2 the velocity (in the weave's frame), n4 weave
throw:  IN    n15, REACH
        MUL   n15, n15            ; reach², to compare without a square root
        MOV   n14, n0
        MUL   n14, n0
        MOV   n12, n1
        MUL   n12, n1
        ADD   n14, n12
        MOV   n12, n2
        MUL   n12, n2
        ADD   n14, n12
        MUL   n14, #0.0025        ; (5% of the speed)²
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
        MOV   n11, n0
        SUB   n11, n8
        MOV   n12, n1
        SUB   n12, n9
        MOV   n13, n2
        SUB   n13, n10            ; what it lacks
        MOV   n8, n11
        MUL   n8, n11
        MOV   n9, n12
        MUL   n9, n12
        ADD   n8, n9
        MOV   n9, n13
        MUL   n9, n13
        ADD   n8, n9              ; squared
        CMP   n8, n14
        JLE   .done               ; fast enough
        CMP   n8, #0.01
        JLE   .all                ; no more than 0.1 m/tick: all of it
        SQRT  n8
        LDI   n9, #0.1
        DIV   n9, n8
        MUL   n11, n9
        MUL   n12, n9
        MUL   n13, n9             ; 0.1 m/tick of it
.all:   PCNT  n5, n4
        CMP   n5, #0
        JEQ   .done               ; nothing left of it to push
        LDI   n6, #0
.p:     SHOV  m0, n4, n6, n11:13
        ADD   n6, #1
        CMP   n6, n5
        JLT   .p
        JMP   .pass
.done:  RET

; heave: one pass of moving a weave by hand. It works out what the weave as a whole still lacks of the velocity n0:2 (in
; its frame), at most n14 m/tick of it across and n15 up or down, and gives every particle it feels that same push, off
; the caster's body. Rock passes a push on through itself, so a pass that takes a few ticks still moves it as one. Across
; is gentle: the body is pushed back as hard as it pushes, and its feet hold only so much. Each push pays from m0.
;   in: n0:2 the velocity, n4 weave, n14 the most across a pass, n15 the most up or down
heave:  CIRC  m0                  ; keep holding what it pays with
        WVEL  n8:10, n4
        MOV   n11, n0
        SUB   n11, n8
        MOV   n12, n1
        SUB   n12, n9
        MOV   n13, n2
        SUB   n13, n10            ; what it lacks
        MOV   n8, n11
        MUL   n8, n11
        MOV   n9, n13
        MUL   n9, n13
        ADD   n8, n9              ; across, squared
        MOV   n9, n14
        MUL   n9, n14
        CMP   n8, n9
        JLE   .up                 ; no more than the most: all of it
        SQRT  n8
        MOV   n9, n14
        DIV   n9, n8
        MUL   n11, n9
        MUL   n13, n9             ; the most across, of it
.up:    MIN   n12, n15
        MOV   n9, n15
        NEG   n9
        MAX   n12, n9             ; and up or down
        PCNT  n5, n4
        LDI   n6, #0
.p:     CMP   n6, n5
        JGE   .done
        SHOV  m0, n4, n6, n11:13
        ADD   n6, #1
        JMP   .p
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

`throw` works out once a pass what the weave as a whole still lacks, and gives every particle it feels the same push, at
most 0.1 m/tick of it, so it costs about 16 beats a particle a pass. A mind pushes one particle at a time: a ball whose
first particles were thrown at full speed while its last waited would tear apart, so it goes in steps. An adept gets
round a 45-particle fireball about every two ticks. Each push goes off the caster's body, which is pushed back (D33).

### Shapes

A shape lays the mana in `m1` out in the weave, around its origin. It's plain geometry. A big shape takes a mind many
ticks to lay out, longer than a hold lasts, so each shape re-`CIRC`s its mana as it goes. Without that, the mana slips into
the body's flow halfway through, and half a wall is laid out with nothing. The weave is in hand while it's laid out, so
what's laid stays where it's laid (D26), and only what's within reach can be laid at all.

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
; wall: a block of the ground under the weave's origin, laid out in hand. In hand it binds its earth and, packed and
; still, is rock. Raising it is up to its caster: it weighs what the ground does.
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
        RET
```

The wall isn't made of the spell's mana. It's the ground, lifted: the earth mana *influences* the earth around each particle,
binding it and carrying it up, and the ground it came from is left as a trench. A caster with poor earth affinity binds less
earth, and the wall rises full of holes. It's raised by hand (`heave`, in Basics): 400 kg of earth, lifted 2 m, is about
8 kJ, which the caster's mind transforms out of the residue in `m0` (about 9 M), and moving it back pulls the caster along
as hard as they pull it, so it's moved gently. Once it stands on the ground, its order holds it as it stands, which costs
it, and the wall slowly crumbles as its order burns its mana away, and its rock comes apart as its mana lets go of the
earth.

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
order that works something out once (a sine and a cosine) keeps it in its registers, and the particles touching it hear
of it. Every kick pushes off the air the particle is in (D33).

```
; heading: flies along one angle, w1 (up from forward, in radians), speeding up 0.04 m/tick every tick for its first 8
; ticks. It knows nothing else: not how fast it's going, not where the rest of its weave is. The sine and cosine are
; worked out once and kept in w5 and w6, and pass on to the particles touching it: a weave can remember for its mana.
heading: CMP  n4, #8
        JGE   .done
        CALL  angle
        MUL   n7, #0.04
        MUL   n8, #0.04
        LDI   n6, #0
        KICK  n6:8
.done:  RET

; angle: the sine and cosine of w1, into n7 and n8. Worked out once and kept in w5 and w6 (a cosine of 0 means not yet),
; where the particles touching it hear of it; or given there by the caster, who can work it out once for the whole weave.
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
        FILT  m1, m0, #EARTH      ; FILTER: earth × affinity; the residue stays in m0, to push with
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
        CALL  wall                ; the ground laid out in hand: it takes hold of its earth, and is rock
        MANI  n20                 ; SEND: let go of it, and hold it up by pushing
        WPOS  n21:23, n20
        MOV   n24, n22
        ADD   n24, #2.05          ; how high its middle is to go: out of the ground, and a little more
        IN    n25:27, SELF            ; where the caster stands as it begins
        SUB   n21, n25
        SUB   n23, n27
        MUL   n21, n21
        MUL   n23, n23
        ADD   n21, n23
        SQRT  n21
        SUB   n21, #0.5           ; how far from the caster it's to stand: its thickness nearer, in front of its trench
.climb: LDI   n0, #0              ; RAISE: up, at 0.1 m a tick
        LDI   n1, #0.1
        LDI   n2, #0
        MOV   n4, n20
        LDI   n14, #0.001
        LDI   n15, #0.15
        CALL  heave
        WPOS  n8:10, n20
        CMP   n9, n24
        JLT   .climb
.step:  CALL  .level               ; POSITION: back toward the caster, slowly: it pulls them as hard as they pull it
        LDI   n0, #0
        LDI   n2, #-0.01
        MOV   n4, n20
        LDI   n14, #0.001
        LDI   n15, #0.15
        CALL  heave
        WPOS  n8:10, n20
        SUB   n8, n25
        SUB   n10, n27
        MUL   n8, n8
        MUL   n10, n10
        ADD   n8, n10
        SQRT  n8
        CMP   n8, n21
        JGT   .step
        LDI   n28, #20            ; passes to come to a stop above its place
.stop:  CALL  .level
        LDI   n0, #0
        LDI   n2, #0
        MOV   n4, n20
        LDI   n14, #0.001
        LDI   n15, #0.15
        CALL  heave
        SUB   n28, #1
        CMP   n28, #0
        JGT   .stop
        ORDR  n20, .stand
        MOV   n4, n20
        CALL  ingrain             ; ORDER: it holds itself as it stands, and stays this weave's once its caster goes
        LDI   n28, #40            ; passes to set it down: its middle lowered 4 mm a pass, onto the ground
.down:  SUB   n24, #0.004
        CALL  .level
        LDI   n0, #0
        LDI   n2, #0
        MOV   n4, n20
        LDI   n14, #0.001
        LDI   n15, #0.15
        CALL  heave
        SUB   n28, #1
        CMP   n28, #0
        JGT   .down
        MOV   n0, n20             ; let go: the ground holds it up from then on
        HALT                      ; m0 isn't held any more: it joins the flow
.fail:  FAIL  #1                  ; no earth there

.level: WPOS  n8:10, n20          ; how fast to rise or sink to keep its middle at the height it climbed to, n24
        MOV   n1, n24
        SUB   n1, n9
        MUL   n1, #0.3
        RET

.stand: IN    n5:7, VEL           ; LOCK: it keeps itself as it stands, kicking against whatever moves it across. Its
        NEG   n5                  ; base is rough, so it rests on a few points, and would tip over with nothing holding
        LDI   n6, #0              ; it. Up and down, the ground holds it.
        NEG   n7
        KICK  n5:7
        RET
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

The wall used to climb out of the ground by its own order, kicking off the air in its cells. That only worked while the
air was as heavy as rock. Real air (D40) can't hold up 400 kg of earth, and the way you'd raise a wall is the way a mage
does: by hand. In hand, the ground laid out takes hold of its earth and becomes rock. Let go, the caster pushes it up,
the whole wall alike, pass after pass, holding it at the height it climbed to; moves it back by its own thickness, gently,
since it pulls them as hard as they pull it; ingrains its order while it's still held; and sets it down on the solid
ground in front of its trench. The trench is a ditch in front of it.

Its base is as rough as its particles were poured, so it rests on a few points, and would tip over: its order holds it,
kicking against whatever moves it across, and it leans a little as it settles. That costs its mana, and the wall holds
half its earth for about 8 s. Real ground would bed it in: PLAN step 2. Raising 400 kg by hand is a master's spell.

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
.hit:   PUTW  #0, n5              ; it bursts from the next tick, and the word passes by touch
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

Once it's out of reach, nothing but its order holds it. In the test world an adept's fireball flies 8 m to the pillar in
0.73 s with 98% of its mana within a metre of its middle, and bursts against it: the particles that touch it first pass
the word to the rest by touch. It weighs 5 g, lighter than the air it pushes aside, so it rises 1.2 m on the way.
Throwing it costs its caster's mind a fraction of a joule.

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
long Gust overcharges. It needs only two streams. Its speed is transformed out of a share of what it sends (D32). A breath
of it is a few grams of mana: it can't blow anyone over by striking them, and the wind it drives is a breeze. Bodies feel
the air (D50), but at its target the Gust's air moves 6 cm/s, where it takes a gale, some 40 m/s, to slide a person
whose feet are on the ground (§11, *How the spells do now*).

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

This shield *makes* its water instead of finding it. The water is real, and held up by the mana left free, pushing off the
air. Every tick its order burns a little of that mana, and every step its caster takes costs it the push to follow, so it
holds less and less water. It follows only while its caster keeps it up, telling it each tick how to move (they write it
into what they touch of it, and the rest hears by touch); let be, it holds still where it is. Its caster can walk about
in it, but not into it: its water strikes them as it strikes anyone. When its mana is gone, the water falls in a splash at
the caster's feet. A shield cast beside a river could *influence* the river's water instead, and keep all of its mana free
to hold it.

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
  - *Body*: load against capacity, flow, drain, focus, harm, the mind's strain against its capacity and any madness,
    condition, and `m0`–`m7` as four coloured parts, each with how long it's still held or that it's slipping.
  - *Weaves*: each weave's state, locks, order, its particles and how many are ingrained, mana, the matter it holds, and
    `w0`–`w7`: the newest copy among its particles.
  - *Profile*: beats by routine and the costliest lines (D18).
  - *Events* and the ledger.
  - *Caster*: child, adept or master; every stat as genetics and training, with what it comes to now; condition; how
    many times they've cast this spell before; and the will (amount, force, maintain), which a running spell reads live.
  - *Bytes*: the assembled program, with where the mind is.
  - *Reference*: every instruction and port.

- *Order*: any particle of any weave, chosen by number or by clicking it in the world, and its order from the last tick,
  instruction by instruction: step forward and back, its 16 registers at each step, its copy of the weave's registers, how it kicked,
  the mana its thinking burned, and its beats against the 64 it has. A breakpoint in order code (a weave's `.order`, or a
  library routine it calls) pauses the run after the tick a particle hit it, on that particle and that instruction. The
  machine records this only when asked (`Sim::trace_orders`), since every particle of every weave is recorded every tick.

---

## 10. How it's built

The engine is Rust (D41): the machine, the world, the assembler and the tools, in a Cargo workspace. It runs natively
for the tests, the bench and the terminal, and compiled to WebAssembly for the spell tester (and for Quire, later: it
runs wherever JavaScript does). It was ported from TypeScript and matched it to the last bit, so it computes with
JavaScript's numbers: V8's math, its rounding, and how it writes a number (`js/`).

```
engine/
  mana/            the engine
    src/asm/       isa.rs (the instruction table), assembler.rs, disassembler.rs, code.rs (a program, decoded once),
                   docs.rs (a line on each instruction and port)
    src/vm/        physics.rs (the numbers), parts.rs, world.rs, caster.rs, weave.rs, sim.rs (the machine and the tick),
                   fluid.rs (mana's particles), matter.rs (matter's points), air.rs (the air),
                   energy.rs (the Energy ledger)
    src/js/        JavaScript's numbers: V8's math (fdlibm), Math.round, Math.hypot, toFixed, Number::toString
    src/scenes.rs  test worlds for the four spells and the bench, in 2D and 3D
    src/bench.rs   the bench: every way of holding and throwing, measured
    src/render.rs  a slice of the world as text
    src/profile.rs where a cast's beats went
    tests/         the assembler, the machine, the physics, the Energy ledger, the four spells, the bench, V8's math
  mana-cli/        mas, mvm and bench
  mana-wasm/       the engine as the tester sees it
tester/            the spell tester: Svelte 5, Vite, CodeMirror 6; src/lib/engine.ts is how it talks to the engine
lib/               Elements, Basics, Shapes, Reactions, Transformations, Orders, Holding, in .masm
spells/            StoneWall, Fireball, Gust, WaterShield, in .masm
bench/             the bench's spells
scripts/           listings.ts: SPEC's code listings, from the .masm files
```

```
npm install
npm test                                       the tests
npm run check                                  clippy, tsc and svelte-check
npm run tester                                 the spell tester, at http://localhost:5175
npm run mas -- spells/Fireball.masm            the listing: addresses, bytes, instructions
npm run mvm -- spells/StoneWall.masm           cast it in its test world, in the terminal
npm run mvm -- spells/Gust.masm --ticks 40 --maintain 30
npm run bench                                  every way of holding and throwing, side by side
npm run listings                               SPEC's code listings, brought up to date
```

It needs Rust with its WebAssembly target, wasm-bindgen and wasm-pack.

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
   - **The main objective** (§0): the audit's sixteen places, made organic (D32–D39). *Built.* What it turned up is the
     next work (§0, *What's left*).
   - **Mass is mana** (D40). *Built.*
   - **The engine in Rust** (D41). *Built:* the same machine, to the last bit, two to five times faster on one core.
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
hold of Energy directly. A mind can: it transforms mana into Energy, which is what a push is (the Law of Transformation,
D32). The mana isn't used up; it goes loose, still mana. What the Energy costs is the mind's strain. Fire mana is the
substance of heat: what burns, what is hot. The heat itself, the motion in it, is Energy.

The machine keeps two more ledgers that physics needs.

**Momentum.** Everything inside the world pushes on everything else equally and oppositely (particle and particle, particle
and air, air and air, particle and body), so the world's momentum changes only by what comes from outside it, and the tests
check that every tick (`World::momentum_error`).

**Energy** (`engine/mana/src/vm/energy.rs`, `Sim::keep_energy`). The world holds Energy as motion (of particles, the air,
bodies and matter), as height (weight lifted, of all of it), and stored: in mana's gas pressed together, in matter
stretched, in what coheres pulled apart, in the air pressed or drawn thin, and in mana held in bodies. The air's is
R·T·(N·ln(n/n̄) + n̄·V − N) for each cell holding N M in the room V its air has, against how thick the air is on average:
so gas let into the air or drawn from it as thick as it is brings nothing with it, and the air at rest, thicker below
than above, is what holds things up. A parcel of mana rising takes the room of air that comes down; that's its buoyancy,
and the ledger counts it in the air's height and in how thick the air is where each is. Motion becomes **heat** wherever
two things even out their speeds: in the mana's thickness, the air's drag and its own thickness, landing on the ground
and sliding on it, a body's feet, a hand holding still what the world pushes on, matter giving way and striking,
particles merging, mana or air let into air moving otherwise, a push that slows. That heat is counted where it happens, to
the joule, and kept by how it was made (`World::heat`). Minds put Energy in with every push and kick, counted push by
push (`minds`); bodies change it by gathering mana and pouring it (`bodies`). What the air carries past the world's open
edges, or seeps in and out through the ground, is counted as gone beyond (`beyond`, D48). So, every tick:

```
held now + heat + gone beyond  =  held at the start + what minds and bodies put in + what the numbers got wrong
```

The last term is real, and is counted, not hidden in the heat: each step that should keep Energy is measured before and
after, and what it got wrong is kept by step. It's the price of stepping time rather than flowing it. For a Fireball, a
Gust or a Water Shield it's under 0.4% of the Energy that moves through; for a Stone Wall, 1.8%, most of it in the matter
step. With real air, `bodies` is most of what the small spells put in: pouring a fireball's 18 M into one point presses
the air there, and the air rushing out carries about 400 J (§0). A Stone Wall, gathering 500 M at once, takes 30 kJ out.

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

**The air** is a grid of cells (`engine/mana/src/vm/air.rs`, D47). Each holds air matter (condensed air mana, as dense as
real air) and the free mana in it, and moves as one. A particle that slows down and belongs to no weave settles into the
air and loses its order; `GATH` draws free mana from it. A particle moving through the air drags on it and is dragged by
it, both ways, each part by its own amount (`air_drag`). That drag is wind.

The air is a real gas. It presses by the ideal gas law, every M alike (`moles_per_m`, at `temperature`), so a cell's air
presses by its M over the room it has, and sound crosses it at 280 m/s. It carries itself along, and its gases, its mana
and its momentum with it, from cell to cell, stepping finely enough for sound to cross a fraction of a cell a step. At
each face the air moves as sound meeting there would have it: a difference in pressure drives it through. Solid cells
are closed: it flows around them, and what it pushes on them goes to the matter there. The world's sides and sky are
open, onto more air at rest (D48): what reaches them goes on, as a wave going out does, and still air comes in where it's
drawn. Its thickness (`air_viscosity`, a stand-in, §0) evens out its speed between neighbours and holds it still against
the ground. It weighs: at rest it's thicker low down than high up (by e every 8 km), its weight and its pressure in
balance, and that balance is kept exactly: only what differs from rest pushes or falls. So a gust travels on as a jet once
it's let go, a fireball pushes air ahead of it, wind turns up and over a pillar, and the hole a caster gathers from fills
back in, at once, with a pressure wave that leaves the world in a few ticks. Air that has all but stopped stops, and air
at rest, as thick as rest would have it, costs nothing to run.

Everything in a cell feels the air by the room it takes (D49): earth and water by theirs, a body by its own volume, a
parcel of free mana by as much as as many M of the air there. The push of the air at rest, its weight's worth, holds them
up by what the air in their room weighs: free mana of each part has its own mass (`mana_mass`, D40), and the air's M weigh
0.47 g each on average, so fire mana (0.3 g) rises at about 0.6 g, air mana (0.45 g) floats up slowly, water mana (0.55 g)
sinks at about 0.14 g, and earth mana (0.7 g) at a third of g. Matter feels it too, by a thousandth of its weight. Where
there's no air, everything falls alike. What's different from rest, a wind's pressure, pushes the same way. As mana and
matter move, the air gives way: what's where they come in goes where they left. And what the air flows past, it drags
(D50): a person in a 45 m/s gale is pushed with 560 N, and slides once that's more than their feet hold.

When particles come to rest beside each other, they merge, to keep their number down: closer than `merge_range`, moving
within `merge_speed` of each other, and together no more than `max_mote`. The new particle sits at their centre of mass with
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

Each part has its own numbers, in `physics.rs`, by part number. The machine still knows no element names (D2).

| Part | Pressure | In the air, free | Holds together | So |
|---|---|---|---|---|
| 0 (fire) | high | the lightest: rises | barely | spreads fast and rises. Easy to pour, hard to hold. |
| 1 (water) | low | a little heavier: sinks slowly | some | its matter flows down. A shell of it sags unless held up. |
| 2 (air) | high | as heavy as the air: floats | no | fills what's empty. The easiest to pour. |
| 3 (earth) | very low | the heaviest: sinks slowly | strongly | barely spreads. It has to be laid out by hand. |

*Built:* each part's pressure, thickness, mass (`mana_mass`; the air holds it up, D38) and how it holds together
(`cohesion`, a pull between neighbours, strongest at half the smoothing length). Matter held by mana adds its weight and
its mass, as much as the mana it was made of (D40), can't be packed past full (`density`, `matter_stiffness`), and pulls on
its neighbours as water does
(`matter_cohesion`). Earth held by mana becomes rock where it's packed as full as solid ground and still (D30, D36): each
particle is bound to its neighbours within `bond_range`, and the bonds hold their length, a dozen passes a step, so that
the ground's support reaches up through a wall. The ground holds up what rests on it, with friction.

An adept's fireball is 72 particles of a quarter of an M each (`mote`).


### Pushing

A push pours mana onto a particle and changes its velocity. A mind transforms the poured mana into the particle's motion
(D32): a push costs the **kinetic energy it adds**, to the particle and to what it's pushed off (D33), at `push_energy` for
each M (D29). One M is 900 J. It gets as much as the mind's power has left this tick: an adept's 450 J a tick, an order's
`order_power` for each M its particle holds. Every push pushes something back: a caster's goes off their body, through
their reach, and the ground under their feet takes what friction holds; an order's goes off the ground behind its particle,
or the air it's in. So:

- Speeding something up from rest costs ½mv². Speeding it up further costs more for the same change, the faster it
  already goes: the same 0.05 m/tick costs a 0.3 m/tick fireball thirteen times what it costs one at rest.
- Slowing something down costs nothing. What's taken out of its motion becomes heat.
- Holding something up against its weight costs nothing on the ground: each tick the push only takes back the speed it
  gained falling. Held up in the air, it pushes the air down, a little more each tick, and pays for that downdraft.
  Lifting it costs its weight times the height: lifting a 2 m Stone Wall of 400 kg of earth is about 8 kJ, 9 M poured.
- Pushing sideways across a motion costs only what the sideways speed adds.

The poured mana goes **loose where it was poured**, still mana: nothing is used up. A construct pushed for a long time sits
in a haze of the mana it was pushed with, and that mana could be gathered and used again. What runs out is the mind.

So a fireball isn't thrown in one instruction. The caster pushes it along the aim tick after tick, holding it together
while it speeds up. A heavier ball takes longer to get going.

### Holding

A weave is the particles that carry its order, and, until they're given one, those its caster can reach (D34). The caster
feels and pushes only what's within their reach. A particle with no order that strays out of it leaves the weave.

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

A construct the caster lets go of, or throws past their reach, with no order in it, is held by nothing. Its particles share one velocity, so in
its own frame the ball stands still, and only its own pressure pulls it apart. It holds together for about its radius over
how fast it spreads, and travels as far as its speed carries it in that time. Faster goes further, and costs more to throw.
Denser hits harder, and comes apart sooner. The air strips its front as it flies.

When it hits something, its front stops and its back keeps coming: it piles up, packs denser, and splashes out. A burst on
impact needs no code.

### Orders

An order is a reaction or a phenomenon ingrained in mana: each particle carries it and runs it every tick, within its
beats (§5). An order can sense the particle and its neighbourhood, touch, condense, and **push its own particle off the
air or the ground around it, paying with that particle's own mana** (D32, D33). So:

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
- *Pushing* is paid from the particle, and only as hard as its mana lets its order push.

So a well-written order is short, and quick to say "nothing to do": D18 again, inside the mana.

### The second flaw

Orders don't spread. A particle that settles into the air leaves its order behind, and mana that was never ordered is
never given one. Except for one case.

When two particles merge, the new particle keeps the order of the bigger one. That rule was written to keep the number of
particles down, and nobody asked whose particles they were. A big ordered particle that comes to rest against someone
else's mana takes it over: loose mana, the mana of another weave, another caster's fireball. With enough mana packed into
one place, an order spreads through whatever it merges with, like a chemical reaction running through a substance.

It's held back by the same rule that made it: two particles merge only if together they're no more than `max_mote`, so an
ordered particle of 0.75 M can take in one mote and no more, until it splits. A particle of only one mote, ordered, ties with
the mote beside it, and the older one wins. Every takeover is logged as `taken`: which weave took in whose mana, and whether
it gave it its order.

Like the first flaw, it isn't an instruction, nothing in the libraries uses it, and the tester only shows what it does.

### Built

1. A 2D sandbox, outside the machine: a ball of particles with pressure, and a caster with beats and mana pushing it by each
   strategy in *Holding*. The bench (below) has taken its place.
2. Particles in the world, beside the air grid, which now moves (`engine/mana/src/vm/fluid.rs`). The ledger counts them, and a second
   ledger counts momentum.
3. The instructions in §5: sensing and pushing particles (`PCNT`, `PPOS`, `PVEL`, `SHOV`, `WPOS`, `WVEL`), ingraining
   (`INGR`), and in orders `KICK`, `DENS`, `GRAD`, `NVEL` and `VEL`. `MOVE` and `LOCK SHAPE` are gone, and since D34 so is
   the field (`HOLD`).
4. The libraries and the four spells, rewritten (§6, §7).
5. The bench (below).
6. Weight, matter's mass, the ground's support and friction, water that holds together and can't be squeezed, rock, and
   pushes priced by the energy they add (D28–D30).
7. Particles merging and splitting, and the second flaw.
8. The air as a gas that flows (`engine/mana/src/vm/air.rs`).
9. The Energy ledger (`engine/mana/src/vm/energy.rs`).
10. The main objective's audit, made organic (§0, D32–D39): pushes that push back, Energy from the mind, reach on
    everything, registers passed by touch, particles that go alone, touch as a force felt, rock from a condition, the hand
    that holds by force, buoyancy and an air that weighs, friction under bodies, flame that diffuses.
11. One matter (D42–D45, `engine/mana/src/vm/matter.rs`): matter as material points, earth that bends, gives way, cracks
    and packs, water that flows, mana that grips matter by force, and matter at rest that sleeps.
12. Real air (D47–D50, `engine/mana/src/vm/air.rs`): air matter with mana in it, pressing by the ideal gas law, sound at
    280 m/s, open edges onto more air, everything feeling the air's pressure by the room it takes, and drag on bodies and
    matter.

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
  cards: nothing resists shear. Bound to their diagonal neighbours too (`bond_range` 0.2 m), it stands.
- **Rock has to be held from the ground up.** Bonds that keep their length are solved a pass at a time; solved in any order,
  the ground's support climbs a 2 m wall too slowly, and it sags. Solved lowest first, with the ground in the same passes,
  it stands in one.
- **Air that's too soft piles up.** With mana's own gas stiffness, the air's speed of sound was 1 m/s, and wind piled up
  against a pillar instead of going over it. The air has its own, faster (`air_sound`, until real air made it real, D47).
- **Stepping waves the wrong way makes them grow.** Moving the air with its old speeds while the pressure pushed it made
  every disturbance grow into a storm. Pushed first, then moved, they die away.
- **Holding is thinking, not mana.** With pushes priced by energy, pushing a straying particle back in mostly slows it:
  holding a ball by hand costs almost no mana, and what limits a caster is how fast they think, as the sandbox found.
- **Merging is an optimisation nobody wrote.** A fireball held by its order merges as it flies (72 particles to about 45),
  and a merged particle thinks once where two did: the ball burns about 40% less.

What making it organic found (D32–D39):

- **Throwing is shoving.** A push goes off the body. While mana weighed a kilogram a M, throwing an 18 M fireball slid an
  adept back a metre. Since D40 it weighs 5 g, and nothing much happens to the thrower; moving a 400 kg wall by hand is
  another matter.
- **A strong mind tears what it throws.** With no limit on how fast a particle can be sped up but the mind's power, a mind
  that pushes particles one at a time sends its first ones off at full speed while the last wait, and the ball tears. A
  good throw works out the push once a pass and gives every particle the same step (`throw`, 0.1 m/tick): cheaper too.
- **A hand runs out of grip on its own mana.** A pass over a fireball took longer than a hold lasts (`focus`): the mana
  to push with slipped into the body's flow halfway through, and the ball slowed. Holding it while you work (`CIRC`) is
  part of throwing.
- **Mana strikes its maker.** A shield's caster who walks into its front is pushed along by it, and the shield follows
  them on. A fireball let go of too close would burst in its caster's face: the hand is at arm's length.
- **A word spreads only by touch.** A particle that strayed from the ball never hears of the hit: it goes on clinging,
  alone, until its mana is gone.
- **Hovering costs a downdraft.** An order that holds its particle up in the air pushes the air down, a little more each
  tick, and pays for it: a Water Shield held up by its order lasts nearly a minute, and its water then comes down.
- **A parcel's buoyancy has to be the air's weight.** Pushing particles by the moving air's pressure, without the air
  knowing the particle is there, drove the air away from every particle: wind from nothing, which the Energy ledger caught
  (38 units of error in one Fireball). Archimedes in air at rest is exact.

**What making mass mana found (D40):**

- **Real air can't hold up rock.** The Stone Wall climbed out of its trench by kicking off the air in its cells. That
  worked while a cell of air weighed 40 kg. At 20 g, kicking 400 kg up off it would take a gale its order can't pay for,
  and it sank. It's raised by hand now, as a mage would.
- **Heavy air hid three things.** It damped every wobble, so a wall on a rough base stood; it was half of the `bodies`
  term in the Energy ledger, hiding what rock's bonds get wrong; and it made a breath of air mana heavy enough to knock
  someone over. With real air, all three show (§0, What's left).
- **Light things are cheap to throw.** A fireball weighs 5 g: throwing it at 13 m/s is a fraction of a joule. What a mind
  pays for is moving matter.
- **Making matter is dear.** Condensing keeps mass, so 11.5 M of water mana makes 6 g of water. A shield worth having
  takes a river's water (*influence*), not the caster's own mana.
- **Soil packs.** With earth packing full at rock's density, the ground's soil had room to spare, and a wall standing in it
  sank into itself, 2 m to 1.5 m. Earth packs full at soil's density until step 2 tells soil from rock.

**What one matter found (D42–D45):**

- **Soft rock buckles.** With sound in matter at 50 m/s, a column of rock 3.5 m tall bent under its own weight and fell,
  and a ledge sagged off its pillar. At 150 m/s both stand, for three times the steps.
- **Mana slides out of what it holds** if it moves first. The fluid stepped, then the matter: held mana moved, its earth
  lagged a step behind, and the grip, which acts only within a cell, lost it. Matter steps first now, and the mana clings
  to it as it moves.
- **A step that stops something is an impact.** Moving matter to the grid and back averages the motions that meet
  there, and a step changes a node's speed at once: both lose motion as a collision does. Uncounted, a block of earth
  dropped a metre lost a third of its fall's Energy to nowhere; counted as heat, the ledger closes to 0.3%.
- **Plastic heat is plastic work.** A step stretches matter a little past where its stress did the work, and that wasn't
  paid for. Counting the heat as the energy stored before giving way less after counted it anyway: a block landing on the
  world's floor made energy, a whole fall's worth.
- **A pulled stream of water comes apart.** Water here has no surface tension: pulled at one end it thins and parts,
  and the far end stays where it was. Real water at this size does the same. What holds a stream of water together is
  the mana in it, pushed along with it (D46).
- **A Stone Wall's grip didn't tear its rock out.** Lifting a wall out of the ground means breaking it free of the soil
  around it: its cohesion (about 8 kN along the cut), its friction (about 5 kN) and its weight (4 kN). At 5 kg a M the
  spell's mana gripped about 5 kN, and the wall crept up a millimetre a tick. The author chose a harder grip: 20 kg a M.
- **Matter that moves blocked like a wall.** Free mana stopped dead at any cell full of matter, and the ground took its
  momentum. A Stone Wall's earth rose a cell ahead of its own mana, and then stood in the mana's way: the mana stayed
  in the ground while the wall went up without it. Striking moving matter as an impact (D46) lets mana rise inside its
  earth, and gives a fireball's blow to what it hits.
- **A grip as hard as a weight vanished without weight.** Gripping as hard as `bind` kilograms weigh under the world's
  gravity, mana in weightlessness held nothing. A grip is a force: it's weighed at 9.81 m/s².
- **Mana spread inside what it held.** Gripped as a whole, a cell's mana was held to its matter's speed on average, while
  its particles spread apart inside it, and out of it. Each particle grips on its own now.
- **The Stone Wall rises, unsteered.** At 20 kg a M its earth comes out of the trench, but the heave, written for rock
  that moved with its mana, overshoots and sheds it: after 7 s its mana holds a sixth of the earth it took. Steering a
  wall made of matter is the spell's work (§7).

**What real air found (D47–D50):**

- **A closed world rings.** With its edges as walls, the air a caster gathers from rushes back in as a pressure wave of
  about 1.7 m/s, and echoed for seconds: a wind every spell had to fight. Open onto more air (the author's answer, D48),
  it leaves the world in a few ticks.
- **Mana pressing as a gas in the cell pushes itself.** Counted as gas that presses, a lone parcel's own pressure,
  shared over the cells around it, threw it at 170 m/tick in empty space. A parcel takes room at the air's pressure
  instead (Dalton), and feels only the air's.
- **The air has to give way as things move.** The fluid moves a fireball two cells a tick before the air steps: had the
  air waited, it would have been packed by several percent at once, and thrown the ball back. Sound is twenty times
  faster than a fireball, so the air makes way as it comes. The same went for matter: the Stone Wall's trench opened as
  empty hollows that sucked the soil into them at 2 atm, making 27 kJ a tick from nothing, until the air gave way there
  too, and through the ground where it couldn't get round (a stand-in, §0).
- **Cells pressing harder and softer by turns push on nothing.** With each face pressing by the average of its two cells,
  a pattern alternating cell by cell, ±1 kPa, filled the air around a caster and never evened out; it pushed a held
  fireball apart. Air moving through a face as sound meeting there would (a pressure difference drives it) ends it.
  Stepped too coarsely in 3D, that same evening out overshot and grew to ±100 kPa: the air steps more finely in 3D.
- **A placed body bangs.** A body put into air takes room at once, and its cells are pressed 15%. The air it displaces
  goes out over the rest of the open air as it's placed.
- **Drag is per step, not per tick.** At 45 m/s the air through a body's cells is replaced six times a tick; dragged once
  a tick, a gale pushed a person with a sixth of its force.
- **The ground isn't blown about.** Sleeping matter woke at a gather's 1 kPa wave, the whole 3D ground with it. It
  holds still, as resting things do, against a push no harder than its friction.
- **Faint ripples kept the whole world busy.** Counting only air standing exactly still as at rest, the faintest ripple
  made every cell active within a tick, a hundred times over. Air slower than air that stops counts as at rest (§0).
- **A Gust is a breeze.** Its few grams of air mana a tick set the air at its target moving at 6 cm/s. A person's feet
  hold against about 350 N, which takes a wind of some 40 m/s.
- **The ledger's buoyancy term is gone.** Counting the air's Energy as what it really holds (its thickness at each
  height, and the room parcels take), a parcel's buoyancy is in the air's Energy already.

### How the spells do now

The targets, and where the four spells stand against them (2D), with ticks of 1/30 s:

| Spell | Target | Now |
|---|---|---|
| Fireball | Leaves the hand within a second, reaches a pillar 9 m away with most of its mana | Let go after 0.73 s, hits 0.5 s later, 99% of its mana together, having risen 0.96 m on the way |
| Stone Wall | Rises in seconds, stands on its own for half a minute or more | Its earth comes out of the trench in 2 s, but the spell's heave doesn't steer it, and sheds most of it (§11, *What one matter found*) |
| Water Shield | Holds its water around its caster for several seconds | Makes 11.5 M of water mana into 6 g of water, holds half of it for 42 s (measured before real air) |
| Gust | Knocks someone back | Drives a breeze of 6 cm/s at its target; a person's feet hold against a wind of some 40 m/s |

Two spells miss their targets, and both wait for the author. The Stone Wall's heave has to steer a wall of matter, not a
rigid one. The Gust drives the air, and bodies feel it (D50), but a few grams of mana a tick make a breeze, not a gale.

### The numbers, and what they come from

| Number | Value | From |
|---|---|---|
| `mana_mass` | fire 0.3 g, water 0.55 g, air 0.45 g, earth 0.7 g a M | The author's order; raw mana as heavy, at 40 M a cell, as real air |
| `air_mana` | 40 M a cell of air, at the ground | 20 g in (0.25 m)³, in air matter as dense as real air: the air weighs both, 2.5 kg/m³ (the author's) |
| `density` | water 1,000, earth 1,600 kg/m³; air 1.2, flame 0.3 kg/m³ in the open air when a world is made | Water, packed soil; real air, a hot gas |
| `temperature` | 293.15 K | 20 °C, everywhere, until heat has a place (PLAN step 4) |
| `moles_per_m` | 0.45 g of real air's worth: 0.0155 mol | So that air matter is real air (29 g a mole); every M of any part alike |
| `bind` | 20 kg of matter a M of free mana, weighed at 9.81 m/s² | The author's: hard enough for a Stone Wall's mana to tear its earth out of the ground |
| `gravity` | 9.81 m/s², a tick 1/30 s | Real |
| `air_drag` | fire, water, earth 0.01; air 0.005 a tick | How fast free mana comes to the air's speed; air mana slips through it most easily (the author's guess, C1) |
| `drag_coefficient` | 1 | A person standing, a blunt lump |
| `body_density` | 985 kg/m³ | A person: 60 kg take 0.06 m³ |
| `air_viscosity` | a fifth of the difference a tick | A stand-in for eddies smaller than a cell, some 25,000 times real air's (§0) |
| `push_energy` | 900 J a M poured | A guess, kept |
| `stiffness` | fire 0.0009, water 0.0001, air 0.002, earth 0 (m/tick)² | How fast each part's free mana spreads: kept |
| `cohesion` | free water 0.037, free earth 0.15 a kg | Rescaled so free mana pulls as it did, now it's lighter |
| `matter_sound` | 150 m/s | A stand-in for kilometres a second (§0); matter's stiffness is its density times this squared |
| `earth_friction`, `earth_poisson` | 35°, 0.3 | Packed soil |
| `earth_cohesion` | 8 kPa | Firm soil (clay holds 10–100 kPa) |
| `earth_crush` | 150 kPa | Where soil starts to pack under a load: a guess |
| `packing_hardening` | 12 | Snow and soil models use 5–20 |
| `water_tension` | 200 Pa | A little: real water parts at far more, but only very clean water |
| `sleep_speed`, `wake_speed`, `rest_ticks` | 1 mm, 2 mm a tick; 15 ticks | Low enough that nothing visible sleeps |
| mind `power`, `capacity`, `recovery` | adept 450 J a tick, 54 kJ, 18 J a tick | Guesses (D32) |

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
finds now, as a baseline (nothing tuned):

- **Thrown, feeling is enough.** Held by `cling` or `cohere`, every thrown ball reaches the pillar, in 2D and 3D, with
  97–100% of its mana together and 92–97% still in it.
- **Held still, it depends on how finely a particle can feel.** In 2D `cling` and `cohere` keep 97–98% of the ball. In 3D
  they keep 67–79% (nothing keeps 31–33%; the old `feel`, before the flat kernel, about 40%): a 3D ball of 0.5 m is two
  smoothing lengths across, too coarse to feel its edge well (§12, question 6).
- **Lengths:** `cling` makes a 14-instruction order and `cohere` a 26; with `touch` and `burst`, the Fireball's order is
  52 instructions, and an adept ingrains it in 15 ticks.

**Since D32–D39** (2D, nothing tuned). Belonging is by order now, so `together` is the share of what a weave still holds
that's within 1 m of its middle (twice the ball's radius), and a throw arrives when the front of that reaches the pillar:

- **By hand, a thrown ball isn't the caster's once it leaves their reach.** Its particles carry no order, so they go
  loose, and nothing holds them: `ThrowHand` and `ThrowHandHeld` keep nothing of it at the pillar.
- **Held still by hand, it's thinking.** Every particle, every other one, or the surface: they all keep the ball in the
  weave (it's in reach), and a master holds it to 0.42 m where an adept's spreads to 0.5–1 m.
- **Thrown, an order still holds it.** `cling` gets 98–99% of a thrown ball to the pillar together, with 95% of its mana
  still in it; `cohere` 87–93%. `steer` is still the fastest, in 17 ticks, all of it together, having spent 15% of its
  mana flying itself; `heading` gets there, but as a scatter (35–37% together).

**Since real air** (D47–D50, 2D, nothing tuned; against the same bench just before):

- **Thrown, nothing much changes.** Every ball held by its order reaches the pillar as before, `cling` and `steer` with
  98–100% together, in the same ticks. A fireball weighs 5 g and the air it pushes aside a little more: real air's
  pressure and drag don't change its flight much.
- **Held still, balls spread a little further.** By their order, 0.30–0.38 m against 0.21–0.30 before; by nothing, 1.0–1.3 m
  against 0.7–1.0. A parcel of fire is pushed by the air's pressure, and moves more than the air does for it, being
  lighter: every wobble in the air shakes it.
- **Held by hand, they stay in the weave, and spread.** A master holding every particle, every other one or the surface
  keeps 66–70% in the weave where it kept 8–9%, spread over 0.36–0.37 m against 0.19–0.20.
- **It's slower.** The bench takes 30 s where it took 5: the air steps 70 times a tick in 2D to carry sound.

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
   `engine/mana/src/vm/physics.rs`, to be tuned in the tester.
2. **Runes as machine code.** Elvish *Runic Magic* is written "their own way", and a glyph already means a step. Glyphs could
   be **opcodes**, and the marks around them (the lattice, its families) the **operands**. A carved ring would then be a
   program the elves have always read straight, with no language in between. This needs its own design pass.
3. **Who found the flaw?** A flaw nobody teaches still has a history: who first condensed less than nothing, what it cost
   them, and who keeps it quiet. That's lore for Quire.
4. *Closed by D34: there's no field; a caster acts within their reach, and a weave is the particles that carry its order.*
   **The field.** Is a caster's field a ball around the weave's origin, or the particles they're keeping up with? Which body
   stat sets how far it reaches, and does it weaken with distance?
5. *Closed by D32: a mind transforms mana into Energy, and the mind pays, in strain. Whether heat should warm anything is
   still open (§0, What's left).* **Energy's price.** A push costs mana, by the kinetic energy it adds (D29), and the Energy
   ledger now counts every joule (§11). Whether a caster's own Energy (stamina, `condition`) pays too, and whether heat
   should warm anything (fire mana is what's hot; heat is the motion), is left for you: the ledger shows how much there'd be
   to pay. A Fireball's throw is about 1 kJ, lifting a Stone Wall about 9 kJ.
6. **How far does a particle feel?** *(Question 6, what an order knows, is closed: D31.)* A particle feels the mana within
   the smoothing length, 0.25 m. A 3D fireball of 0.5 m is only two of those across, with about two particles in each, so a
   few particles bunched at its edge feel as thick as its middle, and an order can't tell where the edge is: in 3D, holding
   a still ball by feel keeps little more than nothing does. Letting particles feel further than they press (say 0.5 m,
   worked out once a tick, only for orders) would fix it, at some cost in speed. So would more, smaller particles.
7. *Closed by D46: a spell keeps its water together, by the mana in it.* **How strongly does water stick to itself?** Real
   water's surface tension holds drops of a few millimetres together, and nothing at a metre. A whip of water that held
   by itself would need water a thousand times stickier than ours.
8. *Closed by D43: mana grips harder, 20 kg a M.* **What lifts a Stone Wall?** Its mana grips its earth, and the soil
   around holds it in by its cohesion and friction. How the spell steers what it lifts is the spell's.
