//! The engine, as the spell tester sees it (PLAN step R6): the assembler, the instruction table, and a `Machine` holding
//! a scene and the spell cast in it. What the tester shows comes back as JSON, and the world as arrays of numbers.

use std::collections::HashSet;
use std::sync::Arc;

use mana::asm::docs::{OP_DOCS, PORT_DOCS};
use mana::asm::isa::{Kind, LOCK_NAMES, OPS, PORTS};
use mana::asm::{AsmError, Code, Program, assemble as asm, format};
use mana::js;
use mana::profile::profile;
use mana::scenes::{Scene, scene};
use mana::vm::caster::{CasterStats, Will};
use mana::vm::energy::JOULES;
use mana::vm::physics::physics;
use mana::vm::sim::{CastState, OrderTrace};
use mana::vm::world::{fill_of, mass_of};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

/// The engine's version: for the tester to check it loaded.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn kind(k: Kind) -> &'static str {
    match k {
        Kind::N => "n",
        Kind::M => "m",
        Kind::S => "s",
        Kind::T => "t",
        Kind::A => "a",
        Kind::L => "L",
        Kind::P => "P",
        Kind::K => "K",
        Kind::I => "I",
    }
}

/// The instruction set, the ports and what each does: for the editor and the reference.
#[wasm_bindgen]
pub fn isa() -> String {
    let ops: Vec<Value> = OPS
        .iter()
        .map(|o| json!({ "code": o.code, "name": o.name, "operands": o.operands.iter().map(|k| kind(*k)).collect::<Vec<_>>(), "beats": o.beats, "order": o.order }))
        .collect();
    let ports: Vec<Value> =
        PORTS.iter().map(|p| json!({ "name": p.name, "code": p.code, "size": p.size, "order": p.order })).collect();
    let mut op_docs = serde_json::Map::new();
    for (name, d) in &OP_DOCS {
        let mut doc = json!({ "syntax": d.syntax, "group": d.group, "doc": d.doc });
        if let Some(step) = d.step {
            doc["step"] = json!(step);
        }
        op_docs.insert(name.to_string(), doc);
    }
    let port_docs: serde_json::Map<String, Value> = PORT_DOCS.iter().map(|(n, d)| (n.to_string(), json!(d))).collect();
    json!({ "ops": ops, "ports": ports, "locks": LOCK_NAMES, "opDocs": op_docs, "portDocs": port_docs }).to_string()
}

/// The numbers the world runs on, as they are now.
#[wasm_bindgen]
pub fn physics_table() -> String {
    let p = physics();
    json!({
        "cell": p.cell, "density": p.density, "airMana": p.air_mana, "bind": p.bind, "solid": p.solid, "mote": p.mote,
        "manaMass": p.mana_mass, "orderBudget": p.order_budget, "orderRegisters": p.order_registers,
        "weaveRegisters": p.weave_registers, "joules": JOULES,
    })
    .to_string()
}

fn resolver(libraries: &js_sys::Function) -> impl Fn(&str) -> Option<String> + '_ {
    move |name: &str| libraries.call1(&JsValue::NULL, &JsValue::from_str(name)).ok().and_then(|v| v.as_string())
}

fn program_json(p: &Program) -> Value {
    let lines: Vec<Value> = p.lines.iter().map(|(a, l)| json!([a, l.file, l.line, l.text])).collect();
    json!({
        "ok": true,
        "bytes": p.bytes,
        "labels": p.labels.iter().map(|(n, a)| json!([n, a])).collect::<Vec<_>>(),
        "consts": p.consts.iter().map(|(n, v)| json!([n, v])).collect::<Vec<_>>(),
        "lines": lines,
    })
}

fn problems_json(e: &AsmError) -> Value {
    json!({ "ok": false, "problems": e.problems })
}

/// Assembles a spell. `libraries(name)` gives a library's source, or undefined.
#[wasm_bindgen]
pub fn assemble(source: &str, file: &str, libraries: &js_sys::Function) -> String {
    match asm(source, file, &resolver(libraries)) {
        Ok(p) => program_json(&p),
        Err(e) => problems_json(&e),
    }
    .to_string()
}

/// Every instruction of some bytes: where it is, how long, what it costs, and as text (labels by address name jump
/// targets).
#[wasm_bindgen]
pub fn disassemble(bytes: &[u8], labels: &str) -> String {
    let names: std::collections::HashMap<usize, String> =
        serde_json::from_str::<Vec<(usize, String)>>(labels).unwrap_or_default().into_iter().collect();
    let mut out = Vec::new();
    let mut addr = 0;
    // As far as it decodes: a program the assembler made decodes to its end.
    while addr < bytes.len() {
        match mana::asm::decode(bytes, addr) {
            Ok(i) => {
                out.push(json!({ "addr": i.addr, "size": i.size, "beats": i.op.beats, "text": format(&i, Some(&names)) }));
                addr += i.size;
            }
            Err(_) => break,
        }
    }
    json!(out).to_string()
}

fn stats_json(s: &CasterStats) -> Value {
    let st = |s: mana::vm::caster::Stat| json!({ "genetics": s.genetics, "training": s.training });
    let b = &s.body;
    let m = &s.mind;
    json!({
        "body": { "capacity": st(b.capacity), "baseline": st(b.baseline), "drain": st(b.drain), "focus": st(b.focus),
                  "streams": st(b.streams), "reach": st(b.reach), "affinity": b.affinity.map(st) },
        "mind": { "speed": st(m.speed), "registers": st(m.registers), "memory": st(m.memory), "power": st(m.power),
                  "capacity": st(m.capacity), "recovery": st(m.recovery) },
    })
}

fn stats_from(v: &Value) -> Option<CasterStats> {
    let st = |v: &Value| -> Option<mana::vm::caster::Stat> {
        Some(mana::vm::caster::Stat { genetics: v["genetics"].as_f64()?, training: v["training"].as_f64()? })
    };
    let b = &v["body"];
    let m = &v["mind"];
    let aff = b["affinity"].as_array()?;
    Some(CasterStats {
        body: mana::vm::caster::BodyStats {
            capacity: st(&b["capacity"])?,
            baseline: st(&b["baseline"])?,
            drain: st(&b["drain"])?,
            focus: st(&b["focus"])?,
            streams: st(&b["streams"])?,
            reach: st(&b["reach"])?,
            affinity: [st(&aff[0])?, st(&aff[1])?, st(&aff[2])?, st(&aff[3])?],
        },
        mind: mana::vm::caster::MindStats {
            speed: st(&m["speed"])?,
            registers: st(&m["registers"])?,
            memory: st(&m["memory"])?,
            power: st(&m["power"])?,
            capacity: st(&m["capacity"])?,
            recovery: st(&m["recovery"])?,
        },
    })
}

/// A caster's stats as each preset has them: what the tester offers.
#[wasm_bindgen]
pub fn presets() -> String {
    use mana::vm::caster::{adept, child, master};
    json!({ "child": stats_json(&child()), "adept": stats_json(&adept()), "master": stats_json(&master()) }).to_string()
}

/// The scenes the tester can load, in order.
#[wasm_bindgen]
pub fn scene_names() -> String {
    json!(mana::scenes::SCENES.iter().map(|(n, _)| *n).collect::<Vec<_>>()).to_string()
}

fn trace_json(t: &OrderTrace) -> Value {
    json!({
        "tick": t.tick, "weave": t.weave, "particle": t.particle, "id": t.id, "off": t.off, "w": t.w,
        "steps": t.steps.iter().map(|s| json!({ "addr": s.addr, "n": s.n })).collect::<Vec<_>>(),
        "n": t.n, "outcome": t.outcome, "beats": t.beats, "burned": t.burned, "kick": t.kick, "cnds": t.cnds,
    })
}

/// A scene, and the spell cast in it.
#[wasm_bindgen]
pub struct Machine {
    scene: Scene,
    cast: Option<usize>,
    code: Option<Arc<Code>>,
}

#[wasm_bindgen]
impl Machine {
    /// A scene by name (Field, StoneWall, Fireball, Gust, WaterShield, Hold), in 2D or 3D.
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, dims: u8) -> Result<Machine, JsValue> {
        let s = scene(name, dims).ok_or_else(|| JsValue::from_str(&format!("no scene called {name}")))?;
        Ok(Machine { scene: s, cast: None, code: None })
    }

    /// The world's size, and the scene's caster: their will and stats as the scene set them.
    pub fn info(&self) -> String {
        let w = &self.scene.sim.world;
        let c = &self.scene.sim.casters[self.scene.caster];
        json!({
            "w": w.w, "h": w.h, "d": w.d, "cell": w.cell, "size": w.size,
            "will": { "aim": c.will.aim, "amount": c.will.amount, "force": c.will.force, "maintain": c.will.maintain },
            "stats": stats_json(&c.stats),
            "body": c.body,
            "pos": w.bodies[c.body].pos,
        })
        .to_string()
    }

    pub fn set_stats(&mut self, stats: &str) {
        if let Some(s) = serde_json::from_str::<Value>(stats).ok().as_ref().and_then(stats_from) {
            self.scene.sim.casters[self.scene.caster].stats = s;
        }
    }

    pub fn set_condition(&mut self, body: f64, mind: f64) {
        let c = &mut self.scene.sim.casters[self.scene.caster];
        c.condition.body = body;
        c.condition.mind = mind;
    }

    pub fn set_will(&mut self, x: f64, y: f64, z: f64, amount: f64, force: f64, maintain: bool) {
        self.scene.sim.casters[self.scene.caster].will = Will { aim: [x, y, z], amount, force, maintain };
    }

    pub fn trace_orders(&mut self, on: bool) {
        self.scene.sim.trace_orders = on;
    }

    /// Keeps the Energy ledger from now on, or stops keeping it.
    pub fn keep_energy(&mut self, on: bool) {
        if on && !self.scene.sim.track_energy {
            self.scene.sim.keep_energy();
        }
        if !on {
            self.scene.sim.track_energy = false;
        }
    }

    pub fn track_energy(&self) -> bool {
        self.scene.sim.track_energy
    }

    /// Casts a spell, practised `practice` times before. Returns the program, or the problems assembling it.
    pub fn cast(&mut self, source: &str, file: &str, libraries: &js_sys::Function, practice: f64) -> String {
        match asm(source, file, &resolver(libraries)) {
            Err(e) => problems_json(&e).to_string(),
            Ok(p) => {
                let out = program_json(&p);
                let code = Code::new(p);
                let name =
                    code.program.labels.iter().find(|(_, a)| **a == 0).map(|(n, _)| n.clone()).unwrap_or_else(|| "spell".into());
                if practice > 0.0 {
                    self.scene.sim.casters[self.scene.caster].conditioning.insert(name, practice);
                }
                self.cast = self.scene.sim.cast(self.scene.caster, code.clone(), None).ok();
                self.code = Some(code);
                out.to_string()
            }
        }
    }

    pub fn tick(&self) -> f64 {
        self.scene.sim.tick() as f64
    }

    pub fn mid_tick(&self) -> bool {
        self.scene.sim.mid_tick
    }

    pub fn step(&mut self) {
        self.scene.sim.step();
    }

    /// One instruction of the cast's mind, or a tick if it has ended.
    pub fn step_instruction(&mut self) {
        match self.cast {
            Some(c) if self.scene.sim.casts[c].state == CastState::Running => {
                self.scene.sim.step_instruction(c);
            }
            _ => self.scene.sim.step(),
        }
    }

    /// Runs the cast's mind up to `ticks` ticks, stopping before any instruction at `breaks`. True if it stopped there.
    pub fn run_until(&mut self, breaks: &[u32], ticks: u32) -> bool {
        let Some(c) = self.cast else { return false };
        let at: HashSet<usize> = breaks.iter().map(|&a| a as usize).collect();
        self.scene.sim.run_until(c, &mut |a| at.contains(&a), ticks as usize)
    }

    pub fn running(&self) -> bool {
        self.cast.is_some_and(|c| self.scene.sim.casts[c].state == CastState::Running)
    }

    /// The cast's mind: its state, where it is, its registers, stack, calls and memory, its beats and where they went.
    pub fn cast_view(&self) -> String {
        let Some(ci) = self.cast else { return "null".into() };
        let sim = &self.scene.sim;
        let c = &sim.casts[ci];
        let caster = &sim.casters[c.caster];
        let f = &c.frame;
        let per_tick = caster.speed() * (1.0 + caster.conditioning.get(&c.name).copied().unwrap_or(0.0));
        json!({
            "state": c.state.as_str(), "fault": c.fault, "code": c.code, "name": c.name, "pc": f.pc, "n": f.n.to_vec(),
            "limit": f.limit, "flags": f.flags, "stack": f.stack, "calls": f.calls, "memory": f.memory,
            "left": c.left, "perTick": per_tick, "beats": c.beats, "startedAt": c.started_at, "endedAt": c.ended_at,
            "profile": c.profile.iter().map(|(a, p)| json!([a, p.runs, p.beats])).collect::<Vec<_>>(),
            "regs": caster.regs.iter().map(|r| json!({ "parts": r.parts, "holdUntil": r.hold_until })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Where the cast's thought went, by routine and by line.
    pub fn profile_view(&self) -> String {
        let Some(ci) = self.cast else { return "null".into() };
        let p = profile(&self.scene.sim.casts[ci]);
        let src =
            |s: &Option<mana::asm::SourceLine>| s.as_ref().map(|l| json!({ "file": l.file, "line": l.line, "text": l.text }));
        json!({
            "beats": p.beats,
            "routines": p.routines.iter().map(|r| json!({ "name": r.name, "beats": r.beats, "share": r.share })).collect::<Vec<_>>(),
            "lines": p.lines.iter().map(|l| json!({ "addr": l.addr, "source": src(&l.source), "runs": l.runs, "beats": l.beats, "share": l.share })).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// The caster: their body and mind as they are now.
    pub fn caster_view(&self) -> String {
        let sim = &self.scene.sim;
        let ci = self.scene.caster;
        let c = &sim.casters[ci];
        let in_hand = sim.weaves.values().filter(|w| w.maker == ci && w.in_hand).fold(0.0, |s, w| s + w.mana(&sim.world));
        json!({
            "held": c.held(), "capacity": c.capacity(), "baseline": c.baseline(), "drain": c.drain(), "focus": c.focus(),
            "streams": c.streams(), "reach": c.reach(), "affinity": [c.affinity(0), c.affinity(1), c.affinity(2), c.affinity(3)],
            "speed": c.speed(), "registers": c.registers(), "memory": c.memory(), "power": c.power(),
            "mindCapacity": c.mind_capacity(), "recovery": c.recovery(),
            "flow": c.flow, "harm": c.harm, "strain": c.strain, "madness": c.madness,
            "condition": { "body": c.condition.body, "mind": c.condition.mind },
            "regs": c.regs.iter().map(|r| json!({ "parts": r.parts, "holdUntil": r.hold_until })).collect::<Vec<_>>(),
            "inHand": in_hand, "hand": c.hand(&sim.world), "body": c.body, "tick": sim.tick(),
        })
        .to_string()
    }

    /// Every weave: whose, where, what's in it, its locks and order, and its registers as its mana has them.
    pub fn weaves_view(&self) -> String {
        let sim = &self.scene.sim;
        let w = &sim.world;
        let out: Vec<Value> = sim
            .weaves
            .values()
            .map(|wv| {
                let mut free = [0.0; 4];
                let mut carried = [0.0; 4];
                let mut ingrained = 0;
                for p in wv.parts(w) {
                    for k in 0..4 {
                        free[k] += p.free[k];
                        carried[k] += p.carried[k];
                    }
                    if p.order.is_some() {
                        ingrained += 1;
                    }
                }
                let regs: Vec<f64> = (0..wv.regs.len()).map(|k| wv.reg(w, k)).collect();
                json!({
                    "id": wv.id, "maker": sim.casters[wv.maker].name, "inHand": wv.in_hand, "manifestedAt": wv.manifested_at,
                    "origin": wv.origin, "particles": wv.particles.len(), "ingrained": ingrained, "free": free, "carried": carried,
                    "locks": { "input": wv.locks.input, "order": wv.locks.order }, "order": wv.order, "regs": regs,
                })
            })
            .collect();
        json!(out).to_string()
    }

    /// The last events, newest last, and the mana ledger.
    pub fn events_view(&self, last: usize) -> String {
        let sim = &self.scene.sim;
        let from = sim.events.len().saturating_sub(last);
        let events: Vec<Value> = sim.events[from..]
            .iter()
            .map(|e| json!({ "tick": e.tick, "kind": e.kind.as_str(), "caster": e.caster, "weave": e.weave, "detail": e.detail }))
            .collect();
        json!({ "events": events, "ledger": self.ledger_json() }).to_string()
    }

    fn ledger_json(&self) -> Value {
        let l = self.scene.sim.ledger();
        json!({ "air": l.air, "matter": l.matter, "loose": l.loose, "weaves": l.weaves, "carried": l.carried, "casters": l.casters,
                "free": l.free, "condensed": l.condensed, "total": l.total })
    }

    pub fn ledger_total(&self) -> f64 {
        self.scene.sim.ledger().total
    }

    /// The Energy ledger now, and the heat by how it came about.
    pub fn energy_view(&mut self) -> String {
        if !self.scene.sim.track_energy {
            return "null".into();
        }
        let e = self.scene.sim.energy_now();
        let h = &e.held;
        json!({
            "held": { "motion": h.motion, "height": h.height, "gas": h.gas, "packing": h.packing, "cohesion": h.cohesion, "air": h.air, "bodies": h.bodies },
            "total": e.total, "heat": e.heat, "start": e.start, "outside": e.outside, "minds": e.minds, "bodies": e.bodies,
            "error": e.error.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(), "errorTotal": e.error_total,
            "heatBy": self.scene.sim.world.heat.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// Last tick's orders: which weaves ran, and how many particles of each.
    pub fn trace_counts(&self) -> String {
        json!(self.scene.sim.traces.iter().map(|(id, ts)| json!([id, ts.len()])).collect::<Vec<_>>()).to_string()
    }

    /// One particle's order, last tick, instruction by instruction.
    pub fn trace(&self, weave: u32, k: usize) -> String {
        match self.scene.sim.traces.get(&weave).and_then(|ts| ts.get(k)) {
            Some(t) => trace_json(t).to_string(),
            None => "null".into(),
        }
    }

    /// The first particle whose order ran an instruction at one of `breaks` last tick: its weave, which it was, and the
    /// step.
    pub fn order_hit(&self, breaks: &[u32]) -> String {
        let at: HashSet<usize> = breaks.iter().map(|&a| a as usize).collect();
        for (weave, traces) in &self.scene.sim.traces {
            for (k, t) in traces.iter().enumerate() {
                if let Some(step) = t.steps.iter().position(|s| at.contains(&s.addr)) {
                    return json!({ "weave": weave, "particle": k, "step": step }).to_string();
                }
            }
        }
        "null".into()
    }

    /// The particle nearest (x, y) on slice z whose order ran last tick: its weave, which it was, and how far.
    pub fn pick(&self, x: f64, y: f64, z: i32) -> String {
        let sim = &self.scene.sim;
        let w = &sim.world;
        let mut best: Option<(u32, usize, f64)> = None;
        for (id, traces) in &sim.traces {
            if !sim.weaves.contains_key(id) {
                continue;
            }
            for (k, t) in traces.iter().enumerate() {
                let Some(p) = w.particle(t.id) else { continue };
                if w.d > 1 && (p.pos[2] / w.cell).floor() as i64 != z as i64 {
                    continue;
                }
                let d = js::hypot2(p.pos[0] - x, p.pos[1] - y);
                if best.is_none_or(|b| d < b.2) {
                    best = Some((*id, k, d));
                }
            }
        }
        match best {
            Some((weave, k, d)) => json!({ "weave": weave, "k": k, "d": d }).to_string(),
            None => "null".into(),
        }
    }

    /// The air in slice z: four parts a cell, row by row.
    pub fn slice_air(&self, z: i32) -> Vec<f64> {
        let w = &self.scene.sim.world;
        let mut out = Vec::with_capacity(w.w * w.h * 4);
        for y in 0..w.h as i64 {
            for x in 0..w.w as i64 {
                out.extend(w.air[w.index(x, y, z as i64).max(0) as usize]);
            }
        }
        out
    }

    /// How the air in slice z moves: three numbers a cell.
    pub fn slice_air_vel(&self, z: i32) -> Vec<f64> {
        let w = &self.scene.sim.world;
        let mut out = Vec::with_capacity(w.w * w.h * 3);
        for y in 0..w.h as i64 {
            for x in 0..w.w as i64 {
                let i = w.index(x, y, z as i64).max(0) as usize;
                out.extend_from_slice(&w.air_vel[i * 3..i * 3 + 3]);
            }
        }
        out
    }

    /// The matter in slice z: four parts a cell, then the share of the cell it fills.
    pub fn slice_matter(&self, z: i32) -> Vec<f64> {
        let w = &self.scene.sim.world;
        let mut out = Vec::with_capacity(w.w * w.h * 5);
        for y in 0..w.h as i64 {
            for x in 0..w.w as i64 {
                let m = &w.matter[w.index(x, y, z as i64).max(0) as usize];
                out.extend(m);
                out.push(fill_of(m));
            }
        }
        out
    }

    /// Every particle, `PARTICLE` numbers each: id, position, velocity, free mana, matter held, weave, whether it carries
    /// an order, the tick it was last pushed, how dense the matter held around it is, and its mass.
    pub fn particles(&self) -> Vec<f64> {
        let w = &self.scene.sim.world;
        let mut out = Vec::with_capacity(w.particles.len() * 20);
        for p in &w.particles {
            out.push(p.id as f64);
            out.extend(p.pos);
            out.extend(p.vel);
            out.extend(p.free);
            out.extend(p.carried);
            out.push(p.weave as f64);
            out.push(if p.order.is_some() { 1.0 } else { 0.0 });
            out.push(p.pushed_at);
            out.push(p.rho_m);
            out.push(mass_of(p));
        }
        out
    }

    /// Every bond of rock: where its two particles are, six numbers each.
    pub fn bonds(&self) -> Vec<f64> {
        let w = &self.scene.sim.world;
        let mut out = Vec::with_capacity(w.bonds.len() * 6);
        for b in &w.bonds {
            if let (Some(a), Some(c)) = (w.particle(b.a), w.particle(b.b)) {
                out.extend(a.pos);
                out.extend(c.pos);
            }
        }
        out
    }

    /// Every body: its name, where it is and how big, and whether it's a caster.
    pub fn bodies(&self) -> String {
        let sim = &self.scene.sim;
        let out: Vec<Value> = sim
            .world
            .bodies
            .iter()
            .enumerate()
            .map(|(i, b)| json!({ "name": b.name, "pos": b.pos, "half": b.half, "caster": sim.casters.iter().any(|c| c.body == i) }))
            .collect();
        json!(out).to_string()
    }
}

/// Numbers each particle takes in `Machine::particles`.
#[wasm_bindgen]
pub fn particle_stride() -> usize {
    20
}
