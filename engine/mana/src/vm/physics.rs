//! The numbers the world runs on. They are guesses, made to be tuned in the tester.
//!
//! Arrays of four are by part: fire, water, air, earth. The world doesn't know those names; it only knows that each part
//! behaves its own way.
//!
//! The table is one per thread, and anything may change it at any time (a test makes orders costlier halfway through a
//! cast): each step reads it as it begins, with `physics()`.

use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Physics {
    /// Metres per cell.
    pub cell: f64,
    /// How dense each part's matter is, in kg/m³: water and earth packed full (earth as packed soil; packed denser, it's
    /// rock, D42). Matter is condensed mana, so this is how much mana a cell holds condensed: a full cell of earth is
    /// (0.25 m)³ × 1,600 kg/m³ = 25 kg, which at 0.7 g a M is 35,700 M. How full a cell is, is the room its earth and
    /// water take: each one's mass over its density, summed. Flame and air are gases: they have no packing, and fill the
    /// room that's left at the pressure their amount makes. For them this is how dense they are in the open air at the
    /// ground when a world is made: air as real air (1.2 kg/m³, 41.7 M a cell), flame as thin as a hot gas.
    pub density: [f64; 4],
    /// Free mana in each cell of open air at the ground, when a world is made, in the air matter there: 20 g. The air
    /// weighs both (2.5 kg/m³), the author's answer: the air is condensed air mana, and the mana in it weighs too.
    pub air_mana: f64,
    /// How hot the world is, kelvin: everything, until heat has a place (PLAN step 4). A gas presses by it.
    pub temperature: f64,
    /// How many moles of gas 1 M is, of any part, free or condensed: as many as 0.45 g of real air holds (29 g a mole),
    /// so that air matter is real air. The ideal gas law counts M: a gas presses p = n·R·T, n its moles a m³, whatever
    /// it's made of, so the lighter parts make a lighter gas at the same pressure, and rise (Avogadro).
    pub moles_per_m: f64,
    /// How much matter 1 M of free mana can hold up (influence), in kilograms: mana pulls the matter of its own parts in
    /// its cell toward its own speed, at most as hard as that much matter weighs at the standard 9.81 m/s² (its `grip`,
    /// a force, whatever the world's gravity is). Pulled harder, the matter slips.
    pub bind: f64,
    /// How full of earth and water a cell has to be, as a share of its room, to count as solid: to block and touch.
    pub solid: f64,
    /// Metres around the body that GATH draws from.
    pub gather_radius: f64,
    /// How fast flame spreads into the air around it, thinning into warmth: the share of the difference between a cell's
    /// flame and its neighbour's that evens out each tick.
    pub flame_spread: f64,
    /// Beats of thought a particle may spend in one tick of its order before its weave frays.
    pub order_budget: f64,
    /// Registers a particle thinks with in an order.
    pub order_registers: usize,
    /// Registers a weave has (w0–w7).
    pub weave_registers: usize,
    /// Below this, an amount is nothing.
    pub epsilon: f64,

    // Mana as a fluid (SPEC §11)
    /// M in one particle: the crowd of real mana particles that one simulated particle stands for.
    pub mote: f64,
    /// Metres over which particles feel each other.
    pub smoothing: f64,
    /// How hard each part's free mana presses outward, as the square of the speed it spreads at, (m/tick)²: a gas of it
    /// presses p = stiffness × its mass a M × how many M a m³ (the ideal gas law, by part). Earth's free mana doesn't
    /// spread at all.
    pub stiffness: [f64; 4],
    /// How much each part's neighbours' motions even out: water is thick, earth thicker.
    pub viscosity: [f64; 4],
    /// Seconds in a tick. Weight needs it: things fall at 9.8 m/s².
    pub tick: f64,
    /// How fast things fall, m/tick²: 9.81 m/s² at 30 ticks a second.
    pub gravity: f64,
    /// The mass of 1 M of mana of each part, in kilograms, free or condensed: mass is mana (D40). Fire is the lightest,
    /// then air, then water, then earth. Raw mana, a quarter of each, weighs half a gram a M. Free mana is a gas: a parcel
    /// of it pushes aside its own M's worth of air, which weighs half a gram a M, so fire rises through the air and earth
    /// sinks. Matter is heavy because it's packed: many M to a cell.
    pub mana_mass: [f64; 4],
    /// How strongly each part of free mana pulls on its neighbours, for each kilogram of them: water some, earth strongly.
    pub cohesion: [f64; 4],

    // Matter (SPEC §11, One matter): material points over the grid. Stresses are in kg/(m·tick²), a Pa over 900.
    /// Material points a cell holds along each axis, when matter is laid out.
    pub matter_points: usize,
    /// How fast sound crosses matter, m/tick: 150 m/s. *A stand-in* (SPEC §0): rock carries it at kilometres a second,
    /// which would take thirty times more steps. Its stiffness follows: its density times this squared. Softer, and rock
    /// bends like rubber: a column of it buckles under its own weight past √(c²) × a few metres (at 50 m/s, 3 m).
    pub matter_sound: f64,
    /// The most of a cell a wave in matter may cross in one step.
    pub matter_cfl: f64,
    /// Earth's Poisson's ratio: how much it bulges sideways as it's squeezed.
    pub earth_poisson: f64,
    /// Earth's angle of friction, radians: loose earth piles at about this slope.
    pub earth_friction: f64,
    /// How hard earth holds together as soil (Mohr–Coulomb's cohesion): 8 kPa, so a cut 2–3 m high stands.
    pub earth_cohesion: f64,
    /// The pressure that crushes soil denser: 150 kPa. Pressed past it, earth packs, and packed, it holds harder.
    pub earth_crush: f64,
    /// How much harder earth holds, and crushes, packed denser: × e^(this × how much denser).
    pub packing_hardening: f64,
    /// How much tension water holds before it parts: 200 Pa, so it sticks to itself.
    pub water_tension: f64,
    /// Matter slower than this (m/tick) for `rest_ticks` ticks, and held by nothing, sleeps; and sleeping matter that
    /// would move faster than `wake_speed` wakes.
    pub sleep_speed: f64,
    pub wake_speed: f64,
    pub rest_ticks: u32,
    /// Particles that have come to rest beside each other merge, to keep their number down: closer than `merge_range`
    /// metres, moving within `merge_speed` m/tick of each other, and together no more than `max_mote` M. The new particle
    /// keeps the bigger one's weave and order (SPEC §11, The second flaw).
    pub merge_range: f64,
    pub merge_speed: f64,
    pub max_mote: f64,
    /// A particle of at least two motes that has spread thin splits in two, side by side: thin, when more than
    /// `split_alone` of the density it feels is its own. Both halves keep its weave and order.
    pub split_alone: f64,
    /// How much of what presses something onto the ground holds it from sliding (Coulomb friction): earth on earth, and a
    /// body's feet.
    pub friction: f64,
    /// Metres around a point that mana poured into it is spread over (within its cell), so that its pressure has somewhere
    /// to push.
    pub pour: f64,
    /// Energy a mind transforms out of 1 M of mana poured onto a particle, in kg·(m/tick)² (the kilogram being the mass of
    /// 1 M of free mana; one is 900 J). The mana isn't used up: it goes loose where it was poured, still mana (the Law of
    /// Transformation, D32). A push costs the kinetic energy it adds to what's pushed and to what it's pushed off:
    /// speeding up costs, the faster it's already going the more, and slowing down costs nothing (what it takes out of
    /// the motion is heat). Holding something up against its weight costs nothing; lifting it costs its weight times the
    /// height.
    pub push_energy: f64,
    /// The most Energy an order can transform out of its particle's mana in one tick, for each M it holds: a bigger
    /// particle's order pushes harder. In kg·(m/tick)² per M.
    pub order_power: f64,
    /// How fast a particle's speed comes to the speed of the air around it, per tick, in air as thick as the world's at
    /// the ground, by part: air mana slips through the air most easily (the author's guess, C1).
    pub air_drag: [f64; 4],
    /// How hard the air pushes on something blunt it flows past (a person, a lump of earth): its drag coefficient, in
    /// ½·ρ·C·A·u². About 1 for a person standing.
    pub drag_coefficient: f64,
    /// How dense a body is, kg/m³: how much room it takes in the air, out of its box. A person's 60 kg take 0.06 m³.
    pub body_density: f64,
    /// Share of the difference in speed between neighbouring cells of air that evens out each tick.
    pub air_viscosity: f64,
    /// Slower than this against the air around it (m/tick), mana that belongs to no weave settles into the air.
    pub loose_rest: f64,
    /// M of its own mana an order burns for every beat it thinks.
    pub order_burn: f64,
    /// How many times a tick a particle passes its copy of its weave's registers on to those touching it: a write crosses
    /// this many smoothing lengths a tick (2 m, at 8).
    pub relay: usize,
    /// Steps the fluid takes in one tick.
    pub substeps: usize,
}

pub const PHYSICS: Physics = Physics {
    cell: 0.25,
    density: [0.3, 1000.0, 1.2, 1600.0],
    air_mana: 40.0,
    temperature: 293.15,
    moles_per_m: 0.45 / 28.97,
    bind: 20.0,
    solid: 0.3,
    gather_radius: 2.0,
    flame_spread: 0.5,
    order_budget: 64.0,
    order_registers: 16,
    weave_registers: 8,
    epsilon: 1e-9,
    mote: 0.25,
    smoothing: 0.25,
    stiffness: [0.0009, 0.0001, 0.002, 0.0],
    viscosity: [0.0005, 0.003, 0.0005, 0.01],
    tick: 1.0 / 30.0,
    gravity: 9.81 / 900.0,
    mana_mass: [0.0003, 0.00055, 0.00045, 0.0007],
    cohesion: [0.0, 0.037, 0.0, 0.15],
    matter_points: 2,
    matter_sound: 150.0 / 30.0,
    matter_cfl: 0.4,
    earth_poisson: 0.3,
    earth_friction: 35.0 * std::f64::consts::PI / 180.0,
    earth_cohesion: 8000.0 / 900.0,
    earth_crush: 150_000.0 / 900.0,
    packing_hardening: 12.0,
    water_tension: 200.0 / 900.0,
    sleep_speed: 0.001,
    wake_speed: 0.002,
    rest_ticks: 15,
    merge_range: 0.075,
    merge_speed: 0.005,
    max_mote: 1.0,
    split_alone: 0.5,
    friction: 0.6,
    pour: 0.125,
    push_energy: 1.0,
    order_power: 0.05,
    air_drag: [0.01, 0.01, 0.005, 0.01],
    drag_coefficient: 1.0,
    body_density: 985.0,
    air_viscosity: 0.2,
    loose_rest: 0.02,
    order_burn: 0.00002,
    relay: 8,
    substeps: 8,
};

thread_local! {
    static CURRENT: Cell<Physics> = const { Cell::new(PHYSICS) };
}

/// The numbers as they are now.
pub fn physics() -> Physics {
    CURRENT.with(Cell::get)
}

/// Changes the numbers, from now on.
pub fn tune(f: impl FnOnce(&mut Physics)) {
    CURRENT.with(|c| {
        let mut p = c.get();
        f(&mut p);
        c.set(p);
    })
}

/// Puts the numbers back as they were written.
pub fn reset() {
    CURRENT.with(|c| c.set(PHYSICS))
}

/// Changes the numbers while `run` runs, and puts them back after, even if it panics.
pub fn tuned<T>(f: impl FnOnce(&mut Physics), run: impl FnOnce() -> T) -> T {
    struct Restore(Physics);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|c| c.set(self.0))
        }
    }
    let _restore = Restore(physics());
    tune(f);
    run()
}

/// The gas constant: R, J/(mol·K).
pub const GAS_CONSTANT: f64 = 8.314462618;

/// How hard a gas presses for each M of it in each m³, p = this × n, in the engine's units (kg·(m/tick)² a M): R·T
/// times the moles in a M (the ideal gas law). 38 J a M at 20 °C.
pub fn gas_energy() -> f64 {
    let ph = physics();
    GAS_CONSTANT * ph.temperature * ph.moles_per_m / 900.0
}

/// How hard 1 M of free mana grips matter, kg·m/tick²: as hard as `bind` kilograms weigh at the standard 9.81 m/s².
pub fn grip() -> f64 {
    physics().bind * 9.81 / 900.0
}
