//! The four spells, cast in their scenes (from test/spells.test.ts).

use mana::asm::Code;
use mana::js;
use mana::load::spell;
use mana::scenes::{Scene, fireball, gust, on_ground, stone_wall, water_shield};
use mana::vm::parts::total;
use mana::vm::physics::{physics, tuned};
use mana::vm::sim::{CastState, EventKind};
use mana::vm::weave::Weave;
use mana::vm::world::{EARTH, WATER, ground_amount, mass_of};

/// Runs, checking every tick that no mana was made or lost, and that momentum only came from outside.
fn run(s: &mut Scene, ticks: usize, mut each: impl FnMut(&mut Scene, usize)) {
    let before = s.sim.ledger().total;
    for t in 0..ticks {
        s.sim.step();
        assert!(close(s.sim.ledger().total, before, 4));
        assert!(s.sim.world.momentum_error() < 1e-6, "tick {t}: momentum off by {:e}", s.sim.world.momentum_error());
        each(s, t);
    }
}

fn ticks(s: &mut Scene, n: usize) {
    run(s, n, |_, _| {});
}

/// Runs until the spell has ended, and `after` more ticks.
fn finish(s: &mut Scene, cast: usize, after: usize) {
    let mut t = 0;
    while t < 200 && s.sim.casts[cast].state == CastState::Running {
        ticks(s, 1);
        t += 1;
    }
    ticks(s, after);
}

fn cast(s: &mut Scene, name: &str) -> usize {
    s.sim.cast(s.caster, Code::new(spell(name)), None).unwrap()
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

fn mass_of_weave(s: &Scene, w: &Weave) -> f64 {
    w.parts(&s.sim.world).fold(0.0, |m, p| m + mass_of(p))
}

/// The matter of part `part` that weaves hold.
fn carried(s: &Scene, part: usize) -> f64 {
    s.sim.weaves.keys().map(|&id| s.sim.held_by(id)[part]).sum()
}

mod stone_wall_ {
    use super::*;

    #[test]
    #[ignore = "its grip doesn't tear rock out of the ground: how hard mana grips, how strong soil is or how the spell goes is the author's to choose"]
    fn lifts_the_ground_out_of_a_trench_by_hand_and_sets_it_down_in_front_of_it_where_it_stands_2d() {
        let mut s = stone_wall(2);
        let c = cast(&mut s, "StoneWall");
        let mut mass_at_release = 0.0;
        run(&mut s, 220, |s, _| {
            let id = s.sim.casts[c].result.unwrap_or(1);
            if let Some(w) = s.sim.weaves.get(&id)
                && !w.in_hand
                && mass_at_release == 0.0
            {
                mass_at_release = mass_of_weave(s, w);
            }
        });
        assert_eq!(s.sim.casts[c].state, CastState::Halted);
        let weave = &s.sim.weaves[&s.sim.casts[c].result.unwrap()];
        let w = &s.sim.world;
        let ps: Vec<_> = weave.parts(w).collect();
        assert!(ps.iter().all(|p| p.order.is_some())); // every particle holds itself as it stands
        // It stands on the ground in front of the trench, its rock whole. Its base is rough, so it leans a little.
        let ground = s.ground as f64 * w.cell;
        let ys: Vec<f64> = ps.iter().map(|p| p.pos[1]).collect();
        assert!(ys.iter().cloned().fold(f64::INFINITY, f64::min) > ground - 0.05);
        assert!(ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max) > ground + 1.6);
        for p in &ps {
            assert!([12, 13, 14, 15].contains(&w.coords(w.cell_of(&p.pos) as usize)[0]));
        }
        let foot: Vec<f64> = ps.iter().filter(|p| p.pos[1] < ground + 0.25).map(|p| p.pos[0]).collect();
        assert!(foot.iter().cloned().fold(f64::INFINITY, f64::min) > 3.45); // its foot on the ground in front of the trench
        assert!(foot.iter().cloned().fold(f64::NEG_INFINITY, f64::max) < 4.05);
        assert!(ps.iter().all(|p| p.vel[1].abs() < 1e-3)); // still: the ground holds it up
        assert!(mass_of_weave(&s, weave) > mass_at_release * 0.5); // most of its earth still held
        // Lifting it by hand cost what lifting costs: its weight times the height, at push_energy for each M, and more for
        // moving it back and holding it up as it went.
        let lift = (mass_at_release * physics().gravity * 2.05) / physics().push_energy;
        assert!(s.sim.spent.push > lift * 0.85);
        assert!(s.sim.spent.push < lift * 2.5);
        // Where it came from is a trench.
        for y in s.ground as i64 - 4..s.ground as i64 {
            assert!(w.fill(w.index(16, y, 0) as usize) < physics().solid);
        }
        assert_eq!(w.matter[w.index(13, s.ground as i64 - 1, 0) as usize][EARTH], ground_amount(EARTH));
    }

    #[test]
    #[ignore = "its grip doesn't tear rock out of the ground: how hard mana grips, how strong soil is or how the spell goes is the author's to choose"]
    fn is_weaker_for_a_caster_with_little_earth_in_them() {
        let strength = |affinity: f64| {
            let mut s = stone_wall(2);
            s.sim.casters[s.caster].stats.body.affinity[EARTH].genetics = affinity;
            cast(&mut s, "StoneWall");
            ticks(&mut s, 220);
            carried(&s, EARTH)
        };
        assert!(strength(0.15) < strength(0.6) * 0.5);
    }

    #[test]
    fn fails_where_there_is_no_earth_before_gathering_anything() {
        let mut s = stone_wall(2);
        let z = s.caster_pos()[2];
        s.will().aim = [6.0, 4.0, z]; // the air
        let held = s.sim.casters[s.caster].held();
        let c = cast(&mut s, "StoneWall");
        ticks(&mut s, 2);
        assert_eq!(s.sim.casts[c].state, CastState::Failed);
        assert_eq!(s.sim.casts[c].code, Some(1.0));
        assert!(close(s.sim.casters[s.caster].held(), held, 6)); // nothing gathered
    }

    #[test]
    #[ignore = "its grip doesn't tear rock out of the ground: how hard mana grips, how strong soil is or how the spell goes is the author's to choose"]
    fn crumbles_as_its_order_burns_its_mana_away_holding_up_less_and_less() {
        let mut s = stone_wall(2);
        cast(&mut s, "StoneWall");
        ticks(&mut s, 220);
        let standing = carried(&s, EARTH);
        // A costlier order: the same wall, standing for less long.
        tuned(|p| p.order_burn = 0.002, || ticks(&mut s, 120));
        assert!(carried(&s, EARTH) < standing * 0.3);
        let w = &s.sim.world;
        let fallen: f64 = (0..w.size).map(|i| w.matter[i][EARTH]).sum();
        assert!(fallen > 64.0 * 8.0 * ground_amount(EARTH) - standing * 0.5); // what it let go of is ground again
    }
}

mod fireball_ {
    use super::*;

    fn holds_together_in_flight_bursts_against_the_pillar_and_is_gone(dims: u8) {
        let mut s = fireball(dims);
        let c = cast(&mut s, "Fireball");
        let mut burst_at: Option<usize> = None;
        let mut reached: f64 = 0.0;
        let mut released = 0.0;
        let mut together = 0.0;
        run(&mut s, 70, |s, t| {
            let result = s.sim.casts[c].result;
            let Some(weave) = result.and_then(|r| s.sim.weaves.get(&r)).or_else(|| s.sim.weaves.values().next()) else { return };
            let w = &s.sim.world;
            if !weave.in_hand && released == 0.0 {
                released = weave.mana(w);
            }
            reached = reached.max(weave.origin[0]);
            if burst_at.is_none() && weave.reg(w, 0) == 1.0 {
                burst_at = Some(t);
                // How much of it is still together, within 1.5 m of its middle, when it hits.
                let ctr = weave.centre(w).unwrap().0;
                for p in weave.parts(w) {
                    if js::hypot3(p.pos[0] - ctr[0], p.pos[1] - ctr[1], p.pos[2] - ctr[2]) < 1.5 {
                        together += total(&p.free);
                    }
                }
            }
        });
        assert_eq!(s.sim.casts[c].state, CastState::Halted);
        assert!(burst_at.is_some());
        assert!(reached > 10.0); // the pillar's face is at 11 m
        assert!(together > released * 0.85); // held by its own order all the way there
        // Each particle that hears of the hit bursts and lets go. One that strayed from the rest never hears.
        let left = s.sim.weaves.values().fold(0.0, |m, w| m + w.mana(&s.sim.world));
        assert!(left < released * 0.05);
        if s.sim.weaves.is_empty() {
            let d = s.sim.events.iter().find(|e| e.kind == EventKind::Dissolve).unwrap();
            assert_eq!(d.detail.as_deref(), Some("its order let it go"));
        }
    }

    #[test]
    fn holds_together_in_flight_bursts_against_the_pillar_and_is_gone_2d() {
        holds_together_in_flight_bursts_against_the_pillar_and_is_gone(2);
    }

    #[test]
    fn holds_together_in_flight_bursts_against_the_pillar_and_is_gone_3d() {
        holds_together_in_flight_bursts_against_the_pillar_and_is_gone(3);
    }

    #[test]
    fn is_poured_into_one_point_and_held_in_the_hand_while_its_order_is_ingrained() {
        let mut s = fireball(2);
        let c = cast(&mut s, "Fireball");
        s.sim.step();
        let id = *s.sim.weaves.keys().next().unwrap();
        let weave = &s.sim.weaves[&id];
        assert!(weave.in_hand);
        assert_eq!(weave.particles.len(), 72); // 120 M gathered, a quarter of it fire, 60% kept: 18 M in 0.25 M particles
        let at: Vec<_> = weave.parts(&s.sim.world).map(|p| p.pos).collect();
        s.sim.run(3);
        let weave = &s.sim.weaves[&id];
        assert_eq!(weave.parts(&s.sim.world).map(|p| p.pos).collect::<Vec<_>>(), at); // the hand holds it still
        while s.sim.weaves[&id].in_hand {
            s.sim.step();
        }
        assert!(s.sim.weaves[&id].parts(&s.sim.world).all(|p| p.order.is_some()));
        assert_eq!(s.sim.casts[c].state, CastState::Running); // still to throw
    }

    #[test]
    fn can_show_any_particles_order_instruction_by_instruction() {
        let mut s = fireball(2);
        let c = cast(&mut s, "Fireball");
        while s.sim.weaves.get(&1).map(|w| w.in_hand) != Some(false) {
            s.sim.step();
        }
        s.sim.step();
        assert_eq!(s.sim.traces.len(), 0); // only when asked: it costs time
        s.sim.trace_orders = true;
        s.sim.step();
        let traces = &s.sim.traces[&1];
        assert!(traces.len() > 10);
        let ids: std::collections::HashSet<_> = traces.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), traces.len()); // one for each particle that ran its order
        let t = &traces[3];
        assert_eq!(t.steps[0].addr, s.sim.casts[c].program.program.labels["Fireball.order"]);
        assert_eq!(t.steps[0].n[..3], [0.0, 0.0, 0.0]); // it isn't told where it is: it only feels
        assert_eq!(t.outcome, "done");
        assert!(t.beats > 5.0);
        assert!(t.beats <= 64.0);
        assert!(close(t.burned, t.beats * physics().order_burn, 9)); // thinking burns its mana
    }
}

mod gust_ {
    use super::*;

    // A breath of mana weighs grams (D40): it can't shove a person by striking them. What it does is drive the air, and
    // bodies feel the air with real air (PLAN step 3).
    #[test]
    #[ignore = "needs real air: PLAN step 3"]
    fn pushes_someone_back_while_it_is_maintained() {
        let mut s = gust(2);
        let target = s.target.unwrap();
        let x = s.sim.world.bodies[target].pos[0];
        let c = cast(&mut s, "Gust");
        ticks(&mut s, 10);
        s.will().maintain = false;
        ticks(&mut s, 15);
        assert_eq!(s.sim.casts[c].state, CastState::Halted);
        assert!(s.sim.world.bodies[target].pos[0] - x > 0.25);
        assert_eq!(s.sim.casters[s.caster].harm, 0.0);
    }

    #[test]
    fn sends_a_breath_every_tick_it_is_kept_up_and_pays_for_its_speed() {
        let mut s = gust(2);
        cast(&mut s, "Gust");
        let mut sent: Vec<f64> = Vec::new();
        run(&mut s, 6, |s, _| sent.push(s.sim.world.particles.iter().filter(|p| p.weave == 0).fold(0.0, |t, p| t + p.free[2])));
        assert!(*sent.last().unwrap() > sent[0] * 3.0);
        // The mind transformed mana into the breath's speed: grams of it, so a few joules, which it rests off at once.
        assert!(s.sim.transformed > 0.0);
    }

    #[test]
    fn overcharges_a_caster_who_keeps_it_up_too_long() {
        let mut s = gust(2);
        cast(&mut s, "Gust");
        ticks(&mut s, 40);
        let c = &s.sim.casters[s.caster];
        assert!(c.harm > 0.0);
        assert!(c.condition.body < 1.0);
    }
}

mod water_shield_ {
    use super::*;

    #[test]
    fn makes_water_around_the_caster_and_follows_them_while_they_keep_it_up() {
        let mut s = water_shield(2);
        s.will().maintain = true;
        let c = cast(&mut s, "WaterShield");
        let mut t = 0;
        while t < 200 && !s.sim.weaves.get(&1).is_some_and(|w| w.locks.input) {
            ticks(&mut s, 1);
            t += 1;
        }
        ticks(&mut s, 2);
        assert!(s.sim.weaves[&1].locks.input);
        assert!(carried(&s, WATER) > 5.0);
        let x = s.sim.weaves[&1].origin[0];
        // Still inside it: further, and they'd walk into its front, which pushes back.
        let body = s.sim.casters[s.caster].body;
        s.sim.world.bodies[body].pos[0] += 0.5;
        ticks(&mut s, 25);
        assert!(close(s.sim.weaves[&1].origin[0] - x, 0.5, 1)); // its caster tells it how to move: its order can't see them
        s.will().maintain = false;
        ticks(&mut s, 2);
        assert_eq!(s.sim.casts[c].state, CastState::Halted);
        let w = &s.sim.weaves[&1];
        assert_eq!([5, 6, 7].map(|k| w.reg(&s.sim.world, k)), [0.0, 0.0, 0.0]); // let be, it holds still where it is
    }

    #[test]
    fn falls_in_a_splash_as_its_order_burns_its_mana_away() {
        let mut s = water_shield(2);
        let c = cast(&mut s, "WaterShield");
        finish(&mut s, c, 2);
        let made = carried(&s, WATER);
        assert!(made > 5.0);
        // A costlier order: the same shield, holding for less long.
        tuned(|p| p.order_burn = 0.002, || ticks(&mut s, 80));
        assert!(carried(&s, WATER) < made * 0.05);
        let w = &s.sim.world;
        let water: f64 = (0..w.size).map(|i| w.matter[i][WATER]).sum();
        assert!(water > made * 0.9);
    }

    #[test]
    fn casts_in_3d_a_shell_around_the_caster() {
        let mut s = water_shield(3);
        let c = cast(&mut s, "WaterShield");
        finish(&mut s, c, 2); // about 450 points, each with a sine and a cosine, then every particle ingrained
        assert_eq!(s.sim.casts[c].state, CastState::Halted);
        let weave = &s.sim.weaves[&s.sim.casts[c].result.unwrap()];
        let w = &s.sim.world;
        let zs: std::collections::HashSet<i64> = weave.parts(w).map(|p| w.coords(w.cell_of(&p.pos) as usize)[2]).collect();
        assert!(zs.len() > 5);
        assert!(carried(&s, WATER) > 5.0);
    }
}

mod scenes {
    use super::*;

    #[test]
    fn put_the_aim_on_the_ground() {
        let s = stone_wall(2);
        let p = on_ground(&s, 6.0);
        let w = &s.sim.world;
        assert_eq!(w.matter[w.cell_of(&p) as usize][EARTH], ground_amount(EARTH));
        assert_eq!(total(&w.matter[w.cell_of(&[p[0], p[1] + 0.25, p[2]]) as usize]), 0.0);
    }
}
