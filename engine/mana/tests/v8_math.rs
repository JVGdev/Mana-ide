//! The engine's math is V8's, to the last bit (src/js). The answers are V8's own: tests/fixtures/v8-math.mjs.

use mana::js;

#[test]
fn the_engines_math_is_v8s() {
    let text = include_str!("fixtures/v8-math.txt");
    let f = |s: &str| f64::from_bits(u64::from_str_radix(s, 16).unwrap());
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in text.lines() {
        let w: Vec<&str> = line.split(' ').collect();
        let want = f(w[w.len() - 1]);
        let x = f(w[1]);
        let got = match w[0] {
            "sin" => js::sin(x),
            "cos" => js::cos(x),
            "tan" => js::tan(x),
            "atan" => js::atan(x),
            "exp" => js::exp(x),
            "log" => js::log(x),
            "atan2" => js::atan2(x, f(w[2])),
            "pow" => js::pow(x, f(w[2])),
            "hypot" => js::hypot2(x, f(w[2])),
            "hypot3" => js::hypot3(x, f(w[2]), f(w[3])),
            other => panic!("{other}"),
        };
        n += 1;
        if got.to_bits() != want.to_bits() && !(got.is_nan() && want.is_nan()) {
            wrong.push(format!("{} {:?} → {got:e}, V8 {want:e}", w[0], &w[1..w.len() - 1]));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {n} differ:\n{}",
        wrong.len(),
        wrong.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
