# Mana

## The main objective (read first, every time)

**Every feature of a spell comes from Ikozu's physics.** It's the project's first rule, and it wins over every other
decision. SPEC §0 has it in full.

- What happens in the machine has three sources only: **the mage's mind** (the assembly, priced like a real processor),
  **the mage's body** (what a living caster can do to mana, within biological limits: reach, power, capacity), and **the
  world's physics** (everything mana and matter do by themselves).
- No instruction, port or rule may exist to make a particular spell work. If a spell needs something, it has to follow
  from the physics, or from what a mind and a body can do.
- The physics is real science, as close as possible, and logical in Ikozu's world: mana, momentum and Energy are
  conserved exactly, every force comes in an equal and opposite pair, and nothing acts at a distance unless something
  carries it.
- When a simple spell is too hard to make, **tune the world's numbers** (`src/vm/physics.ts`), never add a special case.
- **Making and balancing spells is the user's job.** The libraries and spells here show how the system works. Keep them
  working with the least change; don't optimise or tune them on your own.

Before adding or changing any instruction, port, rule or constant, ask: *is this something the world does, or something a
spell was given?* SPEC §0 lists the places the machine doesn't meet the objective yet. Don't add to that list, and fix
from it when asked.

## Working here

- `npm test` (vitest; every test checks the mana and momentum ledgers each tick), `npm run check` (tsc and svelte-check).
- `npm run listings` refreshes SPEC's code listings from `lib/`, `spells/` and `bench/`; run it after editing `.masm`.
- `npm run bench` compares ways of holding and throwing (slow with `--3d`).
- The design and every decision are in SPEC.md. Record a new decision there (§1) as you make it.
