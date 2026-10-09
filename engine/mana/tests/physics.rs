//! The physics of mana and matter (SPEC §11): weight, the ground, water, rock, and what pushing costs (from
//! test/physics.test.ts). What pushing costs, and the second flaw spreading, need a caster: they're in machine.rs.

use mana::js;
use mana::vm::air::step_air;
use mana::vm::fluid::{Change, FluidHooks, merge_and_split, step_fluid};
use mana::vm::parts::{Parts, total};
use mana::vm::physics::{physics, tuned};
use mana::vm::world::{Bond, EARTH, Ingrained, Vec3, WATER, World, mass_of};

fn g() -> f64 {
    physics().gravity
}

fn parts(k: usize, m: f64) -> Parts {
    let mut p = [0.0; 4];
    p[k] = m;
    p
}

/// Open ground 2 m deep, no air mana: nothing drags.
fn ground() -> World {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    for a in w.air.iter_mut() {
        *a = [0.0; 4];
    }
    w
}

/// Particles of free mana of part `k`, each holding `held` of matter of part `k`, at `at`.
fn drop(w: &mut World, at: Vec3, k: usize, n: f64, held: f64) -> Vec<u64> {
    let ids = w.pour_still(&mut parts(k, physics().mote * n), at, 1);
    for &id in &ids {
        w.particle_mut(id).unwrap().carried[k] = held;
    }
    ids
}

fn one(w: &mut World, at: Vec3, k: usize, held: f64) -> u64 {
    drop(w, at, k, 1.0, held)[0]
}

fn p(w: &World, id: u64) -> &mana::vm::world::Particle {
    w.particle(id).unwrap()
}

fn run(w: &mut World, ticks: usize) {
    for _ in 0..ticks {
        step_fluid(w, &FluidHooks::default());
        w.tick += 1;
        assert!(w.momentum_error() < 1e-6);
    }
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

mod weight {
    use super::*;

    #[test]
    fn pulls_everything_down_at_9_8_m_s2_where_there_is_no_air_a_heavier_particle_falls_just_as_fast() {
        let mut w = ground();
        let water = one(&mut w, [1.0, 5.0, 0.125], WATER, 0.0);
        let earth = one(&mut w, [3.0, 5.0, 0.125], EARTH, 2.0);
        let fire = one(&mut w, [5.0, 5.0, 0.125], 0, 0.0);
        let air = one(&mut w, [7.0, 5.0, 0.125], 2, 0.0);
        run(&mut w, 10);
        for id in [water, earth, fire, air] {
            assert!(close(p(&w, id).vel[1], -10.0 * g(), 6));
        }
        assert!(close(g() * 900.0, 9.81, 6)); // ticks are 1/30 s
    }

    #[test]
    fn holds_mana_up_in_the_air_by_what_the_air_it_pushes_aside_weighs_the_lighter_rises_the_heavier_sinks_matter_falls() {
        let mut w = World::with_ground(40, 24, 1, 8, EARTH);
        let fire = one(&mut w, [1.0, 4.0, 0.125], 0, 0.0);
        let air = one(&mut w, [3.0, 4.0, 0.125], 2, 0.0);
        let water = one(&mut w, [5.0, 4.0, 0.125], WATER, 0.0);
        let earth = one(&mut w, [7.0, 4.0, 0.125], EARTH, 0.0);
        let rock = one(&mut w, [9.0, 4.0, 0.125], EARTH, 4.0);
        run(&mut w, 10);
        let vy = |id| p(&w, id).vel[1];
        // Fire, then air, then water, then earth: the air, raw mana, weighs between air's and water's.
        assert!(vy(fire) > vy(air));
        assert!(vy(air) > 0.0);
        assert!(vy(water) < 0.0);
        assert!(vy(earth) < vy(water));
        assert!(vy(earth) > -0.5 * 10.0 * g()); // held up, but not enough
        assert!(vy(rock) < -0.9 * 10.0 * g()); // matter the air doesn't hold up: it falls
    }

    #[test]
    fn stops_what_falls_on_the_ground_which_holds_it_up_from_then_on() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 3.0, 0.125], EARTH, 5.0);
        run(&mut w, 60);
        assert!(p(&w, id).pos[1] > 2.0 - 1e-9);
        assert!(p(&w, id).pos[1] < 2.05);
        assert!(p(&w, id).vel[1].abs() < 1e-9);
    }

    #[test]
    fn gives_matter_held_by_mana_its_weight_and_inertia() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 3.0, 0.125], EARTH, 4.0);
        // Matter weighs what its mana did.
        assert!(close(mass_of(p(&w, id)), (physics().mote + 4.0) * physics().mana_mass[EARTH], 12));
    }

    #[test]
    fn keeps_sliding_things_from_sliding_by_friction() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 2.01, 0.125], EARTH, 5.0);
        w.particle_mut(id).unwrap().vel[0] = 0.05;
        w.impulse.outside[0] += mass_of(p(&w, id)) * 0.05;
        run(&mut w, 30);
        assert_eq!(p(&w, id).vel[0], 0.0);
        assert!(p(&w, id).pos[0] < 2.5);
    }
}

mod water {
    use super::*;

    #[test]
    fn pulls_together_as_it_falls_lands_and_spreads_into_a_puddle_on_the_ground() {
        let mut w = ground();
        let ps = drop(&mut w, [2.0, 3.2, 0.125], WATER, 12.0, 4.0);
        run(&mut w, 120);
        for id in ps {
            let q = p(&w, id);
            assert!(q.pos[1] > 1.99);
            assert!(q.pos[1] < 2.3);
            assert!(q.vel[1].abs() < 0.01);
        }
    }

    #[test]
    fn holds_together_neighbours_close_by_pull_each_other_in() {
        let mut w = World::with_ground(40, 24, 1, 0, EARTH);
        for a in w.air.iter_mut() {
            *a = [0.0; 4];
        }
        tuned(
            |p| p.gravity = 0.0,
            || {
                let a = one(&mut w, [2.0, 3.0, 0.125], WATER, 4.0);
                let b = one(&mut w, [2.18, 3.0, 0.125], WATER, 4.0);
                let before = p(&w, b).pos[0] - p(&w, a).pos[0];
                run(&mut w, 5);
                assert!(p(&w, b).pos[0] - p(&w, a).pos[0] < before);
            },
        );
    }
}

mod rock {
    use super::*;

    /// A column of earth particles 1 m tall, bound like rock, standing on the ground.
    fn column(w: &mut World) -> Vec<u64> {
        let mut ps = Vec::new();
        for y in 0..16 {
            for x in 0..3 {
                ps.extend(drop(w, [2.0 + x as f64 * 0.06, 2.01 + y as f64 * 0.06, 0.125], EARTH, 1.0, 3.0));
            }
        }
        let first = ps[0];
        for &id in &ps {
            let k = (id - first) as f64;
            w.particle_mut(id).unwrap().pos = [2.0 + (k % 3.0) * 0.06, 2.01 + (k / 3.0).floor() * 0.06, 0.125];
        }
        for i in 0..ps.len() {
            for j in i + 1..ps.len() {
                let (a, b) = (p(w, ps[i]).pos, p(w, ps[j]).pos);
                let d = js::hypot2(a[0] - b[0], a[1] - b[1]);
                if d < physics().bond_range {
                    w.bonds.push(Bond { a: ps[i], b: ps[j], rest: d });
                }
            }
        }
        ps
    }

    fn top(w: &World, ps: &[u64]) -> f64 {
        ps.iter().map(|&id| p(w, id).pos[1]).fold(f64::NEG_INFINITY, f64::max)
    }

    #[test]
    fn keeps_its_shape_standing_on_the_ground_under_its_own_weight() {
        let mut w = ground();
        let ps = column(&mut w);
        let before = top(&w, &ps);
        let bonds = w.bonds.len();
        run(&mut w, 60);
        assert!(top(&w, &ps) > before - 0.03);
        assert_eq!(w.bonds.len(), bonds);
    }

    #[test]
    fn breaks_when_it_has_to_hold_more_than_it_can() {
        let mut w = ground();
        column(&mut w);
        let bonds = w.bonds.len();
        tuned(|p| p.bond_strength = 0.001, || run(&mut w, 30));
        assert!(w.bonds.len() < bonds);
    }
}

mod merging_and_splitting {
    use super::*;

    fn order(addr: usize) -> Ingrained {
        Ingrained { program: std::sync::Arc::new(mana::asm::Program::empty()), addr }
    }

    #[test]
    fn merges_particles_at_rest_beside_each_other_keeping_their_mana_matter_and_momentum() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], WATER, 2.0);
        let b = one(&mut w, [2.05, 3.0, 0.125], WATER, 1.0);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.particle_mut(a).unwrap().vel = [0.001, 0.0, 0.0];
        w.particle_mut(b).unwrap().vel = [0.003, 0.0, 0.0];
        w.impulse.outside[0] += mass_of(p(&w, a)) * 0.001 + mass_of(p(&w, b)) * 0.003;
        let changes = merge_and_split(&mut w, &FluidHooks::default());
        assert_eq!(changes, [Change::Merge { into: a, from: b, weave: 1 }]);
        assert_eq!(w.particles.iter().map(|p| p.id).collect::<Vec<_>>(), [a]);
        assert!(close(p(&w, a).free[WATER], 2.0 * physics().mote, 12));
        assert_eq!(p(&w, a).carried[WATER], 3.0);
        assert!(w.momentum_error() < 1e-12);
        assert!(p(&w, a).pos[0] > 2.0);
        assert!(p(&w, a).pos[0] < 2.05);
    }

    #[test]
    fn doesnt_merge_particles_moving_apart_or_ones_that_would_be_too_big() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], WATER, 0.0);
        let b = one(&mut w, [2.05, 3.0, 0.125], WATER, 0.0);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.particle_mut(b).unwrap().vel = [0.02, 0.0, 0.0];
        let merges = |w: &mut World| {
            merge_and_split(w, &FluidHooks::default()).into_iter().filter(|c| matches!(c, Change::Merge { .. })).count()
        };
        assert_eq!(merges(&mut w), 0);
        w.particle_mut(b).unwrap().vel = [0.0; 3];
        w.particle_mut(a).unwrap().free[WATER] = physics().max_mote;
        assert_eq!(merges(&mut w), 0);
    }

    #[test]
    fn splits_a_big_particle_that_has_spread_thin_and_both_halves_keep_its_weave_and_order() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 4.0, 0.125], WATER, 0.0);
        w.particle_mut(id).unwrap().free[WATER] = 1.0;
        let ord = order(7);
        w.particle_mut(id).unwrap().order = Some(ord.clone());
        let changes = merge_and_split(&mut w, &FluidHooks::default());
        assert!(matches!(changes.as_slice(), [Change::Split { .. }]));
        assert_eq!(w.particles.len(), 2);
        for q in &w.particles {
            assert!(close(q.free[WATER], 0.5, 12));
            assert_eq!(q.weave, 1);
            let o = q.order.as_ref().unwrap();
            assert!(std::sync::Arc::ptr_eq(&o.program, &ord.program) && o.addr == 7);
        }
        merge_and_split(&mut w, &FluidHooks::default());
        assert_eq!(w.particles.len(), 4); // alone, they split again, down to motes
    }

    #[test]
    fn leaves_rock_and_what_is_in_a_hand_whole() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], EARTH, 2.0);
        let b = one(&mut w, [2.05, 3.0, 0.125], EARTH, 2.0);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.bonds.push(Bond { a, b, rest: 0.05 });
        assert_eq!(merge_and_split(&mut w, &FluidHooks::default()), []);
        w.bonds.clear();
        let hand = w.add_person("Hand", [0.0; 3]);
        let hooks = FluidHooks { holder: Some(Box::new(move |_| Some(hand))), ..Default::default() };
        assert_eq!(merge_and_split(&mut w, &hooks), []);
    }
}

mod the_second_flaw {
    use super::*;

    #[test]
    fn lets_a_big_ordered_particle_take_over_someone_elses_mana_that_comes_to_rest_against_it() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], EARTH, 0.0);
        let b = one(&mut w, [2.05, 3.0, 0.125], EARTH, 0.0);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.particle_mut(a).unwrap().free[EARTH] = 0.5;
        w.particle_mut(b).unwrap().weave = 2;
        w.particle_mut(a).unwrap().order = Some(Ingrained { program: std::sync::Arc::new(mana::asm::Program::empty()), addr: 3 });
        let changes = merge_and_split(&mut w, &FluidHooks::default());
        assert_eq!(changes[0], Change::Merge { into: a, from: b, weave: 2 });
        assert_eq!(p(&w, a).weave, 1);
        assert!(close(p(&w, a).free[EARTH], 0.75, 12)); // weave 2's mana is weave 1's now, and runs its order
    }
}

mod the_air {
    use super::*;

    fn air_mana(w: &World, x0: i64, x1: i64, y0: i64, y1: i64) -> f64 {
        let mut m = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                m += total(&w.air[w.index(x, y, 0) as usize]);
            }
        }
        m
    }

    fn air_run(w: &mut World, ticks: usize, mut each: impl FnMut(&World)) {
        let mana: f64 = w.air.iter().map(total).sum();
        for _ in 0..ticks {
            step_air(w);
            let now: f64 = w.air.iter().map(total).sum();
            assert!(close(now, mana, 6));
            assert!(w.momentum_error() < 1e-6);
            each(w);
        }
    }

    /// Air in cells x0–x1, y0–y1 set moving at `u` m/tick along x, its momentum given from outside.
    fn blow(w: &mut World, x0: i64, x1: i64, y0: i64, y1: i64, u: f64) {
        for y in y0..y1 {
            for x in x0..x1 {
                let i = w.index(x, y, 0) as usize;
                w.air_vel[i * 3] = u;
                w.impulse.outside[0] += w.air_mass(i) * u;
            }
        }
    }

    #[test]
    fn carries_itself_along_a_puff_of_wind_travels_on_and_takes_its_mana_with_it() {
        let mut w = World::with_ground(48, 24, 1, 8, EARTH);
        blow(&mut w, 4, 8, 12, 16, 0.3);
        let centre = |w: &World| {
            let mut m = 0.0;
            let mut x = 0.0;
            for i in 0..w.size {
                let p = total(&w.air[i]) * w.air_vel[i * 3];
                m += p;
                x += p * w.coords(i)[0] as f64;
            }
            x / m
        };
        let start = centre(&w);
        air_run(&mut w, 15, |_| {});
        assert!(centre(&w) > start + 3.0); // the moving air is cells further on
    }

    #[test]
    fn flows_around_what_stands_in_its_way() {
        let mut w = World::with_ground(48, 24, 1, 8, EARTH);
        let full = mana::vm::world::ground_amount(EARTH);
        w.fill_box([24, 8, 0], [26, 13, 0], EARTH, full); // a pillar 1.5 m high
        blow(&mut w, 8, 20, 8, 20, 0.2);
        let mut up: f64 = 0.0;
        air_run(&mut w, 20, |w| {
            for y in 8..14 {
                up = up.max(w.air_vel[w.index(23, y, 0) as usize * 3 + 1]);
            }
        });
        assert!(up > 0.02); // in front of the pillar, the wind turns up and over it
        for y in 8..14 {
            assert_eq!(total(&w.air[w.index(25, y, 0) as usize]), 0.0); // and none goes through it
        }
    }

    #[test]
    fn fills_back_in_where_mana_was_taken_out_of_it() {
        let mut w = World::with_ground(48, 24, 1, 8, EARTH);
        for y in 12..16 {
            for x in 20..24 {
                let i = w.index(x, y, 0) as usize;
                w.air[i] = [0.0; 4];
            }
        }
        air_run(&mut w, 60, |_| {});
        assert!(air_mana(&w, 20, 24, 12, 16) > 16.0 * physics().air_mana * 0.5);
    }

    #[test]
    fn presses_from_dense_to_thin() {
        let mut w = World::with_ground(48, 24, 1, 8, EARTH);
        let i = w.index(20, 14, 0) as usize;
        w.air[i] = [physics().air_mana / 2.0; 4]; // twice as dense as around it
        let around = |w: &World| air_mana(w, 17, 24, 11, 18) - total(&w.air[i]);
        let before = around(&w);
        air_run(&mut w, 1, |_| {});
        assert!(total(&w.air[i]) < physics().air_mana * 2.0);
        assert!(around(&w) > before); // what it pushed out is around it
    }
}
