//! A 2D slice of the world as text, for the terminal.
//!
//! ```text
//!   #  earth            H  earth a weave holds      @  the caster
//!   :  a little earth   W  water a weave holds      &  another body
//!   ~  water            *  fire mana in a weave     .  loose mana
//!   ^  flame            o  water mana in a weave    =  air mana in a weave
//!                       +  earth mana in a weave
//! ```

use indexmap::IndexMap;

use crate::vm::parts::{Parts, dominant, total};
use crate::vm::sim::Sim;
use crate::vm::world::{EARTH, FIRE, WATER, fill_of};

pub fn render(sim: &Sim, z: Option<i64>) -> String {
    let w = &sim.world;
    let slice = z.unwrap_or_else(|| {
        let pz = sim.casters.first().map(|c| w.bodies[c.body].pos[2]).unwrap_or(0.0);
        (pz / w.cell).floor() as i64
    });
    let mut grid = vec![vec![' '; w.w]; w.h];
    let mut put = |x: i64, y: i64, ch: char| {
        if x >= 0 && y >= 0 && x < w.w as i64 && y < w.h as i64 {
            grid[w.h - 1 - y as usize][x as usize] = ch;
        }
    };

    for y in 0..w.h as i64 {
        for x in 0..w.w as i64 {
            let i = w.index(x, y, slice);
            if i < 0 {
                continue;
            }
            let m = &w.matter[i as usize];
            let t = fill_of(m); // the share of the cell it takes
            if t < 0.05 {
                continue;
            }
            let k = dominant(m);
            put(
                x,
                y,
                if k == EARTH {
                    if t >= 0.4 { '#' } else { ':' }
                } else if k == WATER {
                    '~'
                } else if k == FIRE {
                    '^'
                } else {
                    ' '
                },
            );
        }
    }
    // Mana: the matter weaves hold (matter in a cell their mana is in), then the mana itself.
    let mut held: IndexMap<usize, Parts> = IndexMap::new();
    for p in &w.particles {
        let i = w.cell_of(&p.pos);
        if i < 0 || w.coords(i as usize)[2] != slice {
            continue;
        }
        let e = held.entry(i as usize).or_insert([0.0; 4]);
        for k in 0..4 {
            e[k] += if p.weave != 0 { p.free[k] } else { 0.0 };
        }
        if p.weave == 0 {
            let [x, y, _] = w.coords(i as usize);
            put(x, y, '.');
        }
    }
    for (&i, free) in &held {
        let [x, y, _] = w.coords(i);
        let m = &w.matter[i];
        if total(free) > 0.0 && fill_of(m) >= 0.05 {
            put(x, y, if dominant(m) == WATER { 'W' } else { 'H' });
        } else if total(free) > 0.01 {
            put(x, y, ['*', 'o', '=', '+'][dominant(free)]);
        }
    }
    for (bi, b) in w.bodies.iter().enumerate() {
        let [x, y, zz] = w.coords(w.clamped_cell_of(&b.pos));
        if zz != slice {
            continue;
        }
        let ch = if sim.casters.iter().any(|c| c.body == bi) { '@' } else { '&' };
        put(x, y, ch);
        put(x, y + 1, ch);
        put(x, y - 1, ch);
    }
    let edge = format!("+{}+", "-".repeat(w.w));
    let mut out = vec![edge.clone()];
    out.extend(grid.iter().map(|row| format!("|{}|", row.iter().collect::<String>())));
    out.push(edge);
    out.join("\n")
}
