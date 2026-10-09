//! The bench: the same ball of fire, held or thrown in different ways, on the real machine, measured the same way. It's
//! how to tell what an order is worth: what it costs to ingrain and to run, and what it does (SPEC §11, The bench).

use std::path::PathBuf;
use std::time::Instant;

use crate::asm::Code;
use crate::js;
use crate::load::{assemble_file, repo};
use crate::scenes::{Scene, fireball, hold};
use crate::vm::caster::{CasterStats, adept, master};
use crate::vm::parts::total;
use crate::vm::sim::{Sim, SpentMana};
use crate::vm::weave::Weave;

pub fn bench_dir() -> PathBuf {
    repo().join("bench")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hold,
    Throw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum By {
    Nobody,
    Hand,
    Order,
}

#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub id: &'static str,
    pub kind: Kind,
    pub name: &'static str,
    /// Who holds it: the caster's hand, its own order, or nobody.
    pub by: By,
}

const fn v(id: &'static str, kind: Kind, name: &'static str, by: By) -> Variant {
    Variant { id, kind, name, by }
}

pub const VARIANTS: [Variant; 13] = [
    v("HoldNothing", Kind::Hold, "Nothing", By::Nobody),
    v("HoldEvery", Kind::Hold, "Hand: every particle", By::Hand),
    v("HoldOther", Kind::Hold, "Hand: every other", By::Hand),
    v("HoldSurface", Kind::Hold, "Hand: the surface", By::Hand),
    v("HoldCling", Kind::Hold, "Order: cling (feels the mana thin)", By::Order),
    v("HoldCohere", Kind::Hold, "Order: cohere (and its neighbours' speed)", By::Order),
    v("ThrowHand", Kind::Throw, "Hand, not held", By::Hand),
    v("ThrowHandHeld", Kind::Throw, "Hand, held while in reach", By::Hand),
    v("ThrowCling", Kind::Throw, "Hand, held by cling (the Fireball)", By::Order),
    v("ThrowCohere", Kind::Throw, "Hand, held by cohere", By::Order),
    v("ThrowHeading", Kind::Throw, "Order: heading (an angle)", By::Order),
    v("ThrowSteer", Kind::Throw, "Order: steer (an angle and a speed)", By::Order),
    v("ThrowSteerCling", Kind::Throw, "Order: steer and cling", By::Order),
];

pub fn variant(id: &str) -> Variant {
    *VARIANTS.iter().find(|v| v.id == id).unwrap_or_else(|| panic!("no variant {id}"))
}

pub const CASTERS: [(&str, fn() -> CasterStats); 2] = [("adept", adept), ("master", master)];

/// Layouts: the same throw with a little more or less mana gathered, so a result isn't one lucky arrangement.
pub const LAYOUTS: [usize; 5] = [0, 1, 2, 3, 4];
const AMOUNTS: [f64; 5] = [100.0, 110.0, 120.0, 130.0, 140.0];

/// How long a hold is kept, from when the ball is let go of.
pub const HOLD_TICKS: i64 = 40;
/// The pillar's face, in metres (scenes::fireball).
const PILLAR: f64 = 11.0;

#[derive(Clone, Debug, PartialEq)]
pub struct BenchResult {
    pub variant: String,
    pub caster: String,
    pub dims: u8,
    pub layout: usize,
    /// Ticks from the cast to letting go of it: pouring and ingraining.
    pub ingrain: f64,
    /// Particles and mana when it was let go of.
    pub particles: f64,
    pub mana: f64,
    /// At the end (a hold) or when its front reached the pillar (a throw; `arrived` false if it never did).
    pub arrived: bool,
    /// Ticks from letting go to the end.
    pub ticks: f64,
    /// Share of the mana it still holds that is still a ball: within `BALL` metres of its middle. And share still in it.
    pub together: f64,
    pub kept: f64,
    /// Root-mean-square distance of its particles from its centre, metres.
    pub spread: f64,
    /// M spent: poured by the caster, kicked by its orders, burned by its orders thinking.
    pub push: f64,
    pub kick: f64,
    pub burn: f64,
    /// The caster's beats from letting go to the end, per tick.
    pub hand_beats: f64,
    pub ms: f64,
}

/// How far from its middle a particle can be and still be part of the ball, metres: twice the ball's radius.
const BALL: f64 = 1.0;

/// The mana of a weave that's still a ball: within BALL metres of its middle.
fn ball(sim: &Sim, w: &Weave) -> f64 {
    let Some((c, _)) = w.centre(&sim.world) else { return 0.0 };
    let mut m = 0.0;
    for p in w.parts(&sim.world) {
        if js::hypot3(p.pos[0] - c[0], p.pos[1] - c[1], p.pos[2] - c[2]) < BALL {
            m += total(&p.free);
        }
    }
    m
}

fn spread(sim: &Sim, w: &Weave) -> f64 {
    let Some((c, _)) = w.centre(&sim.world) else { return 0.0 };
    let mut s = 0.0;
    for p in w.parts(&sim.world) {
        s += js::pow(p.pos[0] - c[0], 2.0) + js::pow(p.pos[1] - c[1], 2.0) + js::pow(p.pos[2] - c[2], 2.0);
    }
    (s / w.particles.len() as f64).sqrt()
}

/// Casts one variant in one layout, and measures it. `each` sees every tick, for tests.
pub fn run_variant(v: &Variant, caster: &str, dims: u8, layout: usize, each: Option<&mut dyn FnMut(&Scene)>) -> BenchResult {
    let start = Instant::now();
    let mut s = if v.kind == Kind::Hold { hold(dims) } else { fireball(dims) };
    let stats = CASTERS.iter().find(|(n, _)| *n == caster).unwrap().1();
    s.sim.casters[s.caster].stats = stats;
    s.will().amount = AMOUNTS[layout];
    let program = assemble_file(&bench_dir().join(format!("{}.masm", v.id))).unwrap_or_else(|e| panic!("{e}"));
    let c = s.sim.cast(s.caster, Code::new(program), None).unwrap();
    let mut r = BenchResult {
        variant: v.id.to_string(),
        caster: caster.to_string(),
        dims,
        layout,
        ingrain: 0.0,
        particles: 0.0,
        mana: 0.0,
        arrived: false,
        ticks: 0.0,
        together: 0.0,
        kept: 0.0,
        spread: 0.0,
        push: 0.0,
        kick: 0.0,
        burn: 0.0,
        hand_beats: 0.0,
        ms: 0.0,
    };
    let mut weave: Option<u32> = None;
    let mut let_go: i64 = -1;
    let mut beats_at_let_go = 0.0;
    let mut spent_at_let_go = SpentMana::default();
    let measure = |r: &mut BenchResult, sim: &Sim, live: Option<&Weave>, let_go: i64, beats_at_let_go: f64, spent: SpentMana| {
        r.ticks = (sim.tick() - let_go) as f64;
        let mana = live.map(|w| w.mana(&sim.world)).unwrap_or(0.0);
        r.together = match live {
            Some(w) if mana > 0.0 => ball(sim, w) / mana,
            _ => 0.0,
        };
        r.kept = if live.is_some() { mana / r.mana } else { 0.0 };
        r.spread = live.map(|w| spread(sim, w)).unwrap_or(0.0);
        r.push = sim.spent.push - spent.push;
        r.kick = sim.spent.kick - spent.kick;
        r.burn = sim.spent.burn - spent.burn;
        r.hand_beats = (sim.casts[c].beats - beats_at_let_go) / js::max(1.0, r.ticks);
    };
    let mut each = each;
    for _ in 0..160 {
        s.sim.step();
        if let Some(f) = each.as_mut() {
            f(&s);
        }
        let sim = &s.sim;
        if weave.is_none() {
            weave = sim.weaves.keys().next().copied();
        }
        let Some(id) = weave else { continue };
        let live = sim.weaves.get(&id);
        if let_go < 0 {
            // The weave as it was when last seen: one ended in hand is gone from the machine, and was still in hand.
            if sim.weave(id).is_none_or(|w| w.in_hand) {
                continue;
            }
            let w = sim.weave(id).unwrap();
            let_go = sim.tick() - 1;
            r.ingrain = (let_go - sim.casts[c].started_at) as f64;
            r.particles = w.particles.len() as f64;
            r.mana = w.mana(&sim.world);
            beats_at_let_go = sim.casts[c].beats;
            spent_at_let_go = sim.spent;
        }
        if v.kind == Kind::Hold {
            if sim.tick() - let_go >= HOLD_TICKS {
                measure(&mut r, sim, live, let_go, beats_at_let_go, spent_at_let_go);
                r.arrived = live.is_some();
                s.will().maintain = false;
                break;
            }
            continue;
        }
        // A throw has arrived when the front of the ball reaches the pillar: not a stray particle ahead of it.
        let front = match live.and_then(|w| w.centre(&sim.world).map(|c| (w, c.0))) {
            Some((w, c)) => w
                .parts(&sim.world)
                .filter(|p| js::hypot3(p.pos[0] - c[0], p.pos[1] - c[1], p.pos[2] - c[2]) < BALL)
                .fold(0.0, |m, p| js::max(m, p.pos[0])),
            None => 0.0,
        };
        if front >= PILLAR - 0.05 {
            measure(&mut r, sim, live, let_go, beats_at_let_go, spent_at_let_go);
            r.arrived = true;
            break;
        }
        if live.is_none() {
            break;
        }
    }
    if let_go >= 0 && !r.arrived && v.kind == Kind::Throw {
        let live = s.sim.weaves.get(&weave.unwrap());
        measure(&mut r, &s.sim, live, let_go, beats_at_let_go, spent_at_let_go);
    }
    r.ms = start.elapsed().as_secs_f64() * 1000.0;
    r
}

/// The mean over layouts; `arrived` is the share that arrived.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub variant: String,
    pub caster: String,
    pub dims: u8,
    pub ingrain: f64,
    pub particles: f64,
    pub mana: f64,
    pub arrived: f64,
    pub ticks: f64,
    pub together: f64,
    pub kept: f64,
    pub spread: f64,
    pub push: f64,
    pub kick: f64,
    pub burn: f64,
    pub hand_beats: f64,
    pub ms: f64,
}

pub fn summarise(rs: &[BenchResult]) -> Summary {
    let mean = |f: &dyn Fn(&BenchResult) -> f64| rs.iter().fold(0.0, |s, r| s + f(r)) / rs.len() as f64;
    let r0 = &rs[0];
    Summary {
        variant: r0.variant.clone(),
        caster: r0.caster.clone(),
        dims: r0.dims,
        ingrain: mean(&|r| r.ingrain),
        particles: mean(&|r| r.particles),
        mana: mean(&|r| r.mana),
        arrived: mean(&|r| if r.arrived { 1.0 } else { 0.0 }),
        ticks: mean(&|r| r.ticks),
        together: mean(&|r| r.together),
        kept: mean(&|r| r.kept),
        spread: mean(&|r| r.spread),
        push: mean(&|r| r.push),
        kick: mean(&|r| r.kick),
        burn: mean(&|r| r.burn),
        hand_beats: mean(&|r| r.hand_beats),
        ms: mean(&|r| r.ms),
    }
}

/// The order a variant ingrains, and how long it is: what it costs to ingrain, per particle.
pub fn order_size(v: &Variant) -> usize {
    let program = assemble_file(&bench_dir().join(format!("{}.masm", v.id))).unwrap_or_else(|e| panic!("{e}"));
    let addr = program.labels.get(&format!("{}.order", v.id)).copied();
    let code = Code::new(program);
    addr.map(|a| code.order_length(a)).unwrap_or(0)
}
