//! Small DSP toolkit: wavetables, filters, envelopes, reverb and a limiter.
//! Everything is allocation-free once constructed.

use std::sync::OnceLock;

pub const TAU: f32 = std::f32::consts::TAU;

/// xorshift32 noise / random source.
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9) | 1)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in [0, 1).
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
    /// Uniform in [-1, 1).
    #[inline]
    pub fn bi(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn int(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u32() as usize) % n
        }
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.int(v.len())]
    }
}

// ------------------------------------------------------------------ wavetables

const TN: usize = 2048;
pub type Table = [f32; TN + 1];

pub struct Tables {
    pub sine: Box<Table>,
    /// Soft, band-limited saw (8 harmonics, gently rolled off): warm pads.
    pub soft_saw: Box<Table>,
    /// Sine plus low harmonics: bass that still speaks on small speakers.
    pub bass: Box<Table>,
    /// Hollow, flute-like tone (odd harmonics): melody.
    pub hollow: Box<Table>,
    /// Tine-piano carrier: a few even/odd harmonics for body.
    pub ep: Box<Table>,
}

fn table(f: impl Fn(f32) -> f32) -> Box<Table> {
    let mut t = Box::new([0.0f32; TN + 1]);
    for (i, v) in t.iter_mut().enumerate() {
        *v = f(i as f32 / TN as f32 * TAU);
    }
    let peak = t.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
    for v in t.iter_mut() {
        *v /= peak;
    }
    t
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| Tables {
        sine: table(|x| x.sin()),
        soft_saw: table(|x| {
            let mut s = 0.0;
            for k in 1..=8 {
                let k = k as f32;
                s += (k * x).sin() / k * (1.0 - (k - 1.0) / 10.0);
            }
            s
        }),
        bass: table(|x| x.sin() + 0.45 * (2.0 * x).sin() + 0.22 * (3.0 * x).sin() + 0.1 * (4.0 * x).sin() + 0.04 * (5.0 * x).sin()),
        hollow: table(|x| x.sin() + 0.18 * (3.0 * x).sin() + 0.06 * (5.0 * x).sin()),
        ep: table(|x| x.sin() + 0.3 * (2.0 * x).sin() + 0.16 * (3.0 * x).sin() + 0.09 * (4.0 * x).sin() + 0.04 * (6.0 * x).sin()),
    })
}

/// Wraps any phase (cycles) into [0, 1) without calling `floor`.
#[inline(always)]
pub fn wrap01(p: f32) -> f32 {
    let f = p - (p as i32) as f32;
    if f < 0.0 {
        f + 1.0
    } else {
        f
    }
}

/// Table lookup with linear interpolation; `p` must already be in [0, 1).
#[inline(always)]
pub fn lut(t: &Table, p: f32) -> f32 {
    let x = p * TN as f32;
    let i = (x as usize) & (TN - 1);
    let fr = x - i as f32;
    let a = t[i];
    a + (t[i + 1] - a) * fr
}

/// Table lookup for any phase in cycles.
#[inline(always)]
pub fn lookup(t: &Table, phase: f32) -> f32 {
    lut(t, wrap01(phase))
}

/// Table lookup with a 32-bit fixed-point phase (wraps for free).
#[inline(always)]
pub fn lutp(t: &Table, ph: u32) -> f32 {
    let i = (ph >> 21) as usize;
    let fr = (ph & 0x1F_FFFF) as f32 * (1.0 / 2_097_152.0);
    let a = t[i];
    a + (t[i + 1] - a) * fr
}

/// Phase increment per sample for `lutp`.
#[inline]
pub fn phase_inc(freq: f32, sr: f32) -> u32 {
    ((freq / sr) as f64 * 4_294_967_296.0) as u32
}

#[inline]
pub fn sin01(phase: f32) -> f32 {
    lookup(&tables().sine, phase)
}

pub fn midi_hz(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// Per-sample multiplier for an exponential decay with time constant `tau` seconds.
pub fn decay_mul(tau: f32, sr: f32) -> f32 {
    (-1.0 / (tau.max(1e-4) * sr)).exp()
}

// ------------------------------------------------------------------ filters

/// One-pole lowpass.
#[derive(Clone, Copy, Default)]
pub struct OnePole {
    pub a: f32,
    pub z: f32,
}

impl OnePole {
    pub fn new(fc: f32, sr: f32) -> OnePole {
        let mut o = OnePole::default();
        o.set(fc, sr);
        o
    }
    pub fn set(&mut self, fc: f32, sr: f32) {
        self.a = 1.0 - (-TAU * fc.min(sr * 0.45) / sr).exp();
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        x - self.lp(x)
    }
}

/// Topology-preserving state variable filter (Simper).
#[derive(Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    pub fn new(fc: f32, q: f32, sr: f32) -> Svf {
        let mut f = Svf::default();
        f.set(fc, q, sr);
        f
    }
    pub fn set(&mut self, fc: f32, q: f32, sr: f32) {
        let g = (std::f32::consts::PI * fc.clamp(10.0, sr * 0.45) / sr).tan();
        self.k = 1.0 / q.max(0.05);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }
    /// Returns (low, band, high).
    #[inline]
    pub fn run(&mut self, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1, x - self.k * v1 - v2)
    }
    #[inline]
    pub fn lp(&mut self, x: f32) -> f32 {
        self.run(x).0
    }
    #[inline]
    pub fn bp(&mut self, x: f32) -> f32 {
        self.run(x).1
    }
    #[inline]
    pub fn hp(&mut self, x: f32) -> f32 {
        self.run(x).2
    }
}

/// Brown-ish noise: leaky integrated white noise.
#[derive(Clone)]
pub struct Brown {
    z: f32,
}

impl Brown {
    pub fn new() -> Brown {
        Brown { z: 0.0 }
    }
    #[inline]
    pub fn next(&mut self, white: f32) -> f32 {
        self.z = self.z * 0.996 + white * 0.06;
        self.z
    }
}

// ------------------------------------------------------------------ reverb

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    fb: f32,
    damp: f32,
    store: f32,
}

impl Comb {
    fn new(len: usize, fb: f32, damp: f32) -> Comb {
        Comb {
            buf: vec![0.0; len.max(1)],
            pos: 0,
            fb,
            damp,
            store: 0.0,
        }
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.buf[self.pos];
        self.store = y * (1.0 - self.damp) + self.store * self.damp;
        self.buf[self.pos] = x + self.store * self.fb;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        y
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Allpass {
        Allpass {
            buf: vec![0.0; len.max(1)],
            pos: 0,
        }
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let b = self.buf[self.pos];
        let y = b - x;
        self.buf[self.pos] = x + b * 0.5;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
        y
    }
}

/// Stereo Freeverb-style room: 8 combs + 4 allpasses per side. The dense comb
/// bank keeps sustained chords from being coloured differently left and right.
pub struct Reverb {
    cl: [Comb; 8],
    cr: [Comb; 8],
    al: [Allpass; 4],
    ar: [Allpass; 4],
    pre: OnePole,
}

const COMB_TUNING: [f32; 8] = [1116.0, 1188.0, 1277.0, 1356.0, 1422.0, 1491.0, 1557.0, 1617.0];
const ALLPASS_TUNING: [f32; 4] = [556.0, 441.0, 341.0, 225.0];

impl Reverb {
    pub fn new(sr: f32, size: f32, damp: f32) -> Reverb {
        let s = sr / 44_100.0;
        let fb = 0.70 + 0.26 * size.clamp(0.0, 1.0);
        let spread = 23.0;
        Reverb {
            cl: COMB_TUNING.map(|n| Comb::new((n * s) as usize, fb, damp)),
            cr: COMB_TUNING.map(|n| Comb::new(((n + spread) * s) as usize, fb, damp)),
            al: ALLPASS_TUNING.map(|n| Allpass::new((n * s) as usize)),
            ar: ALLPASS_TUNING.map(|n| Allpass::new(((n + spread) * s) as usize)),
            pre: OnePole::new(5_500.0, sr),
        }
    }
    /// Mono in, stereo out (wet only).
    #[inline]
    pub fn run(&mut self, x: f32) -> (f32, f32) {
        let x = self.pre.lp(x) * 0.024;
        let mut l = 0.0;
        let mut r = 0.0;
        for c in &mut self.cl {
            l += c.run(x);
        }
        for c in &mut self.cr {
            r += c.run(x);
        }
        for a in &mut self.al {
            l = a.run(l);
        }
        for a in &mut self.ar {
            r = a.run(r);
        }
        // partial cross-feed: wide, but level-matched between the sides
        (l * 0.8 + r * 0.2, r * 0.8 + l * 0.2)
    }
}

// ------------------------------------------------------------------ dynamics

/// Peak limiter with a soft knee: keeps the output under `ceiling` without pumping.
pub struct Limiter {
    env: f32,
    rel: f32,
    ceiling: f32,
}

impl Limiter {
    pub fn new(sr: f32, ceiling: f32) -> Limiter {
        Limiter {
            env: 0.0,
            rel: decay_mul(0.25, sr),
            ceiling,
        }
    }
    #[inline]
    pub fn run(&mut self, l: f32, r: f32) -> (f32, f32) {
        let peak = l.abs().max(r.abs());
        self.env = if peak > self.env { peak } else { self.env * self.rel + peak * (1.0 - self.rel) };
        let g = if self.env > self.ceiling { self.ceiling / self.env } else { 1.0 };
        (soft_clip(l * g), soft_clip(r * g))
    }
}

/// Transparent below 0.85, smoothly saturating above (never exceeds 1).
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.85 {
        x
    } else {
        let over = (a - 0.85) / 0.15;
        let y = 0.85 + 0.15 * (over / (1.0 + over));
        y.copysign(x)
    }
}

/// 2x half-band interpolator (stereo). The generators run at half the output
/// rate, which halves their cost; this brings the result up to the device rate.
pub struct Up2 {
    hist: [[f32; 8]; 2],
    carry: Option<(f32, f32)>,
    il: Vec<f32>,
    ir: Vec<f32>,
}

const UP_C: [f32; 4] = [0.6165, -0.1524, 0.0465, -0.0106];

impl Up2 {
    pub fn new() -> Up2 {
        Up2 {
            hist: [[0.0; 8]; 2],
            carry: None,
            il: Vec::new(),
            ir: Vec::new(),
        }
    }

    #[inline(always)]
    fn step(h: &mut [f32; 8], x: f32) -> (f32, f32) {
        h.copy_within(1..8, 0);
        h[7] = x;
        let odd = UP_C[0] * (h[3] + h[4]) + UP_C[1] * (h[2] + h[5]) + UP_C[2] * (h[1] + h[6]) + UP_C[3] * (h[0] + h[7]);
        (h[3], odd)
    }

    /// Fills `l`/`r` (output rate) pulling half-rate frames from `inner`.
    pub fn render(&mut self, l: &mut [f32], r: &mut [f32], mut inner: impl FnMut(&mut [f32], &mut [f32])) {
        let n = l.len();
        let mut o = 0;
        if n == 0 {
            return;
        }
        if let Some((a, b)) = self.carry.take() {
            l[0] = a;
            r[0] = b;
            o = 1;
        }
        let m = (n - o).div_ceil(2);
        if self.il.len() < m {
            self.il.resize(m, 0.0);
            self.ir.resize(m, 0.0);
        }
        inner(&mut self.il[..m], &mut self.ir[..m]);
        for k in 0..m {
            let (le, lo) = Self::step(&mut self.hist[0], self.il[k]);
            let (re, ro) = Self::step(&mut self.hist[1], self.ir[k]);
            l[o] = le;
            r[o] = re;
            o += 1;
            if o < n {
                l[o] = lo;
                r[o] = ro;
                o += 1;
            } else {
                self.carry = Some((lo, ro));
            }
        }
    }
}

/// Rate the generators run at for a given output rate.
pub fn engine_rate(out_sr: f32) -> (f32, bool) {
    if out_sr >= 40_000.0 {
        (out_sr * 0.5, true)
    } else {
        (out_sr, false)
    }
}

/// Equal-power pan gains for pan in [-1, 1].
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
    (p.cos(), p.sin())
}
