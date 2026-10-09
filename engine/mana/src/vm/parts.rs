//! Mana is four parts (the Law of Equality). Every amount of it, free or condensed, is four numbers.
//! The machine only knows them as 0–3; the Elements library calls them fire, water, air and earth.

pub type Parts = [f64; 4];

pub const fn zero() -> Parts {
    [0.0; 4]
}

pub fn total(p: &Parts) -> f64 {
    p[0] + p[1] + p[2] + p[3]
}

/// Adds `b` into `a`.
pub fn add(a: &mut Parts, b: &Parts) {
    a[0] += b[0];
    a[1] += b[1];
    a[2] += b[2];
    a[3] += b[3];
}

/// Moves up to `amount` out of `from`, keeping its proportions, and returns what was moved.
pub fn take(from: &mut Parts, amount: f64) -> Parts {
    let t = total(from);
    if t <= 0.0 || amount <= 0.0 {
        return zero();
    }
    if amount >= t {
        let all = *from;
        *from = zero();
        return all;
    }
    let f = amount / t;
    let out = [from[0] * f, from[1] * f, from[2] * f, from[3] * f];
    from[0] -= out[0];
    from[1] -= out[1];
    from[2] -= out[2];
    from[3] -= out[3];
    out
}

/// Moves a share (0–1) of each part out of `from`.
pub fn share(from: &mut Parts, f: f64) -> Parts {
    let t = total(from);
    take(from, t * f.clamp_js(0.0, 1.0))
}

/// Which part a mix is mostly made of.
pub fn dominant(p: &Parts) -> usize {
    let mut best = 0;
    for k in 1..4 {
        if p[k] > p[best] {
            best = k;
        }
    }
    best
}

/// `Math.max(lo, Math.min(hi, x))`, NaN and all.
pub trait ClampJs {
    fn clamp_js(self, lo: f64, hi: f64) -> f64;
}

impl ClampJs for f64 {
    fn clamp_js(self, lo: f64, hi: f64) -> f64 {
        crate::js::max(lo, crate::js::min(hi, self))
    }
}
