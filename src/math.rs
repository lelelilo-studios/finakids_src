//! Small math helpers: RNG, noise, easing, smoothing and color utilities.

use glam::{Quat, Vec2, Vec3};

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = std::f32::consts::TAU;

/// Deterministic xorshift64* random generator.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        Rng::new(1)
    }
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn int(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    /// Approximately normal distribution (Irwin-Hall with 4 samples).
    pub fn normal(&mut self) -> f32 {
        (self.f() + self.f() + self.f() + self.f() - 2.0) * 1.732
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.int(items.len())]
    }
}

pub fn hash11(n: f32) -> f32 {
    let x = (n * 127.1).sin() * 43758.547;
    x - x.floor()
}

pub fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

pub fn hash3(p: Vec3) -> f32 {
    let h = hash_u32(
        (p.x.floor() as i32 as u32)
            .wrapping_mul(73856093)
            ^ (p.y.floor() as i32 as u32).wrapping_mul(19349663)
            ^ (p.z.floor() as i32 as u32).wrapping_mul(83492791),
    );
    (h & 0xffffff) as f32 / 16777215.0
}

fn lattice(ix: i32, iy: i32, iz: i32) -> f32 {
    let h = hash_u32(
        (ix as u32).wrapping_mul(73856093)
            ^ (iy as u32).wrapping_mul(19349663)
            ^ (iz as u32).wrapping_mul(83492791),
    );
    (h & 0xffffff) as f32 / 16777215.0 * 2.0 - 1.0
}

/// Smooth value noise in [-1, 1].
pub fn noise3(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let (ix, iy, iz) = (i.x as i32, i.y as i32, i.z as i32);
    let mut v = [0.0f32; 8];
    for (k, item) in v.iter_mut().enumerate() {
        let dx = (k & 1) as i32;
        let dy = ((k >> 1) & 1) as i32;
        let dz = ((k >> 2) & 1) as i32;
        *item = lattice(ix + dx, iy + dy, iz + dz);
    }
    let x00 = lerp(v[0], v[1], u.x);
    let x10 = lerp(v[2], v[3], u.x);
    let x01 = lerp(v[4], v[5], u.x);
    let x11 = lerp(v[6], v[7], u.x);
    let y0 = lerp(x00, x10, u.y);
    let y1 = lerp(x01, x11, u.y);
    lerp(y0, y1, u.z)
}

pub fn fbm3(p: Vec3, octaves: u32) -> f32 {
    let mut a = 0.5;
    let mut sum = 0.0;
    let mut q = p;
    for _ in 0..octaves {
        sum += a * noise3(q);
        q = q * 2.03 + Vec3::new(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    sum
}

/// 1D smooth noise for procedural animation.
pub fn noise1(t: f32, seed: f32) -> f32 {
    noise3(Vec3::new(t, seed * 13.7, seed * 3.1))
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp01((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

pub fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    c + (d - c) * clamp01((x - a) / (b - a))
}

/// Exponential smoothing factor for frame-rate independent damping.
#[inline]
pub fn damp_factor(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

pub fn damp(a: f32, b: f32, rate: f32, dt: f32) -> f32 {
    lerp(a, b, damp_factor(rate, dt))
}

pub fn damp_v3(a: Vec3, b: Vec3, rate: f32, dt: f32) -> Vec3 {
    a.lerp(b, damp_factor(rate, dt))
}

pub fn damp_quat(a: Quat, b: Quat, rate: f32, dt: f32) -> Quat {
    a.slerp(b, damp_factor(rate, dt))
}

pub fn wrap_angle(a: f32) -> f32 {
    let mut a = (a + PI) % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a - PI
}

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + wrap_angle(b - a) * t
}

pub fn damp_angle(a: f32, b: f32, rate: f32, dt: f32) -> f32 {
    lerp_angle(a, b, damp_factor(rate, dt))
}

/// Critically damped spring for f32 values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
}

impl Spring {
    pub fn new(x: f32) -> Self {
        Spring { x, v: 0.0 }
    }
    /// `freq` in Hz-ish, `zeta` damping ratio (1 = critical).
    pub fn update(&mut self, target: f32, freq: f32, zeta: f32, dt: f32) -> f32 {
        let w = TAU * freq;
        let dt = dt.min(0.05);
        let f = w * w * (target - self.x) - 2.0 * zeta * w * self.v;
        self.v += f * dt;
        self.x += self.v * dt;
        self.x
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Spring3 {
    pub x: Vec3,
    pub v: Vec3,
}

impl Spring3 {
    pub fn new(x: Vec3) -> Self {
        Spring3 { x, v: Vec3::ZERO }
    }
    pub fn update(&mut self, target: Vec3, freq: f32, zeta: f32, dt: f32) -> Vec3 {
        let w = TAU * freq;
        let dt = dt.min(0.05);
        let f = w * w * (target - self.x) - 2.0 * zeta * w * self.v;
        self.v += f * dt;
        self.x += self.v * dt;
        self.x
    }
}

// ---------------------------------------------------------------- easing

pub fn ease_in_out(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn ease_out(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t
}

pub fn ease_out_back(t: f32) -> f32 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_in_out_sine(t: f32) -> f32 {
    let t = clamp01(t);
    -((PI * t).cos() - 1.0) / 2.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    In,
    Out,
    InOut,
    OutBack,
    Sine,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => clamp01(t),
            Ease::In => ease_in(t),
            Ease::Out => ease_out(t),
            Ease::InOut => ease_in_out(t),
            Ease::OutBack => ease_out_back(t),
            Ease::Sine => ease_in_out_sine(t),
        }
    }
}

// ---------------------------------------------------------------- color

/// Parses 0xRRGGBB into sRGB bytes.
pub const fn rgb(hex: u32) -> [u8; 4] {
    [
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
        255,
    ]
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn color_lin(hex: u32) -> Vec3 {
    let c = rgb(hex);
    Vec3::new(
        srgb_to_linear(c[0] as f32 / 255.0),
        srgb_to_linear(c[1] as f32 / 255.0),
        srgb_to_linear(c[2] as f32 / 255.0),
    )
}

pub fn mix_rgb(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        lerp(a[0] as f32, b[0] as f32, t) as u8,
        lerp(a[1] as f32, b[1] as f32, t) as u8,
        lerp(a[2] as f32, b[2] as f32, t) as u8,
        lerp(a[3] as f32, b[3] as f32, t) as u8,
    ]
}

pub fn scale_rgb(a: [u8; 4], s: f32) -> [u8; 4] {
    [
        (a[0] as f32 * s).clamp(0.0, 255.0) as u8,
        (a[1] as f32 * s).clamp(0.0, 255.0) as u8,
        (a[2] as f32 * s).clamp(0.0, 255.0) as u8,
        a[3],
    ]
}

// ---------------------------------------------------------------- geometry

/// Rotation that maps `from` basis (dir, up-hint) to `to` basis.
pub fn quat_from_to_basis(from_dir: Vec3, from_up: Vec3, to_dir: Vec3, to_up: Vec3) -> Quat {
    let basis = |d: Vec3, u: Vec3| {
        let d = d.normalize_or(Vec3::Y);
        let mut side = d.cross(u);
        if side.length_squared() < 1e-8 {
            side = d.any_orthonormal_vector();
        }
        let side = side.normalize();
        let up = side.cross(d);
        glam::Mat3::from_cols(d, up, side)
    };
    let a = basis(from_dir, from_up);
    let b = basis(to_dir, to_up);
    Quat::from_mat3(&(b * a.transpose())).normalize()
}

pub fn yaw_quat(yaw: f32) -> Quat {
    Quat::from_rotation_y(yaw)
}

/// Yaw angle for facing direction on XZ plane (0 = +Z).
pub fn yaw_of(dir: Vec3) -> f32 {
    dir.x.atan2(dir.z)
}

pub fn v2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

pub fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Formats an amount of money in Chilean style: $30.000
pub fn money(v: i64) -> String {
    let neg = v < 0;
    let s = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if neg {
        format!("-${out}")
    } else {
        format!("${out}")
    }
}

pub fn money_signed(v: i64) -> String {
    if v > 0 {
        format!("+{}", money(v))
    } else {
        money(v)
    }
}
