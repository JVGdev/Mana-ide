//! The physics of mana and matter (SPEC §11): weight, the ground, water, rock, and what pushing costs (from
//! test/physics.test.ts). What pushing costs, and the second flaw spreading, need a caster: they're in machine.rs.

use mana::vm::air::step_air;
use mana::vm::fluid::{Change, FluidHooks, merge_and_split, step_fluid};
use mana::vm::matter::{Grip, det, step_matter};
use mana::vm::parts::{Parts, total};
use mana::vm::physics::{physics, tuned};
use mana::vm::world::{EARTH, Ingrained, Vec3, WATER, World, ground_amount, mass_of, packed};

fn g() -> f64 {
    physics().gravity
}

fn parts(k: usize, m: f64) -> Parts {
    let mut p = [0.0; 4];
    p[k] = m;
    p
}

/// Open ground 2 m deep, no air: nothing drags or holds up.
fn ground() -> World {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    w.empty_air();
    w
}

/// `n` motes of free mana of part `k`, at `at`, in weave 1.
fn drop(w: &mut World, at: Vec3, k: usize, n: f64) -> Vec<u64> {
    w.pour_still(&mut parts(k, physics().mote * n), at, 1)
}

fn one(w: &mut World, at: Vec3, k: usize) -> u64 {
    drop(w, at, k, 1.0)[0]
}

fn p(w: &World, id: u64) -> &mana::vm::world::Particle {
    w.particle(id).unwrap()
}

/// The world moves, `ticks` times: its matter, then its mana.
fn run(w: &mut World, ticks: usize) {
    for _ in 0..ticks {
        step_matter(w, &[]);
        step_fluid(w, &FluidHooks::default());
        w.tick += 1;
        assert!(w.momentum_error() < 1e-6, "tick {}: momentum off by {:e}", w.tick, w.momentum_error());
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
        let water = one(&mut w, [1.0, 5.0, 0.125], WATER);
        let earth = one(&mut w, [3.0, 5.0, 0.125], EARTH);
        let fire = one(&mut w, [5.0, 5.0, 0.125], 0);
        let air = one(&mut w, [7.0, 5.0, 0.125], 2);
        run(&mut w, 10);
        for id in [water, earth, fire, air] {
            assert!(close(p(&w, id).vel[1], -10.0 * g(), 6));
        }
        assert!(close(g() * 900.0, 9.81, 6)); // ticks are 1/30 s
    }

    #[test]
    fn holds_mana_up_in_the_air_by_what_the_air_it_pushes_aside_weighs_the_lighter_rises_the_heavier_sinks_matter_falls() {
        let mut w = World::with_ground(40, 24, 1, 8, EARTH);
        let fire = one(&mut w, [1.0, 4.0, 0.125], 0);
        let air = one(&mut w, [3.0, 4.0, 0.125], 2);
        let water = one(&mut w, [5.0, 4.0, 0.125], WATER);
        let earth = one(&mut w, [7.0, 4.0, 0.125], EARTH);
        w.make_matter([0.0, 0.0, 0.0, 1000.0], [9.0, 4.0, 0.125], [0.0; 3]);
        run(&mut w, 10);
        let vy = |id| p(&w, id).vel[1];
        let rock = w.points.last().unwrap().vel[1];
        // Fire, then air, then water, then earth: the air, raw mana, weighs between air's and water's.
        assert!(vy(fire) > vy(air));
        assert!(vy(air) > 0.0);
        assert!(vy(water) < 0.0);
        assert!(vy(earth) < vy(water));
        assert!(vy(earth) > -0.5 * 10.0 * g()); // held up, but not enough
        assert!(rock < -0.9 * 10.0 * g()); // matter the air doesn't hold up: it falls
    }

    #[test]
    fn stops_what_falls_on_the_ground_which_holds_it_up_from_then_on() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 3.0, 0.125], EARTH);
        run(&mut w, 60);
        assert!(p(&w, id).pos[1] > 2.0 - 1e-9);
        assert!(p(&w, id).pos[1] < 2.05);
        assert!(p(&w, id).vel[1].abs() < 1e-9);
    }

    #[test]
    fn makes_matter_weigh_what_the_mana_it_was_made_of_did_and_move_as_it_did() {
        let mut w = ground();
        w.make_matter([0.0, 0.0, 0.0, 1000.0], [2.0, 3.0, 0.125], [0.01, 0.0, 0.0]);
        let m = w.points.last().unwrap();
        assert!(close(m.mass, 1000.0 * physics().mana_mass[EARTH], 12));
        assert_eq!(m.vel, [0.01, 0.0, 0.0]);
    }

    #[test]
    fn keeps_sliding_things_from_sliding_by_friction() {
        let mut w = ground();
        let id = one(&mut w, [2.0, 2.01, 0.125], EARTH);
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
    fn falls_lands_and_spreads_into_a_puddle_as_big_as_the_water_was() {
        // A square metre of water (in 2D), a metre up.
        let mut w = ground();
        let from = w.points.len();
        w.fill_box([6, 12, 0], [9, 15, 0], WATER, ground_amount(WATER));
        for p in w.points[from..].iter_mut() {
            p.asleep = false;
        }
        run(&mut w, 150);
        let pts = &w.points[from..];
        let xs: Vec<f64> = pts.iter().map(|p| p.pos[0]).collect();
        let wide = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let ys = |f: fn(f64, f64) -> f64, z| pts.iter().map(|p| p.pos[1]).fold(z, f);
        assert!(ys(f64::min, f64::INFINITY) > 1.95 && ys(f64::max, 0.0) < 2.75); // on the ground (it fell from 3 to 4 m), and low
        assert!(pts.iter().all(|p| p.vel[1].abs() < 0.01)); // and settled
        assert!(wide > 2.5, "{wide}"); // spread out
        // As much water as there was: it hasn't been squeezed or stretched.
        let room: f64 = pts.iter().map(|p| p.volume * det(&p.f)).sum();
        let was: f64 = pts.iter().map(|p| p.volume).sum();
        assert!((room / was - 1.0).abs() < 0.01, "{}", room / was);
    }

    /// In no weight, a stream of water 2 m long, its end pulled gently along tick after tick; how far its far end came.
    fn pull_a_stream() -> f64 {
        tuned(
            |p| p.gravity = 0.0,
            || {
                let mut w = World::with_ground(40, 24, 1, 0, EARTH);
                w.empty_air();
                let from = w.points.len();
                w.fill_box([8, 12, 0], [15, 12, 0], WATER, ground_amount(WATER));
                for p in w.points[from..].iter_mut() {
                    p.asleep = false;
                }
                let far = |w: &World| w.points[from..].iter().map(|p| p.pos[0]).fold(f64::INFINITY, f64::min);
                let start = far(&w);
                for _ in 0..40 {
                    let mut pulled = 0.0;
                    for p in w.points[from..].iter_mut() {
                        if p.pos[0] > 3.75 {
                            p.vel[0] += 0.002;
                            pulled += p.mass * 0.002;
                        }
                    }
                    w.impulse.outside[0] += pulled;
                    run(&mut w, 1);
                }
                far(&w) - start
            },
        )
    }

    #[test]
    fn comes_apart_pulled_at_one_end_with_nothing_holding_it() {
        // Like real water at this size: what holds it together is the mana in it (below), not its surface tension.
        assert!(pull_a_stream() < 0.05);
    }

    #[test]
    fn held_by_its_mana_comes_along_whole_and_the_mana_with_it_as_the_mana_pushes_it() {
        tuned(
            |p| p.gravity = 0.0,
            || {
                // The same stream, with 1 M of water mana in each of its cells, pushed along tick after tick as a caster
                // pushes a weave: each particle, and the water it grips (sim.rs, push).
                let mut w = World::with_ground(40, 24, 1, 0, EARTH);
                w.empty_air();
                let from = w.points.len();
                w.fill_box([8, 12, 0], [15, 12, 0], WATER, ground_amount(WATER));
                for p in w.points[from..].iter_mut() {
                    p.asleep = false;
                }
                for i in 8..16 {
                    w.pour_still(&mut parts(WATER, 1.0), [(i as f64 + 0.5) * w.cell, 3.125, 0.125], 1);
                }
                let far = |w: &World| w.points[from..].iter().map(|p| p.pos[0]).fold(f64::INFINITY, f64::min);
                let start = far(&w);
                for _ in 0..40 {
                    let mut pushed = 0.0;
                    for p in w.particles.iter_mut() {
                        p.vel[0] += 0.002;
                        pushed += mass_of(p) * 0.002;
                    }
                    for p in w.points[from..].iter_mut() {
                        p.vel[0] += 0.002;
                        pushed += p.mass * 0.002;
                    }
                    w.impulse.outside[0] += pushed;
                    // What the mana grips: the matter in its cells.
                    let mut grips: Vec<Grip> = Vec::new();
                    for (i, p) in w.particles.iter().enumerate() {
                        let cell = w.cell_of(&p.pos) as usize;
                        match grips.iter_mut().find(|g| g.cell == cell) {
                            Some(g) => g.particles.push(i),
                            None => grips.push(Grip { cell, weave: 1, particles: vec![i], hand: None }),
                        }
                    }
                    step_matter(&mut w, &grips);
                    // Its own water, which doesn't block it: what's in the cells it grips (sim.rs, measure_grips).
                    let own: Vec<(usize, f64)> =
                        grips.iter().map(|g| (g.cell, w.matter[g.cell][WATER] / packed(WATER))).collect();
                    let hooks = FluidHooks {
                        grips: &grips,
                        own_matter: Some(Box::new(move |_, c| own.iter().find(|o| o.0 == c).map_or(0.0, |o| o.1))),
                        ..Default::default()
                    };
                    step_fluid(&mut w, &hooks);
                    w.tick += 1;
                    assert!(w.momentum_error() < 1e-6);
                }
                // The far end came along, and the stream is whole: no gap in it wider than a cell.
                assert!(far(&w) - start > 0.1, "{}", far(&w) - start);
                let mut xs: Vec<f64> = w.points[from..].iter().map(|p| p.pos[0]).collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let gap = xs.windows(2).map(|x| x[1] - x[0]).fold(0.0, f64::max);
                assert!(gap < w.cell, "{gap}");
                // And its mana is still in it.
                for q in &w.particles {
                    let c = w.cell_of(&q.pos) as usize;
                    assert!(w.matter[c][WATER] > 0.0, "{:?}", q.pos);
                }
            },
        );
    }

    #[test]
    fn a_line_of_air_mana_pulled_at_one_end_comes_apart() {
        let mut w = World::with_ground(40, 24, 1, 0, EARTH);
        w.empty_air();
        tuned(
            |p| p.gravity = 0.0,
            || {
                let ids: Vec<u64> = (0..8).map(|i| one(&mut w, [2.0 + i as f64 * 0.25, 3.0, 0.125], 2)).collect();
                let start = p(&w, ids[0]).pos[0];
                let end = *ids.last().unwrap();
                let m = mass_of(p(&w, end));
                w.particle_mut(end).unwrap().vel[0] = 0.08;
                w.impulse.outside[0] += m * 0.08;
                run(&mut w, 40);
                assert!(p(&w, ids[0]).pos[0] - start < 0.05); // its far end didn't come along (air mana spreads by itself)
            },
        );
    }
}

mod matter {
    use super::*;

    /// Lays a box of earth `amount` times as packed as soil, awake, and lets it come to rest.
    fn earth(w: &mut World, from: [i64; 3], to: [i64; 3], amount: f64) -> std::ops::Range<usize> {
        let first = w.points.len();
        w.fill_box(from, to, EARTH, ground_amount(EARTH) * amount);
        for p in w.points[first..].iter_mut() {
            p.asleep = false;
        }
        first..w.points.len()
    }

    fn top(w: &World, r: &std::ops::Range<usize>) -> f64 {
        w.points[r.clone()].iter().map(|p| p.pos[1]).fold(f64::NEG_INFINITY, f64::max)
    }

    #[test]
    fn the_ground_at_rest_stays_at_rest() {
        let mut w = ground();
        let before: Vec<_> = w.points.iter().map(|p| p.pos).collect();
        run(&mut w, 30);
        assert!(w.points.iter().all(|p| p.asleep));
        assert_eq!(w.points.iter().map(|p| p.pos).collect::<Vec<_>>(), before);
    }

    #[test]
    fn loose_earth_slumps_and_piles_at_its_angle_of_repose() {
        let mut w = ground();
        let r = earth(&mut w, [18, 8, 0], [21, 15, 0], 0.75); // a column 1 m wide and 2 m tall, loose
        run(&mut w, 150);
        let pts = &w.points[r.clone()];
        let high = top(&w, &r) - 2.0;
        let xs: Vec<f64> = pts.iter().map(|p| p.pos[0]).collect();
        let half =
            (xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - xs.iter().cloned().fold(f64::INFINITY, f64::min)) / 2.0;
        let slope = (high / half).atan().to_degrees();
        assert!(high < 1.2, "{high}"); // it fell
        assert!(slope > 15.0 && slope < 40.0, "{slope}"); // about its angle of friction (35°)
    }

    #[test]
    fn rock_stands_where_loose_earth_slumps() {
        // Columns 0.5 m wide and 3.5 m tall: earth packed into rock (2,600 kg/m³), and loose earth.
        let mut w = World::with_ground(40, 24, 1, 8, EARTH);
        let rock = earth(&mut w, [8, 8, 0], [9, 21, 0], 2600.0 / 1600.0);
        let loose = earth(&mut w, [28, 8, 0], [29, 21, 0], 0.75);
        let tall = |w: &World, r: &std::ops::Range<usize>| {
            top(w, r) - w.points[r.clone()].iter().map(|p| p.pos[1]).fold(f64::INFINITY, f64::min)
        };
        let (rock_tall, loose_top) = (tall(&w, &rock), top(&w, &loose));
        run(&mut w, 120);
        // Rock keeps its height (it presses the soil under it down a little); loose earth comes down.
        assert!(tall(&w, &rock) > rock_tall - 0.05, "{} {}", tall(&w, &rock), rock_tall);
        assert!(top(&w, &loose) < loose_top - 1.0, "{} {}", top(&w, &loose), loose_top);
    }

    #[test]
    fn a_ledge_of_soil_cracks_off_where_one_of_rock_holds() {
        // A ledge 1.5 m long and 0.5 m thick sticking out of a block of rock 2 m wide, 2 m up: the ledge bends at its
        // root, and the earth there is pulled apart. Rock holds; soil cracks.
        let ledge = |packing: f64| {
            let mut w = World::with_ground(40, 24, 1, 8, EARTH);
            earth(&mut w, [6, 8, 0], [13, 17, 0], 2600.0 / 1600.0);
            let r = earth(&mut w, [14, 16, 0], [19, 17, 0], packing);
            run(&mut w, 90);
            w.points[r].iter().map(|p| p.pos[1]).fold(f64::INFINITY, f64::min)
        };
        let rock = ledge(2600.0 / 1600.0);
        let soil = ledge(1.0);
        assert!(rock > 3.9, "{rock}"); // still where it was (its underside was at 4 m)
        assert!(soil < 3.0, "{soil}"); // fallen
    }

    #[test]
    fn molded_earth_keeps_its_new_shape() {
        // A block of soil shoved hard along the ground at its top: it shears past what it holds, and stays sheared.
        let mut w = ground();
        let r = earth(&mut w, [16, 8, 0], [19, 11, 0], 1.0);
        for p in w.points[r.clone()].iter_mut() {
            if p.pos[1] > 2.5 {
                p.vel[0] = 0.15;
            }
        }
        let m: f64 = w.points[r.clone()].iter().filter(|p| p.pos[1] > 2.5).map(|p| p.mass * 0.15).sum();
        w.impulse.outside[0] += m;
        let x0: f64 = w.points[r.clone()].iter().filter(|p| p.pos[1] > 2.75).map(|p| p.pos[0]).sum::<f64>();
        run(&mut w, 90);
        let moved = (w.points[r.clone()].iter().filter(|p| p.pos[1] > 2.6).map(|p| p.pos[0]).sum::<f64>() - x0)
            / w.points[r.clone()].iter().filter(|p| p.pos[1] > 2.75).count().max(1) as f64;
        assert!(moved > 0.1, "{moved}"); // its top has moved over, and stays there
        assert!(w.points[r].iter().all(|p| p.vel[0].abs() < 0.01)); // at rest
    }

    #[test]
    fn stops_free_mana_held_by_nothing_at_its_face() {
        // A fireball's worth of free mana thrown at a pillar of earth stops at it, and goes no further.
        let mut w = ground();
        let r = earth(&mut w, [20, 8, 0], [21, 15, 0], 2600.0 / 1600.0);
        let ids = w.pour(&mut [4.0, 0.0, 0.0, 0.0], [4.0, 3.0, 0.125], 0, [0.3, 0.0, 0.0], physics().pour);
        w.impulse.outside[0] += 4.0 * physics().mana_mass[0] * 0.3;
        run(&mut w, 40);
        // The pillar's face: its points sit a quarter of a cell in from it.
        let face = w.points[r].iter().map(|p| p.pos[0]).fold(f64::INFINITY, f64::min) - w.cell / 4.0;
        for id in ids {
            if let Some(q) = w.particle(id) {
                assert!(q.pos[0] < face + 0.01, "{} {face}", q.pos[0]);
            }
        }
    }
}

mod merging_and_splitting {
    use super::*;

    fn order(addr: usize) -> Ingrained {
        Ingrained { program: mana::asm::Code::new(mana::asm::Program::empty()), addr }
    }

    #[test]
    fn merges_particles_at_rest_beside_each_other_keeping_their_mana_and_momentum() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], WATER);
        let b = one(&mut w, [2.05, 3.0, 0.125], WATER);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.particle_mut(a).unwrap().vel = [0.001, 0.0, 0.0];
        w.particle_mut(b).unwrap().vel = [0.003, 0.0, 0.0];
        w.impulse.outside[0] += mass_of(p(&w, a)) * 0.001 + mass_of(p(&w, b)) * 0.003;
        let changes = merge_and_split(&mut w, &FluidHooks::default());
        assert_eq!(changes, [Change::Merge { into: a, from: b, weave: 1 }]);
        assert_eq!(w.particles.iter().map(|p| p.id).collect::<Vec<_>>(), [a]);
        assert!(close(p(&w, a).free[WATER], 2.0 * physics().mote, 12));
        assert!(w.momentum_error() < 1e-12);
        assert!(p(&w, a).pos[0] > 2.0);
        assert!(p(&w, a).pos[0] < 2.05);
    }

    #[test]
    fn doesnt_merge_particles_moving_apart_or_ones_that_would_be_too_big() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], WATER);
        let b = one(&mut w, [2.05, 3.0, 0.125], WATER);
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
        let id = one(&mut w, [2.0, 4.0, 0.125], WATER);
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
    fn leaves_what_is_in_a_hand_whole() {
        let mut w = ground();
        let a = one(&mut w, [2.0, 3.0, 0.125], EARTH);
        let b = one(&mut w, [2.05, 3.0, 0.125], EARTH);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
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
        let a = one(&mut w, [2.0, 3.0, 0.125], EARTH);
        let b = one(&mut w, [2.05, 3.0, 0.125], EARTH);
        w.particle_mut(a).unwrap().pos = [2.0, 3.0, 0.125];
        w.particle_mut(b).unwrap().pos = [2.05, 3.0, 0.125];
        w.particle_mut(a).unwrap().free[EARTH] = 0.5;
        w.particle_mut(b).unwrap().weave = 2;
        w.particle_mut(a).unwrap().order =
            Some(Ingrained { program: mana::asm::Code::new(mana::asm::Program::empty()), addr: 3 });
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

    /// The air steps, keeping its mana (counting what went past the world's edge).
    fn air_run(w: &mut World, ticks: usize, mut each: impl FnMut(&World)) {
        let mana = |w: &World| w.air.iter().map(total).sum::<f64>() + total(&w.beyond);
        let before = mana(w);
        for _ in 0..ticks {
            step_air(w, &|_| None);
            let (now, mana) = (mana(w), before);
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

mod real_air {
    use super::*;
    use mana::vm::air::{room, sound_speed};
    use mana::vm::world::FIRE;

    /// The air and the mana in it step, `ticks` times, with mana and momentum kept (counting what went past the edge).
    fn air_and_mana(w: &mut World, ticks: usize) {
        let mana = |w: &World| mana::vm::world::World::mana(w);
        let before = mana(w);
        for _ in 0..ticks {
            step_matter(w, &[]);
            step_fluid(w, &FluidHooks::default());
            step_air(w, &|_| None);
            w.tick += 1;
            let now = mana(w);
            assert!(close(
                now.air + now.matter + now.loose + now.beyond,
                before.air + before.matter + before.loose + before.beyond,
                6
            ));
            assert!(w.momentum_error() < 1e-6, "tick {}: momentum off by {:e}", w.tick, w.momentum_error());
        }
    }

    #[test]
    fn a_balloon_of_light_gas_rises_and_rock_does_not_care() {
        let mut w = World::with_ground(48, 32, 1, 8, EARTH);
        // A parcel of the air made of flame instead of air matter: as many M, so it presses the same, but lighter.
        for y in 10..14 {
            for x in 22..26 {
                let i = w.index(x, y, 0) as usize;
                let m = total(&w.gas[i]);
                w.gas[i] = parts(FIRE, m);
            }
        }
        let height = |w: &World| {
            let (mut m, mut y) = (0.0, 0.0);
            for i in 0..w.size {
                m += w.gas[i][FIRE];
                y += w.gas[i][FIRE] * w.coords(i)[1] as f64;
            }
            y / m * w.cell
        };
        let start = height(&w);
        // And a block of rock, up in the air.
        let from = w.points.len();
        w.fill_box([8, 20, 0], [9, 21, 0], EARTH, 1.6 * ground_amount(EARTH));
        for p in w.points[from..].iter_mut() {
            p.asleep = false;
        }
        air_and_mana(&mut w, 10);
        let fell: f64 = w.points[from..].iter().map(|p| p.vel[1]).sum::<f64>() / (w.points.len() - from) as f64;
        // Rock falls as if there were no air: what the air holds up of it is a thousandth of its weight.
        assert!(fell < -0.99 * 10.0 * g() && fell > -1.0001 * 10.0 * g(), "{fell}");
        air_and_mana(&mut w, 20);
        // The flame rises, slowed by the air's thickness (`air_viscosity`).
        assert!(height(&w) > start + 0.08, "{} {}", start, height(&w));
    }

    #[test]
    fn a_gust_of_mana_drives_a_wind_that_blows_on_after_its_mana_has_stopped() {
        let mut w = World::with_ground(64, 24, 1, 8, EARTH);
        // A breath of 100 M of air mana, thrown along: its momentum given from outside.
        let ids = drop(&mut w, [2.0, 3.0, 0.125], 2, 400.0);
        for &id in &ids {
            let p = w.particle_mut(id).unwrap();
            p.vel = [0.4, 0.0, 0.0];
            let m = mass_of(p);
            w.impulse.outside[0] += m * 0.4;
        }
        air_and_mana(&mut w, 10);
        // Then the mana stops where it is: it settles into the air there, as loose mana does, its motion with it.
        for pi in 0..w.particles.len() {
            let p = &w.particles[pi];
            let c = w.clamped_cell_of(&p.pos) as isize;
            let m = mass_of(p);
            let (parts, momentum) = (p.free, [p.vel[0] * m, p.vel[1] * m, p.vel[2] * m]);
            w.add_air(c, &parts, momentum);
        }
        w.retain_particles(|_| false);
        // Where the wind is, and how much of it.
        let wind = |w: &World| {
            let (mut p, mut px) = (0.0, 0.0);
            for i in 0..w.size {
                let q = w.air_mass(i) * w.air_vel[i * 3];
                if q > 0.0 {
                    p += q;
                    px += q * (w.coords(i)[0] as f64 + 0.5) * w.cell;
                }
            }
            (px / p, p)
        };
        air_and_mana(&mut w, 5);
        let (at, p) = wind(&w);
        air_and_mana(&mut w, 10);
        let (later, still) = wind(&w);
        // The air it set moving goes on along the way the mana went, with no mana left to drive it.
        assert!(later > at + 0.2, "{at} {later}");
        assert!(still > 0.5 * p, "{p} {still}");
    }

    #[test]
    fn a_pressure_wave_crosses_the_world_at_the_speed_of_sound() {
        // A channel 1.75 m high, roofed with rock, open at its ends, its air at rest; a puff of air let in near one end.
        let mut w = World::with_ground(256, 8, 1, 0, EARTH);
        w.fill_box([0, 7, 0], [255, 7, 0], EARTH, ground_amount(EARTH));
        let c = sound_speed(room(&w).mean) * 30.0; // m/s
        assert!(c > 270.0 && c < 290.0, "{c}"); // isothermal: √(R·T/M) for the world's air and the mana in it
        let still: Vec<f64> = (0..256).map(|x| w.air_amount(w.index(x, 3, 0) as usize)).collect();
        for y in 0..7 {
            let i = w.index(4, y, 0) as usize;
            w.gas[i] = w.gas[i].map(|m| m * 1.05);
        }
        let crest = |w: &World| {
            let d = |x: usize| w.air_amount(w.index(x as i64, 3, 0) as usize) / still[x];
            ((0..256usize).max_by(|&a, &b| d(a).partial_cmp(&d(b)).unwrap()).unwrap() as f64 + 0.5) * w.cell
        };
        for _ in 0..2 {
            step_air(&mut w, &|_| None);
        }
        let x2 = crest(&w);
        for _ in 0..4 {
            step_air(&mut w, &|_| None);
        }
        let speed = (crest(&w) - x2) / 4.0 * 30.0;
        assert!((speed - c).abs() < 0.03 * c, "{speed} against {c}");
    }

    #[test]
    fn a_gale_blows_a_body_back_and_a_wind_its_feet_can_hold_against_moves_it_not_at_all() {
        let blown = |u: f64| {
            let mut w = World::with_ground(64, 24, 1, 8, EARTH);
            let b = w.add_person("Target", [8.0, 2.9, 0.125]);
            let x = w.bodies[b].pos[0];
            for _ in 0..15 {
                // A wind over the whole world, kept blowing (its momentum given from outside), but around the body,
                // where the air flows as it will.
                let bx = w.bodies[b].pos[0];
                for i in 0..w.size {
                    let c = w.coords(i);
                    if w.solid_at(i as isize) || (((c[0] as f64 + 0.5) * w.cell - bx).abs() < 1.0 && c[1] < 20) {
                        continue;
                    }
                    let m = w.air_mass(i);
                    w.impulse.outside[0] += m * (u - w.air_vel[i * 3]);
                    w.air_vel[i * 3] = u;
                }
                step_air(&mut w, &|_| None);
                w.move_bodies();
                assert!(w.momentum_error() < 1e-6);
            }
            w.bodies[b].pos[0] - x
        };
        // 15 and 30 m/s push less than its feet hold (μ·m·g); 45 m/s pushes more, and slides it along.
        assert_eq!(blown(0.5), 0.0);
        assert_eq!(blown(1.0), 0.0);
        assert!(blown(1.5) > 0.2);
    }
}
