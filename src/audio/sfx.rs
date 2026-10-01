//! Procedural sound effects, rendered once to mono buffers and cached.

use super::dsp::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sfx {
    /// Soft UI tick for buttons and list items.
    Click,
    /// Picking an option in a decision.
    Select,
    /// A panel / phone opening.
    Open,
    /// A panel closing.
    Close,
    /// Money received.
    Coin,
    /// Money spent / paid.
    Spend,
    /// Phone message.
    Notify,
    /// Goal reached / good outcome.
    Success,
    /// Gentle warning (late payment, customer left...).
    Warn,
    /// Footstep on wood / indoor floors.
    StepWood,
    /// Footstep on stone pavers outdoors.
    StepStone,
    /// Door open + close (changing location).
    Door,
    /// End of the day / week (dreamy swell).
    Sleep,
    /// Café counter bell: order served.
    Bell,
    /// Wrong order.
    Wrong,
    /// A bracelet sold at the stall.
    Sale,
    /// Starting the story from the title screen.
    Start,
    /// Neutral notification bubble.
    Pop,
}

impl Sfx {
    pub const ALL: [Sfx; 18] = [
        Sfx::Click,
        Sfx::Select,
        Sfx::Open,
        Sfx::Close,
        Sfx::Coin,
        Sfx::Spend,
        Sfx::Notify,
        Sfx::Success,
        Sfx::Warn,
        Sfx::StepWood,
        Sfx::StepStone,
        Sfx::Door,
        Sfx::Sleep,
        Sfx::Bell,
        Sfx::Wrong,
        Sfx::Sale,
        Sfx::Start,
        Sfx::Pop,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sfx::Click => "click",
            Sfx::Select => "select",
            Sfx::Open => "open",
            Sfx::Close => "close",
            Sfx::Coin => "coin",
            Sfx::Spend => "spend",
            Sfx::Notify => "notify",
            Sfx::Success => "success",
            Sfx::Warn => "warn",
            Sfx::StepWood => "step_wood",
            Sfx::StepStone => "step_stone",
            Sfx::Door => "door",
            Sfx::Sleep => "sleep",
            Sfx::Bell => "bell",
            Sfx::Wrong => "wrong",
            Sfx::Sale => "sale",
            Sfx::Start => "start",
            Sfx::Pop => "pop",
        }
    }

    /// Number of pre-rendered variations (footsteps vary so repetition doesn't show).
    pub fn variants(self) -> u8 {
        match self {
            Sfx::StepWood | Sfx::StepStone => 4,
            Sfx::Click => 2,
            _ => 1,
        }
    }

    /// Random pitch spread applied at playback (ratio).
    pub fn pitch_spread(self) -> f32 {
        match self {
            Sfx::StepWood | Sfx::StepStone => 0.08,
            Sfx::Click | Sfx::Pop => 0.05,
            Sfx::Door | Sfx::Sleep | Sfx::Start | Sfx::Success => 0.0,
            _ => 0.02,
        }
    }

    /// Mix level relative to the effects bus.
    pub fn level(self) -> f32 {
        match self {
            Sfx::Click => 0.55,
            Sfx::StepWood | Sfx::StepStone => 0.5,
            Sfx::Sleep => 0.8,
            _ => 0.85,
        }
    }
}

struct Buf {
    sr: f32,
    d: Vec<f32>,
    rng: Rng,
}

/// Partial: (frequency ratio, amplitude, decay seconds).
type Partial = (f32, f32, f32);

const MARIMBA: &[Partial] = &[(1.0, 1.0, 0.28), (3.93, 0.22, 0.04), (9.2, 0.05, 0.012)];
const BELL: &[Partial] = &[(1.0, 1.0, 0.42), (2.0, 0.34, 0.2), (3.01, 0.14, 0.09), (4.12, 0.08, 0.05)];

impl Buf {
    fn new(sr: f32, secs: f32, seed: u32) -> Buf {
        Buf {
            sr,
            d: vec![0.0; (secs * sr) as usize],
            rng: Rng::new(seed),
        }
    }

    fn idx(&self, t: f32) -> usize {
        ((t * self.sr) as usize).min(self.d.len())
    }

    /// Additive tone with exponentially decaying partials.
    fn tone(&mut self, start: f32, f: f32, partials: &[Partial], amp: f32, attack: f32, decay_scale: f32) {
        let i0 = self.idx(start);
        let sr = self.sr;
        let sine = &tables().sine;
        let att_n = ((attack * sr) as usize).max(1);
        for &(ratio, a, dec) in partials {
            let fr = f * ratio;
            if fr > sr * 0.45 {
                continue;
            }
            let inc = phase_inc(fr, sr);
            let mul = decay_mul(dec * decay_scale, sr);
            let mut env = a * amp;
            let mut ph = 0u32;
            for (k, s) in self.d[i0..].iter_mut().enumerate() {
                let att = if k < att_n { k as f32 / att_n as f32 } else { 1.0 };
                *s += lutp(sine, ph) * env * att;
                ph = ph.wrapping_add(inc);
                env *= mul;
                if env < 1e-5 {
                    break;
                }
            }
        }
    }

    /// FM electric-piano / bell note (`index` in radians).
    fn fm(&mut self, start: f32, f: f32, amp: f32, index: f32, ratio: f32, decay: f32) {
        let i0 = self.idx(start);
        let sr = self.sr;
        let sine = &tables().sine;
        let inc = phase_inc(f, sr);
        let minc = phase_inc(f * ratio, sr);
        let mul = decay_mul(decay, sr);
        let imul = decay_mul(decay * 0.35, sr);
        // index in cycles, scaled to the 32-bit phase range
        let (mut env, mut ix, mut ph, mut pm) = (amp, index / TAU * 4_294_967_296.0, 0u32, 0u32);
        let att_n = ((0.002 * sr) as usize).max(1);
        for (k, s) in self.d[i0..].iter_mut().enumerate() {
            let att = if k < att_n { k as f32 / att_n as f32 } else { 1.0 };
            let m = (ix * lutp(sine, pm)) as i64 as u32;
            *s += lutp(sine, ph.wrapping_add(m)) * env * att;
            ph = ph.wrapping_add(inc);
            pm = pm.wrapping_add(minc);
            env *= mul;
            ix *= imul;
            if env < 1e-5 {
                break;
            }
        }
    }

    /// Filtered noise burst. The band-pass centre sweeps from `f0` to `f1`
    /// (log), `q` shapes it; attack/decay in seconds.
    #[allow(clippy::too_many_arguments)]
    fn noise(&mut self, start: f32, dur: f32, f0: f32, f1: f32, q: f32, attack: f32, decay: f32, amp: f32, mode: u8) {
        let i0 = self.idx(start);
        let n = ((dur * self.sr) as usize).min(self.d.len() - i0);
        let sr = self.sr;
        let mut svf = Svf::new(f0, q, sr);
        let mul = decay_mul(decay, sr);
        let mut env = 1.0f32;
        for k in 0..n {
            let t = k as f32 / sr;
            if k % 32 == 0 {
                let u = (t / dur).min(1.0);
                svf.set(f0 * (f1 / f0).powf(u), q, sr);
            }
            let att = if t < attack { t / attack } else { 1.0 };
            if t >= attack {
                env *= mul;
            }
            let w = self.rng.bi();
            let (lp, bp, hp) = svf.run(w);
            let y = match mode {
                0 => bp,
                1 => lp,
                _ => hp,
            };
            self.d[i0 + k] += y * env * att * amp;
        }
    }

    /// Sine with a pitch sweep (kicks, thuds, bubbles).
    fn sweep(&mut self, start: f32, f0: f32, f1: f32, sweep_t: f32, decay: f32, amp: f32) {
        let i0 = self.idx(start);
        let sr = self.sr;
        let mul = decay_mul(decay, sr);
        let sine = &tables().sine;
        let (mut env, mut ph) = (amp, 0.0f32);
        for (k, s) in self.d[i0..].iter_mut().enumerate() {
            let t = k as f32 / sr;
            let u = (t / sweep_t).min(1.0);
            let f = f0 + (f1 - f0) * (1.0 - (1.0 - u) * (1.0 - u));
            let att = (k as f32 / (0.001 * sr)).min(1.0);
            *s += lut(sine, ph) * env * att;
            ph += f / sr;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            env *= mul;
            if env < 1e-5 {
                break;
            }
        }
    }

    fn lowpass(&mut self, fc: f32) {
        let mut f = Svf::new(fc, 0.7, self.sr);
        for s in &mut self.d {
            *s = f.lp(*s);
        }
    }

    /// Scales to a given peak and adds tiny fades so buffers never click.
    fn finish(mut self, peak: f32) -> Vec<f32> {
        let m = self.d.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        if m > 1e-6 {
            let g = peak / m;
            for s in &mut self.d {
                *s *= g;
            }
        }
        let n = self.d.len();
        let fade = ((0.004 * self.sr) as usize).min(n / 2);
        for k in 0..fade {
            let g = k as f32 / fade as f32;
            self.d[n - 1 - k] *= g;
        }
        // trim trailing silence
        let mut end = n;
        while end > 1 && self.d[end - 1].abs() < 1e-4 {
            end -= 1;
        }
        self.d.truncate(end.max(1));
        self.d
    }
}

fn hz(m: f32) -> f32 {
    midi_hz(m)
}

/// Renders one effect variation to mono samples at `sr`.
pub fn render(sfx: Sfx, variant: u8, sr: f32) -> Vec<f32> {
    let seed = 1 + sfx as u32 * 97 + variant as u32 * 13;
    match sfx {
        Sfx::Click => {
            let mut b = Buf::new(sr, 0.07, seed);
            let f = if variant == 0 { 1500.0 } else { 1380.0 };
            b.tone(0.0, f, &[(1.0, 1.0, 0.008), (2.1, 0.3, 0.004)], 0.5, 0.0006, 1.0);
            b.tone(0.0, f * 0.47, &[(1.0, 0.6, 0.014)], 0.5, 0.0008, 1.0);
            b.noise(0.0, 0.02, 3500.0, 3500.0, 1.2, 0.0003, 0.003, 0.35, 0);
            b.finish(0.5)
        }
        Sfx::Select => {
            let mut b = Buf::new(sr, 0.6, seed);
            b.tone(0.0, hz(76.0), MARIMBA, 0.8, 0.002, 0.7);
            b.tone(0.055, hz(83.0), MARIMBA, 1.0, 0.002, 0.8);
            b.finish(0.55)
        }
        Sfx::Open => {
            let mut b = Buf::new(sr, 0.36, seed);
            b.noise(0.0, 0.26, 450.0, 2400.0, 1.4, 0.09, 0.06, 1.0, 0);
            b.tone(0.13, hz(84.0), MARIMBA, 0.25, 0.002, 0.4);
            b.finish(0.38)
        }
        Sfx::Close => {
            let mut b = Buf::new(sr, 0.28, seed);
            b.noise(0.0, 0.2, 2200.0, 450.0, 1.4, 0.02, 0.05, 1.0, 0);
            b.tone(0.0, hz(72.0), MARIMBA, 0.15, 0.002, 0.3);
            b.finish(0.32)
        }
        Sfx::Coin => {
            let mut b = Buf::new(sr, 1.0, seed);
            for (t, m) in [(0.0, 83.0), (0.075, 88.0)] {
                b.tone(t, hz(m), BELL, 1.0, 0.001, 1.0);
                b.tone(t, hz(m) + 3.5, BELL, 0.3, 0.001, 0.9);
                b.noise(t, 0.02, 6000.0, 6000.0, 0.8, 0.0002, 0.004, 0.5, 2);
            }
            b.finish(0.55)
        }
        Sfx::Spend => {
            let mut b = Buf::new(sr, 0.5, seed);
            b.tone(0.0, hz(81.0), MARIMBA, 0.8, 0.002, 0.45);
            b.tone(0.065, hz(76.0), MARIMBA, 0.9, 0.002, 0.55);
            b.noise(0.02, 0.14, 2600.0, 3200.0, 0.8, 0.02, 0.05, 0.35, 0);
            b.sweep(0.0, 170.0, 120.0, 0.03, 0.05, 0.35);
            b.finish(0.45)
        }
        Sfx::Notify => {
            let mut b = Buf::new(sr, 0.8, seed);
            b.tone(0.0, hz(79.0), MARIMBA, 0.9, 0.002, 1.2);
            b.tone(0.1, hz(86.0), MARIMBA, 1.0, 0.002, 1.3);
            b.tone(0.1, hz(74.0), &[(1.0, 1.0, 0.3)], 0.18, 0.004, 1.0);
            b.finish(0.5)
        }
        Sfx::Success => {
            let mut b = Buf::new(sr, 1.8, seed);
            for (i, m) in [72.0, 76.0, 79.0, 84.0].iter().enumerate() {
                b.fm(i as f32 * 0.075, hz(*m), 0.7 + i as f32 * 0.1, 2.2, 1.0, 0.7);
            }
            let n = b.d.len();
            let sr_ = b.sr;
            let sine = &tables().sine;
            let (i2093, i3136) = (phase_inc(2093.0, sr_), phase_inc(3136.0, sr_));
            let (mut p1, mut p2) = (0u32, 0u32);
            let start = (0.2 * sr_) as usize;
            let mul = decay_mul(0.55, sr_);
            let att_n = 0.3 * sr_;
            let mut dec = 1.0f32;
            for k in start..n {
                let env = ((k - start) as f32 / att_n).min(1.0) * dec;
                dec *= mul;
                b.d[k] += (lutp(sine, p1) * 0.1 + lutp(sine, p2) * 0.06) * env;
                p1 = p1.wrapping_add(i2093);
                p2 = p2.wrapping_add(i3136);
            }
            b.finish(0.55)
        }
        Sfx::Warn => {
            let mut b = Buf::new(sr, 0.6, seed);
            b.tone(0.0, hz(64.0), MARIMBA, 0.9, 0.003, 0.7);
            b.tone(0.13, hz(63.0), MARIMBA, 1.0, 0.003, 0.9);
            b.tone(0.0, hz(52.0), &[(1.0, 1.0, 0.25)], 0.25, 0.01, 1.0);
            b.lowpass(1800.0);
            b.finish(0.45)
        }
        Sfx::StepWood => {
            let mut b = Buf::new(sr, 0.14, seed);
            let j = 1.0 + (variant as f32 - 1.5) * 0.06;
            b.sweep(0.0, 430.0 * j, 230.0 * j, 0.02, 0.018, 0.7);
            b.noise(0.0, 0.07, 1000.0 * j, 650.0, 1.1, 0.001, 0.016, 2.2, 0);
            b.noise(0.0, 0.012, 2800.0, 2800.0, 1.0, 0.0003, 0.003, 0.5, 0);
            b.finish(0.5)
        }
        Sfx::StepStone => {
            let mut b = Buf::new(sr, 0.12, seed);
            let j = 1.0 + (variant as f32 - 1.5) * 0.08;
            b.noise(0.0, 0.05, 1800.0 * j, 1800.0 * j, 0.7, 0.0005, 0.018, 0.9, 2);
            b.noise(0.006, 0.07, 3400.0 * j, 2600.0, 0.7, 0.004, 0.03, 0.4, 0);
            b.sweep(0.0, 220.0 * j, 130.0, 0.02, 0.016, 0.45);
            b.lowpass(7000.0);
            b.finish(0.5)
        }
        Sfx::Door => {
            let mut b = Buf::new(sr, 1.05, seed);
            // latch
            b.noise(0.0, 0.02, 2500.0, 2500.0, 3.0, 0.0004, 0.006, 0.8, 0);
            b.tone(0.0, 1800.0, &[(1.0, 1.0, 0.005)], 0.3, 0.0005, 1.0);
            // soft creak: stick-slip bursts through resonances
            let sr_ = b.sr;
            let (s0, s1) = ((0.06 * sr_) as usize, (0.5 * sr_) as usize);
            let mut f1 = Svf::new(1100.0, 4.0, sr_);
            let mut f2 = Svf::new(2100.0, 5.0, sr_);
            let mut ph = 0.0f32;
            let mut f = 150.0f32;
            let mut burst = 0.0f32;
            for k in s0..s1.min(b.d.len()) {
                let t = (k - s0) as f32 / (s1 - s0) as f32;
                f = (f + b.rng.bi() * 3.0).clamp(110.0, 220.0);
                ph += f / sr_;
                if ph >= 1.0 {
                    ph -= 1.0;
                    burst = 0.5 + 0.5 * b.rng.f();
                }
                burst *= 0.995;
                let x = burst * (1.0 - ph * 2.0);
                let env = (t * std::f32::consts::PI).sin().powi(2) * 0.2;
                b.d[k] += (f1.bp(x) + 0.6 * f2.bp(x)) * env;
            }
            // close thud + latch
            b.sweep(0.62, 250.0, 130.0, 0.04, 0.05, 0.7);
            b.noise(0.62, 0.1, 800.0, 450.0, 0.8, 0.001, 0.035, 1.6, 0);
            b.noise(0.66, 0.02, 2800.0, 2800.0, 3.0, 0.0004, 0.005, 0.4, 0);
            b.finish(0.5)
        }
        Sfx::Sleep => {
            let mut b = Buf::new(sr, 3.4, seed);
            let sr_ = b.sr;
            let n = b.d.len();
            // shared swell envelope, evaluated every 64 samples
            let step = 64usize;
            let ctl: Vec<f32> = (0..=n / step + 1)
                .map(|j| {
                    let t = (j * step) as f32 / sr_;
                    let att = 0.5 - 0.5 * (std::f32::consts::PI * (t / 1.1).min(1.0)).cos();
                    let dec = if t > 1.6 { (-(t - 1.6) / 0.7).exp() } else { 1.0 };
                    att * dec * 0.2
                })
                .collect();
            let sine = &tables().sine;
            for m in [53.0, 57.0, 60.0, 64.0, 67.0] {
                let inc = phase_inc(hz(m), sr_);
                let mut ph = 0u32;
                for k in 0..n {
                    let j = k / step;
                    let fr = (k % step) as f32 / step as f32;
                    let env = ctl[j] + (ctl[j + 1] - ctl[j]) * fr;
                    b.d[k] += (lutp(sine, ph) + 0.18 * lutp(sine, ph.wrapping_mul(2))) * env;
                    ph = ph.wrapping_add(inc);
                }
            }
            b.tone(0.45, hz(84.0), BELL, 0.25, 0.002, 1.8);
            b.tone(0.9, hz(79.0), BELL, 0.18, 0.002, 1.8);
            b.lowpass(4000.0);
            b.finish(0.45)
        }
        Sfx::Bell => {
            let mut b = Buf::new(sr, 1.5, seed);
            let bell: &[Partial] = &[(1.0, 1.0, 1.0), (1.004, 0.6, 0.9), (2.46, 0.45, 0.45), (3.96, 0.22, 0.25), (5.72, 0.1, 0.12)];
            b.tone(0.0, 2100.0, bell, 1.0, 0.0005, 1.0);
            b.noise(0.0, 0.01, 5000.0, 5000.0, 1.0, 0.0002, 0.003, 0.4, 2);
            b.finish(0.38)
        }
        Sfx::Wrong => {
            let mut b = Buf::new(sr, 0.4, seed);
            let sr_ = b.sr;
            let t = &dsp_tables().hollow;
            for (st, dur, f) in [(0.0f32, 0.09f32, 233.0f32), (0.13, 0.16, 208.0)] {
                let i0 = (st * sr_) as usize;
                let n = (dur * sr_) as usize;
                let mut ph = 0.0f32;
                for k in 0..n.min(b.d.len() - i0) {
                    let tt = k as f32 / sr_;
                    let env = (tt / 0.004).min(1.0) * (1.0 - (tt / dur)).max(0.0).powf(0.5);
                    b.d[i0 + k] += lut(t, ph) * env * 0.6;
                    ph += f / sr_;
                    if ph >= 1.0 {
                        ph -= 1.0;
                    }
                }
            }
            b.lowpass(1200.0);
            b.finish(0.3)
        }
        Sfx::Sale => {
            let mut b = Buf::new(sr, 0.6, seed);
            b.tone(0.0, hz(91.0), MARIMBA, 0.8, 0.002, 0.6);
            b.tone(0.05, hz(88.0), BELL, 0.5, 0.001, 0.6);
            b.noise(0.0, 0.02, 6500.0, 6500.0, 0.8, 0.0002, 0.004, 0.4, 2);
            b.finish(0.45)
        }
        Sfx::Start => {
            let mut b = Buf::new(sr, 2.4, seed);
            b.noise(0.0, 0.55, 300.0, 3000.0, 1.2, 0.35, 0.12, 0.8, 0);
            for (i, m) in [60.0, 64.0, 67.0, 71.0, 74.0].iter().enumerate() {
                b.fm(0.35 + i as f32 * 0.03, hz(*m), 0.55, 1.8, 1.0, 1.1);
            }
            b.tone(0.5, hz(96.0), BELL, 0.12, 0.004, 1.6);
            b.finish(0.5)
        }
        Sfx::Pop => {
            let mut b = Buf::new(sr, 0.12, seed);
            b.sweep(0.0, 380.0, 880.0, 0.035, 0.05, 1.0);
            b.noise(0.0, 0.01, 4000.0, 4000.0, 1.0, 0.0003, 0.002, 0.15, 0);
            b.finish(0.35)
        }
    }
}

fn dsp_tables() -> &'static Tables {
    tables()
}
