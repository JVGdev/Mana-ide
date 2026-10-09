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
  /** Seconds in a tick. Weight needs it: things fall at 9.8 m/s². */
  tick: 1 / 30,
  /** How fast things fall, m/tick²: 9.81 m/s² at 30 ticks a second. */
  gravity: 9.81 / 900,
  /**
   * How free mana of each part falls, as a share of gravity, net of the air mana it pushes aside: fire rises (hot, it's
   * lighter than what's around it), air floats, water and earth fall.
   */
  fall: [-0.03, 1, 0, 1] as number[],
  /**
   * The mass of 1 M of matter of each part, as a share of the mass of 1 M of free mana (which is the unit: call it a
   * kilogram). A full cell (100 M, 0.25 m across) of earth is 25 kg, of water 15.6 kg: as dense as the real things.
   * Flame and air hardly weigh.
   */
  matterMass: [0.0002, 0.156, 0.0002, 0.25] as number[],
  /** How matter of each part falls, as a share of gravity. */
  matterFall: [-1, 1, 0, 1] as number[],
  /**
   * How hard matter pushes back when it's packed denser than it can be (a full cell): pressure = this × (ρ − ρ₀), in
   * (m/tick)². Water and earth can't be squeezed: a column of them holds up what's on it.
   */
  matterStiffness: 0.3,
  /**
   * How strongly each part pulls on its neighbours, free mana by part: water some, earth strongly. Matter too, by part:
   * water's pull is its surface tension.
   */
  cohesion: [0, 0.00002, 0, 0.0001] as number[],
  matterCohesion: [0, 0.0001, 0, 0.0001] as number[],
  /**
   * Earth held by a weave is rock: when the weave is let go of, each particle of it is bound to its neighbours within
   * `bondRange` metres, like the grains of a stone. A bond keeps its length: each step, `bondIterations` passes push
   * every pair of bound particles back to it, equally and oppositely. A bond breaks when it has to pull or push harder
   * than `bondStrength` (m/tick² for each kilogram it holds), when it's bent past `bondBreak` of its length anyway, or
   * when either particle stops holding earth.
   */
  bondRange: 0.2,
  bondIterations: 12,
  bondStrength: 2,
  bondBreak: 0.25,
  /** How much of what presses something onto the ground holds it from sliding (Coulomb friction): earth on earth. */
  friction: 0.6,
  /** Metres around a point that mana poured into it is spread over (within its cell), so that its pressure has somewhere to push. */
  pour: 0.125,
  /**
   * Kinetic energy that 1 M of mana poured onto a particle turns into, in kg·(m/tick)² (the kilogram being the mass of
   * 1 M of free mana; one is 900 J). A push costs the kinetic energy it adds, measured against the ground: speeding up
   * costs, the faster it's already going the more, and slowing down costs nothing (what it takes out of the motion is
   * heat). Holding something up against its weight costs nothing; lifting it costs its weight times the height.
   */
  pushEnergy: 1,
  /** The most a particle's velocity can be changed by pushes in one tick, m/tick. */
  pushRate: 0.1,
  /** How fast a particle's speed comes to the speed of the air around it, per tick, in air as thick as the world's. */
  airDrag: 0.01,
  /** Share of the difference in speed between neighbouring cells of air that evens out each tick. */
  airViscosity: 0.2,
  /** Slower than this against the air around it (m/tick), mana that belongs to no weave settles into the air. */
  looseRest: 0.02,
  /** M of its own mana an order burns for every beat it thinks. */
  orderBurn: 0.00002,
  /**
   * Whether an order is told where its particle is from its weave's centre, and can read ORIGIN and MAKER. Without it, an
   * order only feels its own particle and its neighbours (SPEC §12, question 6). The bench tries both.
   */
  orderKnowsCentre: true,
  /** How far a weave's field reaches around its centre, in metres, until HOLD says otherwise. */
  field: 1,
  /** Steps the fluid takes in one tick. */
  substeps: 8,
}
