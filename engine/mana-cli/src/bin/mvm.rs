// mvm: cast a spell in a test world and watch it in the terminal.
//
//   mvm spells/StoneWall.masm                 the scene named after the spell, in 2D
//   mvm spells/Fireball.masm --ticks 30 --every 3
//   mvm spells/Gust.masm --scene Gust --3d    (3D runs; the terminal shows one slice)
//   mvm spells/Fireball.masm --3d --profile   where the caster's thought went

use std::path::Path;
use std::process::exit;

use mana::asm::Code;
use mana::js::{num, pad_end, pad_start, to_fixed};
use mana::load::assemble_file;
use mana::profile::report;
use mana::render::render;
use mana::scenes::{SCENES, scene};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let file = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with('-') && !(*i > 0 && args[i - 1].starts_with("--")))
        .map(|(_, a)| a.clone());
    let Some(file) = file else {
        eprintln!(
            "usage: mvm <spell.masm> [--scene StoneWall|Fireball|Gust|WaterShield] [--3d] [--ticks n] [--every n] [--maintain n] [--profile] [--quiet]"
        );
        exit(2);
    };
    let program = match assemble_file(Path::new(&file)) {
        Ok(p) => p,
        Err(e) => {
            for p in e.problems {
                eprintln!("{p}");
            }
            exit(1);
        }
    };
    let stem = Path::new(&file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let scene_name = flag("--scene").unwrap_or(stem);
    let dims = if args.iter().any(|a| a == "--3d") { 3 } else { 2 };
    let Some(mut s) = scene(&scene_name, dims).filter(|_| scene_name != "Field") else {
        eprintln!("no scene called {scene_name}: try {}", SCENES.map(|(n, _)| n).join(", "));
        exit(2);
    };
    let number = |name: &str, default: f64| flag(name).map(|v| v.parse::<f64>().unwrap_or(f64::NAN)).unwrap_or(default);
    let ticks = number("--ticks", 40.0);
    let every = number("--every", 5.0);
    let maintain = number("--maintain", 12.0);

    let quiet = args.iter().any(|a| a == "--quiet");
    let before = s.sim.ledger().total;
    let cast = s.sim.cast(s.caster, Code::new(program), None).unwrap();
    let mut t = 0.0;
    while t <= ticks {
        if !quiet && (t % every == 0.0 || t == ticks) {
            let c = &s.sim.casters[s.caster];
            println!(
                "tick {}   load {} / {} M   harm {}",
                s.sim.tick(),
                to_fixed(c.held(), 0),
                to_fixed(c.capacity(), 0),
                to_fixed(c.harm, 0)
            );
            println!("{}", render(&s.sim, None));
        }
        if t == maintain {
            s.will().maintain = false;
        }
        s.sim.step();
        t += 1.0;
    }

    println!("\nevents:");
    for e in &s.sim.events {
        let weave = e.weave.filter(|w| *w != 0).map(|w| format!("weave {w}  ")).unwrap_or_default();
        println!(
            "  {}  {} {weave}{}",
            pad_start(&num(e.tick as f64), 4),
            pad_end(e.kind.as_str(), 10),
            e.detail.clone().unwrap_or_default()
        );
    }
    let l = s.sim.ledger();
    println!(
        "\nledger: free {} + condensed {} = {} M  (was {})",
        to_fixed(l.free, 2),
        to_fixed(l.condensed, 2),
        to_fixed(l.total, 2),
        to_fixed(before, 2)
    );
    if args.iter().any(|a| a == "--profile") {
        println!("\n{}", report(&s.sim.casts[cast], 12));
    }
}
