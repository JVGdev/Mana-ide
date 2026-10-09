//! Port check (PLAN step R3): the world without minds steps as the TypeScript engine's did, to the last bit. The
//! scenarios are engine/parity/world.ts's.

mod parity;

use mana::js;
use mana::vm::air::step_air;
use mana::vm::energy::stored;
use mana::vm::fluid::{FluidHooks, merge_and_split, step_fluid};
use mana::vm::parts::Parts;
use mana::vm::physics::{Physics, physics, tuned};
use mana::vm::world::{Bond, EARTH, Vec3, WATER, World};

fn parts(k: usize, m: f64) -> Parts {
    let mut p = [0.0; 4];
    p[k] = m;
    p
}

fn ground(air: bool) -> World {
    let mut w = World::with_ground(40, 24, 1, 8, EARTH);
    if !air {
        for a in w.air.iter_mut() {
            *a = [0.0; 4];
        }
    }
    w
}

fn drop(w: &mut World, at: Vec3, k: usize, n: f64, held: f64) -> Vec<u64> {
    let ids = w.pour_still(&mut parts(k, physics().mote * n), at, 1);
    for &id in &ids {
        w.particle_mut(id).unwrap().carried[k] = held;
    }
    ids
}

fn column(w: &mut World) {
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
            let (a, b) = (w.particle(ps[i]).unwrap().pos, w.particle(ps[j]).unwrap().pos);
            let d = js::hypot2(a[0] - b[0], a[1] - b[1]);
            if d < physics().bond_range {
                w.bonds.push(Bond { a: ps[i], b: ps[j], rest: d });
            }
        }
    }
}

fn blow(w: &mut World, x0: i64, x1: i64, y0: i64, y1: i64, u: f64) {
    for y in y0..y1 {
        for x in x0..x1 {
            let i = w.index(x, y, 0) as usize;
            w.air_vel[i * 3] = u;
            w.impulse.outside[0] += w.air_mass(i) * u;
        }
    }
}

fn fluid(w: &mut World) {
    step_fluid(w, &FluidHooks::default());
    w.tick += 1;
}

fn everything(w: &mut World) {
    step_fluid(w, &FluidHooks::default());
    step_air(w);
    merge_and_split(w, &FluidHooks::default());
    w.tick += 1;
}

fn check(name: &str, tune: impl Fn(&mut Physics), world: impl Fn() -> World, step: impl Fn(&mut World)) {
    let Some(want) = parity::load(&format!("world-{name}")) else { return };
    let want = want.as_array().unwrap();
    tuned(tune, || {
        let mut w = world();
        for (t, want) in want.iter().enumerate() {
            if t > 0 {
                step(&mut w);
            }
            let mut got = parity::state(&w);
            let s = stored(&mut w, 0.0);
            got["stored"] = parity::stored_json(&s);
            if let Some(d) = parity::differ(want, &got, &format!("{name}, tick {t}")) {
                panic!("{d}");
            }
        }
        eprintln!("{name}: {} ticks, the same", want.len() - 1);
    });
}

fn none(_: &mut Physics) {}

#[test]
fn weight() {
    check(
        "weight",
        none,
        || {
            let mut w = ground(false);
            drop(&mut w, [1.0, 5.0, 0.125], WATER, 1.0, 0.0);
            drop(&mut w, [3.0, 5.0, 0.125], EARTH, 1.0, 2.0);
            drop(&mut w, [5.0, 5.0, 0.125], 0, 1.0, 0.0);
            drop(&mut w, [7.0, 5.0, 0.125], 2, 1.0, 0.0);
            w
        },
        fluid,
    );
}

#[test]
fn buoyancy() {
    check(
        "buoyancy",
        none,
        || {
            let mut w = ground(true);
            drop(&mut w, [1.0, 4.0, 0.125], 0, 1.0, 0.0);
            drop(&mut w, [3.0, 4.0, 0.125], 2, 1.0, 0.0);
            drop(&mut w, [5.0, 4.0, 0.125], WATER, 1.0, 0.0);
            drop(&mut w, [7.0, 4.0, 0.125], EARTH, 1.0, 0.0);
            drop(&mut w, [9.0, 4.0, 0.125], EARTH, 1.0, 4.0);
            w
        },
        fluid,
    );
}

#[test]
fn landing() {
    check(
        "landing",
        none,
        || {
            let mut w = ground(false);
            drop(&mut w, [2.0, 3.0, 0.125], EARTH, 1.0, 5.0);
            w
        },
        fluid,
    );
}

#[test]
fn friction() {
    check(
        "friction",
        none,
        || {
            let mut w = ground(false);
            let id = drop(&mut w, [2.0, 2.01, 0.125], EARTH, 1.0, 5.0)[0];
            w.particle_mut(id).unwrap().vel[0] = 0.05;
            w
        },
        fluid,
    );
}

#[test]
fn puddle() {
    check(
        "puddle",
        none,
        || {
            let mut w = ground(false);
            drop(&mut w, [2.0, 3.2, 0.125], WATER, 12.0, 4.0);
            w
        },
        fluid,
    );
}

#[test]
fn cohesion() {
    check(
        "cohesion",
        |p| p.gravity = 0.0,
        || {
            let mut w = World::with_ground(40, 24, 1, 0, EARTH);
            for a in w.air.iter_mut() {
                *a = [0.0; 4];
            }
            drop(&mut w, [2.0, 3.0, 0.125], WATER, 1.0, 4.0);
            drop(&mut w, [2.18, 3.0, 0.125], WATER, 1.0, 4.0);
            w
        },
        fluid,
    );
}

#[test]
fn column_stands() {
    check(
        "column",
        none,
        || {
            let mut w = ground(false);
            column(&mut w);
            w
        },
        fluid,
    );
}

#[test]
fn column_cracks() {
    check(
        "cracks",
        |p| p.bond_strength = 0.001,
        || {
            let mut w = ground(false);
            column(&mut w);
            w
        },
        fluid,
    );
}

#[test]
fn merging() {
    check(
        "merging",
        none,
        || {
            let mut w = ground(false);
            let a = drop(&mut w, [2.0, 3.0, 0.125], WATER, 1.0, 2.0)[0];
            let b = drop(&mut w, [2.05, 3.0, 0.125], WATER, 1.0, 1.0)[0];
            let pa = w.particle_mut(a).unwrap();
            pa.pos = [2.0, 3.0, 0.125];
            pa.vel = [0.001, 0.0, 0.0];
            let pb = w.particle_mut(b).unwrap();
            pb.pos = [2.05, 3.0, 0.125];
            pb.vel = [0.003, 0.0, 0.0];
            let p = drop(&mut w, [6.0, 4.0, 0.125], WATER, 1.0, 0.0)[0];
            w.particle_mut(p).unwrap().free[WATER] = 1.0;
            w
        },
        |w| {
            merge_and_split(w, &FluidHooks::default());
            w.tick += 1;
        },
    );
}

#[test]
fn wind() {
    check(
        "wind",
        none,
        || {
            let mut w = World::with_ground(48, 24, 1, 8, EARTH);
            blow(&mut w, 4, 8, 12, 16, 0.3);
            w
        },
        step_air,
    );
}

#[test]
fn pillar() {
    check(
        "pillar",
        none,
        || {
            let mut w = World::with_ground(48, 24, 1, 8, EARTH);
            let full = mana::vm::world::ground_amount(EARTH);
            w.fill_box([24, 8, 0], [26, 13, 0], EARTH, full);
            blow(&mut w, 8, 20, 8, 20, 0.2);
            w
        },
        step_air,
    );
}

#[test]
fn hole() {
    check(
        "hole",
        none,
        || {
            let mut w = World::with_ground(48, 24, 1, 8, EARTH);
            for y in 12..16 {
                for x in 20..24 {
                    let i = w.index(x, y, 0) as usize;
                    w.air[i] = [0.0; 4];
                }
            }
            w
        },
        step_air,
    );
}

#[test]
fn drag() {
    check(
        "drag",
        none,
        || {
            let mut w = ground(true);
            let id = w.pour_still(&mut [0.0, 0.0, physics().mote, 0.0], [3.0, 4.0, 0.125], 1)[0];
            w.particle_mut(id).unwrap().vel = [0.2, 0.0, 0.0];
            w
        },
        fluid,
    );
}

#[test]
fn fireball() {
    check(
        "fireball",
        none,
        || {
            let mut w = World::with_ground(64, 32, 1, 8, EARTH);
            let full = mana::vm::world::ground_amount(EARTH);
            w.fill_box([44, 8, 0], [47, 20, 0], EARTH, full);
            let pour = physics().pour;
            w.pour(&mut [18.0, 0.0, 0.0, 0.0], [3.1, 3.1, 0.125], 0, [0.3, 0.02, 0.0], pour);
            w.pour(&mut [0.0, 6.0, 0.0, 2.0], [5.0, 3.0, 0.125], 0, [0.0, 0.0, 0.0], pour);
            w.add_person("Target", [6.0, 2.9, 0.125]);
            w
        },
        everything,
    );
}

#[test]
fn fireball_3d() {
    check(
        "fireball3d",
        none,
        || {
            let mut w = World::with_ground(24, 16, 24, 8, EARTH);
            let pour = physics().pour;
            w.pour(&mut [18.0, 0.0, 0.0, 0.0], [2.1, 3.1, 3.1], 0, [0.2, 0.01, 0.05], pour);
            w.add_person("Target", [4.0, 2.9, 3.0]);
            w
        },
        everything,
    );
}
