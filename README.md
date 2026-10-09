# Mana

Ikozu's magic as a machine you can program. A caster is the computer: a **mind** with number registers and a **body** with
mana registers. Spells are assembly. Elements, shapes and reactions are libraries written in that assembly, down to the
`sin` and `cos`. They run in a world made of mana, where Conservation always holds.

The design is in [SPEC.md](SPEC.md).

```
npm install
npm test
```

## The spell tester

```
npm run tester
```

Opens at http://localhost:5175. Edit a spell or a library, Cast it, then Play, Step one instruction (F10) or run to the end
of the Tick (Shift+F10). Click the gutter for a breakpoint; the second gutter shows what each line has cost in beats. Click
the world to aim. Ctrl+S saves the file back to `spells/`, `bench/` or `lib/`.

Mana is particles (SPEC §11). A weave's order runs in every particle it's ingrained into, every tick, and burns that
particle's mana as it thinks. The Order panel shows any one particle's last run, instruction by instruction, forward and
back; pick the particle by number or click it in the world. A breakpoint in order code pauses on the particle that hit it.

The world runs on real physics, as near as it can (SPEC §11). Things weigh, and fall at 9.8 m/s² (a tick is 1/30 s). A
push costs the kinetic energy it adds, so holding something up is free and lifting it costs its weight times the height.
Earth held by mana is rock, bound particle to particle; water holds together. The air is a gas that flows around things.
Particles at rest together merge, which is also how an order can spread (the second flaw). The Energy panel keeps the
Energy ledger: what the world holds, what casters put in, what turned to heat and how, and how far the numbers are off.

## The bench

```
npm run bench
npm run bench -- --only Throw --3d
```

The same ball of fire, held still or thrown at a pillar in every way the bench knows: by hand (every particle, every
other one, only the surface) or by its own order (one that clings to where it feels the mana thicker, one that also
feels how its neighbours move, one told just an angle to fly along). Orders only feel: they aren't told where they are
(SPEC D31). Each runs on the real machine, for an adept and a
master, on five layouts, and the bench prints what each cost and how well it did: how long the order took to ingrain, how
much of the ball stayed together, and the mana the caster's hand and the ball's own order spent. The bench's spells are in
`bench/`, and the tester lists them too.

## Assemble a spell

```
npx tsx src/cli/mas.ts spells/Fireball.masm
```

```
Fireball:
0000  28 11 03                         IN    n17, AMOUNT
0003  30 80 11                         GATH  m0, n17
0006  31 80                            CIRC  m0
0008  B2 81 80 00 00 00 00             FILT  m1, m0, #0
…
```

## Cast it

```
npx tsx src/cli/mvm.ts spells/StoneWall.masm
npx tsx src/cli/mvm.ts spells/Fireball.masm --ticks 24 --every 4
npx tsx src/cli/mvm.ts spells/Gust.masm --ticks 40 --maintain 30
npx tsx src/cli/mvm.ts spells/WaterShield.masm --3d
```

Add `--profile` to see where the caster's thought went, by routine and by line:

```
npx tsx src/cli/mvm.ts spells/Fireball.masm --3d --quiet --ticks 60 --profile
```

```
Fireball: 9357 beats over 32 ticks

routines:
     5304   56.7%  throw
     3968   42.4%  ingrain
…
lines:
     3384   36.2%  ×72     Basics.masm:30         INGR  n4, n6
     1184   12.7%  ×296    Basics.masm:77         .p:     PVEL  n8:10, n4, n6
     1184   12.7%  ×296    Basics.masm:84         SHOV  m0, n4, n6, n11:13
```

Each spell has a test world (`src/scenes.ts`). The terminal shows a slice of it: here, a Stone Wall that has climbed out
of its trench and stepped back onto the ground in front of it.

```
|             +HH                                                |
|        @    +HH                                                |
|        @    HHH                                                |
|        @    +HH                                                |
|              HH                                                |
|              HH                                                |
|################  ##############################################|
|################  ##############################################|
```

`@` is the caster, `#` is earth, `H` is earth a weave holds, `W` is water a weave holds, `*` is fire mana in a weave, and
`.` is loose mana. The full key is in `src/render.ts`.

## Where things are

- `src/asm/isa.ts`: every instruction, its opcode and operands.
- `src/vm/sim.ts`: the machine. What each instruction does, and what happens each tick.
- `src/vm/physics.ts`: the numbers the world runs on, to be tuned.
- `src/vm/fluid.ts`: mana as a fluid of particles: weight, pressure, cohesion, rock, the ground, merging and splitting.
- `src/vm/air.ts`: the air, a gas that flows.
- `src/vm/energy.ts`: the Energy ledger.
- `src/profile.ts`: where a cast's beats went.
- `lib/`: the libraries.
- `spells/`: Stone Wall, Fireball, Gust and Water Shield.
- `bench/` and `src/bench.ts`: the bench, its spells and what it measures.
- `scripts/listings.ts`: `npm run listings` brings SPEC.md's code listings up to date with the `.masm` files.
