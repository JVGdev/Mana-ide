// The numbers the world runs on. They are guesses, made to be tuned in the tester.

export const PHYSICS = {
  /** Metres per cell. */
  cell: 0.25,
  /** Matter a full cell holds (M of condensed mana). */
  cellMatter: 100,
  /** Free mana in each cell of open air, when a world is made. */
  airMana: 40,
  /** How much matter 1 M of free mana can hold bound (influence). */
  bind: 20,
  /** Share of a loose weave's free mana that leaks back into the air each tick. */
  leak: 0.002,
  /** Unbound earth and water in a cell, from which it counts as something to touch. */
  solid: 30,
  /** Metres around the body that GATH draws from. */
  gatherRadius: 2,
  /** Share of the difference in air mana between two cells that evens out each tick. */
  airDiffusion: 0.05,
  /** Loose mana keeps this share of its speed each tick. */
  looseDrag: 0.85,
  /** Slower than this (m/tick), loose mana settles back into the air. */
  looseRest: 0.02,
  /** How hard moving air mana pushes a body: per M, per m/tick of its speed. */
  push: 0.02,
  /** A pushed body keeps this share of its speed each tick. */
  bodyFriction: 0.7,
  /** Share of a flame's fire that spreads up and around each tick, thinning into warmth. */
  fireSpread: 0.25,
  /** Instructions a cell may run in one tick of its order before the weave frays. */
  orderBudget: 64,
  /** Registers a cell thinks with in an order. */
  orderRegisters: 16,
  /** Registers a weave has (w0–w7). */
  weaveRegisters: 8,
  /** Below this, an amount is nothing. */
  epsilon: 1e-9,
}
