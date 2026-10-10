//! The world: a grid of cells, and matter: material points over the grid (matter.rs), and the gases in the cells. The air
//! is a gas, air matter, with free mana in it (air.rs). A 2D world is one cell deep.

use std::collections::HashMap;
use std::sync::Arc;

use indexmap::IndexMap;

use super::air::{rest_mass, scale_height};
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

/// The share of a cell's room some matter takes: its earth and water, each by its amount over what a full cell of it
/// holds. Gases (flame, air matter) have no packing: they fill the room that's left (air.rs).
pub fn fill_of(m: &Parts) -> f64 {
    m[WATER] / packed(WATER) + m[EARTH] / packed(EARTH)
}

/// How many M of a part a cell of the world's ground holds: full.
pub fn ground_amount(k: usize) -> f64 {
    packed(k)
}

/// Momentum given to the world from outside it: weight, and the ground and walls, which hold up and stop what's against
/// them, and take what's pushed off them; and sleeping matter, which holds still what rests on it. Pushes and
/// kicks are inside it: what's pushed, and what it's pushed off (a caster's body, the air, the ground), take equal and
/// opposite shares. `outside` is what's given by hand, from beyond the world: a test setting something moving. `beyond` is
/// what the air past the world's open edges gives and takes: its pressure there, and what air carries across.
#[derive(Clone, Copy, Debug, Default)]
pub struct Impulses {
    pub gravity: Vec3,
    pub walls: Vec3,
    pub outside: Vec3,
    pub beyond: Vec3,
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
    /// Free mana in the air of each cell.
    pub air: Vec<Parts>,
    /// Where matter is, cell by cell: the points in it and its gases. Worked out from them (`rasterize`); change the points
    /// or the gases, not this.
    pub matter: Vec<Parts>,
    /// Matter: material points (matter.rs).
    pub points: Vec<Point>,
    /// The gases condensed mana makes, in each cell: air matter, which the open air is, and flame.
    pub gas: Vec<Parts>,
    /// How the air in each cell moves, its gases and the free mana in them alike: x, y, z, cell by cell.
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
    /// What has gone past the world's open edges, less what came in from beyond (air.rs): free mana, gases (M), and
    /// Energy (as `heat` counts it). Nothing is lost: it's in the world beyond.
    pub beyond: Parts,
    pub beyond_gas: Parts,
    pub beyond_energy: f64,
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
            beyond: zero(),
            beyond_gas: zero(),
            beyond_energy: 0.0,
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

    /// The mass of the air in a cell: its gases and the free mana in them.
    pub fn air_mass(&self, i: usize) -> f64 {
        mass_of_parts(&self.air[i]) + mass_of_parts(&self.gas[i])
    }

    /// How much gas the air in a cell is, M: its gases and the free mana in them. Each M presses alike (the ideal gas
    /// law counts it).
    pub fn air_amount(&self, i: usize) -> f64 {
        total(&self.air[i]) + total(&self.gas[i])
    }

    /// Earth and water, held or not: what blocks and what counts as a touch.
    pub fn solid_at(&self, i: isize) -> bool {
        if i < 0 {
            return true;
        }
        fill_of(&self.matter[i as usize]) >= physics().solid
    }

    /// The share of a cell's room its matter takes.
    pub fn fill(&self, i: usize) -> f64 {
        fill_of(&self.matter[i])
    }

    // Building worlds

    /// Fills every cell below `top` (in cells) with matter, and the open air above with air: air matter as dense as real
    /// air, and PHYSICS.air_mana of free mana in it, at the ground, thinning as it goes up as the air settles under its own
    /// weight. The ground is at rest, pressed by its own weight as it would have settled, and asleep.
    pub fn with_ground(w: usize, h: usize, d: usize, top: usize, part: usize) -> World {
        let mut world = World::new(w, h, d);
        let (mana, gas) = still_air();
        let hh = scale_height(rest_mass(total(&mana) + total(&gas), mass_of_parts(&mana) + mass_of_parts(&gas)));
        for i in 0..world.size {
            let [x, y, z] = world.coords(i);
            if y < top as i64 {
                let mut amount = zero();
                amount[part] = ground_amount(part);
                world.lay(x, y, z, amount, Some(top as f64 * world.cell));
            } else {
                let f = js::exp(-((y as f64 + 0.5 - top as f64) * world.cell) / hh);
                world.air[i] = mana.map(|m| m * f);
                world.gas[i] = gas.map(|m| m * f);
            }
        }
        world.rasterize();
        world
    }

    /// Takes the air out of the world: its gases and the mana in them, and how they moved. Nothing presses, drags or
    /// holds up (tests).
    pub fn empty_air(&mut self) {
        self.air.fill(zero());
        self.gas.fill(zero());
        self.air_vel.fill(0.0);
        self.rasterize();
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
                    self.gas[i as usize] = zero();
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

    /// Matter condensed out of mana at a point, moving as the mana did: earth and water become a material point, taking
    /// the room its parts take at their own density; flame and air matter a gas in the air of the cell. Mass and momentum
    /// are the mana's.
    pub fn make_matter(&mut self, parts: Parts, at: Vec3, vel: Vec3) {
        if total(&parts) <= 0.0 {
            return;
        }
        let c = self.clamped_cell_of(&at);
        add(&mut self.matter[c], &parts);
        if dominant(&parts) == FIRE || dominant(&parts) == AIR {
            let m = mass_of_parts(&parts);
            self.join_air(c, m, [m * vel[0], m * vel[1], m * vel[2]]);
            add(&mut self.gas[c], &parts);
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
        self.make_room_for(self.bodies.len() - 1);
        self.bodies.len() - 1
    }

    /// A body put into the world takes room in the air, and the air it takes the place of goes out over all the rest of
    /// the open air, each cell taking a share by how much it holds: so the air is pressed no harder anywhere, as if it had
    /// made way for the body as the body came, and every M of it is kept.
    fn make_room_for(&mut self, b: usize) {
        let mut moved_mana = zero();
        let mut moved_gas = zero();
        let mut taken: Vec<usize> = Vec::new();
        for (i, room, _) in super::air::body_room(self, b) {
            if self.solid_at(i as isize) {
                continue;
            }
            add(&mut moved_mana, &share(&mut self.air[i], room));
            add(&mut moved_gas, &share(&mut self.gas[i], room));
            taken.push(i);
        }
        let mut rest = 0.0;
        for i in 0..self.size {
            if !self.solid_at(i as isize) && !taken.contains(&i) {
                rest += self.air_amount(i);
            }
        }
        if rest <= 0.0 {
            return;
        }
        for i in 0..self.size {
            if self.solid_at(i as isize) || taken.contains(&i) {
                continue;
            }
            let f = self.air_amount(i) / rest;
            for k in 0..4 {
                self.air[i][k] += moved_mana[k] * f;
                self.gas[i][k] += moved_gas[k] * f;
            }
        }
        self.rasterize();
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
        self.join_air(i, mass_of_parts(parts), momentum);
        add(&mut self.air[i], parts);
    }

    /// Mass `m` joins the air of cell `i`, carrying `momentum`: the air there comes to one speed with it, and what that
    /// evening out takes from their motion is heat. (The caller adds what joined.)
    pub fn join_air(&mut self, i: usize, m: f64, momentum: Vec3) {
        let before = self.air_mass(i);
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
    }

    /// Moves `amount` M of the air of cell `from` into cell `to`: its gases and the free mana in them alike, with their
    /// momentum. What evening out its speed with the air it joins takes is heat.
    pub fn trade_air(&mut self, from: usize, to: usize, amount: f64) {
        let there = self.air_amount(from);
        if there <= 0.0 || amount <= 0.0 {
            return;
        }
        let f = js::min(1.0, amount / there);
        let mana = share(&mut self.air[from], f);
        let gas = share(&mut self.gas[from], f);
        let mass = mass_of_parts(&mana) + mass_of_parts(&gas);
        let v = [self.air_vel[from * 3], self.air_vel[from * 3 + 1], self.air_vel[from * 3 + 2]];
        self.join_air(to, mass, [mass * v[0], mass * v[1], mass * v[2]]);
        for k in 0..4 {
            self.air[to][k] += mana[k];
            self.gas[to][k] += gas[k];
        }
    }

    /// Matter moved, and the air makes room for it: each cell whose matter changed comes to as much air as its room
    /// holds at rest, the air in the cells matter came into going to the nearest cells it left. Sound is far faster than
    /// matter: the air gives way as it comes, so rising earth pushes the air above it down into the hole it leaves, and
    /// no hole is ever empty. What finds no match, and hollows sealed in the ground, seep to and from the air beyond
    /// (a stand-in, SPEC §0). `before` is each cell's fill before the matter moved.
    pub fn air_gives_way(&mut self, before: &[f64]) {
        let room = super::air::room(self);
        let volume = js::pow(self.cell, 3.0);
        let row = |w: &World, i: usize| (i / w.w) % w.h;
        // Cells the matter came into (air out of them), and cells it left (room for air): each comes to as much air as
        // its room holds at rest's thickness.
        let mut out: Vec<(usize, f64)> = Vec::new();
        let mut room_for: Vec<(usize, f64)> = Vec::new();
        for i in 0..self.size {
            let now = self.fill(i);
            if now == before[i] {
                continue;
            }
            let at_rest = if room.open[i] {
                room.at_rest(self, i)
            } else {
                room.rest[row(self, i)] * volume * js::max(0.0, 1.0 - now)
            };
            let have = self.air_amount(i);
            if have > at_rest {
                out.push((i, have - at_rest));
            } else if have < at_rest {
                room_for.push((i, at_rest - have));
            }
        }
        for k in 0..out.len() {
            let (from, mut m) = out[k];
            let at = self.coords(from);
            let mut near: Vec<(i64, usize)> = room_for
                .iter()
                .enumerate()
                .filter(|(_, r)| r.1 > 0.0)
                .map(|(k, r)| {
                    let c = self.coords(r.0);
                    ((c[0] - at[0]).pow(2) + (c[1] - at[1]).pow(2) + (c[2] - at[2]).pow(2), k)
                })
                .collect();
            near.sort_unstable();
            for (_, k) in near {
                if m <= 0.0 {
                    break;
                }
                let give = js::min(m, room_for[k].1);
                self.trade_air(from, room_for[k].0, give);
                room_for[k].1 -= give;
                m -= give;
            }
            out[k].1 = m;
        }
        // What found no match seeps through the ground, to and from the air beyond the world, at rest. *(A stand-in for
        // air seeping through soil, SPEC §0: matter holds no air of its own yet.)*
        // (The air beyond is as the world's is on average, before any of it seeps.)
        let avg = super::air::average(self, &room);
        for (to, want) in room_for {
            super::air::from_beyond(self, &room, &avg, to, want);
        }
        for (from, m) in out {
            super::air::to_beyond(self, &room, &avg, from, m);
        }
        // A hollow sealed in the ground, with no way through the open air to the world's open edges, holds air at the
        // air's pressure all the same: air seeps through the ground into it, or out of it. *(The same stand-in.)*
        let sealed = super::air::sealed(self, &room);
        for i in 0..self.size {
            if !sealed[i] {
                continue;
            }
            let (have, want) = (self.air_amount(i), room.at_rest(self, i));
            if want > have {
                super::air::from_beyond(self, &room, &avg, i, want - have);
            } else if have > want {
                super::air::to_beyond(self, &room, &avg, i, have - want);
            }
        }
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
    /// (PHYSICS.friction × g), before they move, so a push it can hold against moves them not at all. What it takes is
    /// heat. A body that runs into something solid stops.
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
                // The ground holds it first: a push it can hold against moves it not at all.
                let slow = js::min(1.0, grip / speed);
                let b = &mut self.bodies[bi];
                b.vel[0] -= b.vel[0] * slow;
                b.vel[2] -= b.vel[2] * slow;
                let next = [b.pos[0] + b.vel[0], b.pos[1], b.pos[2] + b.vel[2]];
                let i = self.cell_of(&next);
                let open = i >= 0 && !self.solid_at(i);
                let b = &mut self.bodies[bi];
                if open {
                    b.pos = next;
                } else {
                    b.vel = [0.0; 3];
                }
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

    /// Flame spreads into the air around it as any gas does: from where there's more of it to where there's less, evening
    /// out with its neighbours by a share of the difference each tick (Fick's law), swapped for as much of the gas there,
    /// so that what each cell holds, and so its pressure, stays as it was. What moves carries its momentum, and what
    /// evening out the speeds takes is heat. *(A stand-in: real gases spread far slower, by themselves; a flame thins
    /// by its heat, which has no place yet, PLAN step 4.)* How gases rise and flow is the air's (air.rs).
    pub fn spread_flame(&mut self) {
        let ph = physics();
        let n = self.size;
        let axes = if self.d == 1 { 2 } else { 3 };
        let (w, h) = (self.w, self.h);
        let step = [1, w, w * h];
        let limit = [w, h, self.d];
        // From a snapshot, so the order cells are visited in doesn't matter.
        let fire: Vec<f64> = self.gas.iter().map(|g| g[FIRE]).collect();
        let mut moves: Vec<(usize, usize, f64)> = Vec::new();
        for i in 0..n {
            if fire[i] <= ph.epsilon || self.solid_at(i as isize) {
                continue;
            }
            let at = self.coords(i);
            for ax in 0..axes {
                for dir in [-1i64, 1] {
                    let c = at[ax] + dir;
                    if c < 0 || c >= limit[ax] as i64 {
                        continue;
                    }
                    let j = if dir > 0 { i + step[ax] } else { i - step[ax] };
                    let diff = fire[i] - fire[j];
                    if diff <= 0.0 || self.solid_at(j as isize) {
                        continue;
                    }
                    moves.push((i, j, (ph.flame_spread * diff) / (2.0 * axes as f64 + 1.0)));
                }
            }
        }
        let mut heat = 0.0;
        for (i, j, f) in moves {
            // Flame from i to j, and as much of j's other gas back.
            let f = js::min(f, self.gas[i][FIRE]);
            let mut back = self.gas[j];
            back[FIRE] = 0.0;
            let back = take(&mut back, f);
            if total(&back) <= 0.0 {
                continue;
            }
            let mut flame = zero();
            flame[FIRE] = f;
            let (mf, mb) = (mass_of_parts(&flame), mass_of_parts(&back));
            let (mi, mj) = (self.air_mass(i), self.air_mass(j));
            let ui = [self.air_vel[i * 3], self.air_vel[i * 3 + 1], self.air_vel[i * 3 + 2]];
            let uj = [self.air_vel[j * 3], self.air_vel[j * 3 + 1], self.air_vel[j * 3 + 2]];
            let ke = |m: f64, u: &Vec3| 0.5 * m * (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]);
            let before = ke(mi, &ui) + ke(mj, &uj);
            self.gas[i][FIRE] -= f;
            add(&mut self.gas[j], &flame);
            for k in 0..4 {
                self.gas[j][k] -= back[k];
            }
            add(&mut self.gas[i], &back);
            let (ni, nj) = (mi - mf + mb, mj + mf - mb);
            let mut vi = [0.0; 3];
            let mut vj = [0.0; 3];
            for k in 0..3 {
                vi[k] = (mi * ui[k] - mf * ui[k] + mb * uj[k]) / ni;
                vj[k] = (mj * uj[k] + mf * ui[k] - mb * uj[k]) / nj;
                self.air_vel[i * 3 + k] = vi[k];
                self.air_vel[j * 3 + k] = vj[k];
            }
            heat += before - ke(ni, &vi) - ke(nj, &vj);
        }
        self.warm("mixing", heat);
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
        WorldMana { air, matter, loose, beyond: total(&self.beyond) + total(&self.beyond_gas) }
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
        for j in [self.impulse.gravity, self.impulse.walls, self.impulse.outside, self.impulse.beyond] {
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
    /// Gone past the world's open edges, less what came in.
    pub beyond: f64,
}

fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// The air in a cell of open air at the ground, when a world is made: the free mana in it, raw, and its gases (air
/// matter as dense as real air, PHYSICS.density).
pub fn still_air() -> (Parts, Parts) {
    let mana = physics().air_mana / 4.0;
    let mut gas = zero();
    gas[AIR] = packed(AIR);
    ([mana; 4], gas)
}

/// `Math.floor`, as a cell coordinate.
pub fn floor_i(x: f64) -> i64 {
    x.floor() as i64
}
