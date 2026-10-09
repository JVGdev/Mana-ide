# Plan: the rest of mana physics

*Status: all steps are built (PRs #1–#8). Decisions B–F were taken as recommended; for A you chose b, orders only feel
(SPEC D31).*

What's left after phase 3 of SPEC §10, in the order it should be built. Each step lands on its own: tests pass, both ledgers
balance every tick, the tester shows the new thing, and SPEC says what was built and what it found.

```
0 Land what's built
1 Order bench ─────────► 2 What an order knows (your call)
3 Holding together and weight ─► 4 Merging, splitting, the second flaw
5 Air that flows                                  (3, 4 and 5 feed) ─► 6 The Energy ledger
                                                                  all ─► 7 Tuning and cleanup
```

1 comes first because it measures everything after it. 3 comes before 4, because particles have to come to rest against
each other before merging means anything. 5 is independent of 3 and 4. 6 comes after them, because the Energy ledger has to
count every way motion turns into heat, and 3–5 add most of them.

## Decisions needed

| # | Question | Blocks | Recommendation |
|---|---|---|---|
| A | What does an order know? (open question 6) | 2 | Decide after the bench (1). |
| B | How long is a tick? Weight needs it: gravity is 9.8 m/s². | 3 | **1/30 s.** The Fireball flies at 0.32 m/tick, which is 10 m/s: a hard throw. Gravity is then 0.011 m/tick², and a thrown ball drops about 1.5 m over 10 m. |
| C | Does matter held by mana weigh something? | 3 | **Yes, and the ground holds up what rests on it.** Lifting a wall costs its weight times the height. A wall standing on the ground costs nothing to keep up. Only its order's thinking does. That is the shortcut reality gives. |
| D | Does an order spread into resting air mana too, or only into particles? | 4 | **Only particles**, as SPEC §11 says. Spreading into the air would let an order gather mana for free, which would break the game. |
| E | Who pays for Energy? (open question 5) | 6 | **Build the ledger first and decide after seeing the numbers.** Until then, pushes cost only mana, as now. |
| F | Open a PR to merge `claude/mana-physics-spec` into main? | 0 | Yes, so each later step is its own PR. |

---

## 0. Land what's built *(small)*

- Open a PR for `claude/mana-physics-spec` → `main`, only after you say yes.
- Then each step gets its own branch from main.

## 1. The order bench *(medium)*

This is what you need to answer A: the same spell, run with different orders, measured the same way. It replaces the
sandbox, so there is one physics and not two.

- **Orders to compare**, as a new `lib/Orders.masm`, all used to throw the Fireball:

  | Order | Knows | About |
  |---|---|---|
  | `heading` | one angle from a weave register; kicks along it | 8 instructions |
  | `steer` | angle and speed | 12 |
  | `pull` (exists) | where it is from the centre (preloaded) | 17 |
  | `cohere` (exists) | its neighbourhood: `DENS`, `GRAD`, `NVEL` | 39 |
  | `feel` | its neighbourhood only, with the centre hidden | about 30 |
  | none, by hand | the caster's routines from the sandbox, now in `.masm` | none |

  For `feel`, a bench-only switch (`PHYSICS.orderSees`) hides the preloaded centre offset and `IN ORIGIN`. That shows what
  a world where orders only feel would be like, before anyone decides on it.
- **`src/bench.ts` and `npm run bench`.** Each variant runs on five layouts with an adept and a master, in 2D and 3D. It
  prints a table:
  - ingrain ticks and beats;
  - mana burned while flying;
  - how many particles stay together when it reaches the pillar;
  - how far it gets;
  - the mana each variant has left.
- **Strategies by hand.** Every other particle, only the surface, look first: written as `.masm` routines in
  `lib/Holding.masm`. The sandbox's comparisons then run on the real machine.
- **The tester:** a spell picker for the variants, so you can watch each one and step into its orders.
- **Report:** the table, published as a page you can read on your phone.
- **Remove** `sandbox/`, its test, its scripts and the esbuild dependency. SPEC §11 *The sandbox* becomes *The bench*.
- **Tests:** each variant casts and keeps both ledgers; the bench is deterministic (same numbers on every run).

## 2. What an order knows *(small to medium, depends on A)*

*Chosen: b (SPEC D31).* The options were:

- **a. As now.** The order is told where it is from the centre for free.
- **b. Feel only.** The centre offset is no longer given. `IN ORIGIN` costs as much as sensing does. The libraries' orders
  are rewritten to feel.
- **c. What you said: a simple order gets an angle, a complex one gets more.** `INGR` takes parameters (`INGR n, s, k`): it
  copies `k` registers from the caster into the particle, and each one costs beats to ingrain. The particle keeps them in
  its own registers, so each particle can be told something different. How many it can hold could depend on how long the
  order is, or on how much mana the particle has.

Then the libraries, spells, docs, tester and tests are updated to match.

## 3. Holding together and weight *(large)*

Water stays water and earth stays earth, without the "earth has no pressure" stand-in.

- **Rest density** (`fluid.ts`). Water and earth push back only when packed denser than they rest: pressure =
  k·(ρ − ρ₀), never below zero. Fire and air stay gases: pressure = k·ρ. A cup of water keeps its volume.
- **Cohesion.** An attraction between neighbours of the same kind, equal and opposite, so momentum still balances. It uses
  the cohesion kernel from Akinci et al. 2013, which doesn't clump the way negative pressure does. Per part:
  `cohesion: [0, water, 0, earth]`. Water beads and pools. Earth holds its shape and stacks.
- **Weight.** A signed `gravity` per part replaces `rise`: fire goes up, water and earth fall, air is neutral. If C is yes,
  matter a particle holds adds to its mass and its weight. The ground stops what falls and takes its weight as an impulse,
  as walls already do. So standing costs nothing and lifting costs mana.
- **Spells rebalanced:**
  - Stone Wall pays to lift its earth 2 m, then stands on its own.
  - Water Shield sags unless its order holds it up.
  - Fireball falls a little in flight, and throwing it upward pays for the height.
- **Tests:**
  - a drop of water falls, lands and spreads into a puddle of the right volume;
  - a pile of earth particles stands;
  - lifting costs the right amount of mana (mass × gravity × height, through the push yield);
  - both ledgers balance.
- **Tester:** each particle's density against its rest density, and its weight, on hover.

## 4. Merging, splitting and the second flaw *(medium)*

- **Merging** (`fluid.ts`, at the end of the tick). Two particles merge when all of these hold:
  - they are closer than about a third of the smoothing length;
  - they move at nearly the same speed;
  - together they hold no more than `maxMote` (about 1 M);
  - neither is in a caster's hand.

  The new particle has their summed mana and matter, their mass-weighted position, and their summed momentum. Mana and
  momentum stay exact. It keeps the bigger one's weave **and order**. Ties go to the older particle, so runs repeat.
- **Splitting.** A particle bigger than two motes that has spread thin (low density) splits into two halves, side by side
  across the direction it thins, with the same velocity. Both keep its weave and order.
- **The second flaw follows from merging.** A big ordered particle at rest against other mana takes it over. That includes
  loose particles, another weave's particles and another caster's fireball. Every takeover is logged (`taken`), and the
  tester marks it. Nothing in the libraries uses it.
- **Tests:**
  - merging and splitting keep both ledgers;
  - the number of particles drops when a construct rests;
  - a test spell takes over a still fireball;
  - a moving fireball isn't taken over.
- **Gain:** fewer particles at rest, so big 3D weaves run faster.

## 5. Air that flows *(medium to large)*

The air carries itself along, flows around things, and pushes from dense to thin. This replaces `diffuseAir` and `airFlow`.

- **On the grid** (`world.ts` or a new `air.ts`). Mana and momentum move across each face between cells, taken from the
  upwind side. Pressure in the air = k·ρ pushes across each face, equal and opposite. Solid cells are closed faces. The
  step is substepped, so fast wind can't skip a cell.
  - Mana stays exact, because whatever leaves one cell enters its neighbour.
  - Momentum stays exact, because the walls' share is recorded, as now.
- **Storage:** the air moves from an array of objects to one `Float64Array`, so 3D stays fast.
- **What it does:**
  - Gust becomes a real jet that travels.
  - A fireball leaves a wake behind it and pushes air ahead.
  - Wind flows around the pillar.
  - Air mana spread out by a gather flows back in from around it.
- **Tests:**
  - a puff of wind travels and keeps both ledgers;
  - behind the pillar the wind is slower, and around its sides faster;
  - a gathered hole refills from outside.
- **Tester:** the wind drawn as streaks everywhere, not only where particles are.

## 6. The Energy ledger *(medium)*

D20 says Energy is conserved. Here the machine starts counting it.

- **Kinetic energy** of particles, air and bodies.
- **Heat:** a new per-cell amount. It is Energy, not mana: fire mana is what's hot, heat is the motion. These are turned
  into heat, each computed exactly at the moment it happens:
  - drag and viscosity;
  - stopping against walls;
  - merging;
  - a body's friction.
- **Energy from outside:** pushes, kicks and gravity's work. These are recorded separately, as momentum's impulses are.
- **`World.energyError()`, checked every tick in the tests**, like momentum.
  - Risk: pressure, integrated in steps, doesn't conserve energy exactly.
  - The plan is to measure the leftover error, keep it small with symplectic stepping, and report it honestly rather than
    hide it in the heat.
- **Tester:** an Energy panel beside the ledger.
- **Then decision E:** who pays for the Energy a push puts in (stamina, condition). Building it is a small step on top of the
  ledger.

## 7. Tuning and cleanup *(medium)*

- **Targets written down first.** The Fireball reaches 10 m with most of its mana together, the wall stands about a minute,
  the shield holds while maintained, and so on. Then the bench sweeps the numbers in `physics.ts` toward them.
- **SPEC:** §3–§7 and §11 updated from the code, the phases marked built, and the open questions closed or rewritten.
- **README:** profile figures and pictures refreshed.
- **A PR for each step**, as it lands.

## Risks

- **Speed in 3D.** Cohesion, merging and flowing air all add work per tick. Merging pays some of it back. The bench keeps
  timings, so a slowdown shows up.
- **Every new force has to come in equal and opposite pairs**, or the momentum check fails. That's on purpose: it catches
  mistakes the same day.
- **Rebalancing.** Weight and cohesion change every spell's numbers. The tests check what a spell does, not exact figures,
  so they should hold through it.
