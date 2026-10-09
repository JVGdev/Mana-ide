//! Worlds to cast the four spells in: shared by the tests, the bench and mvm.

use crate::vm::caster::{Will, master};
use crate::vm::sim::Sim;
use crate::vm::world::{EARTH, Vec3, World, ground_amount};

pub struct Scene {
    pub sim: Sim,
    /// The caster, in `sim.casters`.
    pub caster: usize,
    pub dims: u8,
    pub ground: usize,
    /// Someone to blow back (the Gust's), in `sim.world.bodies`.
    pub target: Option<usize>,
}

impl Scene {
    pub fn will(&mut self) -> &mut Will {
        &mut self.sim.casters[self.caster].will
    }

    pub fn caster_pos(&self) -> Vec3 {
        self.sim.world.bodies[self.sim.casters[self.caster].body].pos
    }
}

/// Flat ground, 2 m deep, and a caster standing on it 2 m in.
pub fn field(dims: u8) -> Scene {
    let ground = 8; // cells: 2 m
    let world =
        if dims == 2 { World::with_ground(64, 32, 1, ground, EARTH) } else { World::with_ground(48, 24, 48, ground, EARTH) };
    let mut sim = Sim::new(world);
    let z = if dims == 2 { sim.world.cell / 2.0 } else { 6.0 };
    let y = ground as f64 * sim.world.cell + 0.9;
    let caster = sim.add_caster("Mage", [2.0, y, z], None);
    Scene { sim, caster, dims, ground, target: None }
}

/// A point on the surface of the ground, `x` metres along.
pub fn on_ground(s: &Scene, x: f64) -> Vec3 {
    let w = &s.sim.world;
    let cx = (x / w.cell).floor();
    [(cx + 0.5) * w.cell, (s.ground as f64 - 0.5) * w.cell, s.caster_pos()[2]]
}

pub fn stone_wall(dims: u8) -> Scene {
    let mut s = field(dims);
    // Raised by hand, 400 kg of earth: a master's spell.
    s.sim.casters[s.caster].stats = master();
    let aim = on_ground(&s, 4.0);
    *s.will() = Will { aim, amount: 500.0, force: 0.0, maintain: false };
    if dims == 3 {
        // A wall 4 m long is 256 cells of earth to lift, not 16: it takes a master to hold that much.
        s.will().amount = 8000.0;
        let st = &mut s.sim.casters[s.caster].stats;
        st.body.capacity.training = 12000.0;
        st.mind.speed.training = 1200.0;
    }
    s
}

/// A pillar of earth 9 m away to throw at.
pub fn fireball(dims: u8) -> Scene {
    let mut s = field(dims);
    let cz = s.caster_pos()[2];
    let w = &mut s.sim.world;
    let z = (cz / w.cell).floor() as i64;
    let (z0, z1) = if dims == 2 { (0, 0) } else { (z - 3, z + 3) };
    w.fill_box([44, s.ground as i64, z0], [47, 20, z1], EARTH, ground_amount(EARTH));
    let aim = [44.0 * w.cell, 3.1, cz];
    *s.will() = Will { aim, amount: 120.0, force: 0.5, maintain: false };
    s
}

/// Someone 2.5 m away to blow back.
pub fn gust(dims: u8) -> Scene {
    let mut s = field(dims);
    let c = s.caster_pos();
    let target = s.sim.world.add_person("Target", [4.5, c[1], c[2]]);
    let aim = s.sim.world.bodies[target].pos;
    *s.will() = Will { aim, amount: 40.0, force: 0.6, maintain: true };
    s.target = Some(target);
    s
}

pub fn water_shield(dims: u8) -> Scene {
    let mut s = field(dims);
    let c = s.caster_pos();
    *s.will() = Will { aim: [4.0, 3.0, c[2]], amount: 200.0, force: 0.0, maintain: false };
    s
}

/// A ball to hold in front of the caster for as long as they maintain it (the bench).
pub fn hold(dims: u8) -> Scene {
    let mut s = field(dims);
    let c = s.caster_pos();
    *s.will() = Will { aim: [6.0, 3.0, c[2]], amount: 120.0, force: 0.0, maintain: true };
    s
}

/// The scenes by name, in the order the tester lists them.
pub const SCENES: [(&str, fn(u8) -> Scene); 5] =
    [("StoneWall", stone_wall), ("Fireball", fireball), ("Gust", gust), ("WaterShield", water_shield), ("Hold", hold)];

pub fn scene(name: &str, dims: u8) -> Option<Scene> {
    if name == "Field" {
        return Some(field(dims));
    }
    SCENES.iter().find(|(n, _)| *n == name).map(|(_, f)| f(dims))
}
