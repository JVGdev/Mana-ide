//! Mana as a fluid (SPEC §11): free mana is particles that press on their neighbours, drag the air they move through and
//! are dragged by it, stop against matter, and shove bodies they run into. Matter held by mana gives a particle weight
//! and inertia; water held in mana can't be squeezed, and holds together; earth held in mana is rock, its particles bound
//! to each other.
//!
//! It's smoothed-particle hydrodynamics. Each particle feels its neighbours within PHYSICS.smoothing metres. Every force
//! between two things in the world is equal and opposite (particle and particle, particle and air, air and air, particle
//! and body), so the world's momentum only changes by what comes from outside (weight, pushes, the ground), and
//! `World::momentum_error` checks it.
//!
//! The particles a step works on are lists of indices into `world.particles`, in the order the TypeScript engine kept
//! them: the order things are summed in decides how they round.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use super::air::RAW_MASS;
use super::parts::total;
use super::physics::physics;
use super::world::{EARTH, Particle, Vec3, WATER, World, mass_of, mass_of_parts, packed};
use crate::js;

/// What the fluid needs to know from the machine.
#[derive(Default)]
pub struct FluidHooks<'a> {
    /// The body whose hand holds this particle, if any (an index into `world.bodies`): a weave in its caster's hand stays
    /// where it was laid until it's set loose. It presses on what moves around it, and is pressed by it; the hand holds
    /// it still, and the body feels what that takes.
    pub holder: Option<Box<dyn Fn(&Particle) -> Option<usize> + 'a>>,
    /// The share of a cell's room taken by matter this particle's weave holds there: a weave's own matter never blocks
    /// it, since they move together. Other matter blocks it, held or not.
    pub own_matter: Option<Box<dyn Fn(&Particle, usize) -> f64 + 'a>>,
    /// What weaves' mana grips, cell by cell (matter.rs): it holds its matter as it moves, and its matter holds it.
    pub grips: &'a [super::matter::Grip],
}

impl FluidHooks<'_> {
    fn holder(&self, p: &Particle) -> Option<usize> {
        self.holder.as_ref().and_then(|h| h(p))
    }
}

#[derive(Clone)]
struct Kernels {
    /// What a particle feels is weighed with the flat (Epanechnikov) kernel, W = flat·(h² − r²), and its slope, grad·r:
    /// a neighbour most of a smoothing length away still counts a fair share of what the particle itself does.
    flat: f64,
    grad: f64,
    /// The spiky kernel, W = spikyW·(h − r)³, and its slope, spiky·(h − r)². Density is summed with it.
    spiky_w: f64,
    spiky: f64,
    lap: f64,
    cohesion: f64,
    /// The work cohesion does pulling two particles from r out to h, for each 1/64 of h: for the Energy ledger.
    cohesion_work: [f64; 65],
}

thread_local! {
    static KERNELS: RefCell<HashMap<(u8, u64), Kernels>> = RefCell::new(HashMap::new());
}

fn kernels(dims: u8) -> Kernels {
    let h = physics().smoothing;
    let key = (dims, h.to_bits());
    if let Some(k) = KERNELS.with(|c| c.borrow().get(&key).cloned()) {
        return k;
    }
    let pi = std::f64::consts::PI;
    let (flat, grad, spiky_w, spiky, lap) = if dims == 2 {
        (
            2.0 / (pi * js::pow(h, 4.0)),
            -4.0 / (pi * js::pow(h, 4.0)),
            10.0 / (pi * js::pow(h, 5.0)),
            -30.0 / (pi * js::pow(h, 5.0)),
            40.0 / (pi * js::pow(h, 5.0)),
        )
    } else {
        (
            15.0 / (8.0 * pi * js::pow(h, 5.0)),
            -15.0 / (4.0 * pi * js::pow(h, 5.0)),
            15.0 / (pi * js::pow(h, 6.0)),
            -45.0 / (pi * js::pow(h, 6.0)),
            45.0 / (pi * js::pow(h, 6.0)),
        )
    };
    // Akinci's cohesion kernel (2013), scaled so it sums to one over its reach, in 2D or 3D.
    let steps = 1000;
    let mut sum = 0.0;
    for i in 0..steps {
        let r = ((i as f64 + 0.5) / steps as f64) * h;
        sum += cohesion_shape(r, h) * (if dims == 2 { 2.0 * pi * r } else { 4.0 * pi * r * r }) * (h / steps as f64);
    }
    let cohesion = 1.0 / sum;
    // ∫ from r to h of the scaled shape, tabled at 65 points.
    let mut work = [0.0; 65];
    for t in (0..=63).rev() {
        let mut w = 0.0;
        for i in 0..16 {
            w += cohesion_shape(((t as f64 + (i as f64 + 0.5) / 16.0) / 64.0) * h, h) * (h / 64.0 / 16.0);
        }
        work[t] = work[t + 1] + cohesion * w;
    }
    let k = Kernels { flat, grad, spiky_w, spiky, lap, cohesion, cohesion_work: work };
    KERNELS.with(|c| c.borrow_mut().insert(key, k.clone()));
    k
}

/// How neighbours pull at each distance: most at half the smoothing length, and pushing back when very close.
fn cohesion_shape(r: f64, h: f64) -> f64 {
    if r >= h || r <= 0.0 {
        return 0.0;
    }
    let c = js::pow(h - r, 3.0) * js::pow(r, 3.0);
    if r > h / 2.0 { c } else { 2.0 * c - js::pow(h, 6.0) / 64.0 }
}

fn dims(world: &World) -> u8 {
    if world.d == 1 { 2 } else { 3 }
}

/// Free mana: what presses as a gas, and what the mana senses (DENS, GRAD, NVEL).
fn free_mass(p: &Particle) -> f64 {
    total(&p.free)
}

/// How hard a particle's free mana presses for each M a m³ of it: each part's stiffness (the square of how fast it
/// spreads) times its weight a M, blended by how much of each part it holds. The ideal gas law: p = this × ρ.
fn gas(p: &Particle) -> f64 {
    let m = free_mass(p);
    if m <= 0.0 {
        return 0.0;
    }
    let ph = physics();
    let k = ph.stiffness;
    let w = ph.mana_mass;
    (p.free[0] * k[0] * w[0] + p.free[1] * k[1] * w[1] + p.free[2] * k[2] * w[2] + p.free[3] * k[3] * w[3]) / m
}

/// A blend of a property of free mana by part, by how much of the particle's mass each part is.
fn blend(p: &Particle, by_part: &[f64; 4]) -> f64 {
    let w = physics().mana_mass;
    let mut s = 0.0;
    let mut m = 0.0;
    for k in 0..4 {
        s += p.free[k] * by_part[k] * w[k];
        m += p.free[k] * w[k];
    }
    if m > 0.0 { s / m } else { 0.0 }
}

/// Neighbours this step: pairs of indices into the particles, closer than the smoothing length, and how far apart.
#[derive(Default)]
pub struct Pairs {
    pub n: usize,
    pub a: Vec<usize>,
    pub b: Vec<usize>,
    pub d: Vec<f64>,
    pub r: Vec<f64>,
}

fn grid_key(x: i64, y: i64, z: i64) -> i64 {
    x + 2048 + (y + 2048) * 4096 + (z + 2048) * 16777216
}

/// `Math.floor(x)` stored in an Int32Array.
fn floor32(x: f64) -> i64 {
    x.floor() as i32 as i64
}

/// Every pair of particles closer than the smoothing length, once each. `ps` are indices into `world.particles`; the
/// pairs index into `ps`.
pub fn find_pairs(world: &World, ps: &[usize]) -> Pairs {
    let h = physics().smoothing;
    let flat = world.d == 1;
    let mut grid: HashMap<i64, Vec<usize>> = HashMap::new();
    let n = ps.len();
    let mut cx = vec![0i64; n];
    let mut cy = vec![0i64; n];
    let mut cz = vec![0i64; n];
    for i in 0..n {
        let p = &world.particles[ps[i]].pos;
        cx[i] = floor32(p[0] / h);
        cy[i] = floor32(p[1] / h);
        cz[i] = if flat { 0 } else { floor32(p[2] / h) };
        grid.entry(grid_key(cx[i], cy[i], cz[i])).or_default().push(i);
    }
    let mut out = Pairs::default();
    let h2 = h * h;
    let zr: i64 = if flat { 0 } else { 1 };
    for i in 0..n {
        let pa = world.particles[ps[i]].pos;
        for dz in -zr..=zr {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let Some(list) = grid.get(&grid_key(cx[i] + dx, cy[i] + dy, cz[i] + dz)) else { continue };
                    for &j in list {
                        if j <= i {
                            continue;
                        }
                        let pb = world.particles[ps[j]].pos;
                        let x = pa[0] - pb[0];
                        let y = pa[1] - pb[1];
                        let z = if flat { 0.0 } else { pa[2] - pb[2] };
                        let r2 = x * x + y * y + z * z;
                        if r2 >= h2 {
                            continue;
                        }
                        out.a.push(i);
                        out.b.push(j);
                        out.d.extend_from_slice(&[x, y, z]);
                        out.r.push(r2.sqrt());
                        out.n += 1;
                    }
                }
            }
        }
    }
    out
}

/// Two particles at once, to change both.
fn two(ps: &mut [Particle], i: usize, j: usize) -> (&mut Particle, &mut Particle) {
    assert!(i != j);
    if i < j {
        let (lo, hi) = ps.split_at_mut(j);
        (&mut lo[i], &mut hi[0])
    } else {
        let (lo, hi) = ps.split_at_mut(i);
        (&mut hi[0], &mut lo[j])
    }
}

/// What each particle feels: how dense the mana around it is, which way it thickens, and how its neighbours move. And
/// how dense the matter held around it is.
pub fn feel(world: &mut World, ps: &[usize], pairs: &Pairs) {
    let h = physics().smoothing;
    let kk = kernels(dims(world));
    // In 2D the world is one cell deep: density per square metre, over that depth, is density per cubic metre.
    let depth = if world.d == 1 { world.cell } else { 1.0 };
    let n = ps.len();
    let mut fm = vec![0.0; n];
    let mut weight = vec![0.0; n];
    // Density is summed with the spiky kernel, whose slope is what pressure pushes along: so the pressure forces are
    // exactly the pull of the gas's stored energy (Σ m k ln ρ), and the Energy ledger can count it.
    let own = kk.spiky_w * js::pow(h, 3.0);
    // What a particle feels (DENS, GRAD, NVEL) is summed with the flat kernel, which weighs its neighbours nearly as much
    // as itself. With a peaked one (poly6), a particle a smoothing length from the next would feel mostly itself, and
    // couldn't tell the inside of a ball from its edge.
    let own_felt = kk.flat * h * h;
    let parts = &mut world.particles;
    for i in 0..n {
        let p = &mut parts[ps[i]];
        fm[i] = free_mass(p);
        p.rho = fm[i] * own;
        p.felt = fm[i] * own_felt;
        p.grad = [0.0; 3];
        p.nvel = [0.0; 3];
    }
    for e in 0..pairs.n {
        let i = pairs.a[e];
        let j = pairs.b[e];
        let (a, b) = two(parts, ps[i], ps[j]);
        let r = pairs.r[e];
        let hr = h - r;
        let w = kk.spiky_w * hr * hr * hr;
        let ma = fm[i];
        let mb = fm[j];
        a.rho += mb * w;
        b.rho += ma * w;
        let q = h * h - r * r;
        let wf = kk.flat * q;
        a.felt += mb * wf;
        b.felt += ma * wf;
        let g = kk.grad;
        for k in 0..3 {
            let dk = pairs.d[e * 3 + k];
            a.grad[k] += mb * g * dk;
            b.grad[k] -= ma * g * dk;
            a.nvel[k] += mb * wf * b.vel[k];
            b.nvel[k] += ma * wf * a.vel[k];
        }
        weight[i] += mb * wf;
        weight[j] += ma * wf;
    }
    for i in 0..n {
        let p = &mut parts[ps[i]];
        let w = weight[i];
        p.rho /= depth;
        p.felt /= depth;
        for k in 0..3 {
            p.grad[k] /= depth;
            p.nvel[k] = if w > 0.0 { p.nvel[k] / w } else { p.vel[k] };
        }
    }
}

/// `feel`, with the pairs found here.
pub fn feel_all(world: &mut World, ps: &[usize]) {
    let pairs = find_pairs(world, ps);
    feel(world, ps, &pairs);
}

/// One tick of the fluid: weight, pressure, holding together, the air, the ground, and what the particles run into.
pub fn step_fluid(world: &mut World, hooks: &FluidHooks) {
    let ph = physics();
    let (ps, held) = split(world, hooks);
    if ps.is_empty() {
        return;
    }
    // What's in a hand and near enough to what moves to press on it, or be pressed: after the moving ones.
    let near = if held.is_empty() { vec![] } else { within(world, &ps, &held) };
    let all: Vec<usize> = ps.iter().chain(near.iter()).copied().collect();
    let dt = 1.0 / ph.substeps as f64;
    for _ in 0..ph.substeps {
        for &i in &all {
            let m = mass_of(&world.particles[i]);
            world.particles[i].mass = m;
        }
        let pairs = find_pairs(world, &all);
        feel(world, &all, &pairs);
        forces(world, &all, &pairs, dt, ps.len());
        super::matter::cling(world, hooks.grips, dt);
        ground(world, &ps, dt, hooks);
        drag(world, &all, dt);
        grip(world, &near, hooks);
        move_particles(world, &ps, dt, hooks);
    }
    feel_all(world, &all);
}

/// How much of a parcel of mana (a particle's free mana) the air around it holds up, 0–1: all of it where its cell has
/// more air than mana in parcels, and less where the parcels are most of what's there (they aren't surrounded by air).
/// The air holds a parcel up by what the air it pushes aside weighs (Archimedes): free mana is a gas, all its parts
/// alike, so a parcel pushes aside its own M's worth of air, and that weighs RAW_MASS a M. It's the pressure of the air at
/// rest, thicker below than above, and the ground under the air takes it back.
pub fn buoyed(world: &World, cell: isize, in_parcels: f64) -> f64 {
    if cell < 0 || world.solid_at(cell) {
        return 0.0;
    }
    let air = total(&world.air[cell as usize]);
    if air > 0.0 { js::min(1.0, air / js::max(in_parcels, physics().epsilon)) } else { 0.0 }
}

/// The free mana in parcels in each cell of some particles.
pub fn parcels(world: &World, ps: impl IntoIterator<Item = usize>) -> HashMap<isize, f64> {
    let mut out = HashMap::new();
    for i in ps {
        let p = &world.particles[i];
        let c = world.cell_of(&p.pos);
        if c >= 0 {
            *out.entry(c).or_insert(0.0) += total(&p.free);
        }
    }
    out
}

/// The particles that move, and those a hand holds.
fn split(world: &World, hooks: &FluidHooks) -> (Vec<usize>, Vec<usize>) {
    if hooks.holder.is_none() {
        return ((0..world.particles.len()).collect(), vec![]);
    }
    let mut moving = Vec::new();
    let mut held = Vec::new();
    for (i, p) in world.particles.iter().enumerate() {
        if hooks.holder(p).is_some() {
            held.push(i);
        } else {
            moving.push(i);
        }
    }
    (moving, held)
}

/// Those of `some` within the smoothing length of any of `of`.
fn within(world: &World, of: &[usize], some: &[usize]) -> Vec<usize> {
    let h = physics().smoothing;
    let flat = world.d == 1;
    let mut grid = HashSet::new();
    for &i in of {
        let p = &world.particles[i].pos;
        grid.insert(grid_key(
            (p[0] / h).floor() as i64,
            (p[1] / h).floor() as i64,
            if flat { 0 } else { (p[2] / h).floor() as i64 },
        ));
    }
    let zr: i64 = if flat { 0 } else { 1 };
    some.iter()
        .copied()
        .filter(|&i| {
            let p = &world.particles[i].pos;
            let x = (p[0] / h).floor() as i64;
            let y = (p[1] / h).floor() as i64;
            let z = if flat { 0 } else { (p[2] / h).floor() as i64 };
            for dz in -zr..=zr {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if grid.contains(&grid_key(x + dx, y + dy, z + dz)) {
                            return true;
                        }
                    }
                }
            }
            false
        })
        .collect()
}

/// ½m|v|², the kinetic energy of one particle, by its mass as of this step.
fn ke(p: &Particle) -> f64 {
    0.5 * p.mass * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2])
}

/// A hand holds what's in it still, where it was laid: what the world gave it this step, the hand takes away, and the
/// body behind the hand gets, across the ground (the ground takes what's up and down). What motion it had is heat.
fn grip(world: &mut World, held: &[usize], hooks: &FluidHooks) {
    let mut heat = 0.0;
    for &i in held {
        let Some(body) = hooks.holder(&world.particles[i]) else { continue };
        let p = &world.particles[i];
        let m = p.mass;
        heat += ke(p);
        let vel = p.vel;
        let b = &mut world.bodies[body];
        for k in [0, 2] {
            b.vel[k] += (m * vel[k]) / b.mass;
        }
        world.impulse.walls[1] -= m * vel[1];
        world.particles[i].vel = [0.0; 3];
    }
    world.warm("the hand", heat);
}

/// Weight, pressure, holding together and thickness. The first `free` of `ps` move; the rest are in a hand, which bears
/// their weight, and they press only on what moves (what's in a hand doesn't press on itself: the hand holds it all).
fn forces(world: &mut World, ps: &[usize], pairs: &Pairs, dt: f64, free: usize) {
    let ph = physics();
    let kk = kernels(dims(world));
    let h = ph.smoothing;
    let depth = if world.d == 1 { world.cell } else { 1.0 };
    let g = ph.gravity;
    let n = ps.len();
    // What each particle brings, worked out once: its mass, how its free mana presses, how hard it coheres, and how
    // thick it is.
    let mut fm = vec![0.0; n];
    let mut press = vec![0.0; n];
    let mut coh = vec![0.0; n];
    let mut visc = vec![0.0; n];
    let mut rho = vec![0.0; n];
    let in_cell = parcels(world, ps[..free].iter().copied());
    let raw = RAW_MASS();
    for i in 0..n {
        let c = world.cell_of(&world.particles[ps[i]].pos);
        let lift = if i < free {
            g * free_mass(&world.particles[ps[i]]) * raw * buoyed(world, c, in_cell.get(&c).copied().unwrap_or(0.0))
        } else {
            0.0
        };
        let p = &mut world.particles[ps[i]];
        fm[i] = free_mass(p);
        // Weight: its mass times g; less what the air holds up (buoyancy), which the ground under
        // the air takes. What's in a hand, the hand bears.
        if i < free {
            p.acc = [0.0, -g * p.mass + lift, 0.0];
            p.lifted = lift;
            world.impulse.gravity[1] -= g * p.mass * dt;
            world.impulse.walls[1] += lift * dt;
        } else {
            p.acc = [0.0; 3];
        }
        // Free mana presses like a gas: pressure = k·ρ, so its term is k/ρ.
        press[i] = if fm[i] > 0.0 { gas(p) / (p.rho * depth) } else { 0.0 };
        coh[i] = blend(p, &ph.cohesion);
        visc[i] = blend(p, &ph.viscosity);
        // How dense it is, in kg/m³: its free mana is counted in M, so by its weight a M.
        rho[i] = if fm[i] > 0.0 { (p.rho * mass_of_parts(&p.free)) / fm[i] } else { 0.0 };
    }
    let h6 = js::pow(h, 6.0);
    let parts = &mut world.particles;
    for e in 0..pairs.n {
        let r = pairs.r[e];
        if r < 1e-9 {
            continue;
        }
        let i = pairs.a[e];
        let j = pairs.b[e];
        if i >= free && j >= free {
            continue;
        }
        let (a, b) = two(parts, ps[i], ps[j]);
        let ma = a.mass;
        let mb = b.mass;
        if ma <= 0.0 || mb <= 0.0 {
            continue;
        }
        let hr = h - r;
        let mut s = 0.0;
        if fm[i] > 0.0 && fm[j] > 0.0 {
            s += -(press[i] + press[j]) * kk.spiky * hr * hr * fm[i] * fm[j];
        }
        if coh[i] > 0.0 && coh[j] > 0.0 {
            // Akinci's cohesion: most pull at half the smoothing length, pushing back when very close.
            let c = hr * hr * hr * r * r * r;
            let shape = if r > h / 2.0 { c } else { 2.0 * c - h6 / 64.0 };
            s += (-((coh[i] + coh[j]) / 2.0) * ma * mb * kk.cohesion * shape) / depth;
        }
        for k in 0..3 {
            let fk = (s * pairs.d[e * 3 + k]) / r;
            a.acc[k] += fk;
            b.acc[k] -= fk;
        }
    }
    for &i in ps {
        let p = &mut parts[i];
        let m = p.mass;
        if m <= 0.0 {
            continue;
        }
        for k in 0..3 {
            p.vel[k] += (p.acc[k] / m) * dt;
        }
    }
    thicken(world, ps, pairs, &visc, &rho, dt, free);
}

/// What a particle weighs, less what the air holds up: m·g − lift.
fn weight_of(p: &Particle) -> f64 {
    js::max(0.0, mass_of(p) * physics().gravity - p.lifted)
}

/// Thickness: neighbours' motions even out, pair by pair, equally and oppositely. What the evening out takes from their
/// motion is heat.
fn thicken(world: &mut World, ps: &[usize], pairs: &Pairs, visc: &[f64], rho: &[f64], dt: f64, free: usize) {
    let kk = kernels(dims(world));
    let h = physics().smoothing;
    let depth = if world.d == 1 { world.cell } else { 1.0 };
    let mut heat = 0.0;
    let parts = &mut world.particles;
    for e in 0..pairs.n {
        let i = pairs.a[e];
        let j = pairs.b[e];
        if i >= free && j >= free {
            continue;
        }
        let (a, b) = two(parts, ps[i], ps[j]);
        let ma = a.mass;
        let mb = b.mass;
        let vv = visc[i] + visc[j];
        if ma <= 0.0 || mb <= 0.0 || vv == 0.0 || vv.is_nan() {
            continue;
        }
        let c = (((visc[i] + visc[j]) / 2.0) * kk.lap * (h - pairs.r[e]) * 2.0 * ma * mb * dt) / ((rho[i] + rho[j]) * depth);
        // Never more than evens them out completely.
        let f = js::min(c, (ma * mb) / (ma + mb));
        let before = ke(a) + ke(b);
        for k in 0..3 {
            let jk = f * (b.vel[k] - a.vel[k]);
            a.vel[k] += jk / ma;
            b.vel[k] -= jk / mb;
        }
        heat += before - ke(a) - ke(b);
    }
    world.warm("thickness", heat);
}

/// What merging and splitting did: which particle took in which, and which split off from which.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Merge { into: u64, from: u64, weave: u32 },
    Split { from: u64, into: u64 },
}

/// Particles at rest beside each other merge, and big ones that have spread thin split (SPEC §11). Nothing in a caster's
/// hand, and no rock, does either. A merge keeps mana, matter and momentum: the new particle is at their centre of mass,
/// moving at their shared momentum, and keeps the bigger one's weave and order, whoever the smaller belonged to. A split
/// halves a particle into two, side by side across the way its mana thins, at the same speed.
pub fn merge_and_split(world: &mut World, hooks: &FluidHooks) -> Vec<Change> {
    let ph = physics();
    let ps: Vec<usize> = (0..world.particles.len()).filter(|&i| hooks.holder(&world.particles[i]).is_none()).collect();
    let mut changes = Vec::new();
    if ps.is_empty() {
        return changes;
    }
    let ids: Vec<u64> = ps.iter().map(|&i| world.particles[i].id).collect();
    let pairs = find_pairs(world, &ps);
    let mut gone: HashSet<u64> = HashSet::new();
    let mut done: HashSet<u64> = HashSet::new();
    let near = ph.merge_range;
    for e in 0..pairs.n {
        if pairs.r[e] > near {
            continue;
        }
        let mut ai = ps[pairs.a[e]];
        let mut bi = ps[pairs.b[e]];
        if done.contains(&world.particles[ai].id) || done.contains(&world.particles[bi].id) {
            continue;
        }
        let fa = total(&world.particles[ai].free);
        let fb = total(&world.particles[bi].free);
        if fa + fb > ph.max_mote + ph.epsilon {
            continue;
        }
        {
            let (a, b) = (&world.particles[ai], &world.particles[bi]);
            if js::hypot3(a.vel[0] - b.vel[0], a.vel[1] - b.vel[1], a.vel[2] - b.vel[2]) > ph.merge_speed {
                continue;
            }
        }
        // The bigger keeps its weave and order; the same size, the older.
        if fb > fa || (fb == fa && world.particles[bi].id < world.particles[ai].id) {
            std::mem::swap(&mut ai, &mut bi);
        }
        let ma = mass_of(&world.particles[ai]);
        let mb = mass_of(&world.particles[bi]);
        let m = ma + mb;
        if m <= 0.0 {
            continue;
        }
        let (a, b) = two(&mut world.particles, ai, bi);
        // Two that move as one lose what moved them apart: ½μ|Δv|².
        let dv2 = js::pow(a.vel[0] - b.vel[0], 2.0) + js::pow(a.vel[1] - b.vel[1], 2.0) + js::pow(a.vel[2] - b.vel[2], 2.0);
        let mut to = [0.0; 3];
        for k in 0..3 {
            to[k] = (ma * a.pos[k] + mb * b.pos[k]) / m;
            a.vel[k] = (ma * a.vel[k] + mb * b.vel[k]) / m;
        }
        for k in 0..4 {
            a.free[k] += b.free[k];
            b.free[k] = 0.0;
        }
        a.pushed_at = js::max(a.pushed_at, b.pushed_at);
        let (a_id, b_id, b_weave) = (a.id, b.id, b.weave);
        world.warm("merging", (0.5 * ma * mb * dv2) / m);
        if world.cell_of(&to) >= 0 {
            world.particles[ai].pos = to;
        }
        changes.push(Change::Merge { into: a_id, from: b_id, weave: b_weave });
        gone.insert(b_id);
        done.insert(a_id);
        done.insert(b_id);
    }
    if !gone.is_empty() {
        world.retain_particles(|p| !gone.contains(&p.id));
    }
    // Split what has spread thin: more of what it feels is itself than is its neighbours.
    let left: Vec<usize> = ids.iter().filter(|id| !gone.contains(id)).map(|&id| world.slot(id).unwrap()).collect();
    feel_all(world, &left);
    let kk = kernels(dims(world));
    let depth = if world.d == 1 { world.cell } else { 1.0 };
    for id in &ids {
        if gone.contains(id) || done.contains(id) {
            continue;
        }
        let pi = world.slot(*id).unwrap();
        let p = &world.particles[pi];
        let m = total(&p.free);
        if m < 2.0 * ph.mote - ph.epsilon || p.felt <= 0.0 {
            continue;
        }
        let own = (m * kk.flat * js::pow(ph.smoothing, 2.0)) / depth;
        if own / p.felt <= ph.split_alone && m <= 2.0 * ph.max_mote {
            continue;
        }
        let qi = world.spawn(pi);
        let (p, q) = two(&mut world.particles, pi, qi);
        for k in 0..4 {
            q.free[k] = p.free[k] / 2.0;
            p.free[k] -= q.free[k];
        }
        // Side by side, across the way it thins (or, alone, any way: by its id), a third of the smoothing length apart.
        let g = js::hypot(&p.grad);
        let mut u: Vec3 =
            if g > 1e-12 { [-p.grad[1] / g, p.grad[0] / g, 0.0] } else { [js::cos(p.id as f64), js::sin(p.id as f64), 0.0] };
        if js::hypot(&u) < 1e-9 {
            u = [1.0, 0.0, 0.0];
        }
        let d = ph.smoothing / 6.0;
        let there = [p.pos[0] + u[0] * d, p.pos[1] + u[1] * d, p.pos[2] + u[2] * d];
        let here = [p.pos[0] - u[0] * d, p.pos[1] - u[1] * d, p.pos[2] - u[2] * d];
        let (p_id, q_id) = (p.id, q.id);
        // Into the cells beside it too, as long as nothing solid is there.
        let open = |at: &Vec3| world.cell_of(at) >= 0 && !world.solid_at(world.cell_of(at));
        let (there_open, here_open) = (open(&there), open(&here));
        if there_open && here_open {
            world.particles[qi].pos = there;
            world.particles[pi].pos = here;
        } else if there_open {
            world.particles[qi].pos = there;
        } else if here_open {
            world.particles[qi].pos = here;
        }
        changes.push(Change::Split { from: p_id, into: q_id });
    }
    changes
}

/// Whether what's in a cell stops a particle moving into it from another: solid matter does (matter blocks mana), held
/// or not, except what its own weave holds.
fn blocked(world: &World, hooks: &FluidHooks, p: &Particle, from: isize, to: isize, _down: bool) -> bool {
    if to < 0 {
        return true;
    }
    if to == from {
        return false;
    }
    // Earth and water stop it, held or not, unless it's its own weave's: those move together.
    let m = &world.matter[to as usize];
    let solid = m[EARTH] / packed(EARTH) + m[WATER] / packed(WATER);
    let own = hooks.own_matter.as_ref().map(|f| f(p, to as usize)).unwrap_or(0.0);
    solid - own >= physics().solid
}

/// How close above the ground a particle rests on it, metres.
const CONTACT: f64 = 0.02;

/// The ground holds up what rests on it, and its friction holds it from sliding.
fn ground(world: &mut World, ps: &[usize], dt: f64, hooks: &FluidHooks) {
    let ph = physics();
    let mut grounded: Vec<usize> = Vec::new();
    for &i in ps {
        let p = &world.particles[i];
        let from = world.cell_of(&p.pos);
        let to = world.cell_of(&[p.pos[0], p.pos[1] - CONTACT, p.pos[2]]);
        if to != from && blocked(world, hooks, p, from, to, true) {
            grounded.push(i);
        }
    }
    let mut heat = 0.0;
    let tick = world.tick as f64;
    for &i in &grounded {
        let p = &mut world.particles[i];
        if p.vel[1] >= 0.0 {
            continue;
        }
        p.touched_at = tick;
        // The ground takes what presses down, and its friction what slides, up to its share of that.
        let m = p.mass;
        // Holding up what rests on it, the ground only takes back the speed its weight gave it this step: no heat in
        // that.
        let resting = js::min(-p.vel[1], weight_of(p) * dt / js::max(m, ph.epsilon));
        let before = ke(p) - 0.5 * m * resting * resting;
        let press = -p.vel[1];
        world.impulse.walls[1] += m * press;
        p.vel[1] = 0.0;
        let slide = js::hypot2(p.vel[0], p.vel[2]);
        if slide > 0.0 {
            let stop = js::min(1.0, (ph.friction * press) / slide);
            for k in [0, 2] {
                world.impulse.walls[k] -= m * p.vel[k] * stop;
                p.vel[k] -= p.vel[k] * stop;
            }
        }
        heat += before - ke(p);
    }
    world.warm("the ground", heat);
}

/// A particle and the air it's in pull each other's speeds together: what one loses, the other gains.
fn drag(world: &mut World, ps: &[usize], dt: f64) {
    let ph = physics();
    let raw = RAW_MASS();
    let mut heat = 0.0;
    for &i in ps {
        let m = world.particles[i].mass;
        let c = world.cell_of(&world.particles[i].pos);
        if m <= 0.0 || c < 0 {
            continue;
        }
        let c = c as usize;
        let big_m = mass_of_parts(&world.air[c]);
        if big_m <= 0.0 {
            continue;
        }
        let rate = (ph.air_drag * big_m) / (ph.air_mana * raw);
        let mu = (m * big_m) / (m + big_m);
        let k = mu * (1.0 - js::exp(-rate * (1.0 + m / big_m) * dt));
        let mut rel2 = 0.0;
        let p = &mut world.particles[i];
        for d in 0..3 {
            let rel = p.vel[d] - world.air_vel[c * 3 + d];
            rel2 += rel * rel;
            let j = k * rel;
            p.vel[d] -= j / m;
            world.air_vel[c * 3 + d] += j / big_m;
        }
        // Two bodies whose speeds even out by a share s of their difference lose ½μ|Δv|²·(1 − (1 − s)²).
        let s = k / mu;
        heat += 0.5 * mu * rel2 * (1.0 - (1.0 - s) * (1.0 - s));
    }
    world.warm("the air dragging", heat);
}

/// Each particle moves, one axis at a time, so it can't slip through a corner. It stops dead at the edge of the world,
/// and against matter: free mana against solid matter, and mana holding matter against a cell with no room for what it
/// holds (matter blocks matter). It strikes the bodies it runs into, its maker's as much as anyone's, sharing its speed
/// with them. Whatever stops it or strikes it, it feels: that's a touch.
fn move_particles(world: &mut World, ps: &[usize], dt: f64, hooks: &FluidHooks) {
    let mut struck = 0.0;
    let mut bodies = 0.0;
    let tick = world.tick as f64;
    for &i in ps {
        let m = world.particles[i].mass;
        for k in 0..3 {
            if k == 2 && world.d == 1 {
                let p = &mut world.particles[i];
                world.impulse.walls[2] -= m * p.vel[2];
                struck += 0.5 * m * p.vel[2] * p.vel[2];
                p.vel[2] = 0.0;
                continue;
            }
            let v = world.particles[i].vel[k];
            if v == 0.0 || v.is_nan() {
                continue;
            }
            let p = &world.particles[i];
            let mut next = p.pos;
            next[k] += p.vel[k] * dt;
            let from = world.cell_of(&p.pos);
            let to = world.cell_of(&next);
            if blocked(world, hooks, p, from, to, k == 1 && p.vel[1] < 0.0) {
                let p = &mut world.particles[i];
                world.impulse.walls[k] -= m * p.vel[k];
                struck += 0.5 * m * p.vel[k] * p.vel[k];
                p.vel[k] = 0.0;
                p.touched_at = tick;
            } else {
                world.particles[i].pos = next;
            }
        }
        let body = world.body_at(&world.particles[i].pos, None);
        if let Some(body) = body
            && m > 0.0
        {
            let p = &mut world.particles[i];
            p.touched_at = tick;
            let b = &mut world.bodies[body];
            // A body only slides along the ground: it takes the mana's push across, and the ground takes the rest.
            let mu = (m * b.mass) / (m + b.mass);
            for k in [0, 2] {
                let rel = p.vel[k] - b.vel[k];
                let j = mu * rel;
                p.vel[k] -= j / m;
                b.vel[k] += j / b.mass;
                bodies += 0.5 * mu * rel * rel;
            }
        }
    }
    world.warm("striking", struck);
    world.warm("striking bodies", bodies);
}

/// Energy the fluid stores in where its particles are, for the Energy ledger.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FluidStore {
    pub gas: f64,
    pub cohesion: f64,
}

/// Energy the fluid stores in where its particles are, for the Energy ledger: in its gas pressed together (each particle
/// m·k·ln(ρ/ρ̄), which is what its pressure pushes out of: nothing at the density of the air, `air`, so mana that settles
/// into the air takes none with it), and in particles that cohere pulled apart. What's in a
/// hand counts too: it presses on what's around it.
pub fn stored_in_fluid(world: &mut World, air: f64) -> FluidStore {
    let ph = physics();
    let mut out = FluidStore::default();
    if world.particles.is_empty() {
        return out;
    }
    for p in world.particles.iter_mut() {
        p.mass = mass_of(p);
    }
    let ps: Vec<usize> = (0..world.particles.len()).collect();
    let pairs = find_pairs(world, &ps);
    feel(world, &ps, &pairs);
    let kk = kernels(dims(world));
    let depth = if world.d == 1 { world.cell } else { 1.0 };
    for p in &world.particles {
        let fm = free_mass(p);
        if fm > 0.0 && p.rho > 0.0 {
            out.gas += fm * gas(p) * js::log(p.rho / air);
        }
    }
    let h = ph.smoothing;
    for e in 0..pairs.n {
        let a = &world.particles[pairs.a[e]];
        let b = &world.particles[pairs.b[e]];
        let ca = blend(a, &ph.cohesion);
        let cb = blend(b, &ph.cohesion);
        if !(ca > 0.0 && cb > 0.0) {
            continue;
        }
        let t = js::min(64.0, (pairs.r[e] / h) * 64.0);
        let lo = t.floor();
        let w = &kk.cohesion_work;
        let work = if lo >= 64.0 { 0.0 } else { w[lo as usize] + (w[lo as usize + 1] - w[lo as usize]) * (t - lo) };
        out.cohesion -= (((ca + cb) / 2.0) * a.mass * b.mass * work) / depth;
    }
    out
}
