//! The world: a grid of cells, each with free air mana, and matter: material points over the grid (matter.rs), and the
//! gases (flame and air matter) in the cells. A 2D world is one cell deep.

use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;

use super::air::scale_height;
use super::matter::{Point, settled};
use super::parts::{Parts, add, dominant, share, take, total, zero};
use super::physics::physics;
use crate::asm::Code;
use crate::js;

pub type Vec3 = [f64; 3];

/// A body in the world: a caster, or something to push around. Its position is its centre.
#[derive(Clone, Debug)]
pub struct Body {
    pub id: u32,
    pub name: String,
    pub pos: Vec3,
    pub vel: Vec3,
    pub mass: f64,
    /// Half its size in each axis, in metres.
    pub half: Vec3,
}

/// An order ingrained in a particle: the routine at `addr` of the spell that ingrained it.
#[derive(Clone, Debug)]
pub struct Ingrained {
    pub program: Arc<Code>,
    pub addr: usize,
}

/// A particle of free mana: one simulated particle stands for a crowd of real ones (SPEC §11). It's in a weave, or loose
/// (weave 0): sent, spent, or let go.
#[derive(Clone, Debug)]
pub struct Particle {
    pub id: u64,
    pub pos: Vec3,
    pub vel: Vec3,
    pub free: Parts,
    pub weave: u32,
    pub order: Option<Ingrained>,
    /// Its own copy of its weave's registers (w0–w7), and when each was last written: the newest write wins, as copies
    /// pass from particle to particle by contact (SPEC §5, Weaves). A stamp is the tick it was written, with the writer
    /// to break ties.
    pub regs: Vec<f64>,
    pub stamp: Vec<f64>,
    /// Which way its weave's frame faces, as it was told: the turn about the vertical its order reads and kicks in.
    pub yaw: f64,
    /// The tick it was last pushed, by its caster or its order.
    pub pushed_at: f64,
    /// The tick it last felt something stop it or strike it: matter, the ground, a body. TUCH reads it.
    pub touched_at: f64,
    /// How dense the mana around it is (M/m³), as its pressure works it out (the spiky kernel).
    pub rho: f64,
    /// How dense it feels the mana around it is, which way it thickens, and how its neighbours move: what DENS, GRAD and
    /// NVEL read.
    pub felt: f64,
    pub grad: Vec3,
    pub nvel: Vec3,
    /// The force on it this step.
    pub acc: Vec3,
    /// Its mass, as of this step (mass_of).
    pub mass: f64,
    /// How hard the air held it up, as of its last step in the fluid.
    pub lifted: f64,
}

impl Particle {
    /// Its mass as of now (`mass` is as of the fluid's last step).
    pub fn mass_now(&self) -> f64 {
        mass_of(self)
    }
}

/// A particle's mass: its free mana, each part by its weight a M (PHYSICS.mana_mass): mass is mana (D40). The matter it
/// holds is matter, with its own mass (matter.rs).
pub fn mass_of(p: &Particle) -> f64 {
    mass_of_parts(&p.free)
}

/// The mass of some mana, free or condensed, by part (PHYSICS.mana_mass).
pub fn mass_of_parts(p: &Parts) -> f64 {
    let m = physics().mana_mass;
    p[0] * m[0] + p[1] * m[1] + p[2] * m[2] + p[3] * m[3]
}

/// How many M of a part's matter a cell holds, packed full: its density over its weight a M.
pub fn packed(k: usize) -> f64 {
    let ph = physics();
    (ph.density[k] * js::pow(ph.cell, 3.0)) / ph.mana_mass[k]
}

/// The share of a cell's room some matter takes: each part's amount over what a full cell of it holds.
pub fn fill_of(m: &Parts) -> f64 {
    m[0] / packed(0) + m[1] / packed(1) + m[2] / packed(2) + m[3] / packed(3)
}

/// How many M of a part a cell of the world's ground holds: full.
pub fn ground_amount(k: usize) -> f64 {
    packed(k)
}

/// Momentum given to the world from outside it: weight, and the ground and walls, which hold up and stop what's against
/// them, and take what's pushed off them; and sleeping matter, which holds still what rests on it. Pushes and
/// kicks are inside it: what's pushed, and what it's pushed off (a caster's body, the air, the ground), take equal and
/// opposite shares. `outside` is what's given by hand, from beyond the world: a test setting something moving.
#[derive(Clone, Copy, Debug, Default)]
pub struct Impulses {
    pub gravity: Vec3,
    pub walls: Vec3,
    pub outside: Vec3,
}

pub const FIRE: usize = 0;
pub const WATER: usize = 1;
pub const AIR: usize = 2;
pub const EARTH: usize = 3;

#[derive(Clone, Debug)]
pub struct World {
    pub cell: f64,
    pub w: usize,
    pub h: usize,
    pub d: usize,
    pub size: usize,
    /// Free mana floating in each cell.
    pub air: Vec<Parts>,
    /// Where matter is, cell by cell: the points in it and its gases. Worked out from them (`rasterize`); change the points
    /// or the gases, not this.
    pub matter: Vec<Parts>,
    /// Matter: material points (matter.rs).
    pub points: Vec<Point>,
    /// The gases condensed mana makes (flame and air matter), in each cell. *(They get a pressure in PLAN step 3.)*
    pub gas: Vec<Parts>,
    /// How the air in each cell moves: x, y, z, cell by cell.
    pub air_vel: Vec<f64>,
    /// The particles, in the order they came into the world. Add and take them away only through the world's own
    /// methods (pour, spawn, retain_particles), which keep `slots` up to date.
    pub particles: Vec<Particle>,
    pub bodies: Vec<Body>,
    pub tick: i64,
    pub impulse: Impulses,
    /// Energy that motion has turned into heat, by how: kg·(m/tick)², the kilogram being 1 M of free mana (one is 900 J).
    /// Counted where it happens, exactly: two things that even out their speeds lose what the evening out takes (D29).
    pub heat: IndexMap<String, f64>,
    next_body: u32,
    next_particle: u64,
    next_point: u64,
    /// Where each particle is in `particles`, by id.
    slots: HashMap<u64, usize>,
    /// The sleeping points in each cell, and whether that's out of date.
    asleep_cells: Vec<Vec<usize>>,
    asleep_stale: bool,
    /// Every point in each cell, and whether that's out of date.
    point_cells: Vec<Vec<usize>>,
    point_cells_stale: bool,
}

impl World {
    pub fn new(w: usize, h: usize, d: usize) -> World {
        let size = w * h * d;
        World {
            cell: physics().cell,
            w,
            h,
            d,
            size,
            air: vec![zero(); size],
            matter: vec![zero(); size],
            points: Vec::new(),
            gas: vec![zero(); size],
            air_vel: vec![0.0; size * 3],
            particles: Vec::new(),
            bodies: Vec::new(),
            tick: 0,
            impulse: Impulses::default(),
            heat: IndexMap::new(),
            next_body: 1,
            next_particle: 1,
            next_point: 1,
            slots: HashMap::new(),
            asleep_cells: Vec::new(),
            asleep_stale: true,
            point_cells: Vec::new(),
            point_cells_stale: true,
        }
    }

    /// Motion turned into heat.
    pub fn warm(&mut self, how: &str, energy: f64) {
        // `if (energy)`: nothing for 0, and nothing for NaN.
        if energy != 0.0 && !energy.is_nan() {
            *self.heat.entry(how.to_string()).or_insert(0.0) += energy;
        }
    }

    /// The cell at (x, y, z), or -1 outside the world.
    pub fn index(&self, x: i64, y: i64, z: i64) -> isize {
        if x < 0 || y < 0 || z < 0 || x >= self.w as i64 || y >= self.h as i64 || z >= self.d as i64 {
            return -1;
        }
        ((z * self.h as i64 + y) * self.w as i64 + x) as isize
    }

    pub fn coords(&self, i: usize) -> [i64; 3] {
        let x = i % self.w;
        let y = (i / self.w) % self.h;
        let z = i / (self.w * self.h);
        [x as i64, y as i64, z as i64]
    }

    /// The cell a point is in, or -1 outside the world.
    pub fn cell_of(&self, p: &Vec3) -> isize {
        let c = self.cell;
        self.index(floor_i(p[0] / c + 1e-7), floor_i(p[1] / c + 1e-7), floor_i(p[2] / c + 1e-7))
    }

    /// The centre of a cell, in metres.
    pub fn centre(&self, x: i64, y: i64, z: i64) -> Vec3 {
        [(x as f64 + 0.5) * self.cell, (y as f64 + 0.5) * self.cell, (z as f64 + 0.5) * self.cell]
    }

    /// The nearest cell to a point, inside the world.
    pub fn clamped_cell_of(&self, p: &Vec3) -> usize {
        let c = self.cell;
        let clamp = |v: f64, n: usize| js::max(0.0, js::min(n as f64 - 1.0, (v / c + 1e-7).floor()));
        self.index(clamp(p[0], self.w) as i64, clamp(p[1], self.h) as i64, clamp(p[2], self.d) as i64) as usize
    }

    /// The mass of the air in a cell.
    pub fn air_mass(&self, i: usize) -> f64 {
        mass_of_parts(&self.air[i])
    }

    /// Earth and water, held or not: what blocks and what counts as a touch.
    pub fn solid_at(&self, i: isize) -> bool {
        if i < 0 {
            return true;
        }
        let m = &self.matter[i as usize];
        m[EARTH] / packed(EARTH) + m[WATER] / packed(WATER) >= physics().solid
    }

    /// The share of a cell's room its matter takes.
    pub fn fill(&self, i: usize) -> f64 {
        fill_of(&self.matter[i])
    }

    // Building worlds

    /// Fills every cell below `top` (in cells) with matter, and the open air above with air mana, as thick as it settles
    /// under its own weight: PHYSICS.air_mana a cell at the ground, thinning as it goes up. The ground is at rest, pressed
    /// by its own weight as it would have settled, and asleep.
    pub fn with_ground(w: usize, h: usize, d: usize, top: usize, part: usize) -> World {
        let mut world = World::new(w, h, d);
        let hh = scale_height();
        let air_mana = physics().air_mana;
        for i in 0..world.size {
            let [x, y, z] = world.coords(i);
            if y < top as i64 {
                let mut amount = zero();
                amount[part] = ground_amount(part);
                world.lay(x, y, z, amount, Some(top as f64 * world.cell));
            } else {
                let v = (air_mana / 4.0) * js::exp(-((y as f64 + 0.5 - top as f64) * world.cell) / hh);
                world.air[i] = [v; 4];
            }
        }
        world.rasterize();
        world
    }

    /// Fills a box of cells (inclusive) with matter, where the air mana there was. For building a world, before it runs.
    /// More than `ground_amount` a cell is matter packed denser than its parts' own density: earth packed into rock.
    pub fn fill_box(&mut self, from: [i64; 3], to: [i64; 3], part: usize, amount: f64) {
        for z in from[2]..=to[2] {
            for y in from[1]..=to[1] {
                for x in from[0]..=to[0] {
                    let i = self.index(x, y, z);
                    if i < 0 {
                        continue;
                    }
                    let mut parts = zero();
                    parts[part] = amount;
                    self.lay(x, y, z, parts, None);
                    self.air[i as usize] = zero();
                }
            }
        }
        self.rasterize();
    }

    /// Lays matter into cell (x, y, z) as it's built: material points filling it, asleep, or gas. With a surface at
    /// `top` metres, the points are pressed by the weight above them, as the ground would have settled.
    fn lay(&mut self, x: i64, y: i64, z: i64, parts: Parts, top: Option<f64>) {
        let ph = physics();
        if dominant(&parts) == FIRE || dominant(&parts) == AIR {
            let i = self.index(x, y, z) as usize;
            add(&mut self.gas[i], &parts);
            return;
        }
        let n = ph.matter_points;
        let dims = if self.d == 1 { 2 } else { 3 };
        let count = if dims == 2 { n * n } else { n * n * n };
        let each = [parts[0] / count as f64, parts[1] / count as f64, parts[2] / count as f64, parts[3] / count as f64];
        let volume = js::pow(self.cell, 3.0) / count as f64;
        let nz = if dims == 2 { 1 } else { n };
        for k in 0..nz {
            for j in 0..n {
                for i in 0..n {
                    let at = |c: i64, o: usize| (c as f64 + (o as f64 + 0.5) / n as f64) * self.cell;
                    let pos = [at(x, i), at(y, j), if dims == 2 { (z as f64 + 0.5) * self.cell } else { at(z, k) }];
                    let id = self.next_point;
                    self.next_point += 1;
                    let mut p = Point::new(id, pos, each, volume);
                    if let Some(top) = top {
                        settled(&mut p, top - pos[1], dims);
                    }
                    p.asleep = true;
                    self.points.push(p);
                }
            }
        }
        self.points_moved();
    }

    /// Matter condensed out of mana at a point, moving as the mana did: earth and water become a material point, flame
    /// and air matter a gas in the cell. It takes the room its parts take at their own density. Mass and momentum are
    /// the mana's.
    pub fn make_matter(&mut self, parts: Parts, at: Vec3, vel: Vec3) {
        if total(&parts) <= 0.0 {
            return;
        }
        let c = self.clamped_cell_of(&at);
        add(&mut self.matter[c], &parts);
        if dominant(&parts) == FIRE || dominant(&parts) == AIR {
            add(&mut self.gas[c], &parts);
            // A gas in a cell is at rest: the ground takes its motion. *(Gases move in PLAN step 3.)*
            let m = mass_of_parts(&parts);
            for a in 0..3 {
                self.impulse.walls[a] -= m * vel[a];
            }
            self.warm("condensing", 0.5 * m * (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]));
            return;
        }
        let id = self.next_point;
        self.next_point += 1;
        let mut p = Point::new(id, at, parts, super::matter::room_of(&parts));
        p.vel = vel;
        self.points.push(p);
        self.points_moved();
    }

    /// Works out where matter is, cell by cell (`matter`), from the points and the gases.
    pub fn rasterize(&mut self) {
        self.matter.clone_from(&self.gas);
        for p in &self.points {
            let c = self.clamped_cell_of(&p.pos);
            add(&mut self.matter[c], &p.parts);
        }
    }

    /// Which points sleep, or where they are, has changed: the indexes of points by cell are out of date.
    pub fn points_moved(&mut self) {
        self.asleep_stale = true;
        self.point_cells_stale = true;
    }

    /// The points in a cell, by index into `points`.
    pub fn points_in(&mut self, cell: usize) -> &[usize] {
        if self.point_cells_stale {
            self.point_cells = vec![Vec::new(); self.size];
            for (i, p) in self.points.iter().enumerate() {
                let c = self.clamped_cell_of(&p.pos);
                self.point_cells[c].push(i);
            }
            self.point_cells_stale = false;
        }
        self.point_cells.get(cell).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Brings the index of sleeping points by cell up to date.
    pub fn index_asleep(&mut self) {
        if !self.asleep_stale {
            return;
        }
        self.asleep_cells = vec![Vec::new(); self.size];
        for (i, p) in self.points.iter().enumerate() {
            if p.asleep {
                let c = self.clamped_cell_of(&p.pos);
                self.asleep_cells[c].push(i);
            }
        }
        self.asleep_stale = false;
    }

    /// The sleeping points in a cell (after `index_asleep`).
    pub fn asleep_in(&self, cell: usize) -> &[usize] {
        self.asleep_cells.get(cell).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Keeps only the points `keep` says to.
    pub fn retain_points(&mut self, keep: impl FnMut(&Point) -> bool) {
        self.points.retain(keep);
        self.points_moved();
    }

    /// A body's mass is in M, like mana's: a person is about 60. Returns its index in `bodies`.
    pub fn add_body(&mut self, name: &str, pos: Vec3, mass: f64, half: Vec3) -> usize {
        let id = self.next_body;
        self.next_body += 1;
        self.bodies.push(Body { id, name: name.to_string(), pos, vel: [0.0; 3], mass, half });
        self.bodies.len() - 1
    }

    /// A person: 60, and 0.5 by 1.8 by 0.5 m.
    pub fn add_person(&mut self, name: &str, pos: Vec3) -> usize {
        self.add_body(name, pos, 60.0, [0.25, 0.9, 0.25])
    }

    /// The body whose box holds a point, if any (an index into `bodies`).
    pub fn body_at(&self, p: &Vec3, except: Option<usize>) -> Option<usize> {
        let c = self.cell / 2.0;
        self.bodies.iter().enumerate().position(|(i, b)| {
            Some(i) != except
                && (p[0] - b.pos[0]).abs() <= b.half[0] + c
                && (p[1] - b.pos[1]).abs() <= b.half[1] + c
                && (self.d == 1 || (p[2] - b.pos[2]).abs() <= b.half[2] + c)
        })
    }

    // Particles and air

    /// Where the particle with this id is in `particles`, if it's in the world.
    pub fn slot(&self, id: u64) -> Option<usize> {
        self.slots.get(&id).copied()
    }

    /// The particle with this id, if it's in the world.
    pub fn particle(&self, id: u64) -> Option<&Particle> {
        self.slot(id).map(|i| &self.particles[i])
    }

    pub fn particle_mut(&mut self, id: u64) -> Option<&mut Particle> {
        self.slot(id).map(|i| &mut self.particles[i])
    }

    /// Keeps only the particles `keep` says to, in their order.
    pub fn retain_particles(&mut self, keep: impl FnMut(&Particle) -> bool) {
        self.particles.retain(keep);
        self.slots.clear();
        for (i, p) in self.particles.iter().enumerate() {
            self.slots.insert(p.id, i);
        }
    }

    /// New particles of free mana at a point, at most a mote each, spread over `spread` metres so their pressure can act.
    /// Empties `parts`. Returns the new particles' ids.
    pub fn pour(&mut self, parts: &mut Parts, at: Vec3, weave: u32, vel: Vec3, spread: f64) -> Vec<u64> {
        let ph = physics();
        let mut out = Vec::new();
        let n = (total(parts) / ph.mote - 1e-9).ceil();
        if !(n > 0.0) {
            return out;
        }
        let cell = self.cell;
        for _ in 0..n as usize {
            let id = self.next_particle;
            self.next_particle += 1;
            // A fixed scatter by id, so the same spell always lays out the same way.
            let u = frac(id as f64 * 0.7548776662) - 0.5;
            let v = frac(id as f64 * 0.5698402910) - 0.5;
            let w = frac(id as f64 * 0.3141592653) - 0.5;
            // Within the cell the point is in: mana poured at a point is poured into that cell.
            let in_cell = |x: f64, d: f64| {
                let lo = (x / cell + 1e-7).floor() * cell;
                js::min(lo + cell * 0.999, js::max(lo + cell * 0.001, x + d))
            };
            let pos = [
                in_cell(at[0], u * 2.0 * spread),
                in_cell(at[1], v * 2.0 * spread),
                if self.d == 1 { at[2] } else { in_cell(at[2], w * 2.0 * spread) },
            ];
            self.particles.push(Particle {
                id,
                pos,
                vel,
                free: [parts[0] / n, parts[1] / n, parts[2] / n, parts[3] / n],
                weave,
                order: None,
                regs: vec![0.0; ph.weave_registers],
                stamp: vec![0.0; ph.weave_registers],
                yaw: 0.0,
                pushed_at: -1.0,
                touched_at: -1.0,
                rho: 0.0,
                felt: 0.0,
                grad: [0.0; 3],
                nvel: [0.0; 3],
                acc: [0.0; 3],
                mass: 0.0,
                lifted: 0.0,
            });
            self.slots.insert(id, self.particles.len() - 1);
            out.push(id);
        }
        *parts = zero();
        out
    }

    /// `pour`, spread as mana poured at a point is (PHYSICS.pour), at rest.
    pub fn pour_still(&mut self, parts: &mut Parts, at: Vec3, weave: u32) -> Vec<u64> {
        self.pour(parts, at, weave, [0.0; 3], physics().pour)
    }

    /// A new particle like the one at index `i`, with a new id and nothing in it yet. Returns its index.
    pub fn spawn(&mut self, i: usize) -> usize {
        let id = self.next_particle;
        self.next_particle += 1;
        let mut q = self.particles[i].clone();
        q.id = id;
        q.free = zero();
        q.acc = [0.0; 3];
        // What held the old one up was the old one's.
        q.lifted = 0.0;
        self.particles.push(q);
        self.slots.insert(id, self.particles.len() - 1);
        self.particles.len() - 1
    }

    /// Mana let into the air of a cell, carrying momentum (px, py, pz) into it. It comes to the air's speed there: what
    /// that evening out takes from their motion is heat.
    pub fn add_air(&mut self, i: isize, parts: &Parts, momentum: Vec3) {
        if i < 0 {
            return;
        }
        let i = i as usize;
        let before = self.air_mass(i);
        let m = mass_of_parts(parts);
        let after = before + m;
        if after <= 0.0 {
            return;
        }
        let mut lost = 0.0;
        for k in 0..3 {
            let v = self.air_vel[i * 3 + k];
            let next = (before * v + momentum[k]) / after;
            lost += 0.5 * before * v * v + (if m > 0.0 { (momentum[k] * momentum[k]) / (2.0 * m) } else { 0.0 })
                - 0.5 * after * next * next;
            self.air_vel[i * 3 + k] = next;
        }
        self.warm("mixing", lost);
        add(&mut self.air[i], parts);
    }

    /// Mana drawn out of the air of a cell. Returns it, and the momentum it takes with it.
    pub fn take_air(&mut self, i: usize, f: f64) -> (Parts, Vec3) {
        let parts = share(&mut self.air[i], f);
        let m = mass_of_parts(&parts);
        (parts, [m * self.air_vel[i * 3], m * self.air_vel[i * 3 + 1], m * self.air_vel[i * 3 + 2]])
    }

    // Each tick

    /// Bodies stand on the ground, which holds them up, and slide along it when they're pushed, until friction stops
    /// them: the ground takes from their speed, each tick, as much as their weight pressing on it lets it
    /// (PHYSICS.friction × g). What it takes is heat. A body that runs into something solid stops.
    pub fn move_bodies(&mut self) {
        let ph = physics();
        let grip = ph.friction * ph.gravity;
        let mut heat = 0.0;
        for bi in 0..self.bodies.len() {
            let before = self.bodies[bi].vel;
            let b = &self.bodies[bi];
            let ke = 0.5 * b.mass * (js::pow(b.vel[0], 2.0) + js::pow(b.vel[2], 2.0));
            let speed = js::hypot2(b.vel[0], b.vel[2]);
            if speed > 0.0 {
                let next = [b.pos[0] + b.vel[0], b.pos[1], b.pos[2] + b.vel[2]];
                let i = self.cell_of(&next);
                let open = i >= 0 && !self.solid_at(i);
                let b = &mut self.bodies[bi];
                if open {
                    b.pos = next;
                } else {
                    b.vel = [0.0; 3];
                }
                let slow = js::min(1.0, grip / speed);
                b.vel[0] -= b.vel[0] * slow;
                b.vel[2] -= b.vel[2] * slow;
            }
            let b = &mut self.bodies[bi];
            heat += ke - 0.5 * b.mass * (js::pow(b.vel[0], 2.0) + js::pow(b.vel[2], 2.0));
            for k in [0, 2] {
                self.impulse.walls[k] += b.mass * (b.vel[k] - before[k]);
            }
            // It stands: nothing moves it up or down.
            self.impulse.walls[1] -= b.mass * b.vel[1];
            b.vel[1] = 0.0;
        }
        self.warm("friction", heat);
    }

    /// The gases follow their nature, cell by cell: air matter and flame rise, and flame spreads, thinning into warmth.
    /// The room the points take, they can't go into. *(Earth and water are points, matter.rs; the gases get a pressure in
    /// PLAN step 3.)*
    pub fn settle_gas(&mut self) {
        // The room matter that isn't gas takes, cell by cell.
        let carried: Vec<f64> = (0..self.size).map(|i| js::max(0.0, fill_of(&self.gas[i]) - fill_of(&self.gas[i]))).collect();
        let ph = physics();
        let mut moved = vec![false; self.size];
        // How much of a cell's room each M of a cell's matter takes.
        let each = |m: &Parts| if total(m) > 0.0 { fill_of(m) / total(m) } else { 0.0 };
        let flip: i64 = if self.tick % 2 == 0 { 1 } else { -1 };
        let sides: Vec<(i64, i64)> =
            if self.d > 1 { vec![(flip, 0), (-flip, 0), (0, flip), (0, -flip)] } else { vec![(flip, 0), (-flip, 0)] };

        let room = |w: &World, j: usize| 1.0 - fill_of(&w.gas[j]) - carried[j];
        let try_move = |w: &mut World, moved: &mut Vec<bool>, i: usize, j: isize, amount: f64| -> f64 {
            if j < 0 {
                return 0.0;
            }
            let j = j as usize;
            let r = room(w, j);
            if r <= ph.epsilon {
                return 0.0;
            }
            let e = each(&w.gas[i]);
            let moving = take(&mut w.gas[i], js::min(amount, r / e));
            add(&mut w.gas[j], &moving);
            moved[j] = true;
            total(&moving)
        };

        for y in 0..self.h as i64 {
            for z in 0..self.d as i64 {
                for x in 0..self.w as i64 {
                    let i = self.index(x, y, z) as usize;
                    if moved[i] || total(&self.gas[i]) <= ph.epsilon {
                        continue;
                    }
                    let kind = dominant(&self.gas[i]);
                    let falls = kind == EARTH || kind == WATER;
                    let dy = if falls { -1 } else { 1 };
                    let here = |w: &World| total(&w.gas[i]);
                    let amount = here(self);
                    let below = self.index(x, y + dy, z);
                    try_move(self, &mut moved, i, below, amount);
                    for &(sx, sz) in &sides {
                        if here(self) <= ph.epsilon {
                            break;
                        }
                        let amount = here(self);
                        let j = self.index(x + sx, y + dy, z + sz);
                        try_move(self, &mut moved, i, j, amount);
                    }
                    if here(self) <= ph.epsilon {
                        continue;
                    }
                    // What couldn't fall or rise: water and air spread to the sides, flame thins into warmth.
                    if kind == WATER || kind == AIR {
                        for &(sx, sz) in &sides {
                            let j = self.index(x + sx, y, z + sz);
                            let fj = if j >= 0 { fill_of(&self.gas[j as usize]) + carried[j as usize] } else { 1.0 };
                            let fi = fill_of(&self.gas[i]);
                            if fj < fi {
                                let amount = (fi - fj) / 2.0 / each(&self.gas[i]);
                                try_move(self, &mut moved, i, j, amount);
                            }
                        }
                    } else if kind == FIRE {
                        // Flame spreads as any gas does: from where there's more of it to where there's less, evening out
                        // with its neighbours by a share of the difference (Fick's law), and no further once they're even.
                        for &(sx, sz) in &sides {
                            let j = self.index(x + sx, y, z + sz);
                            if j < 0 || room(self, j as usize) <= 0.0 {
                                continue;
                            }
                            let j = j as usize;
                            let diff = self.gas[i][FIRE] - self.gas[j][FIRE];
                            if diff <= 0.0 {
                                continue;
                            }
                            let m = &self.gas[i];
                            let f = js::min(
                                room(self, j) / each(m),
                                ((ph.flame_spread * diff) / (sides.len() as f64 + 1.0)) * (total(m) / m[FIRE]),
                            );
                            let taken = take(&mut self.gas[i], f);
                            add(&mut self.gas[j], &taken);
                            moved[j] = true;
                        }
                    }
                }
            }
        }
        self.rasterize();
    }

    /// All the mana in the world that isn't in a caster, by where it is. Particles in weaves are counted by the machine.
    pub fn mana(&self) -> WorldMana {
        let mut air = 0.0;
        let mut matter = 0.0;
        for i in 0..self.size {
            air += total(&self.air[i]);
            matter += total(&self.gas[i]);
        }
        matter += super::matter::mana_in(&self.points);
        let mut loose = 0.0;
        for p in &self.particles {
            if p.weave == 0 {
                loose += total(&p.free);
            }
        }
        WorldMana { air, matter, loose }
    }

    /// Momentum in the world (particles, air, bodies) less what came from outside: zero, when nothing is lost.
    pub fn momentum_error(&self) -> f64 {
        let mut m = [0.0; 3];
        for p in &self.particles {
            let mass = mass_of(p);
            for k in 0..3 {
                m[k] += mass * p.vel[k];
            }
        }
        for i in 0..self.size {
            let mass = self.air_mass(i);
            if mass != 0.0 && !mass.is_nan() {
                for k in 0..3 {
                    m[k] += mass * self.air_vel[i * 3 + k];
                }
            }
        }
        for b in &self.bodies {
            for k in 0..3 {
                m[k] += b.mass * b.vel[k];
            }
        }
        for p in &self.points {
            for k in 0..3 {
                m[k] += p.mass * p.vel[k];
            }
        }
        for j in [self.impulse.gravity, self.impulse.walls, self.impulse.outside] {
            for k in 0..3 {
                m[k] -= j[k];
            }
        }
        js::hypot(&m)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldMana {
    pub air: f64,
    pub matter: f64,
    pub loose: f64,
}

fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// `Math.floor`, as a cell coordinate.
pub fn floor_i(x: f64) -> i64 {
    x.floor() as i64
}
