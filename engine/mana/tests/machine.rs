//! The machine: the mind, the body, weaves and mana as a fluid (from test/machine.test.ts), and what pushing costs and
//! the second flaw spreading (from test/physics.test.ts), which need a caster.

use std::sync::Arc;

use mana::asm::{Code, assemble};
use mana::js;
use mana::load::resolver;
use mana::vm::caster::{CasterStats, Will, adept};
use mana::vm::parts::total;
use mana::vm::physics::{physics, tuned};
use mana::vm::sim::{CastState, EventKind, Sim};
use mana::vm::weave::stamp_of;
use mana::vm::world::{EARTH, World, ground_amount, mass_of, packed};

fn code(src: &str) -> Arc<Code> {
    Code::new(assemble(src, "test.masm", &resolver(vec![])).unwrap_or_else(|e| panic!("{e}")))
}

struct Setup {
    sim: Sim,
    caster: usize,
    cast: usize,
}

impl Setup {
    fn n(&self, r: usize) -> f64 {
        self.sim.casts[self.cast].frame.n[r]
    }
    fn state(&self) -> CastState {
        self.sim.casts[self.cast].state
    }
    fn fault(&self) -> String {
        self.sim.casts[self.cast].fault.clone().unwrap_or_default()
    }
    fn weave(&self) -> &mana::vm::weave::Weave {
        self.sim.weaves.values().next().unwrap()
    }
    fn particle(&self, i: usize) -> &mana::vm::world::Particle {
        self.sim.world.particle(self.weave().particles[i]).unwrap()
    }
}

fn setup_with(src: &str, tweak: impl FnOnce(&mut CasterStats)) -> Setup {
    let world = World::with_ground(32, 24, 1, 8, EARTH);
    let mut sim = Sim::new(world);
    let mut stats = adept();
    tweak(&mut stats);
    let caster = sim.add_caster("Mage", [2.0, 2.9, 0.125], Some(stats));
    sim.casters[caster].will = Will { aim: [5.0, 2.9, 0.125], amount: 100.0, force: 0.5, maintain: false };
    let cast = sim.cast(caster, code(src), None).unwrap();
    Setup { sim, caster, cast }
}

fn setup(src: &str) -> Setup {
    setup_with(src, |_| {})
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

mod the_mind {
    use super::*;

    #[test]
    fn counts_loops_and_halts() {
        let mut s = setup(
            "
main:   LDI   n0, #0
        LDI   n1, #1
.loop:  ADD   n0, n1
        ADD   n1, #1
        CMP   n1, #10
        JLE   .loop
        HALT",
        );
        s.sim.run_casts(200);
        assert_eq!(s.state(), CastState::Halted);
        assert_eq!(s.n(0), 55.0);
    }

    #[test]
    fn does_the_math_shapes_need() {
        let mut s = setup(
            "
        LDI   n0, #0.5236     ; 30°
        SIN   n0
        LDI   n1, #2
        SQRT  n1
        LDI   n2, #1
        LDI   n3, #1
        ATN2  n2, n3
        LDI   n4, #7
        DIV   n4, #0          ; dividing by nothing gives nothing
        LDI   n5, #-2.5
        FLOOR n5
        HALT",
        );
        s.sim.run_casts(200);
        assert!(close(s.n(0), 0.5, 4));
        assert!(close(s.n(1), std::f64::consts::SQRT_2, 6));
        assert!(close(s.n(2), std::f64::consts::PI / 4.0, 6));
        assert_eq!(s.n(4), 0.0);
        assert_eq!(s.n(5), -3.0);
    }

    #[test]
    fn calls_routines_keeps_a_stack_and_remembers() {
        let mut s = setup(
            "
main:   LDI   n0, #21
        CALL  double
        PUSH  n0
        LDI   n0, #0
        POP   n1
        LDI   n2, #10
        ST    n1, [n2]
        LD    n3, [n2]
        HALT
double: ADD   n0, n0
        RET",
        );
        s.sim.run_casts(200);
        assert_eq!(s.n(1), 42.0);
        assert_eq!(s.n(3), 42.0);
    }

    #[test]
    fn cant_think_past_the_registers_it_has() {
        let mut s = setup_with("LDI n10, #1\nHALT", |s| s.mind.registers.genetics = 8.0);
        s.sim.run_casts(200);
        assert_eq!(s.state(), CastState::Fault);
        assert!(s.fault().contains("NO_ROOM: n10: this mind thinks with 8 registers"));
    }

    #[test]
    fn thinks_faster_with_training_and_with_each_casting_the_law_of_conditioning() {
        let src = "
main:   LDI   n0, #0
.l:     ADD   n0, #1
        CMP   n0, #400
        JLT   .l
        HALT";
        let ticks = |conditioning: f64, training: f64| {
            let mut s = setup_with(src, |s| s.mind.speed.training = training);
            let name = s.sim.casts[s.cast].name.clone();
            s.sim.casters[s.caster].conditioning.insert(name, conditioning);
            s.sim.run_casts(200);
            let c = &s.sim.casts[s.cast];
            c.ended_at.unwrap() - c.started_at
        };
        assert_eq!(ticks(0.0, 0.0), 4); // 1200 beats at 300 a tick
        assert_eq!(ticks(9.0, 0.0), 0); // ten times as many beats: it all fits in the first tick
        assert_eq!(ticks(0.0, 300.0), 2);
    }

    #[test]
    fn pays_more_for_slow_math_and_counts_where_the_beats_went() {
        let mut s = setup(
            "
main:   LDI   n0, #1
        ADD   n0, #1
slow:   DIV   n0, #2
        SIN   n0
        HALT",
        );
        s.sim.run_casts(200);
        let c = &s.sim.casts[s.cast];
        assert_eq!(c.beats, 1.0 + 1.0 + 4.0 + 8.0 + 1.0);
        let slow = c.program.program.labels["slow"];
        let p = c.profile[&slow];
        assert_eq!((p.runs, p.beats), (1.0, 4.0));
    }

    #[test]
    fn steps_one_instruction_at_a_time_and_the_world_moves_when_its_beats_for_the_tick_run_out() {
        let mut s = setup_with("LDI n0, #1\nLDI n1, #2\nLDI n2, #3\nHALT", |s| s.mind.speed.genetics = 2.0);
        assert!(s.sim.step_instruction(s.cast));
        assert_eq!(s.n(0), 1.0);
        assert_eq!(s.sim.tick(), 0); // one more fits in this tick
        s.sim.step_instruction(s.cast);
        assert_eq!(s.n(1), 2.0);
        assert_eq!(s.sim.tick(), 1); // that was the last: the tick ended
        s.sim.step_instruction(s.cast);
        assert_eq!(s.n(2), 3.0);
        assert_eq!(s.sim.tick(), 1);
    }

    #[test]
    fn stops_before_a_breakpoint_partway_through_a_tick_and_goes_on_from_it() {
        let mut s = setup(
            "
main:   LDI   n0, #0
.loop:  ADD   n0, #1
here:   CMP   n0, #5
        JLT   main.loop
        HALT",
        );
        let here = s.sim.casts[s.cast].program.program.labels["here"];
        assert!(s.sim.run_until(s.cast, &mut |a| a == here, 10));
        assert!(s.sim.mid_tick);
        assert_eq!(s.n(0), 1.0);
        assert!(s.sim.run_until(s.cast, &mut |a| a == here, 10));
        assert_eq!(s.n(0), 2.0);
        s.sim.step(); // finish the tick
        assert!(!s.sim.mid_tick);
        assert_eq!(s.state(), CastState::Halted);
        assert_eq!(s.n(0), 5.0);
    }

    #[test]
    fn counts_a_cast_toward_conditioning_when_it_halts() {
        let mut s = setup("HALT");
        s.sim.run_casts(200);
        let name = &s.sim.casts[s.cast].name;
        assert_eq!(s.sim.casters[s.caster].conditioning.get(name), Some(&1.0));
    }

    #[test]
    fn wont_run_an_orders_instructions() {
        let mut s = setup("KICK n0:2\nHALT");
        s.sim.run_casts(200);
        assert!(s.fault().contains("ORDER_ONLY"));
    }
}

mod the_body {
    use super::*;

    #[test]
    fn loses_mana_to_a_lack_of_affinity_when_it_filters_and_only_then() {
        let mut s = setup_with(
            "
        .use  Elements
        GATH  m0, #100
        CIRC  m0
        FILT  m1, m0, #EARTH
        HALT",
            |s| {
                s.body.affinity[3].genetics = 0.4;
                s.body.drain.genetics = 0.0;
            },
        );
        s.sim.step();
        let c = &s.sim.casters[s.caster];
        let (m0, m1) = (c.regs[0], c.regs[1]);
        assert!(close(total(&m1.parts), 10.0, 9)); // 25 of earth × 40%
        assert!(close(m1.parts[3], 10.0, 9));
        assert_eq!(m0.parts[3], 0.0);
        assert!(close(total(&m0.parts), 75.0, 9)); // the residue: the other three parts
        assert!(close(total(&c.flow), 60.0 + 15.0, 9)); // baseline + what slipped
    }

    #[test]
    fn lets_go_of_what_it_no_longer_holds_unheld_mana_joins_the_flow() {
        let mut s = setup_with("GATH m0, #50\nGATH m1, #50\nCIRC m1\nHALT", |s| {
            s.body.focus.genetics = 3.0;
            s.body.drain.genetics = 0.0;
        });
        s.sim.step(); // tick 0: m0 was never held
        let c = &s.sim.casters[s.caster];
        assert_eq!(total(&c.regs[0].parts), 0.0);
        assert!(close(total(&c.regs[1].parts), 50.0, 9));
        s.sim.run(3); // held through tick 3, let go at its end
        let c = &s.sim.casters[s.caster];
        assert_eq!(total(&c.regs[1].parts), 0.0);
        assert!(close(total(&c.flow), 160.0, 9));
    }

    #[test]
    fn drains_its_flow_back_to_the_air_down_to_its_baseline() {
        let mut s = setup("GATH m0, #100\nHALT");
        s.sim.step();
        assert!(close(total(&s.sim.casters[s.caster].flow), 160.0 - 15.0, 9));
        s.sim.run(20);
        assert!(close(total(&s.sim.casters[s.caster].flow), 60.0, 9));
    }

    #[test]
    fn is_harmed_past_its_capacity() {
        let mut s = setup_with("GATH m0, #300\nCIRC m0\nHALT", |s| s.body.capacity.genetics = 200.0);
        s.sim.run(3);
        let c = &s.sim.casters[s.caster];
        assert!(c.harm > 0.0);
        assert!(c.condition.body < 1.0);
        assert!(s.sim.events.iter().any(|e| e.kind == EventKind::Overcharge));
    }

    #[test]
    fn works_only_the_streams_it_has() {
        let mut s = setup_with("GATH m2, #10\nHALT", |s| s.body.streams.genetics = 2.0);
        s.sim.run_casts(200);
        assert!(s.fault().contains("NO_STREAM"));
    }

    #[test]
    fn answers_to_four_names_only_the_law_of_the_four() {
        let mut s = setup("GATH m0, #10\nFILT m1, m0, #4\nHALT");
        s.sim.run_casts(200);
        assert!(s.fault().contains("NO_NAME"));
    }

    #[test]
    fn never_makes_or_loses_mana_the_law_of_conservation() {
        let mut s = setup(
            "
        .use  Elements
        IN    n0:2, HAND
        IN    n3:5, AIM
        GATH  m0, #200
        CIRC  m0
        FILT  m1, m0, #AIR
        FILT  m2, m0, #FIRE
        LDI   n6, #20
        SEND  m1, n6, n0:2, n3:5
        VENT  m2
        HALT",
        );
        let before = s.sim.ledger().total;
        for _ in 0..30 {
            s.sim.step();
            assert!(close(s.sim.ledger().total, before, 6));
        }
    }
}

mod weaves {
    use super::*;

    #[test]
    fn must_be_set_loose_before_they_are_locked() {
        let mut s = setup("IN n0:2, AIM\nWEAV n3, n0:2\nLOCK n3, INPUT\nHALT");
        s.sim.run_casts(200);
        assert!(s.fault().contains("NOT_LOOSE"));
    }

    #[test]
    fn take_nothing_more_once_their_input_is_locked() {
        let mut s = setup(
            "
        IN    n0:2, AIM
        WEAV  n3, n0:2
        MANI  n3
        LOCK  n3, INPUT
        GATH  m0, #10
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #5
        EMIT  m0, n7, n3, n4:6
        HALT",
        );
        s.sim.run_casts(200);
        assert!(s.fault().contains("LOCKED"));
    }

    #[test]
    fn fall_back_into_the_body_when_a_spell_ends_with_them_in_hand() {
        let mut s = setup_with(
            "
        IN    n0:2, AIM
        WEAV  n3, n0:2
        GATH  m0, #40
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #40
        EMIT  m0, n7, n3, n4:6
        HALT",
            |s| s.body.drain.genetics = 0.0,
        );
        s.sim.step();
        assert_eq!(s.sim.weaves.len(), 0);
        assert!(close(total(&s.sim.casters[s.caster].flow), 100.0, 6));
    }

    #[test]
    fn fray_where_an_order_thinks_too_long_that_particle_lets_go_and_the_rest_of_the_weave_goes_on() {
        let mut s = setup(
            "
        IN    n0:2, AIM
        WEAV  n3, n0:2
        GATH  m0, #40
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #40
        EMIT  m0, n7, n3, n4:6
        ORDR  n3, spin
        INGR  n3, #0
        MANI  n3
        HALT
spin:   JMP   spin",
        );
        let before = s.sim.ledger().total;
        s.sim.run(3);
        let fray = s.sim.events.iter().find(|e| e.kind == EventKind::Fray).unwrap();
        assert!(fray.detail.as_ref().unwrap().contains("FRAYED"));
        let weave = s.weave();
        assert!(!weave.particles.is_empty()); // the rest are still its caster's, in reach
        assert!(weave.parts(&s.sim.world).all(|p| p.order.is_none()));
        let w = &s.sim.world;
        assert_eq!(weave.particles.len(), w.particles.len() - w.particles.iter().filter(|p| p.weave == 0).count());
        assert!(close(s.sim.ledger().total, before, 6));
    }

    #[test]
    fn frees_matter_when_told_to_condense_less_than_nothing_the_flaw() {
        // An earth weave in the ground, 0.6 m down, binds earth. Nothing checks CNDS's sign, so a negative amount runs it
        // backwards.
        let src = |amount: f64| {
            format!(
                "
        .use  Elements
        LDI   n0, #3.125
        LDI   n1, #1.375
        LDI   n2, #0.125
        WEAV  n3, n0:2
        GATH  m0, #40
        CIRC  m0
        FILT  m1, m0, #EARTH
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        MEAS  n7, m1
        EMIT  m1, n7, n3, n4:6
        ORDR  n3, unmake
        PCNT  n8, n3
        LDI   n9, #0
again:  INGR  n3, n9
        ADD   n9, #1
        CMP   n9, n8
        JLT   again
        MANI  n3
        HALT
unmake: CNDS  #{}
        RET",
                js::num(amount)
            )
        };
        let freed = |amount: f64| {
            let mut s = setup(&src(amount));
            // The ground it's poured into is rock, as full as earth gets: nothing it holds can be pushed into it.
            for m in s.sim.world.matter.iter_mut() {
                if m[3] > 0.0 {
                    m[3] = packed(3);
                }
            }
            let before = s.sim.ledger();
            s.sim.run(2); // it's set loose, and its order runs, in the first tick
            let after = s.sim.ledger();
            assert!(close(after.total, before.total, 6));
            let free = s.weave().mana(&s.sim.world) - 6.0; // 40 raw: 6 M of earth
            (free, before.condensed - after.condensed)
        };
        let (free, condensed) = freed(-10.0);
        assert!(free > 9.0);
        assert!(condensed > 9.0);
        // A full cell has no room, so condensing the same amount the right way round makes nothing.
        assert!(close(freed(10.0).1, 0.0, 6));
    }
}

/// A spell that gathers fire and pours it into a weave at a point ahead of the caster, before the code given.
fn pour(rest: &str, gather: f64) -> String {
    format!(
        "
        .use  Elements
        GATH  m0, #{}
        CIRC  m0
        FILT  m1, m0, #FIRE
        LDI   n0, #4
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        MEAS  n7, m1
        EMIT  m1, n7, n3, n4:6
{rest}",
        js::num(gather)
    )
}

mod mana_as_a_fluid {
    use super::*;

    // These look at single particles: none of them merge here (physics.rs has merging).
    fn no_merging<T>(run: impl FnOnce() -> T) -> T {
        tuned(|p| p.merge_range = 0.0, run)
    }

    #[test]
    fn pours_into_particles_held_still_in_the_hand_and_spreads_by_its_own_pressure_once_let_go() {
        no_merging(|| {
            let mut s =
                setup(&pour("        TICK\n        TICK\n        TICK\n        TICK\n        MANI  n3\n        HALT", 120.0));
            s.sim.step();
            assert_eq!(s.weave().particles.len() as f64, (18.0 / physics().mote).ceil()); // 120 raw, a quarter fire, 60% kept
            let spread = |s: &Setup| {
                let c = s.weave().centre(&s.sim.world).unwrap().0;
                s.weave()
                    .parts(&s.sim.world)
                    .map(|p| js::hypot2(p.pos[0] - c[0], p.pos[1] - c[1]))
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            let laid = spread(&s);
            s.sim.run(2);
            assert!(close(spread(&s), laid, 9)); // in hand: still
            s.sim.run(10);
            assert!(spread(&s) > laid * 4.0);
            assert!(s.sim.world.momentum_error() < 1e-9);
        });
    }

    #[test]
    fn pushes_a_particle_off_the_body_through_its_reach_as_hard_as_the_mind_can_transform_mana_into_energy() {
        no_merging(|| {
            // One particle: a quarter of 1⅔ M is fire, and 60% of that is 0.25 M.
            let src = pour(
                "        LDI   n8, #0.5
        LDI   n9, #0
        LDI   n10, #0
        LDI   n11, #0
        MEAS  n20, m0
        SHOV  m0, n3, n11, n8:10
        MEAS  n21, m0
        MANI  n3
        HALT",
                5.0 / 3.0,
            );
            let mut s = setup(&src);
            s.sim.step();
            assert_eq!(s.weave().particles.len(), 1);
            let p = s.particle(0);
            assert!(close(p.vel[0], 0.5, 1)); // all it asked for (less what the air took)
            // It cost the kinetic energy it added to the particle and to the body it pushed off, and that mana went
            // loose into the air, still mana. The body was pushed back, and its feet on the ground held it.
            let m = 0.25 * physics().mana_mass[0]; // 0.25 M of fire; 1⅔ is a float32
            let body = &s.sim.world.bodies[s.sim.casters[s.caster].body];
            let energy = 0.5 * m * 0.5f64.powi(2) * (1.0 + m / body.mass);
            assert!(close(s.sim.transformed, energy, 5));
            assert!(close(s.n(20) - s.n(21), energy / physics().push_energy, 5));
            assert!(close(s.sim.world.impulse.walls[0], m * 0.5, 2)); // what the body took, its feet gave the ground
            assert_eq!(body.vel[0], 0.0);
            // A weaker mind pushes less hard.
            let mut weak = setup_with(&src, |s| s.mind.power.genetics = energy / 4.0);
            weak.sim.step();
            assert!(weak.particle(0).vel[0] < 0.3);
            // Out of reach, nothing happens: it can't even be poured there.
            let mut far = setup_with(&src, |s| s.body.reach.genetics = 0.5);
            far.sim.step();
            assert_eq!(far.sim.world.particles.len(), 0);
            assert_eq!(far.n(20), far.n(21));
        });
    }

    #[test]
    fn ingrains_an_order_particle_by_particle_at_a_beat_for_every_instruction_it_could_run() {
        no_merging(|| {
            let mut s = setup(&pour(
                "        ORDR  n3, order
        INGR  n3, #0
        INGR  n3, #1
        MANI  n3
        HALT
order:  CALL  sub
        CMP   n0, #0
        JEQ   .out
        NOP
.out:   RET
sub:    NOP
        RET",
                120.0,
            ));
            s.sim.run_casts(200);
            let c = &s.sim.casts[s.cast];
            assert_eq!(c.program.order_length(c.program.program.labels["order"]), 7);
            assert_eq!(s.weave().parts(&s.sim.world).filter(|p| p.order.is_some()).count(), 2);
            for (a, l) in &c.program.program.lines {
                if l.text.trim().starts_with("INGR") {
                    assert_eq!(c.profile[a].beats, 4.0 + 7.0);
                }
            }
        });
    }

    #[test]
    fn runs_an_order_in_every_ingrained_particle_which_burns_its_own_mana_to_think_and_pays_to_push_itself() {
        no_merging(|| {
            let mut s = setup(&pour(
                "        ORDR  n3, kick
        INGR  n3, #0
        MANI  n3
        HALT
kick:   LDI   n0, #0
        LDI   n1, #0.05
        LDI   n2, #0
        KICK  n0:2
        RET",
                120.0,
            ));
            s.sim.step();
            let id = s.weave().id;
            let before = total(&s.particle(0).free);
            let vy = s.particle(0).vel[1];
            s.sim.trace_orders = true;
            s.sim.step();
            let trace = &s.sim.traces[&id];
            assert_eq!(trace.len(), 1); // only the ingrained one
            assert!(close(trace[0].burned, trace[0].beats * physics().order_burn, 12));
            // It kicks first, and pays the kinetic energy the kick adds (and a little more, for the air it pushes off);
            // its thinking is paid for after.
            let m = before * physics().mana_mass[0]; // fire
            let kick_cost = (m * vy * 0.05 + 0.5 * m * 0.05f64.powi(2)) / physics().push_energy;
            assert!(close(total(&s.particle(0).free), before - kick_cost - trace[0].burned, 4));
            assert!(s.sim.world.momentum_error() < 1e-9);
        });
    }

    #[test]
    fn lets_a_particle_feel_its_neighbours_how_dense_which_way_it_thickens_how_they_move() {
        no_merging(|| {
            let mut s = setup(&pour(
                "        ORDR  n3, feel
        INGR  n3, #0
        MANI  n3
        HALT
feel:   DENS  n5
        GRAD  n6:8
        NVEL  n9:11
        IN    n12:14, VEL
        RET",
                120.0,
            ));
            s.sim.run(2);
            s.sim.trace_orders = true;
            s.sim.step();
            let n = &s.sim.traces.values().next().unwrap()[0].n;
            assert!(n[5] > 0.0);
            assert!(js::hypot3(n[6], n[7], n[8]) > 0.0);
        });
    }

    #[test]
    fn loses_particles_with_no_order_that_stray_out_of_its_casters_reach_and_they_settle_into_the_air_when_they_slow() {
        no_merging(|| {
            // Poured 2.3 m from a caster who reaches 2.4 m: once let go, what spreads further out is no longer theirs.
            let mut s = setup_with(&pour("        MANI  n3\n        HALT", 120.0), |s| s.body.reach.genetics = 2.4);
            s.sim.run(15);
            let in_weave = s.sim.weaves.values().next().map(|w| w.particles.len()).unwrap_or(0);
            let loose = s.sim.world.particles.iter().filter(|p| p.weave == 0).count();
            assert!(in_weave < 72);
            assert!(loose + in_weave <= 72);
            let before = s.sim.ledger().total;
            s.sim.run(60);
            assert!(close(s.sim.ledger().total, before, 6));
            assert!(s.sim.world.particles.iter().filter(|p| p.weave == 0).count() < loose);
        });
    }

    #[test]
    fn passes_a_weaves_registers_from_particle_to_particle_by_touch_and_not_to_what_nothing_touches() {
        no_merging(|| {
            // Two particles poured together and one far off, each ingrained with an order that does nothing.
            let mut s = setup(
                "        .use  Elements
        GATH  m0, #40
        CIRC  m0
        FILT  m1, m0, #FIRE
        LDI   n0, #3
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #0.5
        EMIT  m1, n7, n3, n4:6
        LDI   n4, #2
        LDI   n7, #0.25
        EMIT  m1, n7, n3, n4:6
        ORDR  n3, idle
        INGR  n3, #0
        INGR  n3, #1
        INGR  n3, #2
        MANI  n3
        HALT
idle:   RET",
            );
            s.sim.step();
            let ps = s.weave().particles.clone();
            let (a, b, far) = (ps[0], ps[1], ps[2]);
            // One of the pair writes w3, as its order would (PUTW): the newest write.
            let tick = s.sim.tick() as f64;
            let pa = s.sim.world.particle_mut(a).unwrap();
            pa.regs[3] = 5.0;
            pa.stamp[3] = stamp_of(tick, a as f64);
            s.sim.step();
            assert_eq!(s.sim.world.particle(b).unwrap().regs[3], 5.0); // touching it: it hears
            assert_eq!(s.sim.world.particle(far).unwrap().regs[3], 0.0); // 2 m off: it doesn't
        });
    }

    #[test]
    fn strains_the_mind_for_every_joule_it_transforms_and_harms_it_past_its_capacity() {
        no_merging(|| {
            // A strong push, again and again, by a mind of little capacity.
            let src = pour(
                "        MANI  n3
again:  CIRC  m0
        LDI   n8, #0.3
        LDI   n9, #0
        LDI   n10, #0
        PCNT  n5, n3
        LDI   n11, #0
each:   SHOV  m0, n3, n11, n8:10
        NEG   n8
        ADD   n11, #1
        CMP   n11, n5
        JLT   each
        TICK
        JMP   again",
                120.0,
            );
            let mut s = setup_with(&src, |s| {
                s.mind.capacity.genetics = 0.001;
                s.mind.recovery.genetics = 0.0;
            });
            s.sim.run(20);
            let c = &s.sim.casters[s.caster];
            assert!(s.sim.transformed > 0.001);
            assert!(close(c.strain, s.sim.transformed, 6)); // nothing eases it
            assert!(c.madness > 0.0);
            assert!(c.condition.mind < 1.0);
            assert!(s.sim.events.iter().any(|e| e.kind == EventKind::Overstrain));
        });
    }

    #[test]
    fn senses_nothing_out_of_reach() {
        no_merging(|| {
            let mut s = setup(
                "        .use  Elements
        LDI   n0, #2.125
        LDI   n1, #1.875
        LDI   n2, #0.125
        PROB  n4, n0:2, #EARTH
        LDI   n0, #8.125
        PROB  n5, n0:2, #EARTH
        HALT",
            );
            s.sim.run_casts(200);
            assert_eq!(s.n(4), ground_amount(3)); // the ground under its feet
            assert_eq!(s.n(5), 0.0); // 6 m away: it can't tell
        });
    }

    #[test]
    fn wont_lock_a_shape_a_shape_is_held_by_pushing() {
        let e =
            assemble("IN n0:2, AIM\nWEAV n3, n0:2\nMANI n3\nLOCK n3, SHAPE\nHALT", "test.masm", &resolver(vec![])).unwrap_err();
        assert!(e.to_string().contains("held by pushing"));
    }
}

mod what_pushing_costs {
    use super::*;

    /// A particle poured at the hand, let go of, and kicked by its own order: `src` is how, tick by tick.
    fn kicked(src: &str, air: bool) -> (Sim, u64) {
        let mut world = World::with_ground(32, 24, 1, 8, EARTH);
        if !air {
            for a in world.air.iter_mut() {
                *a = [0.0; 4];
            }
        }
        let mut sim = Sim::new(world);
        let caster = sim.add_caster("Mage", [2.0, 2.9, 0.125], None);
        let c = &mut sim.casters[caster];
        c.will = Will { aim: [5.0, 2.9, 0.125], amount: 100.0, force: 0.5, maintain: false };
        // Where there's no air to gather, it's given its mana.
        if !air {
            c.regs[0].parts = [25.0; 4];
        }
        c.regs[0].hold_until = 10.0;
        let program = code(&format!(
            "       GATH  m0, #{}
        LDI   n0, #3
        LDI   n1, #4
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #0.25
        EMIT  m0, n7, n3, n4:6
        ORDR  n3, kick
        INGR  n3, #0
        MANI  n3
        HALT
kick:   {src}",
            if air { 100 } else { 0 }
        ));
        sim.cast(caster, program, None).unwrap();
        sim.run(3);
        let p = sim.world.particles.iter().find(|p| p.weave != 0).unwrap().id;
        (sim, p)
    }

    #[test]
    fn costs_the_kinetic_energy_a_push_adds() {
        // Up 0.05 m/tick every tick: it pays for what it gains against its weight, and for its speed.
        let (sim, p) = kicked(
            "LDI n5, #0
        LDI   n6, #0.05
        LDI   n7, #0
        KICK  n5:7
        RET",
            true,
        );
        let spent = sim.spent.kick;
        let m = mass_of(sim.world.particle(p).unwrap());
        assert!(spent > 0.0);
        assert!(spent < (0.5 * m * 0.1f64.powi(2) + m * physics().gravity * 1.0) / physics().push_energy + 0.01);
    }

    #[test]
    fn costs_nothing_to_slow_down_and_next_to_nothing_to_hold_something_up_against_its_weight() {
        // Every tick, it kicks itself back to standing still: it only ever takes speed away.
        let (mut sim, p) = kicked(
            "IN n5:7, VEL
        NEG   n5
        NEG   n6
        NEG   n7
        KICK  n5:7
        RET",
            true,
        );
        sim.run(20);
        let q = sim.world.particle(p).unwrap();
        // Holding itself up, it pushes the air it's in down, a little more each tick: that downdraft is all it pays for,
        // less than lifting itself a centimetre would cost.
        assert!(sim.spent.kick < (mass_of(q) * physics().gravity * 0.01) / physics().push_energy);
        assert!(q.vel[1].abs() <= physics().gravity + 1e-9); // it holds itself up, a tick's fall at most
    }

    #[test]
    fn pushes_off_the_air_where_there_is_none_it_has_nothing_to_push_off() {
        let src = "LDI n5, #0.05
        LDI   n6, #0
        LDI   n7, #0
        KICK  n5:7
        RET";
        let (sim, p) = kicked(src, true);
        assert!(sim.world.particle(p).unwrap().vel[0] > 0.04); // what the air took back, it took from the air
        assert!(sim.spent.kick > 0.0);
        // With no air, and none let out by its thinking, it has nothing to push off.
        tuned(
            |p| p.order_burn = 0.0,
            || {
                let (empty, p) = kicked(src, false);
                assert_eq!(empty.world.particle(p).unwrap().vel[0], 0.0);
                assert_eq!(empty.spent.kick, 0.0);
            },
        );
    }

    #[test]
    fn costs_more_the_faster_it_already_goes_the_same_way() {
        // Forward 0.05 m/tick, once, from standing and from 0.3 m/tick.
        let cost = |v: f64| {
            let (mut sim, p) = kicked(
                "CMP n4, #4
        JNE   .done
        LDI   n5, #0.05
        LDI   n6, #0
        LDI   n7, #0
        KICK  n5:7
.done:  RET",
                true,
            );
            let q = sim.world.particle_mut(p).unwrap();
            q.vel = [v, q.vel[1], 0.0];
            let m = mass_of(q);
            sim.world.impulse.outside[0] += m * v;
            let before = sim.spent.kick;
            sim.run(2);
            sim.spent.kick - before
        };
        let still = cost(0.0);
        let moving = cost(0.3);
        assert!(still > 0.0);
        assert!(moving > still * 5.0);
    }
}

mod the_second_flaw {
    use super::*;

    #[test]
    fn spreads_an_order_through_another_casters_resting_mana_and_says_so() {
        let world = World::with_ground(32, 24, 1, 8, EARTH);
        let mut sim = Sim::new(world);
        let lay = |sim: &mut Sim, name: &str, order: bool| {
            let x = if name == "A" { 1.0 } else { 1.5 };
            let caster = sim.add_caster(name, [x, 2.9, 0.125], None);
            sim.casters[caster].will = Will { aim: [3.0, 2.9, 0.125], amount: 100.0, force: 0.0, maintain: false };
            let ingrain = if order {
                "        ORDR  n3, still\n        INGR  n3, #0\n        INGR  n3, #1\n        INGR  n3, #2\n"
            } else {
                ""
            };
            let src = format!(
                "       .use  Elements
        GATH  m0, #100
        FILT  m1, m0, #EARTH
        LDI   n0, #3
        LDI   n1, #2.1
        LDI   n2, #0.125
        WEAV  n3, n0:2
        LDI   n4, #0
        LDI   n5, #0
        LDI   n6, #0
        LDI   n7, #{}
        EMIT  m1, n7, n3, n4:6
{ingrain}        MANI  n3
        HALT
still:  RET",
                if order { 0.75 } else { 0.25 }
            );
            let program = Code::new(assemble(&src, &format!("{name}.masm"), &resolver(vec![])).unwrap());
            sim.cast(caster, program, None).unwrap();
            sim.step();
        };
        // A's three motes settle on the ground and merge into one particle of 0.75 M, ordered. Then B's one mote lands
        // beside it.
        lay(&mut sim, "A", true);
        sim.run(30);
        lay(&mut sim, "B", false);
        let b = sim.weaves.values().find(|w| sim.casters[w.maker].name == "B").unwrap().id;
        let before = sim.weaves[&b].mana(&sim.world);
        sim.run(60);
        let taken: Vec<_> = sim.events.iter().filter(|e| e.kind == EventKind::Taken).collect();
        assert!(!taken.is_empty());
        let d = taken[0].detail.as_ref().unwrap();
        assert!(d.starts_with("took in weave ") && d.ends_with("'s mana, and gave it its order"), "{d}");
        let after = sim.weave(b).map(|w| w.mana(&sim.world)).unwrap_or(0.0);
        assert!(after < before);
    }
}
