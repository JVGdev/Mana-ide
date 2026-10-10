//! The air (SPEC §11, PLAN step 3): real air. The open air is air matter, a gas, with free mana in it, and flame is a gas
//! too. A cell's air moves as one: its gases and the free mana in them, at one speed.
//!
//! Every gas presses by the ideal gas law, p = n·R·T, with n its moles a m³: every M of any part is as many moles
//! (`moles_per_m`), so a cell's air presses by all the M in it, whatever they are, over the room it has. That room is
//! the cell, less what takes room in it and isn't gas: earth and water, and bodies. Free mana in particles is a gas too,
//! and takes its share of the room by its M (Dalton's law). So everything in a cell feels the same pressure, and each
//! thing in it is pushed by its share of the cell's room: −V∇p. That push, from the air's weight, is what holds up a
//! parcel of mana lighter than the air around it, and a gas lighter than the air rises by it; from a wind, it's the
//! wind's pressure on what's in its way.
//!
//! It's a finite-volume scheme on the grid. Between every two open neighbouring cells, each step, the air and its
//! momentum flow from the one upwind, and the pressure on the face between them pushes them apart, equally and
//! oppositely. Against a solid cell or the world's edge the face is closed: nothing flows through it, and what the air
//! pushes on it goes to the matter there, or to the world from outside (`impulse.walls`). Sound is the air's pressure
//! and motion rippling, and crosses it at √(R·T / molar mass): 280 m/s in the world's air. *(Isothermal: real sound is
//! 343 m/s because air warms as it's pressed, which needs heat, PLAN step 4.)*
//!
//! At rest the air is thicker low down than high up: its weight presses it down, and its pressure holds it up. That
//! balance is kept exactly: the pressure the air at rest would have, and the push it would give, are taken out, so only
//! what's different from rest pushes or falls, and air at rest, as thick as rest would have it, is skipped.
//!
//! Bodies and matter in the air feel it: its pressure, by the room they take, and its drag, as it flows past them.

use super::matter::room_of;
use super::parts::{Parts, total};
use super::physics::{gas_energy, physics};
use super::world::{Particle, Vec3, World, mass_of, mass_of_parts};
use crate::js;

/// The most of a cell's air that may leave it through one face in one step.
const MOST: f64 = 0.15;
/// Slower than this (m/tick), air stops; and within this share of rest's thickness, stopped air is at rest.
const STOP: f64 = 5e-4;
const CALM: f64 = 5e-3;
/// The least share of a cell's room the air keeps, however much else is in it.
const LEAST_ROOM: f64 = 0.05;

/// The mass of one M of some air, kg: its mass over its M.
pub fn rest_mass(amount: f64, mass: f64) -> f64 {
    if amount > 0.0 { mass / amount } else { 0.0 }
}

/// How high air at rest thins by a factor of e, metres, for air whose M weigh `mean` kilograms each: R·T over the
/// weight of a mole of it. Air at rest is as thick, at height y, as e^(−y/H) times what it is at the ground.
pub fn scale_height(mean: f64) -> f64 {
    gas_energy() / (physics().gravity * mean)
}

/// How fast sound crosses air whose M weigh `mean` each, m/tick: √(p/ρ), isothermal.
pub fn sound_speed(mean: f64) -> f64 {
    (gas_energy() / mean).sqrt()
}

/// The mass of a cell of open air at the ground, when a world is made: what a particle's drag is measured against.
pub fn still_air_mass() -> f64 {
    let (mana, gas) = super::world::still_air();
    mass_of_parts(&mana) + mass_of_parts(&gas)
}

/// The cells nearest a point, and how much of it each takes (cloud in cell: linear, by how near it is to each cell's
/// centre): up to 8, only open cells, summing to 1, or none if none of them is open.
pub struct Nearest {
    pub n: usize,
    pub at: [(usize, f64); 8],
}

pub fn nearest(world: &World, open: &[bool], pos: &Vec3) -> Nearest {
    let dx = world.cell;
    let axis = |v: f64, n: usize| -> [(usize, f64); 2] {
        let f = v / dx - 0.5;
        let c = f.floor();
        let t = f - c;
        let c = c as i64;
        if c < 0 {
            [(0, 1.0), (0, 0.0)]
        } else if c + 1 >= n as i64 {
            [(n - 1, 1.0), (0, 0.0)]
        } else {
            [(c as usize, 1.0 - t), (c as usize + 1, t)]
        }
    };
    let xs = axis(pos[0], world.w);
    let ys = axis(pos[1], world.h);
    let zs = if world.d == 1 { [(0, 1.0), (0, 0.0)] } else { axis(pos[2], world.d) };
    let mut out = Nearest { n: 0, at: [(0, 0.0); 8] };
    let mut sum = 0.0;
    for &(z, wz) in &zs {
        for &(y, wy) in &ys {
            for &(x, wx) in &xs {
                let wt = wx * wy * wz;
                if wt <= 0.0 {
                    continue;
                }
                let i = x + world.w * (y + world.h * z);
                if !open[i] {
                    continue;
                }
                out.at[out.n] = (i, wt);
                out.n += 1;
                sum += wt;
            }
        }
    }
    for k in 0..out.n {
        out.at[k].1 /= sum;
    }
    out
}

/// What takes room in each cell this tick, and how the air would rest: worked out from where everything is.
pub struct Room {
    /// Which cells are open to the air: not solid.
    pub open: Vec<bool>,
    /// Each open cell's open neighbour on each side (−1 where the face is closed), x, y, z.
    next: Vec<isize>,
    prev: Vec<isize>,
    /// The share of each cell's room the air has: what earth, water, bodies and parcels of mana don't take.
    pub air: Vec<f64>,
    /// The share earth and water take, and bodies.
    pub matter: Vec<f64>,
    pub bodies: Vec<f64>,
    /// The share free mana in particles takes, and the share each M of it takes: a parcel of mana is a gas at the air's
    /// pressure, so it takes as much room as as many M of the air there (Dalton). Each particle is spread over the
    /// cells nearest it. Where there's no air, it takes none.
    pub parcels: Vec<f64>,
    pub per_m: Vec<f64>,
    /// M a m³ in the air at rest, row by row, and what one M of it weighs, kg.
    pub rest: Vec<f64>,
    pub mean: f64,
    /// How fast sound crosses the air, m/tick: 0 if there's none.
    pub sound: f64,
}

impl Room {
    /// The air's gas in an open cell, M: what presses there.
    pub fn amount(&self, world: &World, i: usize) -> f64 {
        world.air_amount(i)
    }

    /// M in a cell's air at rest: rest's thickness at its height, over the room its air has.
    pub fn at_rest(&self, world: &World, i: usize) -> f64 {
        self.rest[(i / world.w) % world.h] * js::pow(world.cell, 3.0) * self.air[i]
    }
}

/// The share of each cell of a box (centre, half-size) that it takes, by cell: (cell, volume of the box in it, m³).
fn overlap(world: &World, pos: &Vec3, half: &Vec3) -> Vec<(usize, f64)> {
    let dx = world.cell;
    let mut out = Vec::new();
    let range = |a: usize, n: usize| -> (i64, i64) {
        let lo = ((pos[a] - half[a]) / dx).floor() as i64;
        let hi = ((pos[a] + half[a]) / dx).floor() as i64;
        (lo.max(0), hi.min(n as i64 - 1))
    };
    let (x0, x1) = range(0, world.w);
    let (y0, y1) = range(1, world.h);
    let (z0, z1) = if world.d == 1 { (0, 0) } else { range(2, world.d) };
    let span = |a: usize, c: i64| -> f64 {
        let lo = js::max(c as f64 * dx, pos[a] - half[a]);
        let hi = js::min((c + 1) as f64 * dx, pos[a] + half[a]);
        js::max(0.0, hi - lo)
    };
    for z in z0..=z1 {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let v = span(0, x) * span(1, y) * span(2, z);
                if v > 0.0 {
                    out.push((world.index(x, y, z) as usize, v));
                }
            }
        }
    }
    out
}

/// The room a body takes in each cell it's in: its own volume (its mass over `body_density`), spread over its box.
/// (cell, the share of the cell it takes, the share of its box in it).
pub fn body_room(world: &World, b: usize) -> Vec<(usize, f64, f64)> {
    let body = &world.bodies[b];
    let cells = overlap(world, &body.pos, &body.half);
    let in_world: f64 = cells.iter().map(|c| c.1).sum();
    if in_world <= 0.0 {
        return vec![];
    }
    let volume = js::pow(world.cell, 3.0);
    let own = body.mass / physics().body_density;
    let full = 8.0 * body.half[0] * body.half[1] * body.half[2];
    cells.into_iter().map(|(i, v)| (i, v * (own / full) / volume, v / in_world)).collect()
}

/// What takes room in each cell, and how the air would rest.
pub fn room(world: &World) -> Room {
    let n = world.size;
    let open: Vec<bool> = (0..n).map(|i| !world.solid_at(i as isize)).collect();
    let mut next = vec![-1isize; n * 3];
    let mut prev = vec![-1isize; n * 3];
    let (w, h, d) = (world.w, world.h, world.d);
    for z in 0..d {
        for y in 0..h {
            for x in 0..w {
                let i = x + w * (y + h * z);
                if !open[i] {
                    continue;
                }
                if x + 1 < w && open[i + 1] {
                    next[i * 3] = (i + 1) as isize;
                    prev[(i + 1) * 3] = i as isize;
                }
                if y + 1 < h && open[i + w] {
                    next[i * 3 + 1] = (i + w) as isize;
                    prev[(i + w) * 3 + 1] = i as isize;
                }
                if z + 1 < d && open[i + w * h] {
                    next[i * 3 + 2] = (i + w * h) as isize;
                    prev[(i + w * h) * 3 + 2] = i as isize;
                }
            }
        }
    }
    let mut matter = vec![0.0; n];
    let mut bodies = vec![0.0; n];
    for i in 0..n {
        if open[i] {
            matter[i] = world.fill(i);
        }
    }
    for b in 0..world.bodies.len() {
        for (i, share, _) in body_room(world, b) {
            if open[i] {
                bodies[i] += share;
            }
        }
    }
    let mut held = vec![0.0; n];
    for p in &world.particles {
        let m = total(&p.free);
        if m <= 0.0 {
            continue;
        }
        let near = nearest(world, &open, &p.pos);
        for &(i, wt) in &near.at[..near.n] {
            held[i] += m * wt;
        }
    }
    let volume = js::pow(world.cell, 3.0);
    // The air at rest: all its gas in the open cells, laid out as thick at each height as its weight would settle it,
    // over the room the air has. The room depends on how thick it is (parcels of mana take as much as as many M of
    // it), so it's worked out twice.
    let mut amount = 0.0;
    let mut mass = 0.0;
    for i in 0..n {
        if open[i] {
            amount += world.air_amount(i);
            mass += world.air_mass(i);
        }
    }
    let mean = rest_mass(amount, mass);
    let rows: Vec<f64> = if amount > 0.0 {
        let hh = scale_height(mean);
        (0..h).map(|y| js::exp(-((y as f64 + 0.5) * world.cell) / hh)).collect()
    } else {
        vec![0.0; h]
    };
    let (matter_alone, bodies_alone) = (matter.clone(), bodies.clone());
    let mut rest = vec![0.0; h];
    let mut air = vec![0.0; n];
    let mut parcels = vec![0.0; n];
    let mut per_m = vec![0.0; n];
    for _ in 0..2 {
        for i in 0..n {
            if !open[i] {
                continue;
            }
            let r = rest[(i / w) % h];
            per_m[i] = if r > 0.0 { 1.0 / (r * volume) } else { 0.0 };
            parcels[i] = held[i] * per_m[i];
            matter[i] = matter_alone[i];
            bodies[i] = bodies_alone[i];
            let others = matter[i] + bodies[i] + parcels[i];
            air[i] = 1.0 - others;
            if air[i] < LEAST_ROOM {
                // Crowded past what it can hold: everything in it gives up some room to the air's least.
                let f = (1.0 - LEAST_ROOM) / others;
                matter[i] *= f;
                bodies[i] *= f;
                parcels[i] *= f;
                per_m[i] *= f;
                air[i] = LEAST_ROOM;
            }
        }
        let mut shape = 0.0;
        for i in 0..n {
            if open[i] {
                shape += rows[(i / w) % h] * volume * air[i];
            }
        }
        if amount <= 0.0 || shape <= 0.0 {
            break;
        }
        for y in 0..h {
            rest[y] = amount / shape * rows[y];
        }
    }
    let sound = if mean > 0.0 { sound_speed(mean) } else { 0.0 };
    Room { open, next, prev, air, matter, bodies, parcels, per_m, rest, mean, sound }
}

/// How hard the air at rest holds up each M of free mana in particles in each cell (kg·m/tick² a M): the push the
/// pressure of the air at rest gives the whole cell, its weight's worth, by the share of the cell's room the mana takes.
/// What an M of the air weighs (Archimedes). The ground under the air gives it.
pub fn lift(world: &World, room: &Room) -> Vec<f64> {
    let g = physics().gravity;
    let volume = js::pow(world.cell, 3.0);
    (0..world.size).map(|i| g * room.mean * room.rest[(i / world.w) % world.h] * volume * room.per_m[i]).collect()
}

/// One tick of the air: it flows, presses, falls under its weight, pushes on what's in it and is dragged by it, and
/// evens out its speed with its neighbours. `holder` says which body's hand holds a particle, if any: the hand takes
/// what the air pushes on it.
pub fn step_air(world: &mut World, holder: &dyn Fn(&Particle) -> Option<usize>) {
    let ph = physics();
    let n = world.size;
    let dx = world.cell;
    let area = dx * dx;
    let volume = dx * dx * dx;
    let axes = if world.d == 1 { 2 } else { 3 };
    let room = room(world);
    let theta = gas_energy();
    let g = ph.gravity;
    let mean = room.mean;
    let (w, h) = (world.w, world.h);
    let row = move |i: usize| (i / w) % h;
    // The push the pressure of the air at rest gives each cell, all of it, upward: what the air at rest there weighs.
    let push: Vec<f64> = (0..n).map(|i| g * mean * room.rest[row(i)] * volume).collect();
    // What else is in a cell besides its air, which the air pushes on: matter, bodies, parcels of mana.
    let crowded: Vec<bool> =
        (0..n).map(|i| room.open[i] && (room.matter[i] > 0.0 || room.bodies[i] > 0.0 || room.parcels[i] > 0.0)).collect();
    // Air at rest: as slow as air that stops, and within a share of rest's thickness. (Ripples fainter than that
    // aren't followed: they'd keep every cell of the world busy, for nothing a thing in it would feel.)
    let quiet = |world: &World, i: usize| {
        let v = &world.air_vel;
        if crowded[i] || v[i * 3] * v[i * 3] + v[i * 3 + 1] * v[i * 3 + 1] + v[i * 3 + 2] * v[i * 3 + 2] > STOP * STOP {
            return false;
        }
        let r = room.at_rest(world, i);
        (world.air_amount(i) - r).abs() <= CALM * r && (world.air_mass(i) - r * mean).abs() <= CALM * r * mean
    };
    // Air that has all but stopped stops: what little motion it had goes into the ground, as the air's thickness would
    // have taken it there anyway.
    for i in 0..n {
        if !room.open[i] {
            continue;
        }
        let v = &world.air_vel;
        let s2 = js::pow(v[i * 3], 2.0) + js::pow(v[i * 3 + 1], 2.0) + js::pow(v[i * 3 + 2], 2.0);
        if s2 == 0.0 || s2 > STOP * STOP {
            continue;
        }
        let m = world.air_mass(i);
        world.warm("the air", 0.5 * m * s2);
        for k in 0..3 {
            world.impulse.walls[k] -= m * world.air_vel[i * 3 + k];
            world.air_vel[i * 3 + k] = 0.0;
        }
    }

    // What the air pushes on, over the tick: each M of mana in particles in each cell, the matter and the bodies in
    // each open cell, and the matter in each solid cell, through its faces.
    let mut on_parcels = vec![0.0; n * 3];
    let mut on_matter = vec![0.0; n * 3];
    let mut on_bodies = vec![0.0; n * 3];
    let mut on_solid = vec![0.0; n * 3];

    // How thick the air is on average (what its Energy is measured from), and what each M of it is made of: as the air
    // beyond the world is.
    let (usual, like) = average(world, &room);
    let mut dragged = false;
    let mut active: Vec<usize> = (0..n).filter(|&i| room.open[i] && !quiet(world, i)).collect();
    if !active.is_empty() && room.sound > 0.0 {
        // Steps enough that nothing crosses more than a share of a cell in one: the fastest wind or sound sets it. The
        // share is less the more ways the air can go (2D or 3D): the faces' evening out of pressure spreads it along
        // every axis at once, and stepped too far, it overshoots and grows.
        let sound = room.sound;
        let mut fastest = 0.0;
        for &i in &active {
            let v = &world.air_vel;
            fastest = js::max(fastest, js::hypot3(v[i * 3], v[i * 3 + 1], v[i * 3 + 2]) + sound);
        }
        let steps = js::max(1.0, (fastest / (0.8 / axes as f64 * dx)).ceil()) as usize;
        let dt = 1.0 / steps as f64;

        let mut mm = vec![0.0; n];
        let mut pp = vec![0.0; n * 3];
        let mut force = vec![0.0; n * 3];
        let mut press = vec![0.0; n];
        let mut flow_air = vec![0.0; n * 4];
        let mut flow_gas = vec![0.0; n * 4];
        let mut moved = vec![0.0; n * 3];
        let mut u = vec![0.0; n * 3];
        let mut busy = vec![false; n];
        let mut touched: Vec<usize> = Vec::new();
        let mut faces: Vec<(isize, isize, usize)> = Vec::new();
        // The faces at the world's open edges this step: (cell, axis, side, how fast the air goes out through it).
        let mut edges: Vec<(usize, usize, i64, f64)> = Vec::new();
        let mut is_active = vec![false; n];
        for s in 0..steps {
            if s > 0 {
                // Only what the last step touched can have changed: the rest is as quiet as it was.
                for &i in &active {
                    is_active[i] = false;
                }
                active = touched.iter().copied().filter(|&i| !quiet(world, i)).collect();
                if active.is_empty() {
                    break;
                }
            }
            for &i in &active {
                is_active[i] = true;
            }
            // The active cells, and their neighbours: everything a face of an active cell touches.
            for &i in &touched {
                busy[i] = false;
            }
            touched.clear();
            let mut mark = |i: usize, touched: &mut Vec<usize>| {
                if !busy[i] {
                    busy[i] = true;
                    touched.push(i);
                }
            };
            for &i in &active {
                mark(i, &mut touched);
                for ax in 0..axes {
                    if room.next[i * 3 + ax] >= 0 {
                        mark(room.next[i * 3 + ax] as usize, &mut touched);
                    }
                    if room.prev[i * 3 + ax] >= 0 {
                        mark(room.prev[i * 3 + ax] as usize, &mut touched);
                    }
                }
            }
            touched.sort_unstable();
            for &i in &touched {
                let mass = world.air_mass(i);
                mm[i] = mass;
                for k in 0..3 {
                    pp[i * 3 + k] = mass * world.air_vel[i * 3 + k];
                    force[i * 3 + k] = 0.0;
                    moved[i * 3 + k] = 0.0;
                }
                // How much harder (or softer) it presses than the air at rest there.
                press[i] = theta * (room.amount(world, i) / (volume * room.air[i]) - room.rest[row(i)]);
                for k in 0..4 {
                    flow_air[i * 4 + k] = 0.0;
                    flow_gas[i * 4 + k] = 0.0;
                }
            }
            // Each face an active cell has, once: (i, j, axis), j = −1 for a closed face on i's + side, and i = −1 for
            // one on j's − side.
            faces.clear();
            edges.clear();
            for &i in &active {
                for ax in 0..axes {
                    faces.push((i as isize, room.next[i * 3 + ax], ax));
                    let k = room.prev[i * 3 + ax];
                    // The face on i's − side, unless its other cell is active and does it.
                    if k < 0 || !is_active[k as usize] {
                        faces.push((k, i as isize, ax));
                    }
                }
            }
            // The pressure on every face pushes what's on either side of it away from the other. A closed face pushes
            // back with the air's own pressure: the matter there takes it, or the world's edge.
            for &(i, j, ax) in &faces {
                if i >= 0 && j >= 0 {
                    let (i, j) = (i as usize, j as usize);
                    let f = ((press[i] + press[j]) / 2.0) * area * dt;
                    force[i * 3 + ax] -= f;
                    force[j * 3 + ax] += f;
                } else {
                    let (i, side) = if j < 0 { (i as usize, 1) } else { (j as usize, -1) };
                    let s = side as f64;
                    if open_edge(world, i, ax, side) {
                        // Past the world's edge there's more air, at rest. What reaches the edge goes on: the air at the
                        // face presses as a wave going out does, (p + ρ·c·u)/2, and moves at that over ρ·c (no wave
                        // comes back in).
                        let rho = mean * room.rest[row(i)];
                        let un = s * world.air_vel[i * 3 + ax];
                        let (p, out) = if rho > 0.0 {
                            let p = (press[i] + rho * room.sound * un) / 2.0;
                            (p, p / (rho * room.sound))
                        } else {
                            (0.0, js::max(0.0, un))
                        };
                        let f = s * p * area * dt;
                        force[i * 3 + ax] -= f;
                        world.impulse.beyond[ax] -= f;
                        edges.push((i, ax, side, out));
                    } else {
                        let f = s * press[i] * area * dt;
                        force[i * 3 + ax] -= f;
                        against(world, &mut on_solid, i, ax, side, f);
                    }
                }
            }
            // Each thing in a cell takes its share of what's pushed on the cell, by the room it takes: the air (its
            // gases by their M, and the mana in particles by its), the matter and the bodies. And the air falls by what
            // it weighs, and is held up by its share of the air at rest's push.
            for &i in &touched {
                let share = if mm[i] > 0.0 { room.air[i] } else { 0.0 };
                for k in 0..3 {
                    let f = force[i * 3 + k];
                    pp[i * 3 + k] += f * share;
                    on_parcels[i * 3 + k] += f * room.per_m[i];
                    on_matter[i * 3 + k] += f * room.matter[i];
                    on_bodies[i * 3 + k] += f * room.bodies[i];
                    if mm[i] <= 0.0 {
                        // Nothing in its air to push: the world takes it.
                        world.impulse.walls[k] -= f * room.air[i];
                    }
                }
                // (Its weight, and the push of the ground through the air at rest, as one: they all but cancel, and
                // counted apart they'd round.)
                let net = (push[i] * share - g * mm[i]) * dt;
                pp[i * 3 + 1] += net;
                world.impulse.gravity[1] += net;
                // Matter is held up by its share of it too (bodies stand: the ground holds them).
                let up = push[i] * room.matter[i] * dt;
                on_matter[i * 3 + 1] += up;
                world.impulse.walls[1] += up;
            }
            for &i in &touched {
                for k in 0..3 {
                    u[i * 3 + k] = if mm[i] > 0.0 { pp[i * 3 + k] / mm[i] } else { 0.0 };
                }
            }
            // Then the air, at the speed it now has, carries its gases, its mana and its momentum across each open face,
            // from the cell upwind.
            for &(i, j, ax) in &faces {
                if i < 0 || j < 0 {
                    continue;
                }
                let (i, j) = (i as usize, j as usize);
                // The air at a face moves as sound meeting there would have it: at the speed of the cells either side,
                // and from the side that presses harder, by the difference over 2·ρ·c. (Without it, cells pressing
                // harder and softer by turns, all along, push on nothing and never even out.)
                let rho = mean * room.rest[row(i)];
                let speed = (u[i * 3 + ax] + u[j * 3 + ax]) / 2.0
                    - if rho > 0.0 { (press[j] - press[i]) / (2.0 * rho * room.sound) } else { 0.0 };
                if speed == 0.0 {
                    continue;
                }
                let (from, to) = if speed > 0.0 { (i, j) } else { (j, i) };
                if mm[from] <= 0.0 {
                    continue;
                }
                let share = js::min(MOST, (speed.abs() * dt) / dx);
                let (a, gs) = (world.air[from], world.gas[from]);
                for k in 0..4 {
                    let q = a[k] * share;
                    flow_air[from * 4 + k] -= q;
                    flow_air[to * 4 + k] += q;
                    let q = gs[k] * share;
                    flow_gas[from * 4 + k] -= q;
                    flow_gas[to * 4 + k] += q;
                }
                for k in 0..3 {
                    let q = pp[from * 3 + k] * share;
                    moved[from * 3 + k] -= q;
                    moved[to * 3 + k] += q;
                }
            }
            // Through the open edges: the air goes out, carrying what it holds and its Energy beyond the world, or the air
            // beyond comes in, at rest, as the world's air is on average.
            for &(i, _, _, out) in &edges {
                let y = (row(i) as f64 + 0.5) * dx;
                if out > 0.0 {
                    if mm[i] <= 0.0 {
                        continue;
                    }
                    let share = js::min(MOST, (out * dt) / dx);
                    let (a, gs) = (world.air[i], world.gas[i]);
                    let n_here = world.air_amount(i) / (volume * room.air[i]);
                    let speed2 = (0..3).map(|k| js::pow(u[i * 3 + k], 2.0)).sum::<f64>();
                    let amount = (total(&a) + total(&gs)) * share;
                    for k in 0..4 {
                        flow_air[i * 4 + k] -= a[k] * share;
                        flow_gas[i * 4 + k] -= gs[k] * share;
                        world.beyond[k] += a[k] * share;
                        world.beyond_gas[k] += gs[k] * share;
                    }
                    for k in 0..3 {
                        let q = pp[i * 3 + k] * share;
                        moved[i * 3 + k] -= q;
                        world.impulse.beyond[k] -= q;
                    }
                    world.beyond_energy +=
                        amount * theta * js::log(n_here / usual) + mm[i] * share * (0.5 * speed2 + g * y);
                } else if out < 0.0 {
                    let amount = js::min(MOST * room.at_rest(world, i), -out * dt * area * room.rest[row(i)]);
                    for k in 0..4 {
                        let (a, gs) = (amount * like.0[k], amount * like.1[k]);
                        flow_air[i * 4 + k] += a;
                        flow_gas[i * 4 + k] += gs;
                        world.beyond[k] -= a;
                        world.beyond_gas[k] -= gs;
                    }
                    world.beyond_energy -= amount * (theta * js::log(room.rest[row(i)] / usual) + mean * g * y);
                }
            }
            for &i in &touched {
                for k in 0..4 {
                    world.air[i][k] = js::max(0.0, world.air[i][k] + flow_air[i * 4 + k]);
                    world.gas[i][k] = js::max(0.0, world.gas[i][k] + flow_gas[i * 4 + k]);
                }
                let m = world.air_mass(i);
                for k in 0..3 {
                    let p = pp[i * 3 + k] + moved[i * 3 + k];
                    if m > 1e-12 {
                        world.air_vel[i * 3 + k] = p / m;
                    } else {
                        // A cell emptied keeps no momentum: what it had went with its air.
                        world.air_vel[i * 3 + k] = 0.0;
                        world.impulse.walls[k] -= p;
                    }
                }
            }
            // And it drags on what it flows past, step by step: at speed, a cell's air is gone past in a few.
            drag(world, &room, dt);
            dragged = true;
        }
    }
    if !dragged {
        drag(world, &room, 1.0);
    }
    give(world, &room, holder, &on_parcels, &on_matter, &on_bodies, &on_solid);
    viscosity(world, &room, axes);
}

/// How thick the air is on average, M a m³ (what its Energy is measured from), and what each M of it is made of, by
/// part: free mana, and gases. The air beyond the world is like it, at rest.
pub fn average(world: &World, room: &Room) -> (f64, (Parts, Parts)) {
    let volume = js::pow(world.cell, 3.0);
    let mut amount = 0.0;
    let mut space = 0.0;
    let mut mana = [0.0; 4];
    let mut gas = [0.0; 4];
    for i in 0..world.size {
        if room.open[i] {
            amount += world.air_amount(i);
            space += volume * room.air[i];
            for k in 0..4 {
                mana[k] += world.air[i][k];
                gas[k] += world.gas[i][k];
            }
        }
    }
    let per = if amount > 0.0 { 1.0 / amount } else { 0.0 };
    (if space > 0.0 { amount / space } else { 0.0 }, (mana.map(|m| m * per), gas.map(|m| m * per)))
}

/// The Energy one M of the air at rest holds in cell `i`, as the ledger counts it: R·T·ln(n/n̄) for how thick rest is
/// there against the average, and its weight's height, for M weighing `mass` each.
fn at_rest_energy(world: &World, room: &Room, i: usize, usual: f64, mass: f64) -> f64 {
    let r = room.rest[(i / world.w) % world.h];
    let y = (world.coords(i)[1] as f64 + 0.5) * world.cell;
    (if r > 0.0 && usual > 0.0 { gas_energy() * js::log(r / usual) } else { 0.0 }) + mass * physics().gravity * y
}

/// `amount` M of the air beyond the world, at rest, comes into cell `i` (seeping through the ground), bringing the
/// Energy air at rest holds there.
pub fn from_beyond(world: &mut World, room: &Room, avg: &(f64, (Parts, Parts)), i: usize, amount: f64) {
    let (usual, like) = *avg;
    if amount <= 0.0 || usual <= 0.0 {
        return;
    }
    let mana = like.0.map(|f| f * amount);
    let gas = like.1.map(|f| f * amount);
    let mass = mass_of_parts(&mana) + mass_of_parts(&gas);
    // It comes in at rest: what the cell's air then moves at is the same momentum over more mass, and the motion that
    // takes away is heat (join_air).
    world.join_air(i, mass, [0.0; 3]);
    for k in 0..4 {
        world.air[i][k] += mana[k];
        world.gas[i][k] += gas[k];
        world.beyond[k] -= mana[k];
        world.beyond_gas[k] -= gas[k];
    }
    world.beyond_energy -= amount * at_rest_energy(world, room, i, usual, mass / amount);
}

/// `amount` M of the air of cell `i` goes beyond the world (seeping through the ground), taking its momentum, its
/// motion and the Energy air at rest holds there.
pub fn to_beyond(world: &mut World, room: &Room, avg: &(f64, (Parts, Parts)), i: usize, amount: f64) {
    let usual = avg.0;
    let there = world.air_amount(i);
    if amount <= 0.0 || there <= 0.0 || usual <= 0.0 {
        return;
    }
    let f = js::min(1.0, amount / there);
    let mana = super::parts::share(&mut world.air[i], f);
    let gas = super::parts::share(&mut world.gas[i], f);
    let gone = total(&mana) + total(&gas);
    let mass = mass_of_parts(&mana) + mass_of_parts(&gas);
    let u = [world.air_vel[i * 3], world.air_vel[i * 3 + 1], world.air_vel[i * 3 + 2]];
    for k in 0..3 {
        world.impulse.beyond[k] -= mass * u[k];
    }
    for k in 0..4 {
        world.beyond[k] += mana[k];
        world.beyond_gas[k] += gas[k];
    }
    world.beyond_energy +=
        gone * at_rest_energy(world, room, i, usual, mass / gone) + 0.5 * mass * (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]);
}

/// The open cells that no way through the open air joins to the world's open edges: hollows sealed in the ground.
pub fn sealed(world: &World, room: &Room) -> Vec<bool> {
    let n = world.size;
    let axes = if world.d == 1 { 2 } else { 3 };
    let mut reached = vec![false; n];
    let mut queue: Vec<usize> = Vec::new();
    for i in 0..n {
        if !room.open[i] {
            continue;
        }
        let at_edge = (0..axes).any(|ax| open_edge(world, i, ax, 1) || open_edge(world, i, ax, -1));
        if at_edge {
            reached[i] = true;
            queue.push(i);
        }
    }
    while let Some(i) = queue.pop() {
        for ax in 0..axes {
            for j in [room.next[i * 3 + ax], room.prev[i * 3 + ax]] {
                if j >= 0 && !reached[j as usize] {
                    reached[j as usize] = true;
                    queue.push(j as usize);
                }
            }
        }
    }
    (0..n).map(|i| room.open[i] && !reached[i]).collect()
}

/// Whether the face of open cell `i` along axis `ax`, on its `side` (+1 or −1), is the world's edge open to the air beyond:
/// the sides and the sky are, the floor isn't.
pub fn open_edge(world: &World, i: usize, ax: usize, side: i64) -> bool {
    let mut at = world.coords(i);
    at[ax] += side;
    world.index(at[0], at[1], at[2]) < 0 && !(ax == 1 && side < 0) && !(ax == 2 && world.d == 1)
}

/// The air in open cell `i` pushes `f` on its closed face along axis `ax`, on its `side` (+1 or −1): the matter in the
/// solid cell beyond takes it, or the world's edge does.
fn against(world: &mut World, on_solid: &mut [f64], i: usize, ax: usize, side: i64, f: f64) {
    let mut at = world.coords(i);
    at[ax] += side;
    let j = world.index(at[0], at[1], at[2]);
    if j >= 0 {
        on_solid[j as usize * 3 + ax] += f;
    } else {
        // The world gives back what the air pushes on it.
        world.impulse.walls[ax] -= f;
    }
}

/// What the air pushed on, over the tick, gets it: particles (a hand takes it for what's in it), the matter in each cell
/// (what sleeps holds still, as the ground does, unless it's pushed hard enough to wake), and bodies (they stand: the
/// ground takes what's up and down).
fn give(
    world: &mut World,
    room: &Room,
    holder: &dyn Fn(&Particle) -> Option<usize>,
    on_parcels: &[f64],
    on_matter: &[f64],
    on_bodies: &[f64],
    on_solid: &[f64],
) {
    for pi in 0..world.particles.len() {
        let p = &world.particles[pi];
        let m = total(&p.free);
        let mass = mass_of(p);
        if m <= 0.0 || mass <= 0.0 {
            continue;
        }
        let near = nearest(world, &room.open, &p.pos);
        let mut j = [0.0; 3];
        for &(i, wt) in &near.at[..near.n] {
            for k in 0..3 {
                j[k] += on_parcels[i * 3 + k] * m * wt;
            }
        }
        match holder(p) {
            Some(b) => {
                let body = &mut world.bodies[b];
                for k in [0, 2] {
                    body.vel[k] += j[k] / body.mass;
                }
                world.impulse.walls[1] -= j[1];
            }
            None => {
                let p = &mut world.particles[pi];
                for k in 0..3 {
                    p.vel[k] += j[k] / mass;
                }
            }
        }
    }
    let mut woke = false;
    for i in 0..world.size {
        for on in [on_matter, on_solid] {
            let j = [on[i * 3], on[i * 3 + 1], on[i * 3 + 2]];
            if j == [0.0; 3] {
                continue;
            }
            woke |= to_matter(world, i, j);
        }
    }
    if woke {
        world.points_moved();
    }
    // Each body takes of what the bodies in a cell got by its share of the room they take there.
    let mut all = vec![0.0; world.size];
    for b in 0..world.bodies.len() {
        for (i, share, _) in body_room(world, b) {
            all[i] += share;
        }
    }
    for b in 0..world.bodies.len() {
        for (i, share, _) in body_room(world, b) {
            if room.bodies[i] <= 0.0 || all[i] <= 0.0 {
                continue;
            }
            let f = share / all[i];
            let body = &mut world.bodies[b];
            for k in [0, 2] {
                body.vel[k] += on_bodies[i * 3 + k] * f / body.mass;
            }
            world.impulse.walls[1] -= on_bodies[i * 3 + 1] * f;
        }
    }
}

/// An impulse on the matter in cell `i` over a tick, shared among its points by the room each takes. A sleeping point
/// rests on what's under it, and holds still against a push no harder than its friction there (μ·m·g), as the ground
/// does; pushed harder, it wakes. Returns whether it woke any.
fn to_matter(world: &mut World, i: usize, j: Vec3) -> bool {
    let points = world.points_in(i).to_vec();
    let rooms: Vec<f64> = points.iter().map(|&pi| room_of(&world.points[pi].parts)).collect();
    let all: f64 = rooms.iter().sum();
    if all <= 0.0 {
        for k in 0..3 {
            world.impulse.walls[k] -= j[k];
        }
        return false;
    }
    let ph = physics();
    let mut woke = false;
    for (n, &pi) in points.iter().enumerate() {
        let f = rooms[n] / all;
        let pt = &mut world.points[pi];
        let dv = [j[0] * f / pt.mass, j[1] * f / pt.mass, j[2] * f / pt.mass];
        if pt.asleep && js::hypot3(dv[0], dv[1], dv[2]) <= js::max(ph.wake_speed, ph.friction * ph.gravity) {
            for k in 0..3 {
                world.impulse.walls[k] -= j[k] * f;
            }
            continue;
        }
        for k in 0..3 {
            pt.vel[k] += dv[k];
        }
        if pt.asleep {
            pt.asleep = false;
            pt.still = 0;
            woke = true;
        }
    }
    woke
}

/// The air drags on what it flows past, and is dragged by it: bodies and matter in its cells, by ½·ρ·C·A·u², equally and
/// oppositely, with what it takes from their motion turned to heat. A body stands, so the ground takes what's up and
/// down; matter that sleeps holds still. Over `dt` ticks, a speed u between them falls to u / (1 + k·|u|·dt),
/// k = ½·ρ·C·A/μ: what drag that grows with u² does.
fn drag(world: &mut World, room: &Room, dt: f64) {
    let dx = world.cell;
    let volume = dx * dx * dx;
    let mut heat = 0.0;
    // Bodies: the area each cell of a body's box shows the air along each axis is its share of the box's face.
    for b in 0..world.bodies.len() {
        let half = world.bodies[b].half;
        let cells = overlap(world, &world.bodies[b].pos, &half);
        for (i, v) in cells {
            if !room.open[i] {
                continue;
            }
            let area = [v / (2.0 * half[0]), v / (2.0 * half[1]), v / (2.0 * half[2])];
            let body = &world.bodies[b];
            let (mb, vb) = (body.mass, [body.vel[0], 0.0, body.vel[2]]);
            let (j, lost) = exchange(world, room, i, area, mb, vb, [false, true, false], dt);
            heat += lost;
            let body = &mut world.bodies[b];
            for k in [0, 2] {
                body.vel[k] += j[k] / mb;
            }
            world.impulse.walls[1] -= j[1];
        }
    }
    // Matter in open cells: a lump of it shows the air about the square of its size.
    for i in 0..world.size {
        if !room.open[i] || room.matter[i] <= 0.0 {
            continue;
        }
        let points = world.points_in(i).to_vec();
        let mut mass = 0.0;
        let mut p = [0.0; 3];
        let mut still = true;
        for &pi in &points {
            let pt = &world.points[pi];
            if pt.asleep {
                continue;
            }
            still = false;
            mass += pt.mass;
            for k in 0..3 {
                p[k] += pt.mass * pt.vel[k];
            }
        }
        let v = room.matter[i] * volume;
        let a = js::min(dx * dx, if world.d == 1 { (v * dx).sqrt() } else { js::pow(v, 2.0 / 3.0) });
        let vm = if still { [0.0; 3] } else { [p[0] / mass, p[1] / mass, p[2] / mass] };
        let (j, lost) = exchange(world, room, i, [a; 3], if still { f64::INFINITY } else { mass }, vm, [still; 3], dt);
        heat += lost;
        if still {
            for k in 0..3 {
                world.impulse.walls[k] -= j[k];
            }
        } else {
            for &pi in &points {
                let pt = &mut world.points[pi];
                if !pt.asleep {
                    for k in 0..3 {
                        pt.vel[k] += j[k] / mass;
                    }
                }
            }
        }
    }
    world.warm("the air dragging", heat);
}

/// The drag between the air of cell `i` and something in it of mass `m` moving at `v`, showing the air `area` along each
/// axis; along the axes `fixed`, it holds still (as if it had no end of mass). Changes the air's speed, and returns the
/// impulse the thing gets, and the heat.
fn exchange(world: &mut World, room: &Room, i: usize, area: Vec3, m: f64, v: Vec3, fixed: [bool; 3], dt: f64) -> (Vec3, f64) {
    let ph = physics();
    let ma = world.air_mass(i);
    if ma <= 0.0 {
        return ([0.0; 3], 0.0);
    }
    let rho = ma / (js::pow(world.cell, 3.0) * room.air[i]);
    let mut rel = [0.0; 3];
    for k in 0..3 {
        rel[k] = world.air_vel[i * 3 + k] - v[k];
    }
    if world.d == 1 {
        rel[2] = 0.0;
    }
    let speed = js::hypot3(rel[0], rel[1], rel[2]);
    let mut j = [0.0; 3];
    let mut heat = 0.0;
    if speed == 0.0 {
        return (j, heat);
    }
    for k in 0..3 {
        let mu = if fixed[k] || !m.is_finite() { ma } else { ma * m / (ma + m) };
        let kk = 0.5 * rho * ph.drag_coefficient * area[k] / mu;
        let after = rel[k] / (1.0 + kk * speed * dt);
        j[k] = mu * (rel[k] - after);
        heat += 0.5 * mu * (rel[k] * rel[k] - after * after);
        world.air_vel[i * 3 + k] -= j[k] / ma;
    }
    (j, heat)
}

/// The air carries its speed to its neighbours (it's thick), and the ground, walls and the world's edge hold still the
/// air against them.
fn viscosity(world: &mut World, room: &Room, axes: usize) {
    let k = physics().air_viscosity;
    let (open, next, prev) = (&room.open, &room.next, &room.prev);
    let mut heat = 0.0;
    // A share k of a speed taken away takes 1 − (1 − k)² of the energy in it.
    let lost = 1.0 - (1.0 - k) * (1.0 - k);
    let still = |world: &mut World, heat: &mut f64, a: usize, m: f64| {
        let v = &world.air_vel;
        *heat += 0.5 * m * (js::pow(v[a * 3], 2.0) + js::pow(v[a * 3 + 1], 2.0) + js::pow(v[a * 3 + 2], 2.0)) * lost;
        for i in 0..3 {
            let j = k * m * world.air_vel[a * 3 + i];
            world.air_vel[a * 3 + i] -= j / m;
            world.impulse.walls[i] -= j;
        }
    };
    let moving = |world: &World, i: usize| {
        world.air_vel[i * 3] != 0.0 || world.air_vel[i * 3 + 1] != 0.0 || world.air_vel[i * 3 + 2] != 0.0
    };
    for a in 0..world.size {
        if !open[a] {
            continue;
        }
        let ma = world.air_mass(a);
        if ma <= 0.0 {
            continue;
        }
        // Still air beside still air: nothing to even out.
        let mut near = moving(world, a);
        let mut ax = 0;
        while ax < axes && !near {
            let b = next[a * 3 + ax];
            if b >= 0 && moving(world, b as usize) {
                near = true;
            }
            ax += 1;
        }
        if !near {
            continue;
        }
        for ax in 0..axes {
            let b = next[a * 3 + ax];
            let mb = if b < 0 { 0.0 } else { world.air_mass(b as usize) };
            if mb <= 0.0 {
                if b >= 0 || !open_edge(world, a, ax, 1) {
                    still(world, &mut heat, a, ma);
                }
                continue;
            }
            let b = b as usize;
            let mu = (ma * mb) / (ma + mb);
            let mut rel2 = 0.0;
            for c in 0..3 {
                let rel = world.air_vel[a * 3 + c] - world.air_vel[b * 3 + c];
                rel2 += rel * rel;
                let j = k * mu * rel;
                world.air_vel[a * 3 + c] -= j / ma;
                world.air_vel[b * 3 + c] += j / mb;
            }
            heat += 0.5 * mu * rel2 * lost;
        }
        // The faces below, behind and to the side, which the loop above doesn't reach from this cell.
        for ax in 0..axes {
            let b = prev[a * 3 + ax];
            if (b < 0 && !open_edge(world, a, ax, -1)) || (b >= 0 && world.air_amount(b as usize) <= 0.0) {
                still(world, &mut heat, a, ma);
            }
        }
    }
    world.warm("the air", heat);
}
