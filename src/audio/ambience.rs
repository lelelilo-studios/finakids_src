//! Procedural ambience beds: room tone, birds, crickets, plaza murmur,
//! distant traffic, the fountain and night wind. Layers crossfade slowly.

use super::dsp::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    None,
    Indoor,
    Plaza,
}

/// Where and when: picks the layer mix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Amb {
    pub place: Place,
    /// Hour of the day, 0..24.
    pub hour: f32,
}

impl Amb {
    pub const NONE: Amb = Amb { place: Place::None, hour: 12.0 };
}

const L_ROOM: usize = 0;
const L_BIRDS: usize = 1;
const L_CRICKETS: usize = 2;
const L_BABBLE: usize = 3;
const L_TRAFFIC: usize = 4;
const L_FOUNTAIN: usize = 5;
const L_WIND: usize = 6;
const LAYERS: usize = 7;

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Layer levels for a place and hour. Also returns how muffled the outside is (indoors).
fn mix_for(a: Amb) -> ([f32; LAYERS], f32) {
    let h = a.hour.rem_euclid(24.0);
    // 1 in full daylight, 0 at night
    let day = smooth(5.8, 7.5, h) * (1.0 - smooth(19.0, 21.0, h));
    let night = 1.0 - day;
    let morning = smooth(5.5, 7.0, h) * (1.0 - smooth(10.5, 13.0, h));
    let busy = smooth(8.0, 11.0, h) * (1.0 - smooth(19.5, 22.0, h));
    let mut m = [0.0; LAYERS];
    match a.place {
        Place::None => {}
        Place::Indoor => {
            m[L_ROOM] = 1.0;
            m[L_BIRDS] = 0.6 * day + 0.6 * morning;
            m[L_CRICKETS] = 0.7 * night;
            m[L_TRAFFIC] = 0.25 * day + 0.1;
        }
        Place::Plaza => {
            m[L_BIRDS] = 0.5 * day + 0.5 * morning;
            m[L_CRICKETS] = 0.9 * night;
            m[L_BABBLE] = 0.15 + 0.85 * busy;
            m[L_TRAFFIC] = 0.35 + 0.5 * day;
            m[L_FOUNTAIN] = 1.0;
            m[L_WIND] = 0.25 + 0.4 * night;
        }
    }
    (m, if a.place == Place::Indoor { 1.0 } else { 0.0 })
}

struct Bird {
    /// Samples until the next song starts.
    wait: u32,
    syll_left: u32,
    syll_pos: u32,
    syll_len: u32,
    gap: u32,
    f0: f32,
    f1: f32,
    trill: f32,
    ph: f32,
    tph: f32,
    amp: f32,
    gl: f32,
    gr: f32,
}

struct Cricket {
    ph: f32,
    inc: f32,
    /// Position inside the chirp cycle, in samples.
    t: u32,
    period: u32,
    pulses: u32,
    pulse_len: u32,
    amp: f32,
    gl: f32,
    gr: f32,
}

struct Talker {
    f: Svf,
    env: f32,
    target: f32,
    hold: u32,
    gl: f32,
    gr: f32,
}

pub struct AmbEngine {
    sr: f32,
    up: Option<Up2>,
    rng: Rng,
    level: [f32; LAYERS],
    target: [f32; LAYERS],
    muffle: f32,
    muffle_target: f32,
    slew: f32,
    cur: Amb,
    // room
    room: Brown,
    room_lp: OnePole,
    room_hp: OnePole,
    // birds
    birds: Vec<Bird>,
    // crickets
    crickets: Vec<Cricket>,
    // babble
    talkers: Vec<Talker>,
    babble_lp: [OnePole; 2],
    // traffic
    traffic: [Brown; 2],
    traffic_lp: [Svf; 2],
    traffic_hp: [OnePole; 2],
    car: f32,
    car_target: f32,
    car_wait: u32,
    // fountain
    fount_bp: [Svf; 2],
    fount_hp: [OnePole; 2],
    flutter: [f32; 2],
    // wind
    wind: [Svf; 2],
    wind_g: f32,
    wind_target: f32,
    wind_wait: u32,
    // outside heard through a window
    out_lp: [OnePole; 2],
}

impl AmbEngine {
    /// `out_sr` is the device rate; the generators run at half of it.
    pub fn new(out_sr: f32) -> AmbEngine {
        let (sr, half) = engine_rate(out_sr);
        let mut rng = Rng::new(2024);
        let birds = (0..4)
            .map(|i| {
                let (gl, gr) = pan_gains(-0.7 + 0.45 * i as f32);
                Bird {
                    wait: (rng.range(0.3, 4.0) * sr) as u32,
                    syll_left: 0,
                    syll_pos: 0,
                    syll_len: 1,
                    gap: 0,
                    f0: 3000.0,
                    f1: 3600.0,
                    trill: 0.0,
                    ph: 0.0,
                    tph: 0.0,
                    amp: 0.0,
                    gl,
                    gr,
                }
            })
            .collect();
        let crickets = (0..4)
            .map(|i| {
                let (gl, gr) = pan_gains(-0.8 + 0.55 * i as f32);
                Cricket {
                    ph: 0.0,
                    inc: rng.range(4100.0, 4900.0) / sr,
                    t: (rng.f() * sr) as u32,
                    period: (rng.range(0.55, 0.95) * sr) as u32,
                    pulses: 3 + (i as u32 % 2),
                    pulse_len: (rng.range(0.028, 0.036) * sr) as u32,
                    amp: rng.range(0.5, 1.0),
                    gl,
                    gr,
                }
            })
            .collect();
        let talkers = (0..5)
            .map(|i| {
                let (gl, gr) = pan_gains(-0.8 + 0.4 * i as f32);
                Talker {
                    f: Svf::new(700.0, 3.0, sr),
                    env: 0.0,
                    target: 0.0,
                    hold: (rng.f() * 0.3 * sr) as u32,
                    gl,
                    gr,
                }
            })
            .collect();
        AmbEngine {
            sr,
            up: if half { Some(Up2::new()) } else { None },
            rng,
            level: [0.0; LAYERS],
            target: [0.0; LAYERS],
            muffle: 0.0,
            muffle_target: 0.0,
            slew: 1.0 - decay_mul(0.9, sr),
            cur: Amb::NONE,
            room: Brown::new(),
            room_lp: OnePole::new(700.0, sr),
            room_hp: OnePole::new(120.0, sr),
            birds,
            crickets,
            talkers,
            babble_lp: [OnePole::new(2600.0, sr), OnePole::new(2600.0, sr)],
            traffic: [Brown::new(), Brown::new()],
            traffic_lp: [Svf::new(420.0, 0.6, sr), Svf::new(460.0, 0.6, sr)],
            traffic_hp: [OnePole::new(110.0, sr), OnePole::new(110.0, sr)],
            car: 0.0,
            car_target: 0.0,
            car_wait: (3.0 * sr) as u32,
            fount_bp: [Svf::new(2400.0, 0.7, sr), Svf::new(3100.0, 0.7, sr)],
            fount_hp: [OnePole::new(500.0, sr), OnePole::new(500.0, sr)],
            flutter: [0.6, 0.6],
            wind: [Svf::new(420.0, 1.6, sr), Svf::new(500.0, 1.6, sr)],
            wind_g: 0.3,
            wind_target: 0.5,
            wind_wait: 0,
            out_lp: [OnePole::new(2600.0, sr), OnePole::new(2600.0, sr)],
        }
    }

    pub fn set(&mut self, a: Amb) {
        if a == self.cur {
            return;
        }
        self.cur = a;
        let (m, muffle) = mix_for(a);
        self.target = m;
        self.muffle_target = muffle;
    }

    pub fn is_silent(&self) -> bool {
        self.level.iter().all(|v| *v < 1e-4) && self.target.iter().all(|v| *v == 0.0)
    }

    /// Renders `l.len()` frames at the device rate (overwrites the buffers).
    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        if let Some(mut up) = self.up.take() {
            up.render(l, r, |a, b| self.render_inner(a, b));
            self.up = Some(up);
        } else {
            self.render_inner(l, r);
        }
    }

    fn render_inner(&mut self, l: &mut [f32], r: &mut [f32]) {
        let sr = self.sr;
        let t = tables();
        for i in 0..l.len() {
            for k in 0..LAYERS {
                self.level[k] += (self.target[k] - self.level[k]) * self.slew * 0.5;
            }
            self.muffle += (self.muffle_target - self.muffle) * self.slew;
            let lv = self.level;
            // "inside" bus is heard directly; "outside" may be muffled by walls
            let (mut il, mut ir) = (0.0f32, 0.0f32);
            let (mut xl, mut xr) = (0.0f32, 0.0f32);

            if lv[L_ROOM] > 1e-4 {
                let w = self.rng.bi();
                let s = self.room_hp.hp(self.room_lp.lp(self.room.next(w))) * 0.06 * lv[L_ROOM];
                il += s;
                ir += s;
            }

            if lv[L_BIRDS] > 1e-4 {
                for b in &mut self.birds {
                    if b.syll_left == 0 {
                        if b.wait > 0 {
                            b.wait -= 1;
                            continue;
                        }
                        // new song
                        b.syll_left = 2 + self.rng.int(5) as u32;
                        b.f0 = self.rng.range(2400.0, 4600.0);
                        b.f1 = b.f0 * self.rng.range(0.75, 1.45);
                        b.syll_len = (self.rng.range(0.05, 0.14) * sr) as u32;
                        b.gap = (self.rng.range(0.03, 0.09) * sr) as u32;
                        b.trill = if self.rng.chance(0.4) { self.rng.range(25.0, 55.0) } else { 0.0 };
                        b.amp = self.rng.range(0.35, 1.0);
                        b.syll_pos = 0;
                    }
                    let total = b.syll_len + b.gap;
                    if b.syll_pos < b.syll_len {
                        let u = b.syll_pos as f32 / b.syll_len as f32;
                        let f = b.f0 + (b.f1 - b.f0) * u * u;
                        b.tph += b.trill / sr;
                        if b.tph >= 1.0 {
                            b.tph -= 1.0;
                        }
                        let f = f * (1.0 + 0.06 * lut(&t.sine, b.tph));
                        b.ph += f / sr;
                        if b.ph >= 1.0 {
                            b.ph -= 1.0;
                        }
                        let env = (u * std::f32::consts::PI).sin();
                        let s = lut(&t.sine, b.ph) * env * env * b.amp * 0.06 * lv[L_BIRDS];
                        xl += s * b.gl;
                        xr += s * b.gr;
                    }
                    b.syll_pos += 1;
                    if b.syll_pos >= total {
                        b.syll_pos = 0;
                        b.syll_left -= 1;
                        if b.syll_left == 0 {
                            b.wait = (self.rng.range(0.8, 6.0) * sr) as u32;
                        } else if self.rng.chance(0.3) {
                            // vary the next syllable a little
                            b.f1 = b.f0 * self.rng.range(0.75, 1.45);
                        }
                    }
                }
            }

            if lv[L_CRICKETS] > 1e-4 {
                for c in &mut self.crickets {
                    c.t += 1;
                    if c.t >= c.period {
                        c.t = 0;
                    }
                    let burst = c.pulses * c.pulse_len;
                    if c.t < burst {
                        let u = (c.t % c.pulse_len) as f32 / c.pulse_len as f32;
                        let env = (u * std::f32::consts::PI).sin();
                        c.ph += c.inc;
                        if c.ph >= 1.0 {
                            c.ph -= 1.0;
                        }
                        let s = lut(&t.sine, c.ph) * env * env * c.amp * 0.035 * lv[L_CRICKETS];
                        xl += s * c.gl;
                        xr += s * c.gr;
                    }
                }
            }

            if lv[L_BABBLE] > 1e-4 {
                let (mut bl, mut br) = (0.0f32, 0.0f32);
                for tk in &mut self.talkers {
                    if tk.hold == 0 {
                        // next "syllable": new formant, new loudness
                        tk.hold = (self.rng.range(0.07, 0.22) * sr) as u32;
                        tk.target = if self.rng.chance(0.3) { 0.0 } else { self.rng.range(0.3, 1.0) };
                        let f = self.rng.range(350.0, 2300.0);
                        tk.f.set(f, 3.5, sr);
                    }
                    tk.hold -= 1;
                    tk.env += (tk.target - tk.env) * 0.0015;
                    let s = tk.f.bp(self.rng.bi()) * tk.env;
                    bl += s * tk.gl;
                    br += s * tk.gr;
                }
                xl += self.babble_lp[0].lp(bl) * 0.05 * lv[L_BABBLE];
                xr += self.babble_lp[1].lp(br) * 0.05 * lv[L_BABBLE];
            }

            if lv[L_TRAFFIC] > 1e-4 {
                if self.car_wait == 0 {
                    // a car passes: swell up, then away
                    self.car_target = if self.car_target > 0.3 { 0.12 } else { self.rng.range(0.5, 1.0) };
                    self.car_wait = (self.rng.range(2.0, 6.0) * sr) as u32;
                }
                self.car_wait -= 1;
                self.car += (self.car_target - self.car) * (1.2 / sr);
                let g = (0.25 + self.car) * 0.07 * lv[L_TRAFFIC];
                let a = self.traffic[0].next(self.rng.bi());
                let b = self.traffic[1].next(self.rng.bi());
                xl += self.traffic_hp[0].hp(self.traffic_lp[0].lp(a)) * g;
                xr += self.traffic_hp[1].hp(self.traffic_lp[1].lp(b)) * g * 0.9;
            }

            if lv[L_FOUNTAIN] > 1e-4 {
                for ch in 0..2 {
                    let w = self.rng.bi();
                    // random flutter: splashing water never holds still
                    self.flutter[ch] += (0.35 + 0.65 * self.rng.f() - self.flutter[ch]) * 0.002;
                    let s = self.fount_hp[ch].hp(self.fount_bp[ch].bp(w)) * self.flutter[ch] * 0.085 * lv[L_FOUNTAIN];
                    if ch == 0 {
                        xl += s;
                    } else {
                        xr += s;
                    }
                }
            }

            if lv[L_WIND] > 1e-4 {
                if self.wind_wait == 0 {
                    self.wind_target = self.rng.range(0.15, 1.0);
                    self.wind_wait = (self.rng.range(1.5, 5.0) * sr) as u32;
                    let f = 300.0 + 500.0 * self.wind_target;
                    self.wind[0].set(f, 1.6, sr);
                    self.wind[1].set(f * 1.15, 1.6, sr);
                }
                self.wind_wait -= 1;
                self.wind_g += (self.wind_target - self.wind_g) * (0.8 / sr);
                let g = self.wind_g * 0.02 * lv[L_WIND];
                xl += self.wind[0].bp(self.rng.bi()) * g;
                xr += self.wind[1].bp(self.rng.bi()) * g;
            }

            // walls and windows remove the highs of whatever is outside
            let m = self.muffle;
            if m > 1e-3 {
                let fl = self.out_lp[0].lp(xl);
                let fr = self.out_lp[1].lp(xr);
                xl = xl * (1.0 - m) + fl * m * 0.85;
                xr = xr * (1.0 - m) + fr * m * 0.85;
            }
            l[i] = soft_clip(il + xl);
            r[i] = soft_clip(ir + xr);
        }
    }
}
