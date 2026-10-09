//! Port check (PLAN step R4): the four spells cast in their scenes as the TypeScript engine cast them, to the last bit.
//! The scenarios are engine/parity/machine.ts's.

mod parity;

use mana::asm::Code;
use mana::load::spell;
use mana::scenes::scene;
use mana::vm::physics::{Physics, tune, tuned};
use mana::vm::sim::Sim;
use serde_json::{Value, json};

fn machine(sim: &Sim, full: bool) -> Value {
    let mut v = parity::state_of(&sim.world, full);
    let casters: Vec<Value> = sim
        .casters
        .iter()
        .map(|c| {
            let mut row = vec![json!(c.name), json!(c.body)];
            row.extend(c.flow.iter().map(|x| json!(x)));
            for r in &c.regs {
                row.extend(r.parts.iter().map(|x| json!(x)));
                row.push(json!(r.hold_until));
            }
            row.extend([c.harm, c.strain, c.madness, c.power_left, c.condition.body, c.condition.mind].map(|x| json!(x)));
            json!(row)
        })
        .collect();
    let casts: Vec<Value> = sim
        .casts
        .iter()
        .map(|c| {
            let mut row = vec![json!(c.state.as_str()), json!(c.frame.pc)];
            row.extend(c.frame.n.iter().map(|x| json!(x)));
            row.extend([
                json!(c.frame.flags),
                json!(c.left),
                json!(c.beats),
                json!(c.result.map(|r| r as f64).unwrap_or(-1.0)),
                json!(c.code.unwrap_or(-1.0)),
                json!(c.fault.clone().unwrap_or_default()),
                json!(c.ended_at.unwrap_or(-1)),
                json!(c.frame.stack),
                json!(c.frame.calls),
                json!(c.yielded),
            ]);
            json!(row)
        })
        .collect();
    let weaves: Vec<Value> = sim
        .weaves
        .values()
        .map(|w| {
            let mut row = vec![json!(w.id), json!(w.particles)];
            row.extend(w.origin.iter().map(|x| json!(x)));
            row.extend([json!(w.yaw), json!(w.in_hand)]);
            row.extend(w.regs.iter().map(|x| json!(x)));
            row.extend(w.stamp.iter().map(|x| json!(x)));
            row.extend([
                json!(w.order.map(|o| o as f64).unwrap_or(-1.0)),
                json!(w.manifested_at),
                json!(w.last_left),
                json!(w.locks.input),
                json!(w.locks.order),
            ]);
            json!(row)
        })
        .collect();
    let events: Vec<Value> = sim
        .events
        .iter()
        .map(|e| {
            json!([
                e.tick,
                e.kind.as_str(),
                e.caster.clone().unwrap_or_default(),
                e.weave.map(|w| w as f64).unwrap_or(-1.0),
                e.detail.clone().unwrap_or_default()
            ])
        })
        .collect();
    let l = sim.ledger();
    let traces: Vec<Value> = if sim.trace_orders {
        sim.traces
            .iter()
            .map(|(id, ts)| {
                let rows: Vec<Value> = ts
                    .iter()
                    .map(|t| {
                        let steps: Vec<Value> = t
                            .steps
                            .iter()
                            .map(|s| {
                                let mut r = vec![s.addr as f64];
                                r.extend(s.n.iter());
                                json!(r)
                            })
                            .collect();
                        json!([
                            t.tick, t.weave, t.particle, t.id, t.off[0], t.off[1], t.off[2], t.w, steps, t.n, t.outcome, t.beats,
                            t.burned, t.kick[0], t.kick[1], t.kick[2], t.cnds
                        ])
                    })
                    .collect();
                json!([id, rows])
            })
            .collect()
    } else {
        vec![]
    };
    let e = &sim.energy;
    let energy = if sim.track_energy {
        let errors: Vec<(String, f64)> = e.error.iter().map(|(k, v)| (k.clone(), *v)).collect();
        json!([e.start, e.outside, e.minds, e.bodies, errors])
    } else {
        json!([])
    };
    let o = v.as_object_mut().unwrap();
    o.insert("casters".into(), json!(casters));
    o.insert("casts".into(), json!(casts));
    o.insert("weaves".into(), json!(weaves));
    o.insert("events".into(), json!(events));
    o.insert("spent".into(), json!([sim.spent.push, sim.spent.kick, sim.spent.burn, sim.transformed, sim.strayed]));
    o.insert(
        "ledger".into(),
        json!({ "air": l.air, "matter": l.matter, "loose": l.loose, "weaves": l.weaves, "carried": l.carried, "casters": l.casters,
                "free": l.free, "condensed": l.condensed, "total": l.total }),
    );
    o.insert("energy".into(), energy);
    o.insert("traces".into(), json!(traces));
    v
}

struct Scenario {
    name: &'static str,
    scene: &'static str,
    dims: u8,
    ticks: usize,
    energy: bool,
    trace: bool,
    full: bool,
    then: Option<(usize, fn(&mut Physics))>,
}

const BASE: Scenario = Scenario { name: "", scene: "", dims: 2, ticks: 0, energy: false, trace: false, full: true, then: None };

fn check(s: Scenario) {
    let Some(want) = parity::load(&format!("machine-{}", s.name)) else { return };
    let full = std::env::var("MANA_PARITY_FULL").is_ok() || s.full;
    tuned(
        |_| {},
        || {
            let mut sc = scene(s.scene, s.dims).unwrap();
            let sim = &mut sc.sim;
            if s.trace {
                sim.trace_orders = true;
            }
            if s.energy {
                sim.keep_energy();
            }
            sim.cast(sc.caster, Code::new(spell(s.scene)), None).unwrap();
            let ticks = want["ticks"].as_array().unwrap();
            let first = s.ticks;
            for (t, w) in ticks.iter().enumerate() {
                if t > 0 {
                    if t == first + 1
                        && let Some((_, f)) = s.then
                    {
                        tune(f);
                    }
                    sim.step();
                }
                let got = machine(sim, full);
                if let Some(d) = parity::differ(w, &got, &format!("{}, tick {t}", s.name)) {
                    panic!("{d}");
                }
            }
            let profile: Vec<Vec<[f64; 3]>> =
                sim.casts.iter().map(|c| c.profile.iter().map(|(a, p)| [*a as f64, p.runs, p.beats]).collect()).collect();
            if let Some(d) = parity::differ(&want["profile"], &json!(profile), &format!("{}, profile", s.name)) {
                panic!("{d}");
            }
            eprintln!("{}: {} ticks, the same", s.name, ticks.len() - 1);
        },
    );
}

#[test]
fn fireball() {
    check(Scenario { name: "fireball", scene: "Fireball", ticks: 70, ..BASE });
}

#[test]
fn fireball_with_the_energy_ledger_and_orders_traced() {
    check(Scenario { name: "fireball-energy", scene: "Fireball", ticks: 70, energy: true, trace: true, ..BASE });
}

#[test]
fn fireball_3d() {
    check(Scenario { name: "fireball3d", scene: "Fireball", dims: 3, ticks: 45, ..BASE });
}

#[test]
fn gust() {
    check(Scenario { name: "gust", scene: "Gust", ticks: 40, energy: true, ..BASE });
}

#[test]
fn water_shield() {
    check(Scenario { name: "watershield", scene: "WaterShield", ticks: 90, energy: true, ..BASE });
}

#[test]
fn water_shield_3d() {
    check(Scenario { name: "watershield3d", scene: "WaterShield", dims: 3, ticks: 30, ..BASE });
}

#[test]
fn stone_wall_and_its_crumbling() {
    check(Scenario {
        name: "stonewall",
        scene: "StoneWall",
        ticks: 230,
        full: false,
        then: Some((60, |p| p.order_burn = 0.002)),
        ..BASE
    });
}

#[test]
fn stone_wall_with_the_energy_ledger() {
    check(Scenario { name: "stonewall-energy", scene: "StoneWall", ticks: 120, full: false, energy: true, ..BASE });
}
