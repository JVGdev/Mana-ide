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
the world to aim. Ctrl+S saves the file back to `spells/` or `lib/`.

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
Fireball: 14515 beats over 49 ticks

routines:
    14441   99.5%  ball
       51    0.4%  Fireball
…
lines:
     3064   21.1%  ×383    Shapes.masm:52         COS   n13
     3064   21.1%  ×383    Shapes.masm:56         SIN   n15
```

Each spell has a test world (`src/scenes.ts`). The terminal shows a slice of it:

```
|                        HH                                      |
|        @               HH                                      |
|        @               HH                                      |
|        @               HH                                      |
|                        HH                                      |
|########################  ######################################|
|########################  ######################################|
```

`@` is the caster, `#` is earth, `H` is earth a weave holds, `W` is water a weave holds, `*` is fire mana, and `.` is loose
mana. The full key is in `src/render.ts`.

## Where things are

- `src/asm/isa.ts`: every instruction, its opcode and operands.
- `src/vm/sim.ts`: the machine. What each instruction does, and what happens each tick.
- `src/vm/physics.ts`: the numbers the world runs on, to be tuned.
- `src/profile.ts`: where a cast's beats went.
- `lib/`: the libraries.
- `spells/`: Stone Wall, Fireball, Gust and Water Shield.
