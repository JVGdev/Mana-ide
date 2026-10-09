// The numbers the world runs on. They are guesses, made to be tuned in the tester.
//
// Arrays of four are by part: fire, water, air, earth. The world doesn't know those names; it only knows that each part
// behaves its own way.

export const PHYSICS = {
  /** Metres per cell. */
  cell: 0.25,
  /** Matter a full cell holds (M of condensed mana). */
  cellMatter: 100,
  /** Free mana in each cell of open air, when a world is made. */
  airMana: 40,
  /** How much matter 1 M of free mana can hold bound (influence). Bound matter moves with its mana and is held up by it. */
  bind: 20,
  /** Unbound earth and water in a cell, from which it counts as something to touch, and blocks mana. */
  solid: 30,
  /** Metres around the body that GATH draws from. */
  gatherRadius: 2,
  /** Share of the difference in air mana between two cells that evens out each tick. */
  airDiffusion: 0.05,
  /** A pushed body keeps this share of its speed each tick. */
  bodyFriction: 0.7,
  /** Share of a flame's fire that spreads up and around each tick, thinning into warmth. */
  fireSpread: 0.25,
  /** Beats of thought a particle may spend in one tick of its order before its weave frays. */
  orderBudget: 64,
  /** Registers a particle thinks with in an order. */
  orderRegisters: 16,
  /** Registers a weave has (w0–w7). */
  weaveRegisters: 8,
  /** Below this, an amount is nothing. */
  epsilon: 1e-9,

  // Mana as a fluid (SPEC §11)

  /** M in one particle: the crowd of real mana particles that one simulated particle stands for. */
  mote: 0.25,
  /** Metres over which particles feel each other. */
  smoothing: 0.25,
  /** How hard each part presses outward: pressure = stiffness × density. It spreads at about √stiffness m/tick. Earth
   * doesn't spread at all: it holds together (a stand-in until earth and water get cohesion of their own). */
  stiffness: [0.0009, 0.0001, 0.002, 0] as number[],
  /** How much each part's neighbours' motions even out: water is thick, earth thicker. */
  viscosity: [0.0005, 0.003, 0.0005, 0.01] as number[],
  /** Upward pull on free mana of each part, m/tick²: fire rises. */
  rise: [0.0003, 0, 0, 0] as number[],
  /** Metres that mana poured into one point is spread over, so that its pressure has somewhere to push. */
  pour: 0.06,
  /** m/tick that one M of poured mana gives one M of mana. */
  pushYield: 2,
  /** The most a particle's velocity can be changed by pushes in one tick, m/tick. */
  pushRate: 0.1,
  /** How fast a particle's speed comes to the speed of the air around it, per tick, in air as thick as the world's. */
  airDrag: 0.01,
  /** Share of the difference in speed between neighbouring cells of air that evens out each tick. */
  airViscosity: 0.2,
  /** Slower than this against the air around it (m/tick), mana that belongs to no weave settles into the air. */
  looseRest: 0.02,
  /** M of its own mana an order burns for every beat it thinks. */
  orderBurn: 0.00005,
  /** How far a weave's field reaches around its centre, in metres, until HOLD says otherwise. */
  field: 1,
  /** Steps the fluid takes in one tick. */
  substeps: 8,
}
