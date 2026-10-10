//! The Energy ledger (D20, SPEC §11): what the world holds as motion and as stored energy, so that where it goes can be
//! counted. Mana is one ledger; Energy is the other, and each balances on its own.
//!
//! Units: kg·(m/tick)². One is 900 J.

use indexmap::IndexMap;

use super::air::{Room, room, scale_height};
use super::fluid::stored_in_fluid;
use super::parts::total;
use super::physics::{gas_energy, physics};
use super::world::{World, mass_of};
use crate::js;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stored {
    /// ½mv² of every particle, the air, every body and all matter.
    pub motion: f64,
    /// Weight lifted: of particles, the air and matter.
    pub height: f64,
    /// Mana's gas pressed together, matter stretched, and what coheres pulled apart.
    pub gas: f64,
    pub strain: f64,
    pub cohesion: f64,
    /// The air pressed together, or drawn thin.
    pub air: f64,
    /// Mana in casters' bodies, worth what the same mana is worth in the air at rest (air_level): so drawing it in, or
    /// letting it out, as thick as the air is, brings or takes no energy.
    pub bodies: f64,
}

pub const JOULES: f64 = 900.0;

/// Everything the world holds as Energy, by kind. `in_bodies` is the free mana in casters' bodies (M).
pub fn stored(world: &mut World, in_bodies: f64) -> Stored {
    let ph = physics();
    let g = ph.gravity;
    let room = room(world);
    let mut motion = 0.0;
    let mut height = 0.0;
    // A particle's height is worth its weight. What holds it up is the air, and the air's Energy counts it: rising, the
    // mana takes the room of air that comes down (its height), and the air it presses on is thinner up there (`air`).
    for p in &world.particles {
        let m = mass_of(p);
        motion += 0.5 * m * (js::pow(p.vel[0], 2.0) + js::pow(p.vel[1], 2.0) + js::pow(p.vel[2], 2.0));
        height += g * m * p.pos[1];
    }
    for b in &world.bodies {
        motion += 0.5 * b.mass * (js::pow(b.vel[0], 2.0) + js::pow(b.vel[1], 2.0) + js::pow(b.vel[2], 2.0));
    }
    for i in 0..world.size {
        let mass = world.air_mass(i);
        if mass != 0.0 && !mass.is_nan() {
            let v = &world.air_vel;
            let y = (world.coords(i)[1] as f64 + 0.5) * world.cell;
            motion += 0.5 * mass * (js::pow(v[i * 3], 2.0) + js::pow(v[i * 3 + 1], 2.0) + js::pow(v[i * 3 + 2], 2.0));
            height += g * mass * y;
        }
    }
    let air = pressed(world, &room);
    // Matter: its motion, its height, and what its stretching stores.
    let dims = if world.d == 1 { 2 } else { 3 };
    let mut strain = 0.0;
    for p in &world.points {
        motion += 0.5 * p.mass * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2]);
        height += g * p.mass * p.pos[1];
        strain += super::matter::stored(p, dims);
    }
    let usual = usual_air(world);
    let fluid = stored_in_fluid(world, if usual > 0.0 { usual / js::pow(world.cell, 3.0) } else { 1.0 });
    let level = air_level(world, &room);
    Stored {
        motion,
        height,
        gas: fluid.gas,
        strain,
        cohesion: fluid.cohesion,
        air,
        bodies: g * room.mean * level * in_bodies,
    }
}

/// The Energy the air holds pressed together, or drawn thin, against how thick it is on average: R·T·(N·ln(n/n̄) + n̄·V
/// − N) for each open cell holding N M in the room V its air has (n = N/V), counting the mana in particles there as gas
/// too. It's nothing at the average thickness, so gas let into the air, or drawn out of it, as thick as the air is,
/// brings or takes no Energy with it; and thicker below than above, as the air at rest is, it's what holds things up.
pub fn pressed(world: &World, room: &Room) -> f64 {
    let theta = gas_energy();
    let volume = js::pow(world.cell, 3.0);
    let mut amount = 0.0;
    let mut space = 0.0;
    for i in 0..world.size {
        if room.open[i] {
            amount += room.amount(world, i);
            space += volume * room.air[i];
        }
    }
    if space <= 0.0 || amount <= 0.0 {
        return 0.0;
    }
    let usual = amount / space;
    let mut air = 0.0;
    for i in 0..world.size {
        if !room.open[i] {
            continue;
        }
        let v = volume * room.air[i];
        let m = room.amount(world, i);
        air += theta * (if m > 0.0 { m * js::log(m / v / usual) + usual * v - m } else { usual * v });
    }
    air
}

/// The height where the air at rest is as thick as the air is on average, metres.
pub fn air_level(world: &World, room: &Room) -> f64 {
    if room.mean <= 0.0 {
        return 0.0;
    }
    let hh = scale_height(room.mean);
    let mut space = 0.0;
    let mut shape = 0.0;
    for i in 0..world.size {
        if room.open[i] {
            space += room.air[i];
            shape += room.air[i] * js::exp(-((world.coords(i)[1] as f64 + 0.5) * world.cell) / hh);
        }
    }
    if space <= 0.0 || shape <= 0.0 {
        return 0.0;
    }
    // At rest, n(y) = S·e^(−y/H), and on average it's S·shape/space: there, y = H·ln(space/shape).
    hh * js::log(space / shape)
}

/// How much free mana a cell of air holds, on average, among the cells open to it.
pub fn usual_air(world: &World) -> f64 {
    let mut sum = 0.0;
    let mut cells = 0usize;
    for i in 0..world.size {
        if !world.solid_at(i as isize) {
            sum += total(&world.air[i]);
            cells += 1;
        }
    }
    if cells > 0 { sum / cells as f64 } else { 0.0 }
}

pub fn sum(s: &Stored) -> f64 {
    s.motion + s.height + s.gas + s.strain + s.cohesion + s.air + s.bodies
}

pub fn heat_of(world: &World) -> f64 {
    world.heat.values().fold(0.0, |a, b| a + b)
}

/// The ledger kept tick by tick: what it held when counting began, what casters and orders put in or took out, and what
/// the numbers got wrong, by step. Always: now + heat = start + outside + error. Of what came from outside, `minds` is
/// what minds transformed out of mana (D32), counted push by push, and `bodies` the rest: what bodies did to the world's
/// mana by gathering it, pouring it and letting it out.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    pub start: f64,
    pub outside: f64,
    pub minds: f64,
    pub bodies: f64,
    pub error: IndexMap<String, f64>,
}
