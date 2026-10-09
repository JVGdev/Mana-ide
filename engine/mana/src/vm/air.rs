//! The air (SPEC §11): the free mana spread through the world's cells, as a gas. It presses from dense to thin, carries
//! itself along (and its mana, and its momentum, with it), flows around what's solid, and is held still against the
//! ground and the world's edge. Its pressure is its density in M times its gas constant, which is the square of its speed
//! of sound (PHYSICS.air_sound) times what a M of it weighs (RAW_MASS): the ideal gas law. That speed is fast enough
//! beside the winds in it that it flows around things, as air does, rather than piling up against them. It weighs, so at
//! rest it's thicker low down than high up: its own weight presses it down, and its pressure holds it up. That pressure,
//! thicker below than above, is what holds up a parcel of mana lighter than the air around it.
//!
//! It's a finite-volume scheme on the grid. Between every two open neighbouring cells, each step, mana and momentum flow
//! from the one upwind, and the pressure on the face between them pushes them apart, equally and oppositely. Against a
//! solid cell or the world's edge, the face is closed: nothing flows through it, and what the air pushes on it is
//! momentum given to the world from outside (`impulse.walls`).

use super::parts::total;
use super::physics::physics;
use super::world::{World, mass_of_parts};
use crate::js;

/// The most of a cell's mana that may leave it through one face in one step.
const MOST: f64 = 0.15;
/// Slower than this (m/tick), air stops; and within this share of still air's density, stopped air is at rest.
const STOP: f64 = 5e-4;
const CALM: f64 = 5e-3;

/// The mass of 1 M of raw mana: what the air weighs for each M of it, before anything's taken out or let into it.
#[allow(non_snake_case)]
pub fn RAW_MASS() -> f64 {
    let m = physics().mana_mass;
    (((0.0 + m[0]) + m[1]) + m[2] + m[3]) / 4.0
}

/// The air's gas constant: how hard it presses for each M a m³ of it, p = this × ρ.
#[allow(non_snake_case)]
pub fn AIR_GAS() -> f64 {
    js::pow(physics().air_sound, 2.0) * RAW_MASS()
}

/// How high the air at rest thins by a factor of e, metres: its gas constant over what a M of it weighs, times g. Air at
/// rest is as thick, at height y, as e^(−y/H) times what it is at the ground.
pub fn scale_height() -> f64 {
    AIR_GAS() / (physics().gravity * RAW_MASS())
}

/// The air at rest, cell by cell: as much mana as the air has now, laid out as thick at each height as it would settle
/// (M in each cell). It's what the air's pressure and weight are measured from: air at rest, at rest's thickness,
/// presses and weighs nothing that the ground doesn't already hold up, and needs no work.
fn at_rest(world: &World, open: &[bool]) -> Vec<f64> {
    let hh = scale_height();
    let rows: Vec<f64> = (0..world.h).map(|y| js::exp(-((y as f64 + 0.5) * world.cell) / hh)).collect();
    let mut mana = 0.0;
    let mut shape = 0.0;
    for i in 0..world.size {
        if open[i] {
            mana += total(&world.air[i]);
            shape += rows[(i / world.w) % world.h];
        }
    }
    let mut out = vec![0.0; world.size];
    if shape <= 0.0 {
        return out;
    }
    let s = mana / shape;
    for i in 0..world.size {
        if open[i] {
            out[i] = s * rows[(i / world.w) % world.h];
        }
    }
    out
}

/// One tick of the air: it flows, presses, falls under its weight, and evens out its speed with its neighbours.
pub fn step_air(world: &mut World) {
    let ph = physics();
    let n = world.size;
    let dx = world.cell;
    let area = dx * dx;
    let volume = dx * dx * dx;
    let axes = if world.d == 1 { 2 } else { 3 };
    let Layout { open, next, prev } = layout(world);
    let k2 = AIR_GAS();
    let g = ph.gravity;
    let raw = RAW_MASS();
    // Pressure and weight are measured from the air at rest (at_rest): the pressure that holds it up there, and the weight
    // it holds up, cancel, and the ground takes the rest. So air at rest, as thick as rest would have it, and as heavy,
    // needs no work, and is skipped. Only what's different from rest pushes or falls.
    let rest = at_rest(world, &open);
    let quiet = |world: &World, i: usize| {
        let a = &world.air[i];
        let v = &world.air_vel;
        let m = a[0] + a[1] + a[2] + a[3];
        let r = rest[i];
        v[i * 3] == 0.0
            && v[i * 3 + 1] == 0.0
            && v[i * 3 + 2] == 0.0
            && (m - r).abs() <= CALM * r
            && (mass_of_parts(a) - r * raw).abs() <= CALM * r * raw
    };
    // Air that has all but stopped stops: what little motion it had goes into the ground, as the air's thickness would
    // have taken it there anyway.
    for i in 0..n {
        if !open[i] {
            continue;
        }
        let v = &world.air_vel;
        let s2 = js::pow(v[i * 3], 2.0) + js::pow(v[i * 3 + 1], 2.0) + js::pow(v[i * 3 + 2], 2.0);
        if s2 == 0.0 || s2 > STOP * STOP {
            continue;
        }
        let m = mass_of_parts(&world.air[i]);
        world.warm("the air", 0.5 * m * s2);
        for k in 0..3 {
            world.impulse.walls[k] -= m * world.air_vel[i * 3 + k];
            world.air_vel[i * 3 + k] = 0.0;
        }
    }

    // Steps enough that nothing crosses more than a share of a cell in one: the fastest wind or pressure wave sets it.
    let mut active: Vec<usize> = (0..n).filter(|&i| open[i] && !quiet(world, i)).collect();
    if active.is_empty() {
        return;
    }
    let mut fastest = 0.0;
    for &i in &active {
        let v = &world.air_vel;
        fastest = js::max(fastest, js::hypot3(v[i * 3], v[i * 3 + 1], v[i * 3 + 2]) + ph.air_sound);
    }
    let steps = js::max(1.0, (fastest / (0.4 * dx)).ceil());
    let dt = 1.0 / steps;

    let mut mm = vec![0.0; n];
    let mut pp = vec![0.0; n * 3];
    let mut sink = vec![0.0; n];
    let mut press = vec![0.0; n];
    let mut flow = vec![0.0; n * 4];
    let mut moved = vec![0.0; n * 3];
    let mut u = vec![0.0; n * 3];
    let mut busy = vec![false; n];
    for s in 0..steps as usize {
        if s > 0 {
            active = (0..n).filter(|&i| open[i] && !quiet(world, i)).collect();
            if active.is_empty() {
                break;
            }
        }
        // The active cells, and their neighbours: everything a face of an active cell touches.
        busy.fill(false);
        for &i in &active {
            busy[i] = true;
            for ax in 0..axes {
                if next[i * 3 + ax] >= 0 {
                    busy[next[i * 3 + ax] as usize] = true;
                }
                if prev[i * 3 + ax] >= 0 {
                    busy[prev[i * 3 + ax] as usize] = true;
                }
            }
        }
        let mut touched: Vec<usize> = Vec::new();
        for i in 0..n {
            if !busy[i] {
                continue;
            }
            touched.push(i);
            let a = &world.air[i];
            let m = a[0] + a[1] + a[2] + a[3];
            let mass = mass_of_parts(a);
            mm[i] = mass;
            pp[i * 3] = mass * world.air_vel[i * 3];
            pp[i * 3 + 1] = mass * world.air_vel[i * 3 + 1];
            pp[i * 3 + 2] = mass * world.air_vel[i * 3 + 2];
            press[i] = (k2 * (m - rest[i])) / volume;
            // How much heavier (or lighter) it is than the air at rest there, which its pressure holds up.
            sink[i] = g * (mass - rest[i] * raw);
            for k in 0..4 {
                flow[i * 4 + k] = 0.0;
            }
            moved[i * 3] = 0.0;
            moved[i * 3 + 1] = 0.0;
            moved[i * 3 + 2] = 0.0;
        }
        // Each face an active cell has, once: (i, j, axis), j = −1 for a closed face on i's + side, and i = −1 for one on
        // j's − side.
        let mut faces: Vec<(isize, isize, usize)> = Vec::new();
        for &i in &active {
            for ax in 0..axes {
                faces.push((i as isize, next[i * 3 + ax], ax));
                let k = prev[i * 3 + ax];
                // The face on i's − side, unless its other cell is active and does it.
                if k < 0 || quiet(world, k as usize) {
                    faces.push((k, i as isize, ax));
                }
            }
        }
        // First the pressure on every face pushes the air on either side of it away from the other: a closed face, the
        // world's edge or something solid, pushes back with the air's own pressure.
        for &(i, j, ax) in &faces {
            if i >= 0 && j >= 0 {
                let (i, j) = (i as usize, j as usize);
                let f = ((press[i] + press[j]) / 2.0) * area * dt;
                pp[i * 3 + ax] -= f;
                pp[j * 3 + ax] += f;
            } else if j < 0 {
                let i = i as usize;
                let f = press[i] * area * dt;
                pp[i * 3 + ax] -= f;
                world.impulse.walls[ax] -= f;
            } else {
                let j = j as usize;
                let f = press[j] * area * dt;
                pp[j * 3 + ax] += f;
                world.impulse.walls[ax] += f;
            }
        }
        // And it falls by what it weighs beyond the air at rest there, or rises by what it weighs less.
        for &i in &touched {
            pp[i * 3 + 1] -= sink[i] * dt;
            world.impulse.gravity[1] -= sink[i] * dt;
        }
        for &i in &touched {
            for k in 0..3 {
                u[i * 3 + k] = if mm[i] > 0.0 { pp[i * 3 + k] / mm[i] } else { 0.0 };
            }
        }
        // Then the air, at the speed it now has, carries its mana and momentum across each open face, from the cell upwind.
        for &(i, j, ax) in &faces {
            if i < 0 || j < 0 {
                continue;
            }
            let (i, j) = (i as usize, j as usize);
            let speed = (u[i * 3 + ax] + u[j * 3 + ax]) / 2.0;
            if speed == 0.0 {
                continue;
            }
            let (from, to) = if speed > 0.0 { (i, j) } else { (j, i) };
            if mm[from] <= 0.0 {
                continue;
            }
            let share = js::min(MOST, (speed.abs() * dt) / dx);
            let parts = world.air[from];
            for k in 0..4 {
                let q = parts[k] * share;
                flow[from * 4 + k] -= q;
                flow[to * 4 + k] += q;
            }
            for k in 0..3 {
                let q = pp[from * 3 + k] * share;
                moved[from * 3 + k] -= q;
                moved[to * 3 + k] += q;
            }
        }
        for &i in &touched {
            let parts = &mut world.air[i];
            for k in 0..4 {
                parts[k] = js::max(0.0, parts[k] + flow[i * 4 + k]);
            }
            let m = mass_of_parts(&world.air[i]);
            for k in 0..3 {
                let p = pp[i * 3 + k] + moved[i * 3 + k];
                if m > 1e-12 {
                    world.air_vel[i * 3 + k] = p / m;
                } else {
                    // A cell emptied keeps no momentum: what it had went with its mana.
                    world.air_vel[i * 3 + k] = 0.0;
                    world.impulse.walls[k] -= p;
                }
            }
        }
    }
    viscosity(world, &open, &next, &prev, axes);
}

struct Layout {
    open: Vec<bool>,
    next: Vec<isize>,
    prev: Vec<isize>,
}

/// Which cells are open to the air, and each one's open neighbour on each side (−1 where the face is closed).
fn layout(world: &World) -> Layout {
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
    Layout { open, next, prev }
}

/// The air carries its speed to its neighbours (it's thick), and the ground, walls and the world's edge hold still the air
/// against them.
fn viscosity(world: &mut World, open: &[bool], next: &[isize], prev: &[isize], axes: usize) {
    let k = physics().air_viscosity;
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
                still(world, &mut heat, a, ma);
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
            if b < 0 || total(&world.air[b as usize]) <= 0.0 {
                still(world, &mut heat, a, ma);
            }
        }
    }
    world.warm("the air", heat);
}
