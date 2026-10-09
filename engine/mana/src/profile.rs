//! Where a cast's thought went: by routine, and by line. What to look at first when making a spell faster.

use indexmap::IndexMap;

use crate::asm::{Program, SourceLine};
use crate::js;
use crate::vm::sim::Cast;

#[derive(Clone, Debug, PartialEq)]
pub struct RoutineCost {
    pub name: String,
    pub beats: f64,
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineCost {
    pub addr: usize,
    pub source: Option<SourceLine>,
    pub runs: f64,
    pub beats: f64,
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub beats: f64,
    pub routines: Vec<RoutineCost>,
    pub lines: Vec<LineCost>,
}

/// The routine an address is in: the last global label at or before it.
fn routines(program: &Program) -> impl Fn(usize) -> String + '_ {
    let mut globals: Vec<(&String, usize)> =
        program.labels.iter().filter(|(n, _)| !n.contains('.')).map(|(n, a)| (n, *a)).collect();
    globals.sort_by_key(|g| g.1);
    move |addr| {
        let mut name = "?".to_string();
        for (n, a) in &globals {
            if *a <= addr {
                name = n.to_string();
            }
        }
        name
    }
}

/// Sorts by beats, most first, keeping the order of equals (JavaScript's sort is stable too).
fn most_first<T>(xs: &mut [T], beats: impl Fn(&T) -> f64) {
    xs.sort_by(|a, b| {
        let d = beats(b) - beats(a);
        if d < 0.0 {
            std::cmp::Ordering::Less
        } else if d > 0.0 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
}

pub fn profile(cast: &Cast) -> Profile {
    let program = &cast.program.program;
    let routine_of = routines(program);
    let mut by_routine: IndexMap<String, f64> = IndexMap::new();
    let mut lines = Vec::new();
    for (&addr, spent) in &cast.profile {
        let name = routine_of(addr);
        *by_routine.entry(name).or_insert(0.0) += spent.beats;
        lines.push(LineCost {
            addr,
            source: program.lines.get(&addr).cloned(),
            runs: spent.runs,
            beats: spent.beats,
            share: spent.beats / cast.beats,
        });
    }
    let mut routines: Vec<RoutineCost> =
        by_routine.into_iter().map(|(name, beats)| RoutineCost { name, beats, share: beats / cast.beats }).collect();
    most_first(&mut routines, |r| r.beats);
    most_first(&mut lines, |l| l.beats);
    Profile { beats: cast.beats, routines, lines }
}

/// A plain-text report: the routines, then the costliest lines.
pub fn report(cast: &Cast, top: usize) -> String {
    let p = profile(cast);
    let pct = |x: f64| format!("{}%", js::pad_start(&js::to_fixed(x * 100.0, 1), 5));
    let ticks = cast.ended_at.unwrap_or(0) - cast.started_at + 1;
    let mut out =
        vec![format!("{}: {} beats over {} ticks", cast.name, js::num(p.beats), ticks), String::new(), "routines:".into()];
    for r in &p.routines {
        out.push(format!("  {}  {}  {}", js::pad_start(&js::num(r.beats), 7), pct(r.share), r.name));
    }
    out.push(String::new());
    out.push("lines:".into());
    for l in p.lines.iter().take(top) {
        let where_ = match &l.source {
            Some(s) => js::pad_end(&format!("{}:{}", s.file, s.line), 22),
            None => js::pad_end("", 22),
        };
        let text = l.source.as_ref().map(|s| s.text.trim().to_string()).unwrap_or_default();
        out.push(format!(
            "  {}  {}  ×{} {} {}",
            js::pad_start(&js::num(l.beats), 7),
            pct(l.share),
            js::pad_end(&js::num(l.runs), 6),
            where_,
            text
        ));
    }
    out.join("\n")
}
