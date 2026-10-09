//! The machine running in the world: casters casting, weaves running their orders, the world moving. One tick at a time.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::air::step_air;
use super::caster::{Caster, CasterStats, ManaRegister, adept};
use super::energy::{Ledger, Stored, heat_of, stored, sum};
use super::fluid::{Change, FluidHooks, find_pairs, merge_and_split, step_fluid};
use super::parts::{Parts, add, take, total, zero};
use super::physics::physics;
use super::weave::{FeltKey, Weave, centre_of, stamp_of, turn_to_frame, turn_to_world};
use super::world::{Bond, EARTH, Ingrained, Vec3, World, WorldMana, fill_of, mass_of, mass_of_parts, packed};
use crate::asm::isa::{Mn, port_by_code};
use crate::asm::{Code, FetchError, Instr};
use crate::js;

/// Something a mind or an order can't do: it stops the spell, or frays the particle.
#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub code: &'static str,
    pub message: String,
}

impl Fault {
    fn new(code: &'static str, message: impl Into<String>) -> Fault {
        Fault { code, message: message.into() }
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Cast,
    Halt,
    Fail,
    Fault,
    Weave,
    Manifest,
    Lock,
    Dissolve,
    Fray,
    ThinAir,
    Overcharge,
    Overstrain,
    Taken,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Cast => "cast",
            EventKind::Halt => "halt",
            EventKind::Fail => "fail",
            EventKind::Fault => "fault",
            EventKind::Weave => "weave",
            EventKind::Manifest => "manifest",
            EventKind::Lock => "lock",
            EventKind::Dissolve => "dissolve",
            EventKind::Fray => "fray",
            EventKind::ThinAir => "thin-air",
            EventKind::Overcharge => "overcharge",
            EventKind::Overstrain => "overstrain",
            EventKind::Taken => "taken",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimEvent {
    pub tick: i64,
    pub kind: EventKind,
    pub caster: Option<String>,
    pub weave: Option<u32>,
    pub detail: Option<String>,
}

/// A thinking frame: a mind casting a spell, or a weave's particle running its order.
#[derive(Clone, Debug)]
pub struct Frame {
    pub n: [f64; 32],
    /// How many of `n` exist.
    pub limit: usize,
    pub flags: f64,
    pub stack: Vec<f64>,
    pub calls: Vec<usize>,
    pub memory: Vec<f64>,
    pub pc: usize,
}

fn frame(registers: usize, memory: usize, pc: usize) -> Frame {
    Frame { n: [0.0; 32], limit: registers, flags: 0.0, stack: Vec::new(), calls: Vec::new(), memory: vec![0.0; memory], pc }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastState {
    Running,
    Halted,
    Failed,
    Fault,
}

impl CastState {
    pub fn as_str(self) -> &'static str {
        match self {
            CastState::Running => "running",
            CastState::Halted => "halted",
            CastState::Failed => "failed",
            CastState::Fault => "fault",
        }
    }
}

/// How many times an instruction ran, and the beats it took.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spent {
    pub runs: f64,
    pub beats: f64,
}

#[derive(Clone, Debug)]
pub struct Cast {
    /// Its caster, in `sim.casters`.
    pub caster: usize,
    pub program: Arc<Code>,
    pub name: String,
    pub frame: Frame,
    pub state: CastState,
    pub code: Option<f64>,
    pub fault: Option<String>,
    /// The weave in n0 when it halted, if any.
    pub result: Option<u32>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    /// Beats of thought spent so far.
    pub beats: f64,
    /// Where they went: how many times each instruction ran, and its beats, by address.
    pub profile: IndexMap<usize, Spent>,
    /// Beats left in the tick being thought.
    pub left: f64,
    /// Done thinking for this tick: out of beats, or it said `TICK`.
    pub yielded: bool,
}

/// One instruction of a particle's order: where it is, and the particle's registers just before it ran.
#[derive(Clone, Debug)]
pub struct OrderStep {
    pub addr: usize,
    pub n: Vec<f64>,
}

/// One particle's order, run for one tick, instruction by instruction (when `trace_orders` is on).
#[derive(Clone, Debug)]
pub struct OrderTrace {
    pub tick: i64,
    pub weave: u32,
    /// Which of its weave's particles, as PPOS counted them as the tick began.
    pub particle: usize,
    /// The particle's id: particles merge and split, so the count can change before the next tick.
    pub id: u64,
    /// Where it was from its weave's centre, in the weave's frame, as the tick began.
    pub off: Vec3,
    /// Its copy of the weave's registers as the tick began: what GETW reads.
    pub w: Vec<f64>,
    pub steps: Vec<OrderStep>,
    /// Its registers when it was done.
    pub n: Vec<f64>,
    /// Done, let go (DISS), or the fault that frayed it.
    pub outcome: String,
    pub beats: f64,
    /// Its own mana its thinking burned.
    pub burned: f64,
    /// How it pushed itself (KICK), in the weave's frame.
    pub kick: Vec3,
    pub cnds: f64,
}

/// M spent so far: poured onto particles by casters (`push`, and what SEND spends) and by orders on themselves
/// (`kick`), and burned thinking. Poured mana isn't used up: it goes loose, still mana.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpentMana {
    pub push: f64,
    pub kick: f64,
    pub burn: f64,
}

/// Every M of mana in the world, wherever it is. Conservation says `total` never changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManaLedger {
    pub air: f64,
    pub matter: f64,
    pub loose: f64,
    pub weaves: f64,
    pub carried: f64,
    pub casters: f64,
    pub free: f64,
    pub condensed: f64,
    pub total: f64,
}

/// The Energy ledger now: what the world holds, as heat and otherwise, and how far it is off.
#[derive(Clone, Debug)]
pub struct EnergyNow {
    pub held: Stored,
    pub total: f64,
    pub heat: f64,
    pub start: f64,
    pub outside: f64,
    pub minds: f64,
    pub bodies: f64,
    pub error: IndexMap<String, f64>,
    pub error_total: f64,
}

/// What a push goes off: a caster's body (through their reach), the air in a cell, or the ground.
#[derive(Clone, Copy, Debug)]
enum Against {
    Body(usize),
    Air(usize),
    Ground,
}

/// Where a push's mana comes from: a caster's mana register, or the particle's own.
#[derive(Clone, Copy, Debug)]
enum Payer {
    Register(usize, usize),
    Itself,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum From {
    Push,
    Kick,
}

/// What a mind's instruction came to.
enum Mind {
    Next,
    Tick,
    End(CastState, Option<String>),
}

/// What the instructions every frame shares came to.
#[derive(PartialEq)]
enum Common {
    Next,
    Jump,
    RetEmpty,
    Halt,
    Tick,
    Unhandled,
}

/// How an order's run for the tick ended.
enum OrderEnd {
    Done,
    Diss,
}

/// A register write an order made: its particle's copy of the weave's register `k` becomes `v` from the next tick.
struct Write {
    p: usize,
    k: usize,
    v: f64,
}

/// `Math.round(v * 100) / 100`, as text.
fn round(v: f64) -> String {
    js::num(js::round(v * 100.0) / 100.0)
}

/// A weave's id, from a number in a register: only a whole number names one.
fn weave_id(v: f64) -> Option<u32> {
    if v.fract() == 0.0 && v >= 0.0 && v <= u32::MAX as f64 { Some(v as u32) } else { None }
}

fn fetch_fault(e: FetchError) -> Fault {
    match e {
        FetchError::OffTheEnd(addr) => Fault::new("NO_ROOM", format!("ran off the end of the spell at {addr}")),
        // Bytes that aren't an instruction: the assembler never lays any out, so only a mind that jumped into the middle
        // of one finds them. The TypeScript engine stopped the whole machine here; here it's a fault.
        FetchError::Decode(d) => Fault::new("BAD_CODE", d.0),
    }
}

pub struct Sim {
    pub world: World,
    pub casters: Vec<Caster>,
    pub weaves: IndexMap<u32, Weave>,
    pub casts: Vec<Cast>,
    pub events: Vec<SimEvent>,
    next_weave: u32,
    /// Partway through a tick: some minds may still be thinking, and the world hasn't moved yet.
    pub mid_tick: bool,
    /// Record every particle's order, instruction by instruction, into `traces`. For the tester; it costs time.
    pub trace_orders: bool,
    /// Last tick's orders, by weave id, one trace per particle that ran one (in the order they ran).
    pub traces: IndexMap<u32, Vec<OrderTrace>>,
    /// Matter held by mana, per world cell, as of the start of this tick's orders.
    carried: Vec<f64>,
    carried_by: HashMap<u32, HashMap<usize, f64>>,
    /// Beats the instruction being run costs beyond its own: ingraining a long order.
    extra: f64,
    /// Weaves that have come apart. Their casters can still name them: there's nothing in them to sense or push, and
    /// mana emitted into one goes loose.
    gone: IndexMap<u32, Weave>,
    pub spent: SpentMana,
    /// Energy minds (casters' and orders') have transformed out of mana into the world so far (D32).
    pub transformed: f64,
    /// M of free mana that has strayed out of its caster's reach before it was given an order, and left its weave.
    pub strayed: f64,
    /// Bumped whenever a weave's particles change outside the world's step, so that what's felt is felt again.
    version: u64,
    /// Whether the Energy ledger is kept (keep_energy).
    pub track_energy: bool,
    pub energy: Ledger,
}

impl Sim {
    pub fn new(world: World) -> Sim {
        let size = world.size;
        Sim {
            world,
            casters: Vec::new(),
            weaves: IndexMap::new(),
            casts: Vec::new(),
            events: Vec::new(),
            next_weave: 1,
            mid_tick: false,
            trace_orders: false,
            traces: IndexMap::new(),
            carried: vec![0.0; size],
            carried_by: HashMap::new(),
            extra: 0.0,
            gone: IndexMap::new(),
            spent: SpentMana::default(),
            transformed: 0.0,
            strayed: 0.0,
            version: 0,
            track_energy: false,
            energy: Ledger::default(),
        }
    }

    pub fn tick(&self) -> i64 {
        self.world.tick
    }

    /// A caster standing at `pos`: a body in the world, and a mind. Returns their index in `casters`.
    pub fn add_caster(&mut self, name: &str, pos: Vec3, stats: Option<CasterStats>) -> usize {
        let body = self.world.add_person(name, pos);
        self.casters.push(Caster::new(name, body, stats.unwrap_or_else(adept)));
        self.casters.len() - 1
    }

    /// A weave, live or gone, by id.
    pub fn weave(&self, id: u32) -> Option<&Weave> {
        self.weaves.get(&id).or_else(|| self.gone.get(&id))
    }

    fn weave_mut(&mut self, id: u32) -> &mut Weave {
        match self.weaves.get_mut(&id) {
            Some(w) => w,
            None => self.gone.get_mut(&id).unwrap(),
        }
    }

    /// Starts a spell. It begins at its first instruction, or at `entry`. Returns its index in `casts`.
    pub fn cast(&mut self, caster: usize, program: Arc<Code>, entry: Option<&str>) -> Result<usize, String> {
        let pc = match entry {
            Some(e) => *program.program.labels.get(e).ok_or_else(|| format!("no label {e}"))?,
            None => 0,
        };
        let name = match entry {
            Some(e) => e.to_string(),
            None => {
                program.program.labels.iter().find(|(_, a)| **a == 0).map(|(n, _)| n.clone()).unwrap_or_else(|| "spell".into())
            }
        };
        let c = &self.casters[caster];
        let f = frame(c.registers() as usize, c.memory() as usize, pc);
        self.casts.push(Cast {
            caster,
            program,
            name: name.clone(),
            frame: f,
            state: CastState::Running,
            code: None,
            fault: None,
            result: None,
            started_at: self.tick(),
            ended_at: None,
            beats: 0.0,
            profile: IndexMap::new(),
            left: 0.0,
            yielded: true,
        });
        let caster_name = self.casters[caster].name.clone();
        self.log(EventKind::Cast, Some(caster_name), None, Some(name));
        Ok(self.casts.len() - 1)
    }

    pub fn run(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// Runs until every cast has ended, or `max` ticks.
    pub fn run_casts(&mut self, max: usize) {
        let mut i = 0;
        while i < max && self.casts.iter().any(|c| c.state == CastState::Running) {
            self.step();
            i += 1;
        }
    }

    pub fn step(&mut self) {
        self.start_thinking();
        self.end_tick();
    }

    /// Runs one instruction of a cast's mind. When that's its last for this tick, the tick ends: the other minds finish
    /// thinking and the world moves. Returns false if the cast has nothing left to run.
    pub fn step_instruction(&mut self, ci: usize) -> bool {
        let mut tries = 0;
        while tries < 2 && self.casts[ci].state == CastState::Running {
            self.start_thinking();
            let ran = self.think(ci, Some(1), &mut |_| false);
            if !self.casts[ci].yielded {
                self.think(ci, Some(0), &mut |_| false); // will the next one fit in this tick?
            }
            if self.casts[ci].yielded || self.casts[ci].state != CastState::Running {
                self.end_tick();
            }
            if ran > 0 {
                return true;
            }
            tries += 1;
        }
        false
    }

    /// Runs until `stop` is true just before one of the cast's instructions, and leaves the machine there, partway
    /// through a tick. The instruction it's resuming from doesn't count. Returns true if it stopped, false if `ticks` ran
    /// out or the cast ended.
    pub fn run_until(&mut self, ci: usize, stop: &mut dyn FnMut(usize) -> bool, ticks: usize) -> bool {
        let mut from: Option<usize> = if self.mid_tick { Some(self.casts[ci].frame.pc) } else { None };
        let mut check = |addr: usize| {
            if Some(addr) == from {
                from = None;
                return false;
            }
            from = None;
            stop(addr)
        };
        let mut t = 0;
        while t < ticks && self.casts[ci].state == CastState::Running {
            self.start_thinking();
            self.think(ci, None, &mut check);
            if !self.casts[ci].yielded && self.casts[ci].state == CastState::Running {
                return true;
            }
            self.end_tick();
            t += 1;
        }
        false
    }

    /// The start of a tick: every running mind gets its beats, less what it went over by last tick.
    fn start_thinking(&mut self) {
        if self.mid_tick {
            return;
        }
        self.mid_tick = true;
        for c in self.casters.iter_mut() {
            c.power_left = c.power();
        }
        for cast in self.casts.iter_mut() {
            if cast.state == CastState::Running {
                let c = &self.casters[cast.caster];
                cast.left = c.speed() * (1.0 + c.conditioning.get(&cast.name).copied().unwrap_or(0.0)) + js::min(0.0, cast.left);
                cast.yielded = cast.left <= 0.0;
            }
        }
    }

    /// What the Energy ledger measures as a step begins, when it's kept: the world's Energy, its heat, and what minds
    /// have transformed so far.
    fn ledger_begin(&mut self) -> (f64, f64) {
        if !self.track_energy {
            return (0.0, 0.0);
        }
        (heat_of(&self.world), self.transformed)
    }

    /// The Energy ledger after a step: it put energy in from outside (minds and orders), turned it into heat (counted
    /// where it happens, or measured for the steps that only lose it), or got it wrong, which is counted too.
    fn ledger_end(&mut self, kind: &str, name: &str, e: &mut f64, begin: (f64, f64)) {
        if !self.track_energy {
            return;
        }
        let (heat, minds) = begin;
        let now = self.measure();
        let appeared = now - *e + (heat_of(&self.world) - heat);
        if kind == "outside" {
            // What minds transformed out of mana is counted exactly, where it happened. The rest is what bodies did by
            // gathering, pouring and letting mana out.
            let transformed = self.transformed - minds;
            self.energy.outside += appeared;
            self.energy.minds += transformed;
            self.energy.bodies += appeared - transformed;
        } else if kind == "loses" && appeared < 0.0 {
            self.world.warm(name, -appeared);
        } else {
            *self.energy.error.entry(name.to_string()).or_insert(0.0) += appeared;
        }
        *e = now;
    }

    /// The rest of a tick: minds still thinking finish, then bodies, weaves and the world.
    fn end_tick(&mut self) {
        // The Energy ledger, if it's being kept: each step either puts energy in from outside (minds and orders), turns
        // it into heat (counted where it happens, or measured for the steps that only lose it), or gets it wrong, which
        // is counted too.
        let mut e = if self.track_energy { self.measure() } else { 0.0 };

        let begin = self.ledger_begin();
        // 1. Minds think.
        for ci in 0..self.casts.len() {
            if self.casts[ci].state == CastState::Running {
                self.think(ci, None, &mut |_| false);
            }
        }
        self.mid_tick = false;
        // 2–4. Bodies: holds run down, flows drain, overcharge.
        for c in 0..self.casters.len() {
            self.breathe(c);
        }
        // 5. Weaves take hold of what matter their mana can bind, and let go of what it can't.
        let ids: Vec<u32> = self.weaves.keys().copied().collect();
        for id in &ids {
            self.hold(*id);
        }
        // 6. Particles of weaves set loose run their orders.
        if self.trace_orders {
            self.traces = IndexMap::new();
        }
        self.relay();
        self.measure_carried();
        let ids: Vec<u32> = self.weaves.keys().copied().collect();
        for id in ids {
            if self.weaves.get(&id).is_some_and(|w| !w.in_hand) {
                self.run_orders(id);
            }
        }
        self.ledger_end("outside", "casters and orders", &mut e, begin);

        // 7. The world moves: the mana, the air, bodies, matter.
        self.measure_carried();
        let begin = self.ledger_begin();
        {
            let in_hand: HashMap<u32, usize> =
                self.weaves.values().filter(|w| w.in_hand).map(|w| (w.id, self.casters[w.maker].body)).collect();
            let carried = &self.carried;
            let by = &self.carried_by;
            let hooks = FluidHooks {
                holder: Some(Box::new(|p| if p.weave != 0 { in_hand.get(&p.weave).copied() } else { None })),
                others_carried: Some(Box::new(|p, cell| {
                    carried[cell] - by.get(&p.weave).and_then(|m| m.get(&cell)).copied().unwrap_or(0.0)
                })),
            };
            step_fluid(&mut self.world, &hooks);
        }
        self.ledger_end("exact", "the fluid", &mut e, begin);

        let begin = self.ledger_begin();
        step_air(&mut self.world);
        self.ledger_end("loses", "the air flowing", &mut e, begin);

        // Particles at rest together merge, and big ones spread thin split.
        let begin = self.ledger_begin();
        let changes = {
            let in_hand: HashMap<u32, usize> =
                self.weaves.values().filter(|w| w.in_hand).map(|w| (w.id, self.casters[w.maker].body)).collect();
            let hooks = FluidHooks {
                holder: Some(Box::new(|p| if p.weave != 0 { in_hand.get(&p.weave).copied() } else { None })),
                others_carried: None,
            };
            merge_and_split(&mut self.world, &hooks)
        };
        self.regroup(changes);
        self.ledger_end("exact", "merging", &mut e, begin);

        let begin = self.ledger_begin();
        let ids: Vec<u32> = self.weaves.keys().copied().collect();
        for id in ids {
            if self.weaves.contains_key(&id) {
                self.keep(id);
            }
        }
        self.settle_loose();
        self.unbind();
        self.bond();
        self.world.move_bodies();
        self.measure_carried();
        let carried = std::mem::take(&mut self.carried);
        self.world.settle_matter(&carried);
        self.carried = carried;
        self.ledger_end("loses", "settling", &mut e, begin);
        self.world.tick += 1;
    }

    /// Free mana in casters' bodies.
    fn in_bodies(&self) -> f64 {
        self.casters.iter().fold(0.0, |s, c| s + c.held())
    }

    /// All the Energy the world holds.
    fn measure(&mut self) -> f64 {
        let b = self.in_bodies();
        sum(&stored(&mut self.world, b))
    }

    /// Starts keeping the Energy ledger: from now on every tick counts where energy comes from and goes (it costs time).
    /// The world's heat counts from zero.
    pub fn keep_energy(&mut self) {
        self.track_energy = true;
        self.world.heat = IndexMap::new();
        let start = self.measure();
        self.energy = Ledger { start, ..Ledger::default() };
    }

    /// The ledger now: what the world holds, as heat and otherwise, and how far it is off.
    pub fn energy_now(&mut self) -> EnergyNow {
        let b = self.in_bodies();
        let held = stored(&mut self.world, b);
        let heat = heat_of(&self.world);
        let error_total = self.energy.error.values().fold(0.0, |a, b| a + b);
        EnergyNow {
            held,
            total: sum(&held),
            heat,
            start: self.energy.start,
            outside: self.energy.outside,
            minds: self.energy.minds,
            bodies: self.energy.bodies,
            error: self.energy.error.clone(),
            error_total,
        }
    }

    /// Every M of mana in the world, wherever it is. Conservation says `total` never changes.
    pub fn ledger(&self) -> ManaLedger {
        let WorldMana { air, matter, loose } = self.world.mana();
        let mut weaves = 0.0;
        let mut carried = 0.0;
        for wv in self.weaves.values() {
            for p in wv.parts(&self.world) {
                weaves += total(&p.free);
                carried += total(&p.carried);
            }
        }
        let casters = self.casters.iter().fold(0.0, |s, c| s + c.held());
        ManaLedger {
            air,
            matter,
            loose,
            weaves,
            carried,
            casters,
            free: air + loose + weaves + casters,
            condensed: matter + carried,
            total: air + matter + loose + weaves + carried + casters,
        }
    }

    fn log(&mut self, kind: EventKind, caster: Option<String>, weave: Option<u32>, detail: Option<String>) {
        self.events.push(SimEvent { tick: self.tick(), kind, caster, weave, detail });
    }

    // The mind

    /// Thinks with what's left of this tick's beats, until they run out or it says `TICK`. It can stop sooner: after `max`
    /// instructions, or before one `stop` picks. Returns how many it ran.
    fn think(&mut self, ci: usize, max: Option<usize>, stop: &mut dyn FnMut(usize) -> bool) -> usize {
        let mut ran = 0;
        while self.casts[ci].state == CastState::Running && !self.casts[ci].yielded {
            let instr = match self.casts[ci].program.fetch(self.casts[ci].frame.pc) {
                Ok(i) => i,
                Err(e) => {
                    let f = fetch_fault(e);
                    self.end(ci, CastState::Fault, Some(f.to_string()));
                    return ran;
                }
            };
            let beats = instr.op.beats as f64;
            if self.casts[ci].left < beats {
                self.casts[ci].yielded = true;
                return ran;
            }
            if max.is_some_and(|m| ran >= m) {
                return ran;
            }
            if stop(instr.addr) {
                return ran;
            }
            let cast = &mut self.casts[ci];
            cast.left -= beats;
            ran += 1;
            cast.beats += beats;
            let spent = cast.profile.entry(instr.addr).or_default();
            spent.runs += 1.0;
            spent.beats += beats;
            self.extra = 0.0;
            let mut f = std::mem::replace(&mut self.casts[ci].frame, frame(0, 0, 0));
            let result = self.exec_mind(ci, &mut f, &instr);
            self.casts[ci].frame = f;
            match result {
                Ok(Mind::Tick) => self.casts[ci].yielded = true,
                Ok(Mind::Next) => {}
                Ok(Mind::End(state, detail)) => self.end(ci, state, detail),
                Err(fault) => self.end(ci, CastState::Fault, Some(fault.to_string())),
            }
            // A long order takes more beats to ingrain than there may be left this tick: the next tick pays the rest.
            if self.extra != 0.0 {
                let extra = self.extra;
                let cast = &mut self.casts[ci];
                cast.left -= extra;
                cast.beats += extra;
                cast.profile.get_mut(&instr.addr).unwrap().beats += extra;
                if cast.left <= 0.0 {
                    cast.yielded = true;
                }
            }
        }
        ran
    }

    fn end(&mut self, ci: usize, state: CastState, detail: Option<String>) {
        let tick = self.tick();
        let caster = self.casts[ci].caster;
        let cast = &mut self.casts[ci];
        cast.state = state;
        cast.ended_at = Some(tick);
        if state == CastState::Fault {
            cast.fault = detail.clone();
        }
        if state == CastState::Halted {
            let r = cast.frame.n[0];
            if let Some(id) = weave_id(r)
                && self.weaves.get(&id).is_some_and(|w| w.maker == caster)
            {
                self.casts[ci].result = Some(id);
            }
            let name = self.casts[ci].name.clone();
            *self.casters[caster].conditioning.entry(name).or_insert(0.0) += 1.0;
        }
        let kind = match state {
            CastState::Halted => EventKind::Halt,
            CastState::Failed => EventKind::Fail,
            _ => EventKind::Fault,
        };
        let detail = detail.unwrap_or_else(|| self.casts[ci].name.clone());
        let name = self.casters[caster].name.clone();
        self.log(kind, Some(name.clone()), None, Some(detail));
        // Weaves still in hand were never set loose: their mana falls back into the body's flow.
        let ids: Vec<u32> = self.weaves.keys().copied().collect();
        for id in ids {
            let wv = &self.weaves[&id];
            if wv.maker != caster || !wv.in_hand {
                continue;
            }
            for pid in wv.particles.clone() {
                if let Some(i) = self.world.slot(pid) {
                    let free = take(&mut self.world.particles[i].free, f64::INFINITY);
                    add(&mut self.casters[caster].flow, &free);
                    self.drop_matter(i);
                }
            }
            self.remove_empty();
            self.weaves.shift_remove(&id);
            self.log(EventKind::Dissolve, Some(name.clone()), Some(id), Some("never set loose".into()));
        }
    }

    /// Runs one instruction of a casting mind.
    fn exec_mind(&mut self, ci: usize, f: &mut Frame, instr: &Instr) -> Result<Mind, Fault> {
        let caster = self.casts[ci].caster;
        let a = instr.args;
        let op = instr.op;
        if op.order {
            return Err(Fault::new("ORDER_ONLY", format!("{} only works inside a weave's order", op.name)));
        }

        match exec_common(f, instr)? {
            Common::RetEmpty | Common::Halt => return Ok(Mind::End(CastState::Halted, None)),
            Common::Tick => return Ok(Mind::Tick),
            Common::Unhandled => {}
            _ => return Ok(Mind::Next),
        }

        let next = instr.addr + instr.size;
        let c = &self.casters[caster];
        let streams = c.streams();
        let m = |k: usize| -> Result<usize, Fault> {
            let r = a[k] as usize;
            if r as f64 >= streams {
                return Err(Fault::new("NO_STREAM", format!("m{r}: {} works {} streams", c.name, js::num(streams))));
            }
            Ok(r)
        };
        macro_rules! n {
            ($k:expr) => {
                get(f, a[$k] as usize)?
            };
        }
        macro_rules! s {
            ($k:expr) => {
                source(f, instr, $k)?
            };
        }
        macro_rules! vec3 {
            ($k:expr) => {
                [get(f, a[$k] as usize)?, get(f, a[$k] as usize + 1)?, get(f, a[$k] as usize + 2)?]
            };
        }
        // A weave of this caster's, live or gone, named by the number in register a[k].
        macro_rules! weave {
            ($k:expr) => {{
                let v = n!($k);
                match weave_id(v).filter(|id| self.weave(*id).is_some_and(|w| w.maker == caster)) {
                    Some(id) => id,
                    None => {
                        return Err(Fault::new(
                            "NOT_YOURS",
                            format!("there's no weave {} of {}'s", js::num(v), self.casters[caster].name),
                        ));
                    }
                }
            }};
        }
        // Mana moved into register `md` from `from`: it's held as long as the shorter of the two holds.
        fn into(regs: &mut [ManaRegister; 8], md: usize, from: usize, parts: Parts) {
            let until = regs[from].hold_until;
            let r = &mut regs[md];
            r.hold_until = if total(&r.parts) <= physics().epsilon { until } else { js::min(r.hold_until, until) };
            add(&mut r.parts, &parts);
        }
        let ph = physics();
        let tick = self.tick() as f64;

        match op.mn {
            Mn::Fail => {
                self.casts[ci].code = Some(a[0]);
                return Ok(Mind::End(CastState::Failed, Some(format!("code {}", js::num(a[0])))));
            }
            Mn::In => {
                let port = port_by_code(a[1] as u8).unwrap();
                let values = self.port(caster, port.name)?;
                for (k, v) in values.iter().enumerate() {
                    set(f, a[0] as usize + k, *v)?;
                }
            }

            // Body
            Mn::Gath => {
                let r = m(0)?;
                let want = js::max(0.0, s!(1));
                let got = self.gather(caster, want);
                let c = &mut self.casters[caster];
                if total(&c.regs[r].parts) <= ph.epsilon {
                    c.regs[r].hold_until = tick;
                }
                add(&mut c.regs[r].parts, &got);
                if total(&got) < want - 1e-6 {
                    let name = c.name.clone();
                    self.log(EventKind::ThinAir, Some(name), None, Some(format!("{} of {} M", round(total(&got)), round(want))));
                }
            }
            Mn::Circ => {
                let r = m(0)?;
                let c = &mut self.casters[caster];
                c.regs[r].hold_until = tick + c.focus();
            }
            Mn::Filt => {
                let md = m(0)?;
                let ms = m(1)?;
                let k = s!(2);
                if k.fract() != 0.0 || !(0.0..=3.0).contains(&k) {
                    return Err(Fault::new("NO_NAME", format!("mana answers to four names, 0–3, not {}", js::num(k))));
                }
                let k = k as usize;
                let c = &mut self.casters[caster];
                let part = c.regs[ms].parts[k];
                c.regs[ms].parts[k] = 0.0;
                let kept = part * c.affinity(k);
                let mut out = zero();
                out[k] = kept;
                into(&mut c.regs, md, ms, out);
                c.flow[k] += part - kept;
            }
            Mn::Splt => {
                let md = m(0)?;
                let ms = m(1)?;
                let amount = js::max(0.0, s!(2));
                let c = &mut self.casters[caster];
                let parts = take(&mut c.regs[ms].parts, amount);
                into(&mut c.regs, md, ms, parts);
            }
            Mn::Join => {
                let md = m(0)?;
                let ms = m(1)?;
                let c = &mut self.casters[caster];
                let parts = take(&mut c.regs[ms].parts, f64::INFINITY);
                into(&mut c.regs, md, ms, parts);
            }
            Mn::Meas => {
                let r = m(1)?;
                set(f, a[0] as usize, total(&self.casters[caster].regs[r].parts))?;
            }
            Mn::Part => {
                let k = s!(2);
                let v = if k.fract() == 0.0 && (0.0..4.0).contains(&k) {
                    self.casters[caster].regs[m(1)?].parts[k as usize]
                } else {
                    0.0
                };
                set(f, a[0] as usize, v)?;
            }
            Mn::Vent => {
                let r = m(0)?;
                let cell = self.world.clamped_cell_of(&self.world.bodies[self.casters[caster].body].pos);
                let parts = take(&mut self.casters[caster].regs[r].parts, f64::INFINITY);
                self.world.add_air(cell as isize, &parts, [0.0; 3]);
            }

            // Reach
            Mn::Prob | Mn::Airm => {
                let at = vec3!(1);
                let i = self.world.cell_of(&at);
                let k = s!(2);
                let mut v = 0.0;
                if i >= 0 && self.reaches(caster, &at) && k.fract() == 0.0 && (0.0..4.0).contains(&k) {
                    let (i, k) = (i as usize, k as usize);
                    v = if op.mn == Mn::Airm { self.world.air[i][k] } else { self.world.matter[i][k] + self.carried_part(i, k) };
                }
                set(f, a[0] as usize, v)?;
            }
            Mn::Send => {
                // Mana thrown out of the body's reach at a speed, and the body pushed back by it. The mind transforms some
                // of what's thrown into the Energy the throw takes (D32): that share goes loose where it was let out,
                // still mana.
                let r = m(0)?;
                let pos = self.inside(vec3!(2));
                if !self.reaches(caster, &pos) {
                    f.pc = next;
                    return Ok(Mind::Next); // out of reach: nothing is let out
                }
                let held = total(&self.casters[caster].regs[r].parts);
                let want = js::min(js::max(0.0, n!(1)), held);
                let vel = vec3!(3);
                let body = self.casters[caster].body;
                let big_b = self.world.bodies[body].mass;
                let bvel = self.world.bodies[body].vel;
                // Sending m kg at v costs ½m|v|², and pushes the body back by m·v across the ground: −m(v·u) +
                // m²|v|²/2B, where u is how the body moves. Out of what's let out, `sent` goes, and the rest pays for it:
                // solve sent + E(sent)/pushEnergy = want, and E(sent) ≤ the power left.
                let flat = vel[0] * vel[0] + vel[2] * vel[2];
                let rp = self.casters[caster].regs[r].parts;
                let per = if total(&rp) > 0.0 { mass_of_parts(&rp) / total(&rp) } else { 1.0 }; // kilograms a M
                let a2 = (per * per * flat) / (2.0 * big_b);
                let a1 = per * (0.5 * (flat + vel[1] * vel[1]) - (vel[0] * bvel[0] + vel[2] * bvel[2]));
                let solve = |qa: f64, qb: f64, c: f64| {
                    if qa > 1e-15 { (-qb + (qb * qb + 4.0 * qa * c).sqrt()) / (2.0 * qa) } else { c / qb }
                };
                let mut sent = js::max(0.0, solve(a2 / ph.push_energy, 1.0 + a1 / ph.push_energy, want));
                let mut energy = a1 * sent + a2 * sent * sent;
                let power_left = self.casters[caster].power_left;
                if energy > power_left {
                    sent = js::max(0.0, solve(a2, a1, power_left));
                    energy = a1 * sent + a2 * sent * sent;
                }
                let spent = js::max(0.0, energy) / ph.push_energy;
                let mut parts = take(&mut self.casters[caster].regs[r].parts, sent + spent);
                let cell = self.world.clamped_cell_of(&pos);
                let paid = take(&mut parts, spent);
                self.world.add_air(cell as isize, &paid, [0.0; 3]);
                let t = mass_of_parts(&parts);
                if t > ph.epsilon {
                    for k in [0, 2] {
                        self.world.bodies[body].vel[k] -= (t * vel[k]) / big_b;
                    }
                    self.world.impulse.walls[1] += t * vel[1];
                    self.world.pour(&mut parts, pos, 0, vel, ph.pour);
                }
                if energy > 0.0 {
                    let c = &mut self.casters[caster];
                    c.power_left -= energy;
                    c.strain += energy;
                    self.transformed += energy;
                    self.spent.push += spent;
                } else {
                    self.world.warm("braking", -energy);
                }
            }
            Mn::Wpos => {
                let id = weave!(1);
                let felt = self.felt(caster, id);
                let c = centre_of(felt.iter().filter_map(|&p| self.world.particle(p)));
                let v = c.map(|c| c.0).unwrap_or([0.0; 3]);
                for k in 0..3 {
                    set(f, a[0] as usize + k, v[k])?;
                }
            }
            Mn::Wvel => {
                let id = weave!(1);
                let felt = self.felt(caster, id);
                let c = centre_of(felt.iter().filter_map(|&p| self.world.particle(p)));
                let v = self.weave(id).unwrap().to_frame(c.map(|c| c.1).unwrap_or([0.0; 3]));
                for k in 0..3 {
                    set(f, a[0] as usize + k, v[k])?;
                }
            }

            // Weave
            Mn::Weav => {
                let id = self.next_weave;
                self.next_weave += 1;
                let program = self.casts[ci].program.clone();
                let origin = vec3!(1);
                self.weaves.insert(id, Weave::new(id, caster, program, origin));
                set(f, a[0] as usize, id as f64)?;
                let name = self.casters[caster].name.clone();
                self.log(EventKind::Weave, Some(name), Some(id), None);
            }
            Mn::Turn => {
                let id = weave!(0);
                let d = vec3!(1);
                if d[0].abs() + d[2].abs() > ph.epsilon {
                    self.weave_mut(id).yaw = js::atan2(d[0], d[2]);
                }
                let yaw = self.weave(id).unwrap().yaw;
                // What it touches is told which way it faces.
                for pid in self.felt(caster, id).iter() {
                    if let Some(p) = self.world.particle_mut(*pid) {
                        p.yaw = yaw;
                    }
                }
            }
            Mn::Emit => {
                let r = m(0)?;
                let amount = js::max(0.0, n!(1));
                let id = weave!(2);
                let wv = self.weave(id).unwrap();
                if wv.locks.input {
                    return Err(Fault::new("LOCKED", format!("weave {id}'s input is locked")));
                }
                // In hand, from where it was begun; set loose, from the middle of what the caster feels of it.
                let from = if wv.in_hand {
                    Some(wv.origin)
                } else {
                    let felt = self.felt(caster, id);
                    centre_of(felt.iter().filter_map(|&p| self.world.particle(p))).map(|c| c.0)
                };
                let Some(from) = from else {
                    f.pc = next;
                    return Ok(Mind::Next); // nothing of it within reach to pour into
                };
                let wv = self.weave(id).unwrap();
                let off = wv.to_world(vec3!(3));
                let pos = [from[0] + off[0], from[1] + off[1], from[2] + off[2]];
                if self.world.cell_of(&pos) < 0 || !self.reaches(caster, &pos) {
                    f.pc = next;
                    return Ok(Mind::Next); // nowhere it can pour
                }
                let gone = self.gone.contains_key(&id);
                let (regs, stamp, yaw) = (wv.regs.clone(), wv.stamp.clone(), wv.yaw);
                let mut parts = take(&mut self.casters[caster].regs[r].parts, amount);
                let poured = self.world.pour_still(&mut parts, pos, if gone { 0 } else { id });
                // The mana carries what its caster has written into the weave, and which way it faces.
                for &pid in &poured {
                    let p = self.world.particle_mut(pid).unwrap();
                    p.regs.copy_from_slice(&regs);
                    p.stamp.copy_from_slice(&stamp);
                    p.yaw = yaw;
                }
                if !gone {
                    self.weave_mut(id).particles.extend(poured);
                }
                self.version += 1;
            }
            Mn::Wset => {
                // Written into what of it the caster touches; the rest hears of it from them, by contact.
                let id = weave!(0);
                let k = a[1] as usize;
                let len = self.weave(id).unwrap().regs.len();
                if k >= len {
                    return Err(Fault::new("NO_ROOM", format!("a weave has w0–w{}", len - 1)));
                }
                let v = s!(2);
                let stamp = stamp_of(tick, 0.0);
                let wv = self.weave_mut(id);
                wv.regs[k] = v;
                wv.stamp[k] = stamp;
                for pid in self.felt(caster, id).iter() {
                    if let Some(p) = self.world.particle_mut(*pid) {
                        p.regs[k] = v;
                        p.stamp[k] = stamp;
                    }
                }
            }
            Mn::Wget => {
                // The newest copy among what of it the caster touches; touching none of it, what they wrote last.
                let id = weave!(1);
                let k = a[2] as usize;
                let wv = self.weave(id).unwrap();
                if k >= wv.regs.len() {
                    return Err(Fault::new("NO_ROOM", format!("a weave has w0–w{}", wv.regs.len() - 1)));
                }
                let mut best = wv.stamp[k];
                let mut v = wv.regs[k];
                for pid in self.felt(caster, id).iter() {
                    if let Some(p) = self.world.particle(*pid)
                        && p.stamp[k] > best
                    {
                        best = p.stamp[k];
                        v = p.regs[k];
                    }
                }
                set(f, a[0] as usize, v)?;
            }
            Mn::Ordr => {
                let id = weave!(0);
                if self.weave(id).unwrap().locks.order {
                    return Err(Fault::new("LOCKED", format!("weave {id}'s order is locked")));
                }
                self.weave_mut(id).order = Some(a[1] as usize);
            }
            Mn::Mani => {
                let id = weave!(0);
                if self.weave(id).unwrap().in_hand {
                    let c = self.weave(id).unwrap().centre(&self.world);
                    let wv = self.weave_mut(id);
                    wv.in_hand = false;
                    wv.manifested_at = tick;
                    if let Some(c) = c {
                        wv.origin = c.0;
                    }
                    let name = self.casters[caster].name.clone();
                    self.log(EventKind::Manifest, Some(name), Some(id), None);
                }
            }
            Mn::Lock => {
                let id = weave!(0);
                if self.weave(id).unwrap().in_hand {
                    return Err(Fault::new("NOT_LOOSE", format!("weave {id} has to be manifested before it's locked")));
                }
                let what = match a[1] as usize {
                    1 => "input",
                    2 => "order",
                    k => return Err(Fault::new("NO_ROOM", format!("there's nothing to lock called #{k}"))),
                };
                let wv = self.weave_mut(id);
                if what == "input" {
                    wv.locks.input = true;
                } else {
                    wv.locks.order = true;
                }
                let name = self.casters[caster].name.clone();
                self.log(EventKind::Lock, Some(name), Some(id), Some(what.into()));
            }
            Mn::Rels => {
                // What of it the caster touches goes loose. What they can't reach, and carries its order, keeps going.
                let id = weave!(0);
                let felt = self.felt(caster, id);
                self.release(id, &felt, "let go");
                if self.weave(id).unwrap().particles.is_empty() {
                    self.end_weave(id);
                }
            }
            // A weave's particles, as the caster feels them: those within reach, counted 0, 1, 2… in the order they
            // were laid. Further out, the caster can't tell they're there.
            Mn::Pcnt => {
                let id = weave!(1);
                let n = self.felt(caster, id).len();
                set(f, a[0] as usize, n as f64)?;
            }
            Mn::Ppos | Mn::Pvel => {
                let id = weave!(1);
                let felt = self.felt(caster, id);
                let i = s!(2).floor();
                let mut v = [0.0; 3];
                if i >= 0.0 && (i as usize) < felt.len() {
                    let wv = self.weave(id).unwrap();
                    let p = self.world.particle(felt[i as usize]).unwrap();
                    // Where it is from the middle of what the caster feels of the weave.
                    v = if op.mn == Mn::Pvel {
                        wv.to_frame(p.vel)
                    } else {
                        let c = centre_of(felt.iter().filter_map(|&q| self.world.particle(q)))
                            .expect("PPOS: no mana in what's felt")
                            .0;
                        wv.to_frame([p.pos[0] - c[0], p.pos[1] - c[1], p.pos[2] - c[2]])
                    };
                }
                for k in 0..3 {
                    set(f, a[0] as usize + k, v[k])?;
                }
            }
            Mn::Shov => {
                let r = m(0)?;
                let id = weave!(1);
                let i = n!(2).floor();
                let felt = self.felt(caster, id);
                if i >= 0.0 && (i as usize) < felt.len() {
                    let pi = self.world.slot(felt[i as usize]).unwrap();
                    let dv = self.weave(id).unwrap().to_world(vec3!(3));
                    // Through the body's reach: the push goes off the body, which feels it, and the ground under its feet.
                    let mut power = self.casters[caster].power_left;
                    let body = self.casters[caster].body;
                    let e = self.push(pi, dv, Against::Body(body), Payer::Register(caster, r), &mut power, From::Push);
                    let c = &mut self.casters[caster];
                    c.strain += e;
                    c.power_left = power;
                }
            }
            Mn::Ingr => {
                let id = weave!(0);
                let wv = self.weave(id).unwrap();
                let Some(order) = wv.order else {
                    return Err(Fault::new("NO_ORDER", format!("weave {id} has no order to ingrain: give it one with ORDR")));
                };
                let (program, yaw) = (wv.program.clone(), wv.yaw);
                self.extra = program.order_length(order) as f64;
                let i = s!(1).floor();
                let felt = self.felt(caster, id);
                if i >= 0.0
                    && (i as usize) < felt.len()
                    && let Some(p) = self.world.particle_mut(felt[i as usize])
                {
                    p.order = Some(Ingrained { program, addr: order });
                    p.yaw = yaw;
                }
            }
            _ => return Err(Fault::new("ORDER_ONLY", format!("{} can't run in a mind", op.name))),
        }
        f.pc = next;
        Ok(Mind::Next)
    }

    fn port(&self, caster: usize, name: &str) -> Result<Vec<f64>, Fault> {
        let c = &self.casters[caster];
        let body = &self.world.bodies[c.body];
        Ok(match name {
            "AIM" => c.will.aim.to_vec(),
            "HAND" => c.hand(&self.world).to_vec(),
            "SELF" => body.pos.to_vec(),
            "AMOUNT" => vec![c.will.amount],
            "FORCE" => vec![c.will.force],
            "MAINTAIN" => vec![if c.will.maintain { 1.0 } else { 0.0 }],
            "CELL" => vec![self.world.cell],
            "DEPTH" => vec![self.world.d as f64],
            "LOAD" => vec![self.load(caster)],
            "CAPACITY" => vec![c.capacity()],
            "REACH" => vec![c.reach()],
            _ => return Err(Fault::new("BAD_PORT", format!("{name} can only be read inside a weave's order"))),
        })
    }

    /// Draws up to `amount` of air mana from around the body, from every cell in reach in proportion.
    fn gather(&mut self, caster: usize, amount: f64) -> Parts {
        let ph = physics();
        let w = &self.world;
        let reach = js::min(ph.gather_radius, self.casters[caster].reach());
        let body = self.casters[caster].body;
        let p = w.bodies[body].pos;
        let mut cells: Vec<usize> = Vec::new();
        let mut available = 0.0;
        let lo = |v: f64| ((v - reach) / w.cell).floor() as i64;
        let hi = |v: f64| ((v + reach) / w.cell).floor() as i64;
        for z in lo(p[2]).max(0)..=hi(p[2]).min(w.d as i64 - 1) {
            for y in lo(p[1]).max(0)..=hi(p[1]).min(w.h as i64 - 1) {
                for x in lo(p[0]).max(0)..=hi(p[0]).min(w.w as i64 - 1) {
                    let mut c = w.centre(x, y, z);
                    if w.d == 1 {
                        c[2] = p[2];
                    }
                    if js::hypot3(c[0] - p[0], c[1] - p[1], c[2] - p[2]) > reach {
                        continue;
                    }
                    let i = w.index(x, y, z) as usize;
                    cells.push(i);
                    available += total(&w.air[i]);
                }
            }
        }
        let mut got = zero();
        if available <= ph.epsilon || amount <= 0.0 {
            return got;
        }
        let f = js::min(1.0, amount / available);
        // What's drawn in brings its momentum into the body, across the ground; the ground takes the rest.
        for i in cells {
            let (parts, momentum) = self.world.take_air(i, f);
            add(&mut got, &parts);
            let b = &mut self.world.bodies[body];
            b.vel[0] += momentum[0] / b.mass;
            b.vel[2] += momentum[2] / b.mass;
            self.world.impulse.walls[1] -= momentum[1];
        }
        got
    }

    fn load(&self, caster: usize) -> f64 {
        let mut in_hand = 0.0;
        for wv in self.weaves.values() {
            if wv.maker == caster && wv.in_hand {
                in_hand += wv.mana(&self.world);
            }
        }
        self.casters[caster].held() + in_hand
    }

    /// A point inside the world: the nearest cell's centre, if it's outside.
    fn inside(&self, pos: Vec3) -> Vec3 {
        if self.world.cell_of(&pos) >= 0 {
            return pos;
        }
        let [x, y, z] = self.world.coords(self.world.clamped_cell_of(&pos));
        self.world.centre(x, y, z)
    }

    /// Is a point within the caster's reach? Everything their body does or senses is (SPEC §0).
    fn reaches(&self, caster: usize, at: &Vec3) -> bool {
        let c = &self.casters[caster];
        let b = self.world.bodies[c.body].pos;
        js::hypot3(at[0] - b[0], at[1] - b[1], at[2] - b[2]) <= c.reach()
    }

    /// What a caster feels of a weave: its particles within their reach, in the order they were laid.
    pub fn felt(&mut self, caster: usize, weave: u32) -> Rc<Vec<u64>> {
        let c = &self.casters[caster];
        let b = self.world.bodies[c.body].pos;
        let wv = self.weave(weave).unwrap();
        let key = FeltKey {
            tick: self.world.tick,
            version: self.version,
            len: wv.particles.len(),
            body: [b[0].to_bits(), b[1].to_bits(), b[2].to_bits()],
            reach: c.reach().to_bits(),
        };
        if let Some((k, ps)) = &wv.felt
            && *k == key
        {
            return ps.clone();
        }
        let ps: Rc<Vec<u64>> = Rc::new(
            wv.particles
                .iter()
                .copied()
                .filter(|&id| self.world.particle(id).is_some_and(|p| self.reaches(caster, &p.pos)))
                .collect(),
        );
        self.weave_mut(weave).felt = Some((key, ps.clone()));
        ps
    }

    /// A push: a mind transforms mana into Energy (D32) to change particle `pi`'s velocity by `dv`, pushing it off
    /// something that takes the push back, equally and oppositely: a caster's body, through their reach; the air a
    /// particle is in; or the ground it's against. It costs the kinetic energy it adds to both, at PHYSICS.push_energy for
    /// each M, no more than `power` has left this tick and the payer can give; what's asked beyond that, it doesn't get.
    /// Slowing down costs nothing: what it takes out of the motion is heat. The mana poured goes loose into the air there,
    /// still mana. Returns the Energy transformed.
    fn push(&mut self, pi: usize, dv: Vec3, off: Against, payer: Payer, power: &mut f64, from: From) -> f64 {
        let ph = physics();
        let size2 = dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2];
        if size2 < 1e-24 {
            return 0.0;
        }
        let m = mass_of(&self.world.particles[pi]);
        // What it's pushed off: its mass, and how it moves, along each axis (a body stands: the ground holds it up, so up
        // and down the push goes into the ground).
        let mut big_m = f64::INFINITY;
        let mut v = [0.0; 3];
        match off {
            Against::Body(b) => {
                let body = &self.world.bodies[b];
                big_m = body.mass;
                v = [body.vel[0], 0.0, body.vel[2]];
            }
            Against::Air(cell) => {
                big_m = self.world.air_mass(cell);
                if big_m <= ph.epsilon {
                    return 0.0; // nothing there to push off
                }
                v = [self.world.air_vel[cell * 3], self.world.air_vel[cell * 3 + 1], self.world.air_vel[cell * 3 + 2]];
            }
            Against::Ground => {}
        }
        let share = |k: usize| if matches!(off, Against::Body(_)) && k == 1 { 0.0 } else { m / big_m };
        // The energy a share k of the push adds, with impulse J = k·m·dv: k·lin + k²·quad.
        let mut lin = 0.0;
        let mut quad = 0.0;
        let pvel = self.world.particles[pi].vel;
        for k in 0..3 {
            lin += m * dv[k] * (pvel[k] - v[k]);
            quad += 0.5 * m * dv[k] * dv[k] * (1.0 + share(k));
        }
        let mut k = 1.0;
        let mut energy = lin + quad;
        if energy > 0.0 {
            // As much of it as the power left, and what can be paid, allow: solve k·lin + k²·quad = what there is.
            let can = js::min(energy, *power);
            let paid = if can > 0.0 {
                let want = can / ph.push_energy;
                match payer {
                    Payer::Register(c, r) => take(&mut self.casters[c].regs[r].parts, want),
                    Payer::Itself => take(&mut self.world.particles[pi].free, want),
                }
            } else {
                zero()
            };
            let got = total(&paid) * ph.push_energy;
            if got <= 0.0 {
                return 0.0;
            }
            // The mana poured: a caster's comes from their body, at rest; a particle's own leaves it at its own speed.
            let b = mass_of_parts(&paid);
            let p = &self.world.particles[pi];
            let momentum = if from == From::Kick { [p.vel[0] * b, p.vel[1] * b, p.vel[2] * b] } else { [0.0; 3] };
            let cell = self.world.clamped_cell_of(&p.pos);
            self.world.add_air(cell as isize, &paid, momentum);
            match from {
                From::Push => self.spent.push += total(&paid),
                From::Kick => self.spent.kick += total(&paid),
            }
            if got < energy {
                k = js::max(0.0, js::min(1.0, (-lin + (lin * lin + 4.0 * quad * got).sqrt()) / (2.0 * quad)));
            }
            energy = got;
            *power -= got;
        } else {
            self.world.warm("braking", -energy);
        }
        // The push, and its reaction. (The air there now holds the mana poured, too.)
        let mass = mass_of(&self.world.particles[pi]);
        if let Against::Air(cell) = off {
            big_m = self.world.air_mass(cell);
        }
        for i in 0..3 {
            let j = mass * dv[i] * k;
            self.world.particles[pi].vel[i] += dv[i] * k;
            match off {
                Against::Body(b) if i != 1 => self.world.bodies[b].vel[i] -= j / big_m,
                Against::Air(cell) => self.world.air_vel[cell * 3 + i] -= j / big_m,
                _ => self.world.impulse.walls[i] += j, // the ground gives it
            }
        }
        self.world.particles[pi].pushed_at = self.tick() as f64;
        if energy > 0.0 {
            self.transformed += energy;
        }
        js::max(0.0, energy)
    }

    /// What a particle's own push goes off (KICK): the ground, or other matter that holds still, where it's against it on
    /// the side it pushes away from; otherwise the air it's in.
    fn footing(&self, pi: usize, dv: Vec3) -> Against {
        let w = &self.world;
        let i = w.cell_of(&w.particles[pi].pos);
        if i < 0 {
            return Against::Ground;
        }
        let [x, y, z] = w.coords(i as usize);
        // Along its push's strongest axis, the cell behind it.
        let mut ax = 0;
        for k in 1..3 {
            if dv[k].abs() > dv[ax].abs() {
                ax = k;
            }
        }
        let mut back = [x, y, z];
        back[ax] -= js::sign(dv[ax]) as i64;
        let j = w.index(back[0], back[1], back[2]);
        let ground = if j < 0 { !(ax == 2 && w.d == 1) } else { w.solid_at(j) };
        if ground { Against::Ground } else { Against::Air(i as usize) }
    }

    // The body, each tick

    fn breathe(&mut self, ci: usize) {
        let ph = physics();
        let tick = self.tick() as f64;
        let body = self.casters[ci].body;
        let c = &mut self.casters[ci];
        // Holds run down: unheld mana joins the body's flow.
        for r in 0..8 {
            if total(&c.regs[r].parts) > ph.epsilon && c.regs[r].hold_until <= tick {
                let parts = take(&mut c.regs[r].parts, f64::INFINITY);
                add(&mut c.flow, &parts);
            }
        }
        // The flow drains back to the air, down to its baseline.
        let above = total(&c.flow) - c.baseline();
        if above > 0.0 {
            let drain = c.drain();
            let parts = take(&mut c.flow, js::min(drain, above));
            let cell = self.world.clamped_cell_of(&self.world.bodies[body].pos);
            self.world.add_air(cell as isize, &parts, [0.0; 3]);
        }
        // Overcharge harms.
        let excess = self.load(ci) - self.casters[ci].capacity();
        let c = &mut self.casters[ci];
        if excess > 0.0 {
            c.harm += excess;
            c.condition.body = js::max(0.2, c.condition.body - (excess / c.capacity()) * 0.01);
            let name = c.name.clone();
            self.log(EventKind::Overcharge, Some(name), None, Some(format!("{} M past capacity", round(excess))));
        }
        // Strain eases as the mind rests. Past its capacity, it harms the mind (D32).
        let c = &mut self.casters[ci];
        c.strain = js::max(0.0, c.strain - c.recovery());
        let over = c.strain - c.mind_capacity();
        if over > 0.0 {
            c.madness += over;
            c.condition.mind = js::max(0.2, c.condition.mind - (over / js::max(c.mind_capacity(), 1e-9)) * 0.01);
            let name = c.name.clone();
            self.log(EventKind::Overstrain, Some(name), None, Some(format!("{} J past capacity", round(over * 900.0))));
        }
    }

    // Weaves

    /// Matter held by mana, cell by cell: all of it, and each weave's own.
    fn measure_carried(&mut self) {
        self.carried.fill(0.0);
        self.carried_by.clear();
        for p in &self.world.particles {
            let i = self.world.cell_of(&p.pos);
            let m = fill_of(&p.carried);
            if i < 0 || m <= 0.0 {
                continue;
            }
            self.carried[i as usize] += m;
            *self.carried_by.entry(p.weave).or_default().entry(i as usize).or_insert(0.0) += m;
        }
    }

    fn carried_part(&self, i: usize, k: usize) -> f64 {
        let mut v = 0.0;
        for p in &self.world.particles {
            if self.world.cell_of(&p.pos) == i as isize {
                v += p.carried[k];
            }
        }
        v
    }

    /// A weave's free mana holds what matter it can of its own parts in each cell, shared among its particles there by
    /// how much of that part each has. What it can no longer hold, it lets go of.
    fn hold(&mut self, id: u32) {
        let ph = physics();
        let w = &self.world;
        let mut by_cell: IndexMap<usize, Vec<usize>> = IndexMap::new();
        let ids = self.weaves[&id].particles.clone();
        for &pid in &ids {
            let Some(pi) = w.slot(pid) else { continue };
            let i = w.cell_of(&w.particles[pi].pos);
            if i < 0 {
                continue;
            }
            by_cell.entry(i as usize).or_default().push(pi);
        }
        let before: Vec<(usize, f64)> =
            ids.iter().filter_map(|&pid| w.slot(pid)).map(|pi| (pi, mass_of(&w.particles[pi]))).collect();
        let w = &mut self.world;
        for (&i, ps) in &by_cell {
            for k in 0..4 {
                let mut free = 0.0;
                let mut have = 0.0;
                for &pi in ps {
                    free += w.particles[pi].free[k];
                    have += w.particles[pi].carried[k];
                }
                let can = (free * ph.bind) / ph.mana_mass[k]; // M of matter: `bind` kilograms for each M
                if have < can && free > 0.0 {
                    let got = js::min(can - have, w.matter[i][k]);
                    if got <= 0.0 {
                        continue;
                    }
                    w.matter[i][k] -= got;
                    for &pi in ps {
                        let p = &mut w.particles[pi];
                        p.carried[k] += (got * p.free[k]) / free;
                    }
                } else if have > can && have > 0.0 {
                    let out = have - can;
                    w.matter[i][k] += out;
                    for &pi in ps {
                        let p = &mut w.particles[pi];
                        p.carried[k] -= (out * p.carried[k]) / have;
                    }
                }
            }
        }
        // Matter taken up was at rest: the particle carries it at its own speed now, and slows for it. Matter let go of
        // stops dead in the ground, and its momentum with it.
        for (pi, m) in before {
            let now = mass_of(&self.world.particles[pi]);
            if now > m && now > 0.0 {
                for k in 0..3 {
                    self.world.particles[pi].vel[k] *= m / now;
                }
            } else if now < m {
                self.mass_changed(pi, m);
            }
        }
    }

    /// Every particle carrying an order runs it, on its own: what it writes into its copy of the weave's registers counts
    /// from the next tick, and if it lets go (DISS) or frays, only it does. The rest of its weave goes on.
    fn run_orders(&mut self, id: u32) {
        let ph = physics();
        let tick = self.tick();
        let (age, origin, yaw_w, maker, ids) = {
            let wv = &self.weaves[&id];
            (tick as f64 - wv.manifested_at, wv.origin, wv.yaw, wv.maker, wv.particles.clone())
        };
        let mut writes: Vec<Write> = Vec::new();
        let mut leaving: Vec<(usize, &'static str)> = Vec::new();
        let mut frayed = 0;
        let mut fault = String::new();
        let mut traces: Vec<OrderTrace> = Vec::new();

        for (k, &pid) in ids.iter().enumerate() {
            let Some(pi) = self.world.slot(pid) else { continue };
            let Some(order) = self.world.particles[pi].order.clone() else { continue };
            let mut f = frame(ph.order_registers, 16, order.addr);
            let p = &self.world.particles[pi];
            let off = turn_to_frame(yaw_w, [p.pos[0] - origin[0], p.pos[1] - origin[1], p.pos[2] - origin[2]]);
            // What it's told: its mana and its age. Not where it is: it only feels (D31). The rest it reads from its copy
            // of its weave's registers, and what it senses.
            f.n[..5].copy_from_slice(&[0.0, 0.0, 0.0, total(&p.free), age]);
            let mut trace = if self.trace_orders {
                Some(OrderTrace {
                    tick,
                    weave: id,
                    particle: k,
                    id: p.id,
                    off,
                    w: p.regs.clone(),
                    steps: Vec::new(),
                    n: Vec::new(),
                    outcome: "done".into(),
                    beats: 0.0,
                    burned: 0.0,
                    kick: [0.0; 3],
                    cnds: 0.0,
                })
            } else {
                None
            };
            let mut beats = 0.0;
            match self.exec_order(pi, &mut f, &order.program, &mut writes, &mut trace, &mut beats) {
                Ok(OrderEnd::Diss) => {
                    leaving.push((pi, "its order let it go"));
                    if let Some(t) = trace.as_mut() {
                        t.outcome = "let go (DISS)".into();
                    }
                }
                Ok(OrderEnd::Done) => {}
                Err(e) => {
                    leaving.push((pi, "frayed"));
                    frayed += 1;
                    fault = e.to_string();
                    if let Some(t) = trace.as_mut() {
                        t.outcome = e.to_string();
                    }
                }
            }
            // Thinking burns the particle's own mana, beat by beat. It goes into the air, carrying its momentum.
            let p = &mut self.world.particles[pi];
            let burned = take(&mut p.free, beats * ph.order_burn);
            let b = mass_of_parts(&burned);
            self.spent.burn += total(&burned);
            let momentum = [p.vel[0] * b, p.vel[1] * b, p.vel[2] * b];
            let pos = p.pos;
            let cell = self.world.clamped_cell_of(&pos);
            self.world.add_air(cell as isize, &burned, momentum);
            if let Some(mut t) = trace {
                t.n = f.n[..ph.order_registers].to_vec();
                t.burned = total(&burned);
                traces.push(t);
            }
        }
        if self.trace_orders {
            self.traces.insert(id, traces);
        }
        for Write { p, k, v } in writes {
            let q = &mut self.world.particles[p];
            q.regs[k] = v;
            q.stamp[k] = stamp_of(tick as f64, q.id as f64);
        }
        if frayed > 0 {
            let name = self.casters[maker].name.clone();
            let who = if frayed == 1 { "a particle".to_string() } else { format!("{frayed} particles") };
            self.log(EventKind::Fray, Some(name), Some(id), Some(format!("{who}: {fault}")));
        }
        let left = !leaving.is_empty();
        for (pi, why) in leaving {
            self.world.particles[pi].order = None;
            self.loosen(pi);
            self.weaves.get_mut(&id).unwrap().last_left = why.into();
        }
        if left {
            self.keep_own(id);
        }
    }

    /// A weave's particles are only those that still carry its id.
    fn keep_own(&mut self, id: u32) {
        let w = &self.world;
        let wv = self.weaves.get_mut(&id).or_else(|| self.gone.get_mut(&id)).unwrap();
        wv.particles.retain(|&pid| w.particle(pid).is_some_and(|p| p.weave == id));
    }

    /// A weave's particles pass their copies of its registers on to those touching them (within the smoothing length):
    /// the newest write wins. `relay` times a tick, so a write crosses a ball of fire in a tick or two, and nothing it
    /// doesn't touch hears of it.
    fn relay(&mut self) {
        let ph = physics();
        let ps: Vec<usize> = (0..self.world.particles.len()).filter(|&i| self.world.particles[i].weave != 0).collect();
        if ps.len() < 2 {
            return;
        }
        let pairs = find_pairs(&self.world, &ps);
        let r = ph.weave_registers;
        let mut regs = vec![0.0; ps.len() * r];
        let mut stamps = vec![0.0; ps.len() * r];
        let parts = &mut self.world.particles;
        for _ in 0..ph.relay {
            for i in 0..ps.len() {
                regs[i * r..(i + 1) * r].copy_from_slice(&parts[ps[i]].regs);
                stamps[i * r..(i + 1) * r].copy_from_slice(&parts[ps[i]].stamp);
            }
            let mut changed = false;
            for e in 0..pairs.n {
                let (ia, ib) = (pairs.a[e], pairs.b[e]);
                if parts[ps[ia]].weave != parts[ps[ib]].weave {
                    continue;
                }
                for k in 0..r {
                    // From what each had before this hop: one hop at a time.
                    let sa = stamps[ia * r + k];
                    let sb = stamps[ib * r + k];
                    if sa > sb && sa > parts[ps[ib]].stamp[k] {
                        let b = &mut parts[ps[ib]];
                        b.stamp[k] = sa;
                        b.regs[k] = regs[ia * r + k];
                        changed = true;
                    } else if sb > sa && sb > parts[ps[ia]].stamp[k] {
                        let a = &mut parts[ps[ia]];
                        a.stamp[k] = sb;
                        a.regs[k] = regs[ib * r + k];
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Runs one particle's order for this tick.
    fn exec_order(
        &mut self,
        pi: usize,
        f: &mut Frame,
        program: &Code,
        writes: &mut Vec<Write>,
        trace: &mut Option<OrderTrace>,
        spend: &mut f64,
    ) -> Result<OrderEnd, Fault> {
        let ph = physics();
        // A particle's mind is as strong as the mana it's in.
        let mut power = ph.order_power * total(&self.world.particles[pi].free);
        let mut budget = ph.order_budget;
        loop {
            let instr = program.fetch(f.pc).map_err(fetch_fault)?;
            if let Some(t) = trace.as_mut() {
                t.steps.push(OrderStep { addr: instr.addr, n: f.n[..ph.order_registers].to_vec() });
                t.beats += instr.op.beats as f64;
            }
            budget -= instr.op.beats as f64;
            *spend += instr.op.beats as f64;
            if budget < 0.0 {
                return Err(Fault::new(
                    "FRAYED",
                    format!("an order thought more than {} beats in one tick", js::num(ph.order_budget)),
                ));
            }
            let a = instr.args;
            match exec_common(f, &instr)? {
                Common::RetEmpty | Common::Halt | Common::Tick => return Ok(OrderEnd::Done),
                Common::Unhandled => {}
                _ => continue,
            }
            let next = instr.addr + instr.size;
            let tick = self.tick() as f64;
            let r0 = a[0] as usize;
            let triple = |f: &mut Frame, v: Vec3| -> Result<(), Fault> {
                for k in 0..3 {
                    set(f, r0 + k, v[k])?;
                }
                Ok(())
            };
            let p = &self.world.particles[pi];
            match instr.op.mn {
                Mn::Kick => {
                    let dv = turn_to_world(p.yaw, [get(f, r0)?, get(f, r0 + 1)?, get(f, r0 + 2)?]);
                    // It pays with its own mana, and pushes off what's around it: the ground behind it, or the air it's in.
                    let off = self.footing(pi, dv);
                    self.push(pi, dv, off, Payer::Itself, &mut power, From::Kick);
                    if let Some(t) = trace.as_mut() {
                        for k in 0..3 {
                            t.kick[k] += get(f, r0 + k)?;
                        }
                    }
                }
                Mn::Tuch => {
                    // Whether something stopped it or struck it when the world last moved: matter, the ground, a body,
                    // anyone's.
                    set(f, r0, if p.touched_at >= tick - 1.0 { 1.0 } else { 0.0 })?;
                }
                Mn::Getw => {
                    let k = a[1] as usize;
                    if k >= p.regs.len() {
                        return Err(Fault::new("NO_ROOM", format!("a weave has w0–w{}", p.regs.len() - 1)));
                    }
                    set(f, r0, p.regs[k])?;
                }
                Mn::Putw => {
                    if r0 >= p.regs.len() {
                        return Err(Fault::new("NO_ROOM", format!("a weave has w0–w{}", p.regs.len() - 1)));
                    }
                    writes.push(Write { p: pi, k: r0, v: source(f, &instr, 1)? });
                }
                Mn::Diss => return Ok(OrderEnd::Diss),
                Mn::Cnds => {
                    // As much as asked, up to the room the cell has. Nothing asks whether the amount is above nothing, and
                    // below nothing condensing runs backwards: the matter the particle holds comes apart into free mana.
                    // Nobody designed that; it is the flaw that makes freeing matter possible (SPEC D16).
                    let amount = source(f, &instr, 0)?;
                    let i = self.world.cell_of(&p.pos);
                    if let Some(t) = trace.as_mut() {
                        t.cnds += amount;
                    }
                    if amount > 0.0 && i >= 0 {
                        // As much as the room left in the cell takes, each part of it by its density.
                        let room = js::max(0.0, 1.0 - self.world.fill(i as usize) - self.carried[i as usize]);
                        let p = &mut self.world.particles[pi];
                        let each = if total(&p.free) > 0.0 { fill_of(&p.free) / total(&p.free) } else { 0.0 };
                        let made = take(&mut p.free, js::min(amount, if each > 0.0 { room / each } else { amount }));
                        add(&mut p.carried, &made);
                        self.carried[i as usize] += fill_of(&made);
                    } else if amount < 0.0 {
                        let p = &mut self.world.particles[pi];
                        let freed = take(&mut p.carried, -amount);
                        add(&mut p.free, &freed);
                        if i >= 0 {
                            self.carried[i as usize] -= fill_of(&freed);
                        }
                    }
                    // Matter weighs what the mana it was made of did (D40): its mass, and its momentum, don't change.
                }
                Mn::Dens => set(f, r0, p.felt)?,
                Mn::Grad => {
                    let v = turn_to_frame(p.yaw, p.grad);
                    triple(f, v)?;
                }
                Mn::Nvel => {
                    let v = turn_to_frame(p.yaw, p.nvel);
                    triple(f, v)?;
                }
                Mn::In => {
                    let port = port_by_code(a[1] as u8).unwrap().name;
                    let values: Vec<f64> = match port {
                        "CELL" => vec![self.world.cell],
                        "DEPTH" => vec![self.world.d as f64],
                        "VEL" => turn_to_frame(p.yaw, p.vel).to_vec(),
                        _ => return Err(Fault::new("BAD_PORT", format!("an order can't read {port}"))),
                    };
                    for (k, v) in values.iter().enumerate() {
                        set(f, r0 + k, *v)?;
                    }
                }
                _ => return Err(Fault::new("NOT_IN_ORDER", format!("{} can't run inside an order", instr.op.name))),
            }
            f.pc = next;
        }
    }

    /// After the world moves: a weave keeps the particles that carry its order, and those its caster can still reach. One
    /// with no order that has got out of their reach, or one with no mana left, leaves it. A weave set loose is measured
    /// from its centre again, for whoever watches, and one with nothing left in it is gone.
    fn keep(&mut self, id: u32) {
        let ph = physics();
        let (in_hand, maker, ids) = {
            let wv = &self.weaves[&id];
            (wv.in_hand, wv.maker, wv.particles.clone())
        };
        for pid in ids {
            let Some(pi) = self.world.slot(pid) else { continue };
            let p = &self.world.particles[pi];
            if total(&p.free) <= ph.epsilon {
                self.loosen(pi);
                self.weaves.get_mut(&id).unwrap().last_left = "its mana is gone".into();
            } else if !in_hand && p.order.is_none() && !self.reaches(maker, &p.pos) {
                self.strayed += total(&p.free);
                self.loosen(pi);
                self.weaves.get_mut(&id).unwrap().last_left = "out of reach".into();
            }
        }
        self.keep_own(id);
        if !in_hand {
            let c = self.weaves[&id].centre(&self.world);
            if let Some(c) = c {
                self.weaves.get_mut(&id).unwrap().origin = c.0;
            }
        }
        if !in_hand && self.weaves[&id].particles.is_empty() {
            self.end_weave(id);
        }
    }

    /// A weave with nothing left in it is gone. Its caster can still name it.
    fn end_weave(&mut self, id: u32) {
        let Some(wv) = self.weaves.shift_remove(&id) else { return };
        let name = self.casters[wv.maker].name.clone();
        let detail = if wv.last_left.is_empty() { "its mana is gone".to_string() } else { wv.last_left.clone() };
        self.gone.insert(id, wv);
        self.log(EventKind::Dissolve, Some(name), Some(id), Some(detail));
    }

    /// A particle leaves its weave. It drops the matter it held; its order stays with it.
    fn loosen(&mut self, pi: usize) {
        self.world.particles[pi].weave = 0;
        self.drop_matter(pi);
    }

    /// Loose mana that has come to the speed of the air around it settles into it.
    fn settle_loose(&mut self) {
        let ph = physics();
        for pi in 0..self.world.particles.len() {
            let w = &self.world;
            let p = &w.particles[pi];
            if p.weave != 0 {
                continue;
            }
            let c = w.clamped_cell_of(&p.pos);
            let v = &w.air_vel;
            let rel = js::hypot3(p.vel[0] - v[c * 3], p.vel[1] - v[c * 3 + 1], p.vel[2] - v[c * 3 + 2]);
            if rel >= ph.loose_rest && total(&p.free) > ph.epsilon {
                continue;
            }
            let m = mass_of_parts(&p.free);
            let momentum = [p.vel[0] * m, p.vel[1] * m, p.vel[2] * m];
            let parts = take(&mut self.world.particles[pi].free, f64::INFINITY);
            self.world.add_air(c as isize, &parts, momentum);
            self.drop_matter(pi);
        }
        self.remove_empty();
    }

    /// Particles with no mana left are gone.
    fn remove_empty(&mut self) {
        let eps = physics().epsilon;
        self.world.retain_particles(|p| total(&p.free) > eps || total(&p.carried) > eps);
    }

    /// Some of a weave's particles go loose, and forget its order.
    fn release(&mut self, id: u32, ps: &[u64], why: &str) {
        if ps.is_empty() {
            return;
        }
        for &pid in ps {
            if let Some(pi) = self.world.slot(pid) {
                self.loosen(pi);
                self.world.particles[pi].order = None;
            }
        }
        self.keep_own(id);
        self.weave_mut(id).last_left = why.into();
        self.version += 1;
    }

    /// A particle lets go of the matter it holds. It stops dead where it is: its momentum goes into the ground.
    fn drop_matter(&mut self, pi: usize) {
        let before = mass_of(&self.world.particles[pi]);
        let cell = self.world.clamped_cell_of(&self.world.particles[pi].pos);
        let matter = take(&mut self.world.particles[pi].carried, f64::INFINITY);
        add(&mut self.world.matter[cell], &matter);
        self.mass_changed(pi, before);
    }

    /// A particle has let go of matter, which stops dead in the ground where it is (the ground's matter doesn't move): the
    /// ground takes the momentum it had.
    fn mass_changed(&mut self, pi: usize, before: f64) {
        let p = &self.world.particles[pi];
        let d = mass_of(p) - before;
        let vel = p.vel;
        for k in 0..3 {
            self.world.impulse.walls[k] += d * vel[k];
        }
    }

    /// Earth held by mana becomes rock where it's packed and still (D30): each particle holding earth, in a cell as full
    /// of earth as solid ground is, binds to its neighbours that do too (PHYSICS.bond_range), when they hardly move
    /// against each other. A bond keeps the length it was made at. Whose mana it is doesn't matter.
    fn bond(&mut self) {
        let ph = physics();
        let w = &self.world;
        let earthy = |p: &super::world::Particle| p.carried[EARTH] > 0.5 * total(&p.carried) && p.carried[EARTH] > ph.epsilon;
        let ps: Vec<usize> = (0..w.particles.len()).filter(|&i| earthy(&w.particles[i])).collect();
        if ps.len() < 2 {
            return;
        }
        // Earth held in each cell, by anyone.
        let mut held: HashMap<isize, f64> = HashMap::new();
        for p in &w.particles {
            let i = w.cell_of(&p.pos);
            if i >= 0 && p.carried[EARTH] > 0.0 {
                *held.entry(i).or_insert(0.0) += p.carried[EARTH];
            }
        }
        let tight = |pi: usize| {
            let i = w.cell_of(&w.particles[pi].pos);
            i >= 0 && (w.matter[i as usize][EARTH] + held.get(&i).copied().unwrap_or(0.0)) / packed(EARTH) >= ph.solid
        };
        let key = |a: u64, b: u64| if a < b { (a, b) } else { (b, a) };
        let mut known: HashSet<(u64, u64)> = w.bonds.iter().map(|b| key(b.a, b.b)).collect();
        let pairs = find_pairs(w, &ps);
        let mut is_packed: HashMap<usize, bool> = HashMap::new();
        let mut made: Vec<Bond> = Vec::new();
        for e in 0..pairs.n {
            let r = pairs.r[e];
            if r >= ph.bond_range || r < 1e-6 {
                continue;
            }
            let (ai, bi) = (ps[pairs.a[e]], ps[pairs.b[e]]);
            let (a, b) = (&w.particles[ai], &w.particles[bi]);
            let k = key(a.id, b.id);
            if known.contains(&k) {
                continue;
            }
            if js::hypot3(a.vel[0] - b.vel[0], a.vel[1] - b.vel[1], a.vel[2] - b.vel[2]) > ph.bond_speed {
                continue;
            }
            let mut check = |pi: usize| *is_packed.entry(pi).or_insert_with(|| tight(pi));
            if !check(ai) || !check(bi) {
                continue;
            }
            made.push(Bond { a: a.id, b: b.id, rest: r });
            known.insert(k);
        }
        self.world.bonds.extend(made);
    }

    /// After merging and splitting, each weave's particles are those that are its own. A weave's particle that took in
    /// someone else's mana took it over: their weave lost it, and it's this weave's now, with this weave's order (the
    /// second flaw, SPEC §11). Each tick's takeovers are logged, one event for each weave that took and whose it took.
    fn regroup(&mut self, changes: Vec<Change>) {
        if changes.is_empty() {
            return;
        }
        struct Taken {
            into: u32,
            from: u32,
            ordered: bool,
        }
        let mut taken: IndexMap<(u32, u32), Taken> = IndexMap::new();
        for c in &changes {
            let Change::Merge { into, weave, .. } = *c else { continue };
            let p = self.world.particle(into).unwrap();
            if p.weave == 0 || weave == p.weave {
                continue;
            }
            let t = taken.entry((p.weave, weave)).or_insert(Taken { into: p.weave, from: weave, ordered: false });
            t.ordered |= p.order.is_some();
        }
        for wv in self.weaves.values_mut() {
            wv.particles = self.world.particles.iter().filter(|p| p.weave == wv.id).map(|p| p.id).collect();
        }
        for t in taken.values() {
            let name = self.weaves.get(&t.into).map(|w| self.casters[w.maker].name.clone()).unwrap_or_default();
            let whose = if t.from != 0 { format!("weave {}'s", t.from) } else { "loose".to_string() };
            let gave = if t.ordered { ", and gave it its order" } else { "" };
            self.log(EventKind::Taken, Some(name), Some(t.into), Some(format!("took in {whose} mana{gave}")));
        }
    }

    /// Bonds hold only particles that are still in the world, and still hold earth.
    fn unbind(&mut self) {
        let eps = physics().epsilon;
        let w = &self.world;
        let keep: Vec<bool> = w
            .bonds
            .iter()
            .map(|b| {
                w.particle(b.a).is_some_and(|p| p.carried[EARTH] > eps) && w.particle(b.b).is_some_and(|p| p.carried[EARTH] > eps)
            })
            .collect();
        let mut k = 0;
        self.world.bonds.retain(|_| {
            k += 1;
            keep[k - 1]
        });
    }
}

// What minds and orders share: numbers, jumps, calls, the stack and memory.

fn get(f: &Frame, r: usize) -> Result<f64, Fault> {
    if r >= f.limit {
        return Err(Fault::new("NO_ROOM", format!("n{r}: this mind thinks with {} registers", f.limit)));
    }
    Ok(f.n[r])
}

fn set(f: &mut Frame, r: usize, v: f64) -> Result<(), Fault> {
    if r >= f.limit {
        return Err(Fault::new("NO_ROOM", format!("n{r}: this mind thinks with {} registers", f.limit)));
    }
    f.n[r] = if v.is_finite() { v } else { 0.0 };
    Ok(())
}

fn source(f: &Frame, instr: &Instr, k: usize) -> Result<f64, Fault> {
    let last = k == instr.op.operands.len() - 1;
    if last && instr.immediate { Ok(instr.args[k]) } else { get(f, instr.args[k] as usize) }
}

/// Runs the instructions every frame shares. Returns `Unhandled` for the rest.
fn exec_common(f: &mut Frame, instr: &Instr) -> Result<Common, Fault> {
    let a = instr.args;
    let next = instr.addr + instr.size;
    let r0 = a[0] as usize;
    let depth = f.memory.len();
    let arith = |f: &mut Frame, op: fn(f64, f64) -> f64| -> Result<Common, Fault> {
        let x = get(f, r0)?;
        let y = source(f, instr, 1)?;
        set(f, r0, op(x, y))?;
        f.pc = next;
        Ok(Common::Next)
    };
    let unary = |f: &mut Frame, op: fn(f64) -> f64| -> Result<Common, Fault> {
        let x = get(f, r0)?;
        set(f, r0, op(x))?;
        f.pc = next;
        Ok(Common::Next)
    };
    let jump = |f: &mut Frame, when: bool| {
        f.pc = if when { r0 } else { next };
        Ok(Common::Jump)
    };
    match instr.op.mn {
        Mn::Nop => {
            f.pc = next;
            Ok(Common::Next)
        }
        Mn::Halt => {
            f.pc = next;
            Ok(Common::Halt)
        }
        Mn::Tick => {
            f.pc = next;
            Ok(Common::Tick)
        }
        Mn::Jmp => jump(f, true),
        Mn::Jeq => jump(f, f.flags == 0.0),
        Mn::Jne => jump(f, f.flags != 0.0),
        Mn::Jlt => jump(f, f.flags < 0.0),
        Mn::Jle => jump(f, f.flags <= 0.0),
        Mn::Jgt => jump(f, f.flags > 0.0),
        Mn::Jge => jump(f, f.flags >= 0.0),
        Mn::Call => {
            if f.calls.len() >= 16.max(depth) {
                return Err(Fault::new("NO_ROOM", "calls nested too deep for this mind"));
            }
            f.calls.push(next);
            f.pc = r0;
            Ok(Common::Jump)
        }
        Mn::Ret => match f.calls.pop() {
            None => Ok(Common::RetEmpty),
            Some(pc) => {
                f.pc = pc;
                Ok(Common::Jump)
            }
        },
        Mn::Ldi | Mn::Mov => {
            let v = source(f, instr, 1)?;
            set(f, r0, v)?;
            f.pc = next;
            Ok(Common::Next)
        }
        Mn::Add => arith(f, |x, y| x + y),
        Mn::Sub => arith(f, |x, y| x - y),
        Mn::Mul => arith(f, |x, y| x * y),
        Mn::Div => arith(f, |x, y| if y == 0.0 { 0.0 } else { x / y }),
        Mn::Mod => arith(f, |x, y| if y == 0.0 { 0.0 } else { x % y }),
        Mn::Atn2 => arith(f, js::atan2),
        Mn::Min => arith(f, js::min),
        Mn::Max => arith(f, js::max),
        Mn::Neg => unary(f, |x| -x),
        Mn::Abs => unary(f, f64::abs),
        Mn::Sqrt => unary(f, |x| if x < 0.0 { 0.0 } else { x.sqrt() }),
        Mn::Floor => unary(f, f64::floor),
        Mn::Round => unary(f, js::round),
        Mn::Sin => unary(f, js::sin),
        Mn::Cos => unary(f, js::cos),
        Mn::Tan => unary(f, js::tan),
        Mn::Atan => unary(f, js::atan),
        Mn::Cmp => {
            let d = get(f, r0)? - source(f, instr, 1)?;
            f.flags = if d < 0.0 {
                -1.0
            } else if d > 0.0 {
                1.0
            } else {
                0.0
            };
            f.pc = next;
            Ok(Common::Next)
        }
        Mn::Push => {
            if f.stack.len() >= 16.max(depth) {
                return Err(Fault::new("NO_ROOM", "the stack is full"));
            }
            let v = get(f, r0)?;
            f.stack.push(v);
            f.pc = next;
            Ok(Common::Next)
        }
        Mn::Pop => {
            let Some(v) = f.stack.pop() else { return Err(Fault::new("NO_ROOM", "the stack is empty")) };
            set(f, r0, v)?;
            f.pc = next;
            Ok(Common::Next)
        }
        Mn::Ld | Mn::St => {
            let addr = get(f, a[1] as usize)?.floor();
            if addr < 0.0 || addr >= depth as f64 {
                return Err(Fault::new("NO_ROOM", format!("memory has {depth} numbers, not [{}]", js::num(addr))));
            }
            let addr = addr as usize;
            if instr.op.mn == Mn::Ld {
                let v = f.memory[addr];
                set(f, r0, v)?;
            } else {
                f.memory[addr] = get(f, r0)?;
            }
            f.pc = next;
            Ok(Common::Next)
        }
        _ => Ok(Common::Unhandled),
    }
}
