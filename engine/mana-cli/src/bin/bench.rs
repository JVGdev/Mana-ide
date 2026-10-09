// bench: every way of holding and throwing a ball of fire, on the real machine, side by side (mana::bench).
//
//   npm run bench                     2D, an adept and a master, five layouts each
//   npm run bench -- --3d             3D too (slow)
//   npm run bench -- --only Throw     variants whose name has this in it
//   npm run bench -- --json out.json  every result, for a report

use mana::bench::{BenchResult, CASTERS, Kind, LAYOUTS, VARIANTS, order_size, run_variant, summarise};
use mana::js::{num as js_num, pad_end, pad_start, to_fixed};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let only = flag("--only");
    let dims: Vec<u8> = if args.iter().any(|a| a == "--3d") { vec![2, 3] } else { vec![2] };
    let variants: Vec<_> = VARIANTS.iter().filter(|v| only.as_ref().is_none_or(|o| v.id.contains(o.as_str()))).collect();

    let mut all: Vec<BenchResult> = Vec::new();
    let pct = |x: f64| pad_start(&format!("{}%", to_fixed(100.0 * x, 0)), 5);
    let num = |x: f64, d: usize| pad_start(&to_fixed(x, d), 6);

    for &d in &dims {
        for kind in [Kind::Hold, Kind::Throw] {
            let vs: Vec<_> = variants.iter().filter(|v| v.kind == kind).collect();
            if vs.is_empty() {
                continue;
            }
            let what = if kind == Kind::Hold { "Holding a ball still" } else { "Throwing a ball at the pillar" };
            println!("\n{what}, {d}D (mean of {} layouts)\n", LAYOUTS.len());
            println!(
                "{}",
                if kind == Kind::Hold {
                    "variant          caster  order ingrain  kept  spread  hand M  order M  burned  beats/tick"
                } else {
                    "variant          caster  order ingrain arrived  ticks  togeth  kept  spread  hand M  order M  burned"
                }
            );
            for v in vs {
                let size = order_size(v);
                for (c, _) in CASTERS {
                    let rs: Vec<BenchResult> = LAYOUTS.iter().map(|&l| run_variant(v, c, d, l, None)).collect();
                    all.extend(rs.iter().cloned());
                    let s = summarise(&rs);
                    let size = if size == 0 { "-".to_string() } else { size.to_string() };
                    let head = format!("{} {} {} {}", pad_end(v.id, 16), pad_end(c, 7), pad_start(&size, 5), num(s.ingrain, 1));
                    if kind == Kind::Hold {
                        println!(
                            "{head} {}  {}  {}   {}  {}   {}",
                            pct(s.kept),
                            num(s.spread, 2),
                            num(s.push, 2),
                            num(s.kick, 2),
                            num(s.burn, 2),
                            num(s.hand_beats, 0)
                        );
                    } else {
                        println!(
                            "{head}  {}  {}  {}  {}  {}  {}   {}  {}",
                            pct(s.arrived),
                            num(s.ticks, 1),
                            pct(s.together),
                            pct(s.kept),
                            num(s.spread, 2),
                            num(s.push, 2),
                            num(s.kick, 2),
                            num(s.burn, 2)
                        );
                    }
                }
            }
        }
    }

    if let Some(out) = flag("--json") {
        // As JavaScript's JSON.stringify(…, null, 1) wrote it.
        let s = |x: &str| format!("\"{}\"", x.replace('\\', "\\\\").replace('"', "\\\""));
        let n = |x: f64| if x.is_finite() { js_num(x) } else { "null".into() };
        let mut text = String::from("{\n \"variants\": [\n");
        let vs: Vec<String> = VARIANTS
            .iter()
            .map(|v| {
                let kind = if v.kind == Kind::Hold { "hold" } else { "throw" };
                let by = match v.by {
                    mana::bench::By::Nobody => "nobody",
                    mana::bench::By::Hand => "hand",
                    mana::bench::By::Order => "order",
                };
                format!(
                    "  {{\n   \"id\": {},\n   \"kind\": {},\n   \"name\": {},\n   \"by\": {},\n   \"order\": {}\n  }}",
                    s(v.id),
                    s(kind),
                    s(v.name),
                    s(by),
                    order_size(v)
                )
            })
            .collect();
        text += &vs.join(",\n");
        text += "\n ],\n \"results\": [\n";
        let rs: Vec<String> = all
            .iter()
            .map(|r| {
                let fields = [
                    ("variant", s(&r.variant)),
                    ("caster", s(&r.caster)),
                    ("dims", n(r.dims as f64)),
                    ("layout", n(r.layout as f64)),
                    ("ingrain", n(r.ingrain)),
                    ("particles", n(r.particles)),
                    ("mana", n(r.mana)),
                    ("arrived", r.arrived.to_string()),
                    ("ticks", n(r.ticks)),
                    ("together", n(r.together)),
                    ("kept", n(r.kept)),
                    ("spread", n(r.spread)),
                    ("push", n(r.push)),
                    ("kick", n(r.kick)),
                    ("burn", n(r.burn)),
                    ("handBeats", n(r.hand_beats)),
                    ("ms", n(r.ms)),
                ];
                let body: Vec<String> = fields.iter().map(|(k, v)| format!("   \"{k}\": {v}")).collect();
                format!("  {{\n{}\n  }}", body.join(",\n"))
            })
            .collect();
        text += &rs.join(",\n");
        text += "\n ]\n}";
        std::fs::write(out, text).unwrap();
    }
}
