//! The bench (mana::bench): every way of holding and throwing runs on the machine, keeps its ledgers, and repeats (from
//! test/bench.test.ts).

use mana::bench::{BenchResult, VARIANTS, run_variant, variant};

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

/// A variant keeps mana and momentum, every tick.
fn keeps_mana_and_momentum(id: &str) {
    let mut before: Option<f64> = None;
    let mut each = |s: &mana::scenes::Scene| {
        let total = s.sim.ledger().total;
        let b = *before.get_or_insert(total);
        assert!(close(total, b, 4));
        assert!(s.sim.world.momentum_error() < 1e-6);
    };
    let r = run_variant(&variant(id), "adept", 2, 2, Some(&mut each));
    assert!(r.particles > 0.0);
}

macro_rules! keeps {
    ($($name:ident: $id:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                keeps_mana_and_momentum($id);
            }
        )*
        #[test]
        fn every_variant_is_checked() {
            assert_eq!([$($id),*].len(), VARIANTS.len());
        }
    };
}

keeps! {
    hold_nothing_keeps_mana_and_momentum_every_tick: "HoldNothing",
    hold_every_keeps_mana_and_momentum_every_tick: "HoldEvery",
    hold_other_keeps_mana_and_momentum_every_tick: "HoldOther",
    hold_surface_keeps_mana_and_momentum_every_tick: "HoldSurface",
    hold_cling_keeps_mana_and_momentum_every_tick: "HoldCling",
    hold_cohere_keeps_mana_and_momentum_every_tick: "HoldCohere",
    throw_hand_keeps_mana_and_momentum_every_tick: "ThrowHand",
    throw_hand_held_keeps_mana_and_momentum_every_tick: "ThrowHandHeld",
    throw_cling_keeps_mana_and_momentum_every_tick: "ThrowCling",
    throw_cohere_keeps_mana_and_momentum_every_tick: "ThrowCohere",
    throw_heading_keeps_mana_and_momentum_every_tick: "ThrowHeading",
    throw_steer_keeps_mana_and_momentum_every_tick: "ThrowSteer",
    throw_steer_cling_keeps_mana_and_momentum_every_tick: "ThrowSteerCling",
}

#[test]
fn repeats_exactly() {
    let a = run_variant(&variant("ThrowCling"), "adept", 2, 1, None);
    let b = run_variant(&variant("ThrowCling"), "adept", 2, 1, None);
    assert_eq!(BenchResult { ms: 0.0, ..a }, BenchResult { ms: 0.0, ..b });
}

#[test]
fn an_order_holds_a_still_ball_better_than_nothing_does() {
    let nothing = run_variant(&variant("HoldNothing"), "adept", 2, 2, None);
    let cling = run_variant(&variant("HoldCling"), "adept", 2, 2, None);
    assert!(nothing.spread > cling.spread * 1.5);
    assert!(cling.spread < 0.5);
    assert!(cling.burn > 0.0); // and pays for it from the ball
}

#[test]
fn an_order_told_only_an_angle_flies_but_nothing_holds_it_together() {
    let r = run_variant(&variant("ThrowHeading"), "adept", 2, 2, None);
    assert!(r.ingrain > 0.0);
    assert!(r.together < 0.5); // what's left of the ball when it gets there, if it does, is a scatter
}

#[test]
fn an_order_that_feels_its_neighbours_holds_a_thrown_ball_together_all_the_way() {
    let r = run_variant(&variant("ThrowCohere"), "master", 2, 2, None);
    assert!(r.arrived);
    assert!(r.together > 0.85);
}
