//! A caster: the person the machine runs on. A body that holds mana, a mind that holds numbers.

use std::collections::HashMap;

use super::parts::{Parts, total, zero};
use super::world::{Vec3, World};
use crate::js;

/// A stat is what they were born with, scaled by how they are now, plus what they've trained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stat {
    pub genetics: f64,
    pub training: f64,
}

const fn s(genetics: f64, training: f64) -> Stat {
    Stat { genetics, training }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyStats {
    pub capacity: Stat,
    pub baseline: Stat,
    pub drain: Stat,
    pub focus: Stat,
    pub streams: Stat,
    /// Metres from the body that they can still push mana.
    pub reach: Stat,
    /// Fire, water, air, earth: 0–1.
    pub affinity: [Stat; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MindStats {
    pub speed: Stat,
    pub registers: Stat,
    pub memory: Stat,
    /// The most Energy the mind can transform out of mana in one tick (kg·(m/tick)², one is 900 J): how hard it can push.
    pub power: Stat,
    /// How much strain it bears before transforming more harms it: its mental capacity.
    pub capacity: Stat,
    /// How much strain eases each tick it rests.
    pub recovery: Stat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CasterStats {
    pub body: BodyStats,
    pub mind: MindStats,
}

/// An ordinary trained mage.
pub fn adept() -> CasterStats {
    CasterStats {
        body: BodyStats {
            capacity: s(600.0, 0.0),
            baseline: s(60.0, 0.0),
            drain: s(15.0, 0.0),
            focus: s(6.0, 0.0),
            streams: s(4.0, 0.0),
            reach: s(4.0, 0.0),
            affinity: [s(0.6, 0.0); 4],
        },
        mind: MindStats {
            speed: s(300.0, 0.0),
            registers: s(32.0, 0.0),
            memory: s(256.0, 0.0),
            power: s(0.5, 0.0),
            capacity: s(60.0, 0.0),
            recovery: s(0.02, 0.0),
        },
    }
}

/// A child with the gift: a small body and a mind with 8 registers. It can't hold Shapes.ball as written.
pub fn child() -> CasterStats {
    CasterStats {
        body: BodyStats {
            capacity: s(150.0, 0.0),
            baseline: s(30.0, 0.0),
            drain: s(8.0, 0.0),
            focus: s(3.0, 0.0),
            streams: s(2.0, 0.0),
            reach: s(2.5, 0.0),
            affinity: [s(0.3, 0.0); 4],
        },
        mind: MindStats {
            speed: s(120.0, 0.0),
            registers: s(8.0, 0.0),
            memory: s(32.0, 0.0),
            power: s(0.1, 0.0),
            capacity: s(10.0, 0.0),
            recovery: s(0.005, 0.0),
        },
    }
}

/// A master: years of training on top of an adept's body and mind.
pub fn master() -> CasterStats {
    CasterStats {
        body: BodyStats {
            capacity: s(600.0, 12000.0),
            baseline: s(60.0, 40.0),
            drain: s(15.0, 10.0),
            focus: s(6.0, 6.0),
            streams: s(4.0, 4.0),
            reach: s(3.0, 2.0),
            affinity: [s(0.6, 0.25); 4],
        },
        mind: MindStats {
            speed: s(300.0, 1200.0),
            registers: s(32.0, 0.0),
            memory: s(256.0, 0.0),
            power: s(0.5, 1.5),
            capacity: s(60.0, 180.0),
            recovery: s(0.02, 0.04),
        },
    }
}

/// What the caster wills: the live input a spell reads through its ports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Will {
    pub aim: Vec3,
    pub amount: f64,
    pub force: f64,
    pub maintain: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManaRegister {
    pub parts: Parts,
    pub hold_until: f64,
}

/// How they are right now, 0–1. Overcharge harm lowers the body's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condition {
    pub body: f64,
    pub mind: f64,
}

#[derive(Clone, Debug)]
pub struct Caster {
    pub name: String,
    /// Their body, in `world.bodies`.
    pub body: usize,
    pub stats: CasterStats,
    pub condition: Condition,
    pub flow: Parts,
    pub regs: [ManaRegister; 8],
    pub will: Will,
    /// How many times they've cast each spell, by name.
    pub conditioning: HashMap<String, f64>,
    /// Harm taken from overcharge, in M past capacity, summed over ticks.
    pub harm: f64,
    /// Energy the mind has transformed out of mana and not yet recovered from (SPEC §4, The mind). Past its capacity,
    /// each tick harms it: `madness` counts by how much, summed over ticks.
    pub strain: f64,
    pub madness: f64,
    /// Energy the mind can still transform this tick.
    pub power_left: f64,
}

impl Caster {
    pub fn new(name: &str, body: usize, stats: CasterStats) -> Caster {
        let mut c = Caster {
            name: name.to_string(),
            body,
            stats,
            condition: Condition { body: 1.0, mind: 1.0 },
            flow: zero(),
            regs: [ManaRegister { parts: zero(), hold_until: -1.0 }; 8],
            will: Will { aim: [0.0; 3], amount: 0.0, force: 0.0, maintain: false },
            conditioning: HashMap::new(),
            harm: 0.0,
            strain: 0.0,
            madness: 0.0,
            power_left: 0.0,
        };
        let b = c.baseline() / 4.0;
        c.flow = [b; 4];
        c
    }

    fn body_stat(&self, st: Stat) -> f64 {
        st.genetics * self.condition.body + st.training
    }
    fn mind_stat(&self, st: Stat) -> f64 {
        st.genetics * self.condition.mind + st.training
    }

    pub fn capacity(&self) -> f64 {
        self.body_stat(self.stats.body.capacity)
    }
    pub fn baseline(&self) -> f64 {
        self.body_stat(self.stats.body.baseline)
    }
    pub fn drain(&self) -> f64 {
        self.body_stat(self.stats.body.drain)
    }
    pub fn focus(&self) -> f64 {
        js::max(0.0, self.body_stat(self.stats.body.focus).floor())
    }
    pub fn streams(&self) -> f64 {
        js::max(0.0, js::min(8.0, self.body_stat(self.stats.body.streams).floor()))
    }
    pub fn reach(&self) -> f64 {
        js::max(0.0, self.body_stat(self.stats.body.reach))
    }
    pub fn affinity(&self, k: usize) -> f64 {
        js::max(0.0, js::min(1.0, self.body_stat(self.stats.body.affinity[k])))
    }
    pub fn speed(&self) -> f64 {
        js::max(1.0, self.mind_stat(self.stats.mind.speed))
    }
    pub fn registers(&self) -> f64 {
        js::max(0.0, js::min(32.0, self.mind_stat(self.stats.mind.registers).floor()))
    }
    pub fn memory(&self) -> f64 {
        js::max(0.0, js::min(256.0, self.mind_stat(self.stats.mind.memory).floor()))
    }
    pub fn power(&self) -> f64 {
        js::max(0.0, self.mind_stat(self.stats.mind.power))
    }
    pub fn mind_capacity(&self) -> f64 {
        js::max(0.0, self.mind_stat(self.stats.mind.capacity))
    }
    pub fn recovery(&self) -> f64 {
        js::max(0.0, self.mind_stat(self.stats.mind.recovery))
    }

    /// The casting hand: at arm's length in front of the body, towards where they aim.
    pub fn hand(&self, world: &World) -> Vec3 {
        let p = world.bodies[self.body].pos;
        let dx = self.will.aim[0] - p[0];
        let dz = self.will.aim[2] - p[2];
        let h = js::hypot2(dx, dz);
        // `Math.hypot(dx, dz) || 1`
        let len = if h == 0.0 || h.is_nan() { 1.0 } else { h };
        [p[0] + (0.6 * dx) / len, p[1] + 0.2, p[2] + (0.6 * dz) / len]
    }

    /// Mana in the body: its flow and what its registers hold. Weaves in hand are counted by the machine.
    pub fn held(&self) -> f64 {
        total(&self.flow) + self.regs.iter().fold(0.0, |sum, r| sum + total(&r.parts))
    }
}
