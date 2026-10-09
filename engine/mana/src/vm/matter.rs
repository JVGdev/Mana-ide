//! Matter (PLAN step 2, SPEC §11 One matter): all of it obeys one mechanics, held by mana or not. It's material points
//! over the world's grid (the material point method, MLS-MPM with APIC transfers): each point is a piece of matter with
//! its mass (its parts, D40), how it moves, how it's deformed, and how packed it is. Every step the points tell the grid
//! nodes around them their mass, momentum and stress; the grid moves under its forces; and the points take their new
//! motion back from it. Since it's one grid, matter of every kind pushes on all matter around it.
//!
//! What each part is as matter:
//!
//! - **Earth** is a solid that holds together and gives way (Drucker–Prager plasticity on Hencky strain). Pulled past its
//!   cohesion it cracks; sheared past its friction it flows and keeps its new shape (molding), and loose, it slides and
//!   piles at its angle of repose. Pressed past its crushing pressure it packs denser, and packed denser it holds harder:
//!   loose soil, packed soil and rock are one material at different packings (PLAN answer G).
//! - **Water** is a liquid, nearly incompressible, that holds a little tension: it sticks to itself.
//! - A mix is mostly one or the other, by how much of it is earth.
//! - Fire and air as matter are gases: they stay in the world's cells (PLAN step 3 gives them a pressure).
//!
//! How stiff matter is is a stand-in: sound crosses it at `matter_sound`, 150 m/s, not the kilometres a second it does in
//! rock or water, which would take thirty times more steps (SPEC §0). It's stiff enough to look rigid at the size of a
//! person, and for a column of rock to stand some metres before it buckles.
//!
//! Matter at rest that nothing disturbs sleeps: it's held still, skipped, and its weight rests on what's under it. It
//! wakes when what's around it moves it, or when mana is in its cell. That's a change of how it's kept, not of what it
//! does, like air at rest (SPEC §0, 16): momentum it would have taken goes to the ground under it.

use std::collections::HashMap;

use super::parts::{Parts, total, zero};
use super::physics::physics;
use super::world::{EARTH, Vec3, WATER, World, mass_of_parts};
use crate::js;

/// A 3×3 matrix, row by row.
pub type Mat = [f64; 9];

pub const IDENTITY: Mat = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

/// A piece of matter.
#[derive(Clone, Debug)]
pub struct Point {
    pub id: u64,
    pub pos: Vec3,
    pub vel: Vec3,
    /// How the motion around it varies (APIC's affine velocity), per tick.
    pub c: Mat,
    /// How it's stretched and turned from how it was made, elastically.
    pub f: Mat,
    /// How packed it is, against its parts' own density: below 1, packed denser (rock); above, loosened.
    pub jp: f64,
    /// The mana it's made of: mass is mana (D40).
    pub parts: Parts,
    /// The room it took when it was made, m³.
    pub volume: f64,
    pub asleep: bool,
    /// Ticks it has moved slower than `sleep_speed`.
    pub still: u32,
    /// Its mass, kg (from its parts).
    pub mass: f64,
    /// The energy its stretching stores, as of its last step.
    pub psi: f64,
}

impl Point {
    /// A piece of matter made of `parts`, taking `volume` m³, at rest.
    pub fn new(id: u64, pos: Vec3, parts: Parts, volume: f64) -> Point {
        let mass = mass_of_parts(&parts);
        // How packed: the room it takes, over the room its parts would take at their own density. Less than 1 is packed
        // denser than that (rock); more, loose.
        let own = room_of(&parts);
        let jp = if volume > 0.0 && own > 0.0 { volume / own } else { 1.0 };
        Point { id, pos, vel: [0.0; 3], c: [0.0; 9], f: IDENTITY, jp, parts, volume, asleep: false, still: 0, mass, psi: 0.0 }
    }

    /// How much of it is earth, of what's earth or water: 0 water, 1 earth.
    pub fn earthiness(&self) -> f64 {
        let e = self.parts[EARTH] * physics().mana_mass[EARTH];
        let w = self.parts[WATER] * physics().mana_mass[WATER];
        if e + w > 0.0 { e / (e + w) } else { 0.0 }
    }
}

/// The room some matter takes at its parts' own densities, m³.
pub fn room_of(parts: &Parts) -> f64 {
    let ph = physics();
    let mut v = 0.0;
    for k in 0..4 {
        v += parts[k] * ph.mana_mass[k] / ph.density[k];
    }
    v
}

// Small matrices

fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut out = [0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            out[i * 3 + j] = a[i * 3] * b[j] + a[i * 3 + 1] * b[3 + j] + a[i * 3 + 2] * b[6 + j];
        }
    }
    out
}

fn transpose(a: &Mat) -> Mat {
    [a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]]
}

pub fn det(a: &Mat) -> f64 {
    a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6]) + a[2] * (a[3] * a[7] - a[4] * a[6])
}

fn diag(d: [f64; 3]) -> Mat {
    [d[0], 0.0, 0.0, 0.0, d[1], 0.0, 0.0, 0.0, d[2]]
}

/// The eigenvalues and eigenvectors (columns of V) of a symmetric matrix, by Jacobi rotations: only square roots, so it
/// comes out the same everywhere.
fn eigen_symmetric(a: &Mat) -> ([f64; 3], Mat) {
    let mut m = *a;
    let mut v = IDENTITY;
    for _ in 0..30 {
        let off = m[1] * m[1] + m[2] * m[2] + m[5] * m[5];
        if off < 1e-30 {
            break;
        }
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            let apq = m[p * 3 + q];
            if apq.abs() < 1e-300 {
                continue;
            }
            let theta = (m[q * 3 + q] - m[p * 3 + p]) / (2.0 * apq);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            // m = Jᵀ m J, v = v J, with J the rotation in the (p, q) plane.
            for k in 0..3 {
                let mkp = m[k * 3 + p];
                let mkq = m[k * 3 + q];
                m[k * 3 + p] = c * mkp - s * mkq;
                m[k * 3 + q] = s * mkp + c * mkq;
            }
            for k in 0..3 {
                let mpk = m[p * 3 + k];
                let mqk = m[q * 3 + k];
                m[p * 3 + k] = c * mpk - s * mqk;
                m[q * 3 + k] = s * mpk + c * mqk;
            }
            for k in 0..3 {
                let vkp = v[k * 3 + p];
                let vkq = v[k * 3 + q];
                v[k * 3 + p] = c * vkp - s * vkq;
                v[k * 3 + q] = s * vkp + c * vkq;
            }
        }
    }
    ([m[0], m[4], m[8]], v)
}

/// F = U Σ Vᵀ, with U and V rotations. In 2D (`dims` 2) only the upper 2×2 block is decomposed; the third axis is 1.
pub fn svd(f: &Mat, dims: usize) -> (Mat, [f64; 3], Mat) {
    let ftf = mul(&transpose(f), f);
    let (mut ev, mut v) = if dims == 2 {
        let a = [ftf[0], ftf[1], 0.0, ftf[3], ftf[4], 0.0, 0.0, 0.0, 1.0];
        eigen_symmetric(&a)
    } else {
        eigen_symmetric(&ftf)
    };
    // Largest first, and V a rotation.
    let n = dims;
    for i in 0..n {
        for j in i + 1..n {
            if ev[j] > ev[i] {
                ev.swap(i, j);
                for k in 0..3 {
                    v.swap(k * 3 + i, k * 3 + j);
                }
            }
        }
    }
    if det(&v) < 0.0 {
        for k in 0..3 {
            v[k * 3 + n - 1] = -v[k * 3 + n - 1];
        }
    }
    let mut sigma = [1.0; 3];
    for i in 0..n {
        sigma[i] = js::max(ev[i], 0.0).sqrt();
    }
    // U = F V Σ⁻¹, column by column; a column for a vanishing stretch is made perpendicular to the others.
    let fv = mul(f, &v);
    let mut u = IDENTITY;
    for i in 0..n {
        let col = [fv[i], fv[3 + i], fv[6 + i]];
        let len = js::hypot3(col[0], col[1], col[2]);
        if sigma[i] > 1e-12 && len > 1e-12 {
            for k in 0..3 {
                u[k * 3 + i] = col[k] / len;
            }
        } else {
            u[i * 3 + i] = 1.0;
        }
    }
    if n == 3 {
        // The last column: across the first two, so U is a rotation, carrying the sign of det F into Σ.
        let a = [u[0], u[3], u[6]];
        let b = [u[1], u[4], u[7]];
        let cr = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let dot = cr[0] * u[2] + cr[1] * u[5] + cr[2] * u[8];
        if dot < 0.0 {
            sigma[2] = -sigma[2];
        }
        u[2] = cr[0];
        u[5] = cr[1];
        u[8] = cr[2];
    } else {
        // In 2D the second column is the first turned a quarter.
        let a = [u[0], u[3]];
        let dot = -a[1] * u[1] + a[0] * u[4];
        if dot < 0.0 {
            sigma[1] = -sigma[1];
        }
        u[1] = -a[1];
        u[4] = a[0];
    }
    (u, sigma, v)
}

// The materials

/// What a point is made of, as a material.
struct Material {
    /// A liquid (water, or mostly water) or a solid that gives way (earth).
    liquid: bool,
    /// Lamé's constants, kg/(m·tick²).
    mu: f64,
    lambda: f64,
    /// Its bulk modulus, for a liquid.
    bulk: f64,
    /// The slope of its friction (Drucker–Prager's α), and the pressure its cohesion is worth, and the pressure that
    /// crushes it denser.
    alpha: f64,
    cohesion: f64,
    crush: f64,
    /// How much tension a liquid holds before it parts, kg/(m·tick²).
    tension: f64,
}

fn material(p: &Point) -> Material {
    let ph = physics();
    // Its own density, kg/m³, sets how stiff it is: sound crosses it at `matter_sound`.
    let rho = if p.volume > 0.0 { p.mass / p.volume } else { 0.0 };
    let modulus = rho * ph.matter_sound * ph.matter_sound;
    let e = p.earthiness();
    if e < 0.5 {
        return Material {
            liquid: true,
            mu: 0.0,
            lambda: 0.0,
            bulk: modulus,
            alpha: 0.0,
            cohesion: 0.0,
            crush: 0.0,
            tension: ph.water_tension,
        };
    }
    // Earth: the P-wave modulus is λ + 2μ; Poisson's ratio splits it.
    let nu = ph.earth_poisson;
    let lambda = modulus * nu / (1.0 - nu);
    let mu = modulus * (1.0 - 2.0 * nu) / (2.0 * (1.0 - nu));
    // Mixed with water it holds less, and slides more easily.
    let share = 2.0 * e - 1.0;
    let sin = js::sin(ph.earth_friction);
    let alpha = (2.0f64 / 3.0).sqrt() * 2.0 * sin / (3.0 - sin) * share;
    // Packed denser, it holds harder (and crushes harder); loosened, less.
    let hard = js::exp(js::min(ph.packing_hardening * (1.0 - p.jp), 10.0));
    let tan = js::tan(ph.earth_friction);
    let cohesion = ph.earth_cohesion / tan * hard * share * share;
    let crush = ph.earth_crush * hard;
    Material { liquid: false, mu, lambda, bulk: lambda + 2.0 * mu / 3.0, alpha, cohesion, crush, tension: 0.0 }
}

/// Kirchhoff stress τ for a point's stretch F, and the energy its stretching stores (per m³ of how it was made).
fn stress(p: &Point, m: &Material, dims: usize) -> (Mat, f64) {
    if m.liquid {
        let j = det(&p.f);
        let tau = m.bulk * j * (j - 1.0);
        let mut t = diag([tau, tau, tau]);
        if dims == 2 {
            t[8] = 0.0;
        }
        return (t, 0.5 * m.bulk * (j - 1.0) * (j - 1.0));
    }
    let (u, s, _) = svd(&p.f, dims);
    let mut eps = [0.0; 3];
    for i in 0..dims {
        eps[i] = js::log(js::max(s[i].abs(), 1e-6));
    }
    let tr: f64 = eps[..dims].iter().sum();
    let mut t = [0.0; 3];
    let mut psi = 0.5 * m.lambda * tr * tr;
    for i in 0..dims {
        t[i] = 2.0 * m.mu * eps[i] + m.lambda * tr;
        psi += m.mu * eps[i] * eps[i];
    }
    let tau = mul(&mul(&u, &diag(t)), &transpose(&u));
    (tau, psi)
}

/// Gives way where the point is stretched past what its material holds (the return mapping). Returns how much energy
/// that turned to heat, kg·(m/tick)².
fn give_way(p: &mut Point, m: &Material, dims: usize) -> f64 {
    let (_, before) = stress(p, m, dims);
    let mut work = None;
    if m.liquid {
        // Water pulled apart past its tension parts: it can't be stretched further. Water squeezed back has no shape.
        let j = det(&p.f);
        let most = 1.0 + m.tension / js::max(m.bulk, 1e-30);
        let j = js::min(j, most);
        let side = if dims == 2 { j.sqrt() } else { js::exp(js::log(js::max(j, 1e-12)) / 3.0) };
        p.f = if dims == 2 { diag([side, side, 1.0]) } else { diag([side, side, side]) };
    } else {
        let (u, s, v) = svd(&p.f, dims);
        let mut eps = [0.0; 3];
        for i in 0..dims {
            eps[i] = js::log(js::max(s[i].abs(), 1e-6));
        }
        let d = dims as f64;
        let tr: f64 = eps[..dims].iter().sum();
        // Pressure is −κ·tr ε; cohesion is worth `cohesion` of pressure, and crushing starts at `crush`.
        let kappa = (2.0 * m.mu + d * m.lambda) / d;
        let pressure = -kappa * tr;
        let cracked = kappa > 0.0 && pressure < -m.cohesion;
        let new_tr = if cracked {
            // Pulled apart past what holds it together: it cracks, and loosens.
            m.cohesion / kappa
        } else if kappa > 0.0 && pressure > m.crush {
            // Crushed: it packs denser.
            -m.crush / kappa
        } else {
            tr
        };
        // What it lost in volume, or gained, it keeps: packed or loosened.
        p.jp *= js::exp(tr - new_tr);
        p.jp = js::min(js::max(p.jp, 0.3), 3.0);
        let mean = tr / d;
        let mut dev = [0.0; 3];
        let mut dev_len2 = 0.0;
        for i in 0..dims {
            dev[i] = eps[i] - mean;
            dev_len2 += dev[i] * dev[i];
        }
        let dev_len = dev_len2.sqrt();
        let mut out = [0.0; 3];
        if cracked {
            // Cracked: what's left holds no shape.
            for i in 0..dims {
                out[i] = new_tr / d;
            }
        } else {
            // Friction: sheared harder than its pressure (and cohesion) holds, it slides to the edge of what it holds.
            let shifted = new_tr - m.cohesion / js::max(kappa, 1e-30);
            let gamma = if m.mu > 0.0 { dev_len + (d * m.lambda + 2.0 * m.mu) / (2.0 * m.mu) * shifted * m.alpha } else { 0.0 };
            let scale = if gamma > 0.0 && dev_len > 1e-30 { js::max(0.0, 1.0 - gamma / dev_len) } else { 1.0 };
            for i in 0..dims {
                out[i] = new_tr / d + dev[i] * scale;
            }
        }
        let mut sd = [1.0; 3];
        for i in 0..dims {
            sd[i] = js::exp(out[i]) * s[i].signum();
        }
        p.f = mul(&mul(&u, &diag(sd)), &transpose(&v));
        // What giving way turned to heat: the stress it gave way at, times how far it gave (plastic work). Not the
        // energy its stretching stored before less after: a step stretches it a little past where its stress did work,
        // and that wasn't paid for.
        let out_tr: f64 = out[..dims].iter().sum();
        let mut w = 0.0;
        for i in 0..dims {
            w += (2.0 * m.mu * out[i] + m.lambda * out_tr) * (eps[i] - out[i]);
        }
        work = Some(w);
    }
    let (_, after) = stress(p, m, dims);
    p.psi = after;
    p.volume * work.unwrap_or(before - after)
}

/// The energy a point's stretching stores, kg·(m/tick)²: as of its last step, which is when it was last stretched.
pub fn stored(p: &Point, _dims: usize) -> f64 {
    p.volume * p.psi
}

// The grid

/// What mana holds in one cell for one weave: its particles there (indices into world.particles), and the body whose
/// hand holds them, if any. The mana pulls matter of its own parts toward its own speed, as hard as `bind` lets it.
#[derive(Clone, Debug)]
pub struct Grip {
    pub cell: usize,
    pub weave: u32,
    pub particles: Vec<usize>,
    pub hand: Option<usize>,
}

struct Grid {
    nx: usize,
    ny: usize,
    nz: usize,
    mass: Vec<f64>,
    mom: Vec<f64>,
    /// The momentum the points bring, without what their stress adds: how much motion the transfer keeps.
    bare: Vec<f64>,
    vel: Vec<f64>,
    /// Touched by a point that's awake: these nodes move.
    active: Vec<bool>,
    /// Touched by any point this step.
    seen: Vec<bool>,
    touched: Vec<usize>,
    active_list: Vec<usize>,
}

impl Grid {
    fn new(world: &World) -> Grid {
        let nx = world.w + 3;
        let ny = world.h + 3;
        let nz = if world.d == 1 { 1 } else { world.d + 3 };
        let n = nx * ny * nz;
        Grid {
            nx,
            ny,
            nz,
            mass: vec![0.0; n],
            mom: vec![0.0; n * 3],
            bare: vec![0.0; n * 3],
            vel: vec![0.0; n * 3],
            active: vec![false; n],
            seen: vec![false; n],
            touched: Vec::new(),
            active_list: Vec::new(),
        }
    }

    fn shape(&self) -> (usize, usize, usize) {
        (self.nx, self.ny, self.nz)
    }

    /// The grid coordinates of a node, each from −1.
    fn coords(&self, n: usize) -> (i64, i64, i64) {
        let i = (n % self.nx) as i64 - 1;
        let j = ((n / self.nx) % self.ny) as i64 - 1;
        let k = if self.nz == 1 { 0 } else { (n / (self.nx * self.ny)) as i64 - 1 };
        (i, j, k)
    }

    fn clear(&mut self) {
        for &n in &self.touched {
            self.mass[n] = 0.0;
            self.mom[n * 3] = 0.0;
            self.mom[n * 3 + 1] = 0.0;
            self.mom[n * 3 + 2] = 0.0;
            self.bare[n * 3] = 0.0;
            self.bare[n * 3 + 1] = 0.0;
            self.bare[n * 3 + 2] = 0.0;
            self.vel[n * 3] = 0.0;
            self.vel[n * 3 + 1] = 0.0;
            self.vel[n * 3 + 2] = 0.0;
            self.active[n] = false;
            self.seen[n] = false;
        }
        for &n in &self.active_list {
            self.active[n] = false;
        }
        self.touched.clear();
        self.active_list.clear();
    }
}

/// The nodes around a point, and how much of it each takes (quadratic B-splines): base node, and each offset's weight
/// and distance.
struct Stencil {
    base: [i64; 3],
    w: [[f64; 3]; 3],
    fx: [f64; 3],
}

fn stencil(pos: &Vec3, dx: f64, dims: usize) -> Stencil {
    let mut base = [0i64; 3];
    let mut w = [[0.0; 3]; 3];
    let mut fx = [0.0; 3];
    for a in 0..dims {
        let x = pos[a] / dx;
        let b = (x - 0.5).floor();
        let f = x - b;
        base[a] = b as i64;
        fx[a] = f;
        w[a] = [0.5 * (1.5 - f) * (1.5 - f), 0.75 - (f - 1.0) * (f - 1.0), 0.5 * (f - 0.5) * (f - 0.5)];
    }
    if dims == 2 {
        w[2] = [1.0, 0.0, 0.0];
    }
    Stencil { base, w, fx }
}

/// The nodes around a point: (node, weight, distance from the point in metres), the first `n` of 27.
struct Around {
    n: usize,
    at: [(usize, f64, Vec3); 27],
}

fn around(shape: (usize, usize, usize), s: &Stencil, dx: f64, dims: usize) -> Around {
    let (nx, ny, nz) = shape;
    let mut out = Around { n: 0, at: [(0, 0.0, [0.0; 3]); 27] };
    let nk = if dims == 2 { 1 } else { 3 };
    for k in 0..nk {
        for j in 0..3 {
            for i in 0..3 {
                let wt = s.w[0][i] * s.w[1][j] * s.w[2][k];
                let d = [
                    (i as f64 - s.fx[0]) * dx,
                    (j as f64 - s.fx[1]) * dx,
                    if dims == 2 { 0.0 } else { (k as f64 - s.fx[2]) * dx },
                ];
                let kk = if nz == 1 { 0 } else { (s.base[2] + k as i64 + 1) as usize };
                let node = (kk * ny + (s.base[1] + j as i64 + 1) as usize) * nx + (s.base[0] + i as i64 + 1) as usize;
                out.at[out.n] = (node, wt, d);
                out.n += 1;
            }
        }
    }
    out
}

/// One tick of matter: as many steps as its fastest wave needs, each moving the points over the grid, and mana holding
/// what it grips. Then what has come to rest sleeps, and the world's cells learn where matter is.
pub fn step_matter(world: &mut World, grips: &[Grip]) {
    let ph = physics();
    if world.points.is_empty() {
        world.rasterize();
        return;
    }
    let dims = if world.d == 1 { 2 } else { 3 };
    let dx = world.cell;
    // Mana in a cell wakes the matter there: it's being held, or could be.
    let gripped: std::collections::HashSet<usize> = grips.iter().map(|g| g.cell).collect();
    let mut woke = false;
    for p in world.points.iter_mut() {
        if p.asleep {
            let c = world_cell(world.w, world.h, world.d, dx, &p.pos);
            if gripped.contains(&c) {
                p.asleep = false;
                p.still = 0;
                woke = true;
            }
        }
    }
    if woke {
        world.points_moved();
    }
    let awake = world.points.iter().any(|p| !p.asleep);
    if awake {
        let fastest =
            world.points.iter().filter(|p| !p.asleep).fold(0.0, |m: f64, p| js::max(m, js::hypot3(p.vel[0], p.vel[1], p.vel[2])));
        let steps = js::max(1.0, ((ph.matter_sound + fastest) / (ph.matter_cfl * dx)).ceil()) as usize;
        let dt = 1.0 / steps as f64;
        let mut grid = Grid::new(world);
        let mut struck = 0.0;
        for _ in 0..steps {
            struck += substep(world, &mut grid, dt, dims, grips);
        }
        world.warm("matter striking", struck);
    }
    rest(world);
    world.points_moved();
    world.rasterize();
}

/// The cell a point is in, clamped into the world.
fn world_cell(w: usize, h: usize, d: usize, dx: f64, p: &Vec3) -> usize {
    let c = |v: f64, n: usize| js::max(0.0, js::min(n as f64 - 1.0, (v / dx + 1e-7).floor())) as usize;
    (c(p[2], d) * h + c(p[1], h)) * w + c(p[0], w)
}

/// One substep. Returns the motion lost moving it to the grid and back: where motions meet on a node they're averaged,
/// as when matter strikes matter, and what's lost is heat.
fn substep(world: &mut World, g: &mut Grid, dt: f64, dims: usize, grips: &[Grip]) -> f64 {
    let ph = physics();
    let dx = world.cell;
    let inv = 4.0 / (dx * dx);
    let shape = g.shape();
    g.clear();
    // Which nodes move: those around a point that's awake.
    let n_points = world.points.len();
    for p in world.points.iter().filter(|p| !p.asleep) {
        let a = around(shape, &stencil(&p.pos, dx, dims), dx, dims);
        for &(n, _, _) in &a.at[..a.n] {
            if !g.active[n] {
                g.active[n] = true;
                g.active_list.push(n);
            }
        }
    }
    // Who tells them: every awake point, and every sleeping one beside a node that moves.
    let mut contributors: Vec<usize> = (0..n_points).filter(|&i| !world.points[i].asleep).collect();
    contributors.extend(asleep_near(world, g, dims));
    // P2G: mass, momentum and stress, from each point to its nodes.
    let mut materials: Vec<Option<Material>> = (0..n_points).map(|_| None).collect();
    let ke = |p: &Point| 0.5 * p.mass * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2]);
    let mut struck = 0.0;
    for &pi in &contributors {
        let p = &world.points[pi];
        struck += ke(p);
        let m = material(p);
        let (tau, _) = stress(p, &m, dims);
        materials[pi] = Some(m);
        let mut affine = [0.0; 9];
        for k in 0..9 {
            affine[k] = -dt * p.volume * inv * tau[k] + p.mass * p.c[k];
        }
        let a = around(shape, &stencil(&p.pos, dx, dims), dx, dims);
        for &(n, wt, d) in &a.at[..a.n] {
            if !g.seen[n] {
                g.seen[n] = true;
                g.touched.push(n);
            }
            g.mass[n] += wt * p.mass;
            for x in 0..3 {
                g.mom[n * 3 + x] +=
                    wt * (p.mass * p.vel[x] + affine[x * 3] * d[0] + affine[x * 3 + 1] * d[1] + affine[x * 3 + 2] * d[2]);
                let c = &p.c[x * 3..x * 3 + 3];
                g.bare[n * 3 + x] += wt * p.mass * (p.vel[x] + c[0] * d[0] + c[1] * d[1] + c[2] * d[2]);
            }
        }
    }
    g.touched.sort_unstable();
    // The grid moves: under its weight, against the world's edges. What doesn't move (nodes only sleeping matter
    // touches) gives what it was given to the ground.
    let (w, h, d) = (world.w as i64, world.h as i64, world.d as i64);
    for idx in 0..g.touched.len() {
        let n = g.touched[idx];
        let m = g.mass[n];
        if !g.active[n] || m <= 0.0 {
            for a in 0..3 {
                world.impulse.walls[a] -= g.mom[n * 3 + a];
            }
            continue;
        }
        let b = [g.bare[n * 3], g.bare[n * 3 + 1], g.bare[n * 3 + 2]];
        struck -= 0.5 * (b[0] * b[0] + b[1] * b[1] + b[2] * b[2]) / m;
        world.impulse.gravity[1] -= m * ph.gravity * dt;
        let mut v = [g.mom[n * 3] / m, g.mom[n * 3 + 1] / m - ph.gravity * dt, g.mom[n * 3 + 2] / m];
        let (i, j, k) = g.coords(n);
        let before = v;
        if j <= 0 && v[1] < 0.0 {
            // The world's floor holds up what's on it (and lets it go).
            v[1] = 0.0;
        }
        if (i <= 0 && v[0] < 0.0) || (i >= w && v[0] > 0.0) {
            v[0] = 0.0;
        }
        if j >= h && v[1] > 0.0 {
            v[1] = 0.0;
        }
        if dims == 3 && ((k <= 0 && v[2] < 0.0) || (k >= d && v[2] > 0.0)) {
            v[2] = 0.0;
        }
        if dims == 2 {
            v[2] = 0.0;
        }
        for a in 0..3 {
            if v[a] != before[a] {
                world.impulse.walls[a] += m * (v[a] - before[a]);
            }
        }
        // What a step does to a node's motion, it does at once: half its mass times the change squared is lost, as when
        // it strikes something (or the world's edge). A step's work is its force times where it ends up, so the work of
        // weight is the height the matter loses, of stress what its stretching stores, and of an edge nothing.
        let dv = [v[0] - b[0] / m, v[1] - b[1] / m, v[2] - b[2] / m];
        struck += 0.5 * m * (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]);
        g.vel[n * 3] = v[0];
        g.vel[n * 3 + 1] = v[1];
        g.vel[n * 3 + 2] = v[2];
    }
    // Mana holds what it grips.
    hold(world, g, dt, dims, grips);
    for &n in &g.touched {
        if g.active[n] && g.mass[n] > 0.0 {
            let v = &g.vel[n * 3..n * 3 + 3];
            struck += 0.5 * g.mass[n] * (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
        }
    }
    // G2P: each point takes its motion back from its nodes, and is stretched by how they move.
    let mut yield_heat = 0.0;
    let mut wall = [0.0; 3];
    let wake = ph.wake_speed;
    let (wx, hy, dz) = (world.w as f64 * dx, world.h as f64 * dx, world.d as f64 * dx);
    for &pi in &contributors {
        let p = &world.points[pi];
        let a = around(shape, &stencil(&p.pos, dx, dims), dx, dims);
        let mut v = [0.0; 3];
        let mut b = [0.0; 9];
        for &(n, wt, dpos) in &a.at[..a.n] {
            let gv = [g.vel[n * 3], g.vel[n * 3 + 1], g.vel[n * 3 + 2]];
            for x in 0..3 {
                v[x] += wt * gv[x];
                for c in 0..3 {
                    b[x * 3 + c] += wt * gv[x] * dpos[c];
                }
            }
        }
        let p = &mut world.points[pi];
        if p.asleep {
            if js::hypot3(v[0], v[1], v[2]) <= wake {
                // Still asleep: the ground under it takes what it would have moved with. What its stress did to the
                // nodes that moved, it did as the ground does, without stretching: heat.
                for x in 0..3 {
                    wall[x] += p.mass * v[x];
                }
                let (tau, _) = stress(p, materials[pi].as_ref().unwrap(), dims);
                let mut tc = 0.0;
                for k in 0..9 {
                    tc += tau[k] * inv * b[k];
                }
                struck += dt * p.volume * tc;
                continue;
            }
            p.asleep = false;
            p.still = 0;
        }
        p.vel = v;
        for k in 0..9 {
            p.c[k] = inv * b[k];
        }
        let mut grad = IDENTITY;
        for k in 0..9 {
            grad[k] += dt * p.c[k];
        }
        p.f = mul(&grad, &p.f);
        for x in 0..3 {
            p.pos[x] += dt * v[x];
        }
        // Inside the world.
        p.pos[0] = js::min(js::max(p.pos[0], 0.0), wx - 1e-9);
        p.pos[1] = js::min(js::max(p.pos[1], 0.0), hy - 1e-9);
        if dims == 3 {
            p.pos[2] = js::min(js::max(p.pos[2], 0.0), dz - 1e-9);
        }
        let m = materials[pi].take().unwrap();
        yield_heat += give_way(p, &m, dims);
    }
    for x in 0..3 {
        world.impulse.walls[x] -= wall[x];
    }
    world.warm("matter giving way", yield_heat);
    world.points_moved();
    struck - contributors.iter().map(|&pi| ke(&world.points[pi])).sum::<f64>()
}

/// The sleeping points beside a node that moves: they press on it, and it may wake them.
fn asleep_near(world: &mut World, g: &Grid, dims: usize) -> Vec<usize> {
    let dx = world.cell;
    let (w, h, d) = (world.w as i64, world.h as i64, world.d as i64);
    world.index_asleep();
    // The cells whose points could reach a node that moves: two cells below it, to one above, on each axis.
    let mut cells: Vec<usize> = Vec::new();
    let mut marked = std::collections::HashSet::new();
    for &n in &g.active_list {
        let (i, j, k) = g.coords(n);
        let (k0, k1) = if dims == 2 { (0, 0) } else { (k - 2, k + 1) };
        for z in k0..=k1 {
            for y in j - 2..=j + 1 {
                for x in i - 2..=i + 1 {
                    if x < 0 || y < 0 || z < 0 || x >= w || y >= h || z >= d {
                        continue;
                    }
                    let c = ((z * h + y) * w + x) as usize;
                    if marked.insert(c) {
                        cells.push(c);
                    }
                }
            }
        }
    }
    cells.sort_unstable();
    let shape = g.shape();
    let mut out = Vec::new();
    for c in cells {
        for &pi in world.asleep_in(c) {
            let p = &world.points[pi];
            let a = around(shape, &stencil(&p.pos, dx, dims), dx, dims);
            if a.at[..a.n].iter().any(|&(n, _, _)| g.active[n]) {
                out.push(pi);
            }
        }
    }
    out
}

/// Mana holds matter by force (PLAN step 2, *influence*): in each cell, a weave's free mana pulls the matter of its own
/// parts there toward its own speed, as hard as `bind` kilograms a M would weigh, and the matter pulls it back. In a
/// hand, the hand holds the mana still, and bears what it holds, up to the same grip. Pulled harder than that, the
/// matter slips. What evening out their speeds takes from their motion is heat.
fn hold(world: &mut World, g: &mut Grid, dt: f64, dims: usize, grips: &[Grip]) {
    let ph = physics();
    if grips.is_empty() {
        return;
    }
    let dx = world.cell;
    let shape = g.shape();
    let mut by_cell: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, p) in world.points.iter().enumerate() {
        if !p.asleep {
            by_cell.entry(world_cell(world.w, world.h, world.d, dx, &p.pos)).or_default().push(i);
        }
    }
    let node_ke = |g: &Grid, n: usize| {
        0.5 * g.mass[n]
            * (g.vel[n * 3] * g.vel[n * 3] + g.vel[n * 3 + 1] * g.vel[n * 3 + 1] + g.vel[n * 3 + 2] * g.vel[n * 3 + 2])
    };
    let mut heat = 0.0;
    for grip in grips {
        let Some(points) = by_cell.get(&grip.cell) else { continue };
        // How hard the mana grips each part.
        let mut can = [0.0; 4];
        for &i in &grip.particles {
            for k in 0..4 {
                can[k] += world.particles[i].free[k] * ph.bind * ph.gravity;
            }
        }
        for k in 0..4 {
            if can[k] <= 0.0 {
                continue;
            }
            // The matter of part k there: its mass, and its speed as the grid moves it.
            let mut mk = 0.0;
            let mut pk = [0.0; 3];
            for &i in points {
                let p = &world.points[i];
                let share = p.parts[k] * ph.mana_mass[k];
                if share > 0.0 {
                    let v = point_velocity(g, shape, &p.pos, dx, dims);
                    mk += share;
                    for a in 0..3 {
                        pk[a] += share * v[a];
                    }
                }
            }
            if mk <= 0.0 {
                continue;
            }
            let vm = [pk[0] / mk, pk[1] / mk, pk[2] / mk];
            // The mana: still in a hand (as if it had no end of mass), or as it moves.
            let mut ma = 0.0;
            let mut pa = [0.0; 3];
            for &i in &grip.particles {
                let q = &world.particles[i];
                let m = super::world::mass_of(q);
                ma += m;
                for a in 0..3 {
                    pa[a] += m * q.vel[a];
                }
            }
            let (va, mu) = match grip.hand {
                Some(_) => ([0.0; 3], mk),
                None if ma > 0.0 => ([pa[0] / ma, pa[1] / ma, pa[2] / ma], ma * mk / (ma + mk)),
                None => continue,
            };
            // The impulse that would bring them to one speed, as much of it as the grip can give in this step.
            let mut j = [mu * (va[0] - vm[0]), mu * (va[1] - vm[1]), mu * (va[2] - vm[2])];
            if dims == 2 {
                j[2] = 0.0;
            }
            let size = js::hypot3(j[0], j[1], j[2]);
            let most = can[k] * dt;
            if size > most {
                for a in 0..3 {
                    j[a] *= most / size;
                }
            }
            if js::hypot3(j[0], j[1], j[2]) == 0.0 {
                continue;
            }
            // To the matter, through the nodes around it, each by how much of it is there.
            let mut push: HashMap<usize, f64> = HashMap::new();
            for &i in points {
                let p = &world.points[i];
                let share = p.parts[k] * ph.mana_mass[k];
                if share <= 0.0 {
                    continue;
                }
                let a = around(shape, &stencil(&p.pos, dx, dims), dx, dims);
                for &(n, wt, _) in &a.at[..a.n] {
                    if g.active[n] && g.mass[n] > 0.0 {
                        *push.entry(n).or_insert(0.0) += wt * share / mk;
                    }
                }
            }
            let mut nodes: Vec<(usize, f64)> = push.into_iter().collect();
            nodes.sort_unstable_by_key(|x| x.0);
            let total_share: f64 = nodes.iter().map(|x| x.1).sum();
            if total_share <= 0.0 {
                continue;
            }
            let mut before = 0.0;
            let mut after = 0.0;
            for &(n, f) in &nodes {
                before += node_ke(g, n);
                let m = g.mass[n];
                for a in 0..3 {
                    g.vel[n * 3 + a] += f / total_share * j[a] / m;
                }
                after += node_ke(g, n);
            }
            // And back to the mana, or to the hand.
            match grip.hand {
                Some(body) => {
                    let b = &mut world.bodies[body];
                    let kb = 0.5 * b.mass * (b.vel[0] * b.vel[0] + b.vel[2] * b.vel[2]);
                    for a in [0, 2] {
                        b.vel[a] -= j[a] / b.mass;
                    }
                    let ka = 0.5 * b.mass * (b.vel[0] * b.vel[0] + b.vel[2] * b.vel[2]);
                    world.impulse.walls[1] += j[1];
                    before += kb;
                    after += ka;
                }
                None => {
                    for &i in &grip.particles {
                        let q = &mut world.particles[i];
                        before += ke(q);
                        for a in 0..3 {
                            q.vel[a] -= j[a] / ma;
                        }
                        after += ke(q);
                    }
                }
            }
            heat += before - after;
        }
    }
    world.warm("holding", heat);
}

/// Mana holds what it grips, between matter's steps too (while the mana moves): in each cell, a weave's free mana and
/// the matter of its own parts there pull each other toward one speed, as hard as the grip lets them over `dt` ticks.
/// The mana's weight rests on its matter this way. A hand holds its mana still; there, the matter step does it all.
pub fn cling(world: &mut World, grips: &[Grip], dt: f64) {
    let ph = physics();
    let dims = if world.d == 1 { 2 } else { 3 };
    let mut heat = 0.0;
    for grip in grips {
        if grip.hand.is_some() {
            continue;
        }
        let points = world.points_in(grip.cell).to_vec();
        if points.is_empty() {
            continue;
        }
        for k in 0..4 {
            let can: f64 = grip.particles.iter().map(|&i| world.particles[i].free[k]).sum::<f64>() * ph.bind * ph.gravity;
            if can <= 0.0 {
                continue;
            }
            let mut mk = 0.0;
            let mut pk = [0.0; 3];
            for &i in &points {
                let p = &world.points[i];
                let share = p.parts[k] * ph.mana_mass[k];
                mk += share;
                for a in 0..3 {
                    pk[a] += share * p.vel[a];
                }
            }
            let mut ma = 0.0;
            let mut pa = [0.0; 3];
            for &i in &grip.particles {
                let q = &world.particles[i];
                let m = super::world::mass_of(q);
                ma += m;
                for a in 0..3 {
                    pa[a] += m * q.vel[a];
                }
            }
            if mk <= 0.0 || ma <= 0.0 {
                continue;
            }
            let mu = ma * mk / (ma + mk);
            let mut j = [0.0; 3];
            for a in 0..3 {
                j[a] = mu * (pa[a] / ma - pk[a] / mk);
            }
            if dims == 2 {
                j[2] = 0.0;
            }
            let size = js::hypot3(j[0], j[1], j[2]);
            let most = can * dt;
            if size > most {
                for a in 0..3 {
                    j[a] *= most / size;
                }
            }
            if js::hypot3(j[0], j[1], j[2]) == 0.0 {
                continue;
            }
            let mut before = 0.0;
            let mut after = 0.0;
            for &i in &points {
                let p = &mut world.points[i];
                let share = p.parts[k] * ph.mana_mass[k];
                if share <= 0.0 {
                    continue;
                }
                before += 0.5 * p.mass * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2]);
                for a in 0..3 {
                    p.vel[a] += j[a] * share / mk / p.mass;
                }
                after += 0.5 * p.mass * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2]);
                if p.asleep && js::hypot3(p.vel[0], p.vel[1], p.vel[2]) > ph.wake_speed {
                    p.asleep = false;
                    p.still = 0;
                }
            }
            for &i in &grip.particles {
                let q = &mut world.particles[i];
                let m = super::world::mass_of(q);
                before += 0.5 * m * (q.vel[0] * q.vel[0] + q.vel[1] * q.vel[1] + q.vel[2] * q.vel[2]);
                for a in 0..3 {
                    q.vel[a] -= j[a] / ma;
                }
                after += 0.5 * m * (q.vel[0] * q.vel[0] + q.vel[1] * q.vel[1] + q.vel[2] * q.vel[2]);
            }
            heat += before - after;
        }
    }
    world.warm("holding", heat);
}

fn ke(p: &super::world::Particle) -> f64 {
    0.5 * super::world::mass_of(p) * (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1] + p.vel[2] * p.vel[2])
}

/// How the grid moves at a point.
fn point_velocity(g: &Grid, shape: (usize, usize, usize), pos: &Vec3, dx: f64, dims: usize) -> Vec3 {
    let a = around(shape, &stencil(pos, dx, dims), dx, dims);
    let mut v = [0.0; 3];
    for &(n, wt, _) in &a.at[..a.n] {
        for x in 0..3 {
            v[x] += wt * g.vel[n * 3 + x];
        }
    }
    v
}

/// What has come to rest, and nothing holds, sleeps: what little motion it had is heat, and the ground takes its
/// momentum.
fn rest(world: &mut World) {
    let ph = physics();
    let mut heat = 0.0;
    let mut slept = false;
    for p in world.points.iter_mut() {
        if p.asleep {
            continue;
        }
        let speed = js::hypot3(p.vel[0], p.vel[1], p.vel[2]);
        if speed < ph.sleep_speed {
            p.still += 1;
        } else {
            p.still = 0;
        }
        if p.still >= ph.rest_ticks {
            heat += 0.5 * p.mass * speed * speed;
            for a in 0..3 {
                world.impulse.walls[a] -= p.mass * p.vel[a];
            }
            p.vel = [0.0; 3];
            p.c = [0.0; 9];
            p.asleep = true;
            slept = true;
        }
    }
    world.warm("settling", heat);
    if slept {
        world.points_moved();
    }
}

/// The ground's own weight pressed into it, as it would have settled: a point `depth` metres under the surface is
/// squeezed as much as what's above it weighs.
pub fn settled(p: &mut Point, depth: f64, dims: usize) {
    let ph = physics();
    let m = material(p);
    let rho = if p.volume > 0.0 { p.mass / p.volume } else { 0.0 };
    let pressure = rho * ph.gravity * depth;
    if m.liquid {
        let j = 1.0 - pressure / js::max(m.bulk, 1e-30);
        p.f = diag([1.0, j, 1.0]);
    } else {
        let modulus = m.lambda + 2.0 * m.mu;
        p.f = diag([1.0, js::exp(-pressure / js::max(modulus, 1e-30)), 1.0]);
    }
    p.psi = stress(p, &m, dims).1;
}

/// All the matter in points: mass and parts, for the ledgers.
pub fn total_parts(points: &[Point]) -> Parts {
    let mut out = zero();
    for p in points {
        for k in 0..4 {
            out[k] += p.parts[k];
        }
    }
    out
}

/// Some matter's mass in M, all parts.
pub fn mana_in(points: &[Point]) -> f64 {
    points.iter().fold(0.0, |s, p| s + total(&p.parts))
}
