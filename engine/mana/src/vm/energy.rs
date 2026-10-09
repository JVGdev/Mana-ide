//! The Energy ledger (D20, SPEC §11): what the world holds as motion and as stored energy, so that where it goes can be
//! counted. Mana is one ledger; Energy is the other, and each balances on its own.
//!
//! Units: kg·(m/tick)², the kilogram being the mass of 1 M of free mana. One is 900 J.

use indexmap::IndexMap;

use super::air::{AIR_GAS, RAW_MASS, scale_height};
use super::fluid::{buoyed, parcels, stored_in_fluid};
use super::parts::total;
use super::physics::physics;
use super::world::{World, mass_of, mass_of_parts};
use crate::js;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stored {
    /// ½mv² of every particle, the air and every body.
    pub motion: f64,
    /// Weight lifted: of particles, the air, and the matter in the ground.
    pub height: f64,
    /// Mana's gas pressed together, matter packed past full, and what coheres pulled apart.
    pub gas: f64,
    pub packing: f64,
    pub cohesion: f64,
    /// The air pressed together.
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
    let mut motion = 0.0;
    let mut height = 0.0;
    // A particle's height is worth its weight, less what the air holds up of it. The air holds it up from the height
    // where the air at rest is as thick as it is on average: there, a parcel of mana is worth what the same mana is worth
    // as air (at rest, mana in the air is worth the same at any height, its weight and its pressure trading off), so mana
    // settling out of a parcel into the air brings no energy with it.
    let in_cell = parcels(world, 0..world.particles.len());
    let raw = RAW_MASS();
    let level = air_level(world);
    for p in &world.particles {
        let m = mass_of(p);
        motion += 0.5 * m * (js::pow(p.vel[0], 2.0) + js::pow(p.vel[1], 2.0) + js::pow(p.vel[2], 2.0));
        let c = world.cell_of(&p.pos);
        height += g
            * (m * p.pos[1]
                - total(&p.free) * raw * buoyed(world, c, in_cell.get(&c).copied().unwrap_or(0.0)) * (p.pos[1] - level));
    }
    for b in &world.bodies {
        motion += 0.5 * b.mass * (js::pow(b.vel[0], 2.0) + js::pow(b.vel[1], 2.0) + js::pow(b.vel[2], 2.0));
    }
    let mut air = 0.0;
    let c2 = AIR_GAS();
    // Air stores energy pressed denser, or drawn thinner, than the rest of it is: c²(ln(ρ/ρ̄) + ρ̄/ρ − 1) for each M,
    // which is nothing at the air's own density. So mana let into the air, or gathered from it, as dense as it is
    // brings or takes no energy with it.
    let usual = usual_air(world);
    for i in 0..world.size {
        let mm = total(&world.air[i]);
        let y = (world.coords(i)[1] as f64 + 0.5) * world.cell;
        if mm > 0.0 {
            let v = &world.air_vel;
            let mass = mass_of_parts(&world.air[i]);
            motion += 0.5 * mass * (js::pow(v[i * 3], 2.0) + js::pow(v[i * 3 + 1], 2.0) + js::pow(v[i * 3 + 2], 2.0));
            height += g * mass * y;
        }
        // An empty cell open to the air holds c²ρ̄: what the air around it would give, rushing in.
        if !world.solid_at(i as isize) && usual > 0.0 {
            air += c2 * (if mm > 0.0 { mm * js::log(mm / usual) + usual - mm } else { usual });
        }
        let mt = &world.matter[i];
        let mut w = 0.0;
        for k in 0..4 {
            w += mt[k] * ph.mana_mass[k];
        }
        if w != 0.0 && !w.is_nan() {
            height += g * w * y;
        }
    }
    let fluid = stored_in_fluid(world, if usual > 0.0 { usual / js::pow(world.cell, 3.0) } else { 1.0 });
    Stored {
        motion,
        height,
        gas: fluid.gas,
        packing: fluid.packing,
        cohesion: fluid.cohesion,
        air,
        bodies: g * raw * level * in_bodies,
    }
}

/// The height where the air at rest is as thick as the air is on average, metres.
pub fn air_level(world: &World) -> f64 {
    let hh = scale_height();
    let mut mana = 0.0;
    let mut shape = 0.0;
    let mut cells = 0usize;
    for i in 0..world.size {
        if !world.solid_at(i as isize) {
            mana += total(&world.air[i]);
            shape += js::exp(-((world.coords(i)[1] as f64 + 0.5) * world.cell) / hh);
            cells += 1;
        }
    }
    if mana <= 0.0 || cells == 0 {
        return 0.0;
    }
    // At rest, M(y) = S·e^(−y/H) with S = mana / shape; it's mana/cells at y = H·ln(S·cells/mana).
    hh * js::log(cells as f64 / shape)
}

/// How much mana a cell of air holds, on average, among the cells open to it.
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
    s.motion + s.height + s.gas + s.packing + s.cohesion + s.air + s.bodies
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
