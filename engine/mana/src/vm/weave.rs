//! A weave: mana laid out in the world as a thing. A fireball, a wall, a shield. Its mana is particles (SPEC §11): those
//! that carry its order, and, until they're given one, those its caster can still reach.

use std::rc::Rc;
use std::sync::Arc;

use super::parts::total;
use super::physics::physics;
use super::world::{Particle, Vec3, World};
use crate::asm::Code;
use crate::js;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Locks {
    pub input: bool,
    pub order: bool,
}

/// What a caster felt of a weave, and when: it stands until anything it was worked out from changes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FeltKey {
    pub tick: i64,
    pub version: u64,
    pub len: usize,
    pub body: [u64; 3],
    pub reach: u64,
}

#[derive(Clone, Debug)]
pub struct Weave {
    pub id: u32,
    /// Its caster, in `sim.casters`.
    pub maker: usize,
    /// The spell it was woven by: its order is code there.
    pub program: Arc<Code>,
    /// Where positions in it are measured from. In hand, it's where it was begun, and EMIT lays mana out around it. Set
    /// loose, it's its centre: the middle of its mana, which moves as its mana does.
    pub origin: Vec3,
    /// Turn about the vertical, in radians: 0 faces +z.
    pub yaw: f64,
    /// Its particles, by id.
    pub particles: Vec<u64>,
    /// What its caster has written into its registers (WSET), and when: the mana they pour into it (EMIT) carries this.
    /// The registers themselves live in its particles, each with its own copy (Particle::regs).
    pub regs: Vec<f64>,
    pub stamp: Vec<f64>,
    /// The routine its particles are ingrained with (INGR): an address in `program`.
    pub order: Option<usize>,
    pub in_hand: bool,
    pub manifested_at: f64,
    pub locks: Locks,
    /// Why its last particles left it: how it ended, once it has.
    pub last_left: String,
    pub(crate) felt: Option<(FeltKey, Rc<Vec<u64>>)>,
}

impl Weave {
    pub fn new(id: u32, maker: usize, program: Arc<Code>, origin: Vec3) -> Weave {
        let r = physics().weave_registers;
        Weave {
            id,
            maker,
            program,
            origin,
            yaw: 0.0,
            particles: Vec::new(),
            regs: vec![0.0; r],
            stamp: vec![0.0; r],
            order: None,
            in_hand: true,
            manifested_at: -1.0,
            locks: Locks::default(),
            last_left: String::new(),
            felt: None,
        }
    }

    /// A direction in the weave's frame, turned into the world's.
    pub fn to_world(&self, v: Vec3) -> Vec3 {
        turn_to_world(self.yaw, v)
    }

    /// A direction in the world, turned into the weave's frame.
    pub fn to_frame(&self, v: Vec3) -> Vec3 {
        turn_to_frame(self.yaw, v)
    }

    /// Its particles that are in the world.
    pub fn parts<'w>(&self, world: &'w World) -> impl Iterator<Item = &'w Particle> + use<'w, '_> {
        self.particles.iter().filter_map(|&id| world.particle(id))
    }

    /// Register k as its mana has it: the newest copy among its particles (for whoever watches; nothing in the world
    /// reads it this way). With no particles, what its caster wrote.
    pub fn reg(&self, world: &World, k: usize) -> f64 {
        let mut best = self.stamp[k];
        let mut v = self.regs[k];
        for p in self.parts(world) {
            if p.stamp[k] > best {
                best = p.stamp[k];
                v = p.regs[k];
            }
        }
        v
    }

    /// A point in the weave's frame, in the world.
    pub fn world_pos(&self, off: Vec3) -> Vec3 {
        let r = self.to_world(off);
        [self.origin[0] + r[0], self.origin[1] + r[1], self.origin[2] + r[2]]
    }

    /// The middle of its mana, and how it moves on average.
    pub fn centre(&self, world: &World) -> Option<(Vec3, Vec3)> {
        centre_of(self.parts(world))
    }

    /// Free mana in it.
    pub fn mana(&self, world: &World) -> f64 {
        self.parts(world).fold(0.0, |s, p| s + total(&p.free))
    }
}

/// A direction in a frame turned `yaw` about the vertical, in the world.
pub fn turn_to_world(yaw: f64, v: Vec3) -> Vec3 {
    let c = js::cos(yaw);
    let s = js::sin(yaw);
    [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]]
}

/// A direction in the world, in a frame turned `yaw` about the vertical.
pub fn turn_to_frame(yaw: f64, v: Vec3) -> Vec3 {
    let c = js::cos(yaw);
    let s = js::sin(yaw);
    [c * v[0] - s * v[2], v[1], s * v[0] + c * v[2]]
}

/// A register write's stamp: the tick it was written, and who wrote it (a particle's id; 0, its caster) to break ties
/// the same way every time. The newest wins; in the same tick, the caster, then the oldest particle.
pub fn stamp_of(tick: f64, writer: f64) -> f64 {
    tick * 1048576.0 + (1048575.0 - (writer % 1048576.0))
}

/// The middle of some particles' mana, and how it moves on average: (position, velocity).
pub fn centre_of<'a>(ps: impl IntoIterator<Item = &'a Particle>) -> Option<(Vec3, Vec3)> {
    let mut m = 0.0;
    let mut pos = [0.0; 3];
    let mut vel = [0.0; 3];
    for p in ps {
        let mass = total(&p.free);
        m += mass;
        for k in 0..3 {
            pos[k] += mass * p.pos[k];
            vel[k] += mass * p.vel[k];
        }
    }
    if m <= 0.0 {
        return None;
    }
    Some(([pos[0] / m, pos[1] / m, pos[2] / m], [vel[0] / m, vel[1] / m, vel[2] / m]))
}
