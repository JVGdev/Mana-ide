//! The Energy ledger (D20): where the world's energy comes from and where it goes (from test/energy.test.ts). The four
//! spells' ledgers need a caster: they're in spells.rs.

use mana::vm::energy::{heat_of, stored, sum};
use mana::vm::fluid::{FluidHooks, step_fluid};
use mana::vm::matter::step_matter;
use mana::vm::physics::physics;
use mana::vm::world::{EARTH, World, ground_amount};

/// Open ground 2 m deep, with no air.
fn ground() -> World {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    w.empty_air();
    w
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

#[test]
fn turns_a_fall_into_motion_and_the_landing_into_heat() {
    let ph = physics();
    let mut w = ground();
    // A block of earth half a metre on a side, a metre up.
    let from = w.points.len();
    w.fill_box([10, 12, 0], [11, 13, 0], EARTH, ground_amount(EARTH));
    for p in w.points[from..].iter_mut() {
        p.asleep = false;
    }
    let m: f64 = w.points[from..].iter().map(|p| p.mass).sum();
    let y0: f64 = w.points[from..].iter().map(|p| p.pos[1]).sum::<f64>() / (w.points.len() - from) as f64;
    let before = sum(&stored(&mut w, 0.0));
    for _ in 0..60 {
        step_matter(&mut w, &[]);
        step_fluid(&mut w, &FluidHooks::default());
        w.tick += 1;
        // While it falls, what it loses in height it gains in speed, or gives off as heat as it strikes and gives way:
        // to within a centimetre of fall.
        let now = sum(&stored(&mut w, 0.0)) + heat_of(&w);
        assert!((now - before).abs() < 0.01 * m * ph.gravity, "{}", now - before);
    }
    let y1: f64 = w.points[from..].iter().map(|p| p.pos[1]).sum::<f64>() / (w.points.len() - from) as f64;
    assert!(w.points[from..].iter().all(|p| p.vel[1].abs() < 1e-3)); // landed
    // It fell about a metre: its weight times that went into heat (and a little into the ground's strain).
    assert!(heat_of(&w) > 0.95 * m * ph.gravity * (y0 - y1), "{} {}", heat_of(&w), m * ph.gravity * (y0 - y1));
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

mod spells {
    use super::*;
    use mana::asm::Code;
    use mana::load::spell;
    use mana::scenes::scene;

    fn balances(name: &str, ticks: usize, close_enough: f64) {
        let mut s = scene(name, 2).unwrap();
        s.sim.keep_energy();
        s.sim.cast(s.caster, Code::new(spell(name)), None).unwrap();
        s.sim.run(ticks);
        let e = s.sim.energy_now();
        assert!(close(e.total + e.heat + e.beyond, e.start + e.outside + e.error_total, 6)); // the ledger's own sums
        assert!(e.outside > 0.0); // the caster put energy in
        assert!(e.heat > 0.0);
        // What the numbers get wrong is small beside what moved through.
        assert!(
            e.error_total.abs() < close_enough * (e.heat + e.outside.abs()),
            "{} of {}",
            e.error_total,
            e.heat + e.outside.abs()
        );
    }

    #[test]
    fn balances_through_a_fireball() {
        balances("Fireball", 70, 0.05);
    }

    #[test]
    fn balances_through_a_gust() {
        balances("Gust", 30, 0.05);
    }

    #[test]
    fn balances_through_a_water_shield() {
        balances("WaterShield", 80, 0.05);
    }

    #[test]
    fn balances_through_a_stone_wall() {
        balances("StoneWall", 160, 0.05);
    }
}
