//! Port check (PLAN step R5): the bench measures every variant as the TypeScript bench did, to the last bit. The runs
//! are engine/parity/bench.ts's.

mod parity;

use mana::bench::{order_size, run_variant, variant};
use serde_json::json;

#[test]
fn every_variant_measures_as_it_did() {
    let Some(want) = parity::load("bench") else { return };
    let want = want.as_array().unwrap();
    for w in want {
        let v = variant(w["variant"].as_str().unwrap());
        let r = run_variant(
            &v,
            w["caster"].as_str().unwrap(),
            w["dims"].as_u64().unwrap() as u8,
            w["layout"].as_u64().unwrap() as usize,
            None,
        );
        let got = json!({
            "variant": r.variant, "caster": r.caster, "dims": r.dims, "layout": r.layout, "ingrain": r.ingrain, "particles": r.particles,
            "mana": r.mana, "arrived": r.arrived, "ticks": r.ticks, "together": r.together, "kept": r.kept, "spread": r.spread,
            "push": r.push, "kick": r.kick, "burn": r.burn, "handBeats": r.hand_beats, "ms": 0, "order": order_size(&v),
        });
        let at = format!("{} {} {}D layout {}", r.variant, r.caster, r.dims, r.layout);
        if let Some(d) = parity::differ(w, &got, &at) {
            panic!("{d}");
        }
    }
    eprintln!("{} bench runs, the same", want.len());
}
