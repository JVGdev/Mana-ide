//! Port check (PLAN step R): the world's state, written down the way engine/parity/state.ts writes the TypeScript
//! engine's, and compared with it tick by tick, to the last bit.

#![allow(dead_code)]

use std::path::PathBuf;

use mana::vm::energy::Stored;
use mana::vm::world::World;
use serde_json::{Value, json};

/// FNV-1a over 32-bit words of each number's exact bits.
pub fn hash(xs: impl IntoIterator<Item = f64>) -> String {
    let mut h: u32 = 2166136261;
    for x in xs {
        let b = x.to_bits();
        h = (h ^ (b as u32)).wrapping_mul(16777619);
        h = (h ^ ((b >> 32) as u32)).wrapping_mul(16777619);
    }
    format!("{h:08x}")
}

pub fn state(w: &World) -> Value {
    let particles: Vec<Vec<f64>> = w
        .particles
        .iter()
        .map(|p| {
            let mut v = vec![p.id as f64];
            v.extend(p.pos);
            v.extend(p.vel);
            v.extend(p.free);
            v.extend(p.carried);
            v.push(p.weave as f64);
            v.push(p.order.as_ref().map(|o| o.addr as f64).unwrap_or(-1.0));
            v.extend([p.yaw, p.pushed_at, p.touched_at, p.rho, p.felt]);
            v.extend(p.grad);
            v.extend(p.nvel);
            v.extend([p.rho_m, p.mass]);
            v.extend(p.regs.iter().copied());
            v.extend(p.stamp.iter().copied());
            v
        })
        .collect();
    let bonds: Vec<[f64; 3]> = w.bonds.iter().map(|b| [b.a as f64, b.b as f64, b.rest]).collect();
    let bodies: Vec<Vec<f64>> = w
        .bodies
        .iter()
        .map(|b| {
            let mut v = vec![b.id as f64];
            v.extend(b.pos);
            v.extend(b.vel);
            v.push(b.mass);
            v
        })
        .collect();
    let i = &w.impulse;
    let impulse: Vec<f64> = i.gravity.iter().chain(i.walls.iter()).chain(i.outside.iter()).copied().collect();
    let heat: Vec<(String, f64)> = w.heat.iter().map(|(k, v)| (k.clone(), *v)).collect();
    json!({
        "tick": w.tick,
        "particles": particles,
        "bonds": bonds,
        "bodies": bodies,
        "impulse": impulse,
        "heat": heat,
        "air": hash(w.air.iter().flatten().copied()),
        "airVel": hash(w.air_vel.iter().copied()),
        "matter": hash(w.matter.iter().flatten().copied()),
    })
}

pub fn stored_json(s: &Stored) -> Value {
    json!({ "motion": s.motion, "height": s.height, "gas": s.gas, "packing": s.packing, "cohesion": s.cohesion, "air": s.air, "bodies": s.bodies })
}

/// What the TypeScript engine wrote down, if it has: run `npm run parity`.
pub fn load(name: &str) -> Option<Vec<Value>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/parity").join(format!("{name}.json"));
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(serde_json::from_str(&text).unwrap()),
        Err(_) => {
            eprintln!("no {}: run `npm run parity` first", path.display());
            None
        }
    }
}

/// Where two states first differ, as a path into them, or None if they're the same to the last bit. Numbers are the
/// same if their bits are (and every NaN is the same NaN); JSON writes −0 as 0, so zeros are equal whatever their sign.
pub fn differ(want: &Value, got: &Value, at: &str) -> Option<String> {
    match (want, got) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if a.to_bits() == b.to_bits() || (a == 0.0 && b == 0.0) {
                None
            } else {
                Some(format!("{at}: TypeScript {a:e}, Rust {b:e} (Δ {:e})", b - a))
            }
        }
        // JSON can't hold NaN or ±Infinity: both engines write them as null.
        (Value::Null, Value::Number(b)) if !b.as_f64().is_some_and(f64::is_finite) => None,
        (Value::Null, Value::Null) => None,
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{at}: TypeScript has {} items, Rust {}", a.len(), b.len()));
            }
            a.iter().zip(b).enumerate().find_map(|(i, (x, y))| differ(x, y, &format!("{at}[{i}]")))
        }
        (Value::Object(a), Value::Object(b)) => {
            for (k, x) in a {
                let Some(y) = b.get(k) else { return Some(format!("{at}.{k}: missing in Rust")) };
                if let Some(d) = differ(x, y, &format!("{at}.{k}")) {
                    return Some(d);
                }
            }
            None
        }
        (a, b) if a == b => None,
        (a, b) => Some(format!("{at}: TypeScript {a}, Rust {b}")),
    }
}
