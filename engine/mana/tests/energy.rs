//! The Energy ledger (D20): where the world's energy comes from and where it goes (from test/energy.test.ts). The four
//! spells' ledgers need a caster: they're in spells.rs.

use mana::vm::energy::{heat_of, stored, sum};
use mana::vm::fluid::{FluidHooks, step_fluid};
use mana::vm::physics::physics;
use mana::vm::world::{EARTH, World, mass_of};

/// Open ground 2 m deep, with no air.
fn ground() -> World {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    for a in w.air.iter_mut() {
        *a = [0.0; 4];
    }
    w
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

#[test]
fn turns_a_fall_into_motion_and_the_landing_into_heat_to_the_last_joule() {
    let ph = physics();
    let mut w = ground();
    let id = w.pour_still(&mut [0.0, 0.0, 0.0, ph.mote], [2.0, 3.0, 0.125], 1)[0];
    w.particle_mut(id).unwrap().carried[EARTH] = 4.0;
    let before = sum(&stored(&mut w, 0.0));
    let m = mass_of(w.particle(id).unwrap());
    for _ in 0..40 {
        step_fluid(&mut w, &FluidHooks::default());
        // While it falls, what it loses in height it gains in speed.
        let now = sum(&stored(&mut w, 0.0)) + heat_of(&w);
        assert!((now - before).abs() < 0.01 * m * ph.gravity); // a centimetre of fall, stepping
    }
    let p = w.particle(id).unwrap();
    assert_eq!(p.vel[1], 0.0); // landed
    // It fell about a metre: its weight times that went into the ground as heat.
    assert!(close(heat_of(&w), m * ph.gravity * (3.0 - p.pos[1]), 3));
    let h = |k: &str| w.heat.get(k).copied().unwrap_or(0.0);
    assert!(close(h("striking") + h("the ground"), heat_of(&w), 9));
}

#[test]
fn loses_to_the_air_what_the_air_slows() {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    let id = w.pour_still(&mut [0.0, 0.0, physics().mote, 0.0], [3.0, 4.0, 0.125], 1)[0];
    w.particle_mut(id).unwrap().vel = [0.2, 0.0, 0.0];
    let before = sum(&stored(&mut w, 0.0));
    for _ in 0..10 {
        step_fluid(&mut w, &FluidHooks::default());
    }
    assert!(w.particle(id).unwrap().vel[0] < 0.2);
    assert!(w.heat["the air dragging"] > 0.0);
    // What it lost went into the air's motion and into heat.
    assert!(close(sum(&stored(&mut w, 0.0)) + heat_of(&w), before, 6));
}
