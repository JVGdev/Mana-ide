# Plan: real matter, real air, real heat

*Before anything here: the main objective (SPEC §0) comes first. Every feature of a spell comes from Ikozu's physics. The
last plan (now in git history) is built, and so is the audit's list (SPEC D32–D39). This plan is what that work turned up
(SPEC §0, What's left), shaped by your answers of 9 October.*

*Speed doesn't constrain any of it: realism comes first, and the engine is moving to a better language later. Each step is
specified in SPEC as physics, so the new engine can build the same thing, and the tests go with it.*

```
0 Land organic-physics
1 Mass is mana ─────────► 2 One matter ─────────► 3 Real air ─────────► 4 Heat
5 The mind's reach   (independent)
6 Bodies pay for gathering   (small; after 1)
                                                          all ─► 7 Spells, bench, SPEC
```

1 comes first because every number after it depends on the weights. 2 is the biggest step, and 3 and 4 build on it: real
air is a gas made of matter, and heat lives in matter, mana and air alike. 5 can be built any time.

## Decided (9 October)

| # | Question | Your answer | What it means for the physics |
|---|---|---|---|
| A | How heavy is mana? | Each part has its own weight: fire the lightest, then air, then water, earth the heaviest. | Mass is mana, by part: 1 M of each part weighs its own amount, free or condensed, and matter is heavy because it's packed. The order is set; how far apart the parts are is tuning. Starting numbers in step 1. |
| B | Is the open air real air? | Yes. | The atmosphere is air matter, a gas, with free mana in it (step 3). |
| C | What makes a fire hot? | Fire mana is more easily turned into heat. Heat, like light, is Energy. | A mind can transform mana into heat as well as into motion (D32). How easily depends on the part: fire into heat most easily. Light is Energy too: hot things glow, and carry heat away as they do (step 4). |
| D | How does influence reach past the aura? | The aura's touch is easy. Further, a mage has to work: unaided, he works out the space's geometry in his mind and pinpoints x, y, z. Probing the space with raw free mana, which sends knowledge back, is one way to do it better. | The aura is the body's outline. Past it, every act costs the mind the work of finding where it is, and sensing what's there needs something to carry the knowledge back (step 5). |
| E | A pull request? | Commit, no PR; work on `main` from now on. | Done: `main` has the organic physics. |

**Still open, to answer when its step comes:**

- **C, other parts.** Does each part have an Energy it turns into most easily (air into motion? earth into pressure?), or is
  fire into heat the only one? Until you say, fire turns into heat more easily, and everything else is the same.
- **C, light.** Can a mind transform mana into light directly (a glow, a flash), or does light only come off hot things?
- **D, the knowledge's way back.** Does a probe's knowledge come back to its caster instantly (it's their mana, and the soul
  carries it), or does it travel, at the speed the mana moves?
- **D, pinpointing.** Does working out a far point only cost thought, or can it also be wrong: the further away, the more
  it's off, unless something tells the mind where things really are?

---

## 0. Land what's built *(done)*

- `claude/organic-physics` is committed and on `main`. From here on, everything is committed straight to `main`.

## 1. Mass is mana *(medium)*

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
- **Rock is earth's own cohesion**, not bonds a weave makes. A wall raised by hand, positioned and molded holds because
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
- **Real speed of sound** (340 m/s), since speed is no constraint: `airSound` goes.
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
- **Turning mana into heat** (C): a mind transforms mana into heat as it does into motion (D32). Heat goes straight into
  what the mana touches, with no push. It's a new act for the body and for orders: what a mind can do, not something a
  spell was given. Each part turns into each kind of Energy with its own ease: fire into heat most easily, for less strain
  a joule. That's a table in `physics.ts`, by part and kind.
- **Light:** hot things glow, and the light they give off carries their heat away (radiation). Light that falls on
  something warms it.
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

## 5. The mind's reach *(medium)*

- **The aura is the body's outline.** What touches it, the caster senses and acts on at once, at no extra cost: a fireball
  poured into the hand is easy to hold and aim.
- **Past the aura, the mind has to find the place.** Every act and sense on something further away (push, pour, ingrain,
  probe, write into a weave, feel a particle) costs the beats of working out the space's geometry: more the further it is,
  and more the less the mind knows about what's around it. The 4 m `reach` stat goes. What limits a mage is what their
  mind can afford.
- **Knowing helps.** What the mind has already found (a weave it laid out, a point it worked out last tick) is cheaper to
  find again.
- **Probing:** raw free mana (unfiltered) let out from the body stays in touch with its caster. Whatever it touches, its
  caster learns, and finding a place where their probe is costs little. Your answer says not to take it as exact. It's
  built so that a better way to reach can be found by playing with the physics, not given by a rule.
- **Fixes D33 and D34**, which put the aura out to 4 m. A push still goes off the body: the soul carries it there and back.
- **Tests:**
  - pouring at the hand costs nothing extra;
  - pushing a particle 3 m away costs more beats than 1 m, and the same push near the caster's probe costs less;
  - a weave poured into a probed place is laid out faster than one poured blind.

## 6. Bodies pay for gathering and pouring *(small; after 1)*

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
- **Building it twice.** If the new engine comes soon, 2–4 could be specified here and built there. Otherwise, build here:
  the tests and the SPEC go with it.
