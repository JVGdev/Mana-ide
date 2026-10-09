//! JavaScript's numbers, as the TypeScript engine had them: its `Math` (V8's), its rounding and its way of writing a
//! number. The port has to give the same results to the last bit (PLAN step R), so wherever JavaScript's rules differ
//! from Rust's, the engine calls these.

mod fdlibm;

pub use fdlibm::{atan, atan2, cos, exp, log, sin, tan};

/// `x ** y` (`Math.pow`). V8 squares by multiplying, and hands every other power to the C library's `pow`, which is
/// what Rust's `powf` is natively.
pub fn pow(x: f64, y: f64) -> f64 {
    if y == 2.0 { x * x } else { x.powf(y) }
}

/// `Math.hypot`, as V8 works it out: scaled by the largest, and summed with Kahan's compensation.
pub fn hypot(xs: &[f64]) -> f64 {
    let mut max = 0.0f64;
    let mut nan = false;
    for &x in xs {
        if x.is_nan() {
            nan = true;
        } else if x.abs() > max {
            max = x.abs();
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0f64;
    let mut compensation = 0.0f64;
    for &x in xs {
        let n = x.abs() / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

pub fn hypot2(x: f64, y: f64) -> f64 {
    hypot(&[x, y])
}

pub fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    hypot(&[x, y, z])
}

/// `Math.round`: halves go up, toward +∞, and what rounds to zero from below is −0.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    if (-0.5..0.0).contains(&x) {
        return -0.0;
    }
    let f = x.floor();
    if x - f >= 0.5 { f + 1.0 } else { f }
}

/// `Math.max(a, b)`: NaN if either is, and +0 above −0.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 && a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(a, b)`: NaN if either is, and −0 below +0.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 && a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.sign`.
pub fn sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `Math.fround`: the nearest 32-bit float.
pub fn fround(x: f64) -> f64 {
    x as f32 as f64
}

/// `String(x)`: a number as JavaScript writes it (ECMAScript's Number::toString).
pub fn num(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x < 0.0 {
        return format!("-{}", num(-x));
    }
    if x.is_infinite() {
        return "Infinity".into();
    }
    // The shortest digits that read back as x, and where the point goes: x = 0.digits × 10^n.
    let e = format!("{x:e}");
    let (mantissa, exp) = e.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap() + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{digits}e{sign}{}", (n - 1).abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], (n - 1).abs())
        }
    }
}

/// `x.toFixed(digits)`: rounded to `digits` decimals, a tie going to the larger, and −0 written as 0.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.abs() >= 1e21 {
        return num(x);
    }
    if x < 0.0 {
        let s = to_fixed(-x, digits);
        // (−0.04).toFixed(1) is "-0.0": the sign stays even when the digits round to nothing.
        return format!("-{s}");
    }
    let x = x.abs();
    // Rust rounds an exact tie to even; JavaScript to the larger. A tie shows as a 5 and then nothing in the exact digits.
    let exact = format!("{:.*}", digits + 40, x);
    let tail = &exact[exact.len() - 40..];
    let rounded = format!("{:.*}", digits, x);
    if tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0') {
        let up = format!("{:.*}", digits, x + 0.5 * 10f64.powi(-(digits as i32)));
        if up != rounded {
            return up;
        }
    }
    rounded
}

/// `s.padStart(n)`.
pub fn pad_start(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n { s.to_string() } else { format!("{}{s}", " ".repeat(n - len)) }
}

/// `s.padEnd(n)`.
pub fn pad_end(s: &str, n: usize) -> String {
    let len = s.chars().count();
    if len >= n { s.to_string() } else { format!("{s}{}", " ".repeat(n - len)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_numbers_as_javascript_does() {
        for (x, s) in [
            (0.0, "0"),
            (-0.0, "0"),
            (2.0, "2"),
            (2.5, "2.5"),
            (-2.5, "-2.5"),
            (0.1, "0.1"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.5e-7, "1.5e-7"),
            (0.30000000000000004, "0.30000000000000004"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
        ] {
            assert_eq!(num(x), s, "{x:e}");
        }
    }

    #[test]
    fn fixes_decimals_as_javascript_does() {
        for (x, d, s) in [
            (0.125, 2, "0.13"),
            (0.375, 2, "0.38"),
            (2.5, 0, "3"),
            (1.005, 2, "1.00"),
            (-0.04, 1, "-0.0"),
            (0.0, 1, "0.0"),
            (12.3456, 1, "12.3"),
            (99.95, 1, "100.0"),
            (1e21, 2, "1e+21"),
        ] {
            assert_eq!(to_fixed(x, d), s, "{x}");
        }
    }

    #[test]
    fn rounds_as_javascript_does() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(0.49999999999999994), 0.0);
        assert!(round(-0.4).is_sign_negative());
        assert_eq!(max(-0.0, 0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(min(0.0, -0.0).to_bits(), (-0.0f64).to_bits());
        assert!(max(1.0, f64::NAN).is_nan());
    }
}
