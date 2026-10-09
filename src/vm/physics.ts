// The numbers the world runs on. They are guesses, made to be tuned in the tester.
//
// Arrays of four are by part: fire, water, air, earth. The world doesn't know those names; it only knows that each part
// behaves its own way.

export const PHYSICS = {
  /** Metres per cell. */
  cell: 0.25,
  /**
   * How dense each part's matter is, packed full, in kg/m³: flame (a hot gas), water, air (the gas we breathe), earth (as
   * packed soil). Matter is condensed mana, so this is how much mana a cell holds condensed: a full cell of earth is
   * (0.25 m)³ × 1,600 kg/m³ = 25 kg, which at 0.7 g a M is 35,700 M. How full a cell is, is the room its matter takes:
   * each part's mass over its density, summed. *(Flame and air are gases, and get a pressure of their own in PLAN step 3;
   * soil that packs into rock, 2,600 kg/m³, comes with step 2. Until then they take room at these densities.)*
   */
  density: [0.3, 1000, 1.2, 1600] as number[],
  /** Free mana in each cell of open air at the ground, when a world is made: 20 g, as heavy as real air (1.3 kg/m³). */
  airMana: 40,
  /**
   * How much matter 1 M of free mana can hold bound (influence), in kilograms. Bound matter moves with its mana and is
   * held up by it.
   */
  bind: 5,
  /** How full of unbound earth and water a cell has to be, as a share of its room, to count as solid: to block and touch. */
  solid: 0.3,
  /** Metres around the body that GATH draws from. */
  gatherRadius: 2,
  /**
   * How fast flame spreads into the air around it, thinning into warmth: the share of the difference between a cell's
   * flame and its neighbour's that evens out each tick.
   */
  flameSpread: 0.5,
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
  /**
   * How hard each part's free mana presses outward, as the square of the speed it spreads at, (m/tick)²: a gas of it
   * presses p = stiffness × its mass a M × how many M a m³ (the ideal gas law, by part). Earth's free mana doesn't spread
   * at all.
   */
  stiffness: [0.0009, 0.0001, 0.002, 0] as number[],
  /** How much each part's neighbours' motions even out: water is thick, earth thicker. */
  viscosity: [0.0005, 0.003, 0.0005, 0.01] as number[],
  /** Seconds in a tick. Weight needs it: things fall at 9.8 m/s². */
  tick: 1 / 30,
  /** How fast things fall, m/tick²: 9.81 m/s² at 30 ticks a second. */
  gravity: 9.81 / 900,
  /**
   * The mass of 1 M of mana of each part, in kilograms, free or condensed: mass is mana (D40). Fire is the lightest, then
   * air, then water, then earth. Raw mana, a quarter of each, weighs half a gram a M. Free mana is a gas: a parcel of it
   * pushes aside its own M's worth of air, which weighs half a gram a M, so fire rises through the air and earth sinks.
   * Matter is heavy because it's packed: many M to a cell.
   */
  manaMass: [0.0003, 0.00055, 0.00045, 0.0007] as number[],
  /**
   * How hard matter pushes back when it's packed denser than it can be (its `density`): pressure = this × (ρ − ρ₀), in
   * (m/tick)². Water and earth can't be squeezed: a column of them holds up what's on it.
   */
  matterStiffness: 0.3,
  /**
   * How strongly each part pulls on its neighbours, for each kilogram of them: free mana by part, water some, earth
   * strongly; and matter, by part: water's pull is its surface tension. *(A stand-in for each material's own cohesion,
   * PLAN step 2.)*
   */
  cohesion: [0, 0.037, 0, 0.15] as number[],
  matterCohesion: [0, 0.0001, 0, 0.0001] as number[],
  /**
   * Earth held by mana is rock: where it's packed as full as solid ground (`solid`) and still, moving against its
   * neighbours slower than `bondSpeed` m/tick, each particle of it is bound to its neighbours within `bondRange` metres,
   * like the grains of a stone. A bond keeps its length: each step, `bondIterations` passes push every pair of bound
   * particles back to it, equally and oppositely. A bond breaks when it has to pull or push harder than `bondStrength`
   * (m/tick² for each kilogram it holds), when it's bent past `bondBreak` of its length anyway, or when either particle
   * stops holding earth.
   */
  bondRange: 0.2,
  bondSpeed: 0.005,
  bondIterations: 12,
  bondStrength: 2,
  bondBreak: 0.25,
  /**
   * Particles that have come to rest beside each other merge, to keep their number down: closer than `mergeRange`
   * metres, moving within `mergeSpeed` m/tick of each other, and together no more than `maxMote` M. The new particle
   * keeps the bigger one's weave and order (SPEC §11, The second flaw).
   */
  mergeRange: 0.075,
  mergeSpeed: 0.005,
  maxMote: 1,
  /**
   * A particle of at least two motes that has spread thin splits in two, side by side: thin, when more than `splitAlone`
   * of the density it feels is its own. Both halves keep its weave and order.
   */
  splitAlone: 0.5,
  /**
   * How much of what presses something onto the ground holds it from sliding (Coulomb friction): earth on earth, and a
   * body's feet.
   */
  friction: 0.6,
  /** Metres around a point that mana poured into it is spread over (within its cell), so that its pressure has somewhere to push. */
  pour: 0.125,
  /**
   * Energy a mind transforms out of 1 M of mana poured onto a particle, in kg·(m/tick)² (the kilogram being the mass of
   * 1 M of free mana; one is 900 J). The mana isn't used up: it goes loose where it was poured, still mana (the Law of
   * Transformation, D32). A push costs the kinetic energy it adds to what's pushed and to what it's pushed off: speeding
   * up costs, the faster it's already going the more, and slowing down costs nothing (what it takes out of the motion is
   * heat). Holding something up against its weight costs nothing; lifting it costs its weight times the height.
   */
  pushEnergy: 1,
  /**
   * The most Energy an order can transform out of its particle's mana in one tick, for each M it holds: a bigger
   * particle's order pushes harder. In kg·(m/tick)² per M.
   */
  orderPower: 0.05,
  /** How fast a particle's speed comes to the speed of the air around it, per tick, in air as thick as the world's. */
  airDrag: 0.01,
  /**
   * How fast a push travels through the air, m/tick (18 m/s). Real air carries one at 340 m/s; this is slower, to keep the
   * steps few, and still fast beside its winds, so that it flows around things rather than piling up against them.
   */
  airSound: 0.6,
  /** Share of the difference in speed between neighbouring cells of air that evens out each tick. */
  airViscosity: 0.2,
  /** Slower than this against the air around it (m/tick), mana that belongs to no weave settles into the air. */
  looseRest: 0.02,
  /** M of its own mana an order burns for every beat it thinks. */
  orderBurn: 0.00002,
  /**
   * How many times a tick a particle passes its copy of its weave's registers on to those touching it: a write crosses
   * this many smoothing lengths a tick (2 m, at 8).
   */
  relay: 8,
  /** Steps the fluid takes in one tick. */
  substeps: 8,
}
