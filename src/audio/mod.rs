//! Procedural audio: generative music, ambience beds and synthesized effects.
//!
//! Nothing is loaded from disk: everything is synthesized at runtime.
//! * native (Linux / Windows / macOS / Android / iOS): a generator thread mixes
//!   into a lock-free ring that the cpal callback drains;
//! * web: the same generators run in small time slices on the main thread and
//!   are scheduled ahead of time on a WebAudio `AudioContext`.
//!
//! The game talks to [`Audio`] only: `play`, `set_music`, `set_ambience`.

pub mod ambience;
pub mod dsp;
pub mod music;
pub mod sfx;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

pub use ambience::{Amb, Place};
pub use music::Mood;
pub use sfx::Sfx;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

// ------------------------------------------------------------------ settings

/// Player-facing volume settings (shared process-wide; saved to disk / localStorage).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub music: f32,
    pub sfx: f32,
    pub muted: bool,
}

static MUSIC_VOL: AtomicU32 = AtomicU32::new(0);
static SFX_VOL: AtomicU32 = AtomicU32::new(0);
static MUTED: AtomicBool = AtomicBool::new(false);
static LOADED: AtomicBool = AtomicBool::new(false);
static DIRTY: AtomicBool = AtomicBool::new(false);
static PREVIEW: AtomicBool = AtomicBool::new(false);
static SAVE_DIR: Mutex<Option<String>> = Mutex::new(None);
/// Graphics preference kept in the same settings file: 0 auto, 1 performance, 2 quality.
static GFX_MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn gfx_mode() -> u8 {
    GFX_MODE.load(Ordering::Relaxed)
}

pub fn set_gfx_mode(m: u8) {
    if GFX_MODE.swap(m.min(2), Ordering::Relaxed) != m.min(2) {
        DIRTY.store(true, Ordering::Relaxed);
    }
}
const SETTINGS_KEY: &str = "finakids_settings";

pub fn settings() -> Settings {
    if !LOADED.load(Ordering::Relaxed) {
        return Settings {
            music: 0.75,
            sfx: 0.8,
            muted: false,
        };
    }
    Settings {
        music: f32::from_bits(MUSIC_VOL.load(Ordering::Relaxed)),
        sfx: f32::from_bits(SFX_VOL.load(Ordering::Relaxed)),
        muted: MUTED.load(Ordering::Relaxed),
    }
}

fn store(s: Settings) {
    MUSIC_VOL.store(s.music.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    SFX_VOL.store(s.sfx.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    MUTED.store(s.muted, Ordering::Relaxed);
    LOADED.store(true, Ordering::Relaxed);
}

/// Changes the settings (the UI calls this while dragging a slider).
pub fn set_settings(s: Settings) {
    if s != settings() {
        store(s);
        DIRTY.store(true, Ordering::Relaxed);
    }
}

/// Asks for a short effect so the player hears the new effects volume.
pub fn request_preview() {
    PREVIEW.store(true, Ordering::Relaxed);
}

fn load_settings(dir: Option<&str>) {
    *SAVE_DIR.lock().unwrap_or_else(|e| e.into_inner()) = dir.map(|d| d.to_string());
    let mut s = Settings {
        music: 0.75,
        sfx: 0.8,
        muted: false,
    };
    if let Some(text) = crate::platform::load_text(dir, SETTINGS_KEY) {
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "music" => s.music = v.trim().parse().unwrap_or(s.music),
                "sfx" => s.sfx = v.trim().parse().unwrap_or(s.sfx),
                "mute" => s.muted = v.trim() == "1",
                "gfx" => GFX_MODE.store(v.trim().parse::<u8>().unwrap_or(0).min(2), Ordering::Relaxed),
                _ => {}
            }
        }
    }
    store(s);
}

/// Writes the settings if they changed (call when the pointer is released).
pub fn save_settings_if_dirty() {
    if !DIRTY.swap(false, Ordering::Relaxed) {
        return;
    }
    let s = settings();
    let text = format!("music={:.2}\nsfx={:.2}\nmute={}\ngfx={}\n", s.music, s.sfx, if s.muted { 1 } else { 0 }, gfx_mode());
    let dir = SAVE_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone();
    crate::platform::save_text(dir.as_deref(), SETTINGS_KEY, &text);
}

// ------------------------------------------------------------------ backend commands

#[derive(Clone, Copy, Debug)]
pub(crate) enum Cmd {
    Play { sfx: Sfx, variant: u8, gain: f32, pan: f32, rate: f32 },
    Mood(Mood),
    Amb(Amb),
    /// Music volume, effects volume (already includes mute and ducking).
    Volumes(f32, f32),
    Paused(bool),
}

// ------------------------------------------------------------------ public API

pub struct Audio {
    #[cfg(not(target_arch = "wasm32"))]
    backend: Option<native::Native>,
    #[cfg(target_arch = "wasm32")]
    backend: Option<web::Web>,
    mood: Mood,
    amb: Amb,
    duck: f32,
    duck_target: f32,
    sent: (f32, f32),
    rng: dsp::Rng,
    time: f32,
    last: [f32; Sfx::ALL.len()],
    last_variant: [u8; Sfx::ALL.len()],
    /// Debug: log every effect and mood change (works without a device).
    pub trace: bool,
}

impl Audio {
    /// `enabled = false` gives a silent stub (headless screenshots, bots).
    pub fn new(enabled: bool, save_dir: Option<&str>) -> Audio {
        load_settings(save_dir);
        #[cfg(not(target_arch = "wasm32"))]
        let backend = if enabled { native::Native::new() } else { None };
        #[cfg(target_arch = "wasm32")]
        let backend = if enabled { web::Web::new() } else { None };
        if enabled && backend.is_none() {
            log::warn!("audio: sin dispositivo de salida; el juego continúa en silencio");
        }
        Audio {
            backend,
            mood: Mood::Silent,
            amb: Amb::NONE,
            duck: 1.0,
            duck_target: 1.0,
            sent: (-1.0, -1.0),
            rng: dsp::Rng::new(0xF1A4),
            time: 0.0,
            last: [-10.0; Sfx::ALL.len()],
            last_variant: [0; Sfx::ALL.len()],
            trace: false,
        }
    }

    pub fn enabled(&self) -> bool {
        self.backend.is_some()
    }

    fn send(&mut self, c: Cmd) {
        if let Some(b) = &mut self.backend {
            b.send(c);
        }
    }

    /// Plays an effect centred at full level.
    pub fn play(&mut self, sfx: Sfx) {
        self.play_at(sfx, 1.0, 0.0);
    }

    /// Plays an effect with a gain (0..1) and stereo position (-1 left .. 1 right).
    pub fn play_at(&mut self, sfx: Sfx, gain: f32, pan: f32) {
        if (self.backend.is_none() && !self.trace) || gain <= 0.003 {
            return;
        }
        let i = sfx as usize;
        // the same effect twice within a few ms only gets louder: skip it
        let min_gap = match sfx {
            Sfx::StepWood | Sfx::StepStone => 0.05,
            Sfx::Click => 0.04,
            _ => 0.09,
        };
        if self.time - self.last[i] < min_gap {
            return;
        }
        self.last[i] = self.time;
        if self.trace {
            log::info!("sfx {} gain {:.2} pan {:+.2}", sfx.name(), gain, pan);
        }
        let n = sfx.variants();
        let mut variant = if n > 1 { self.rng.int(n as usize) as u8 } else { 0 };
        if n > 1 && variant == self.last_variant[i] {
            variant = (variant + 1) % n;
        }
        self.last_variant[i] = variant;
        let spread = sfx.pitch_spread();
        let rate = 1.0 + self.rng.bi() * spread;
        let gain = gain.min(1.0) * sfx.level() * (1.0 - 0.12 * self.rng.f());
        self.send(Cmd::Play {
            sfx,
            variant,
            gain,
            pan: pan.clamp(-1.0, 1.0),
            rate,
        });
    }

    pub fn set_music(&mut self, m: Mood) {
        if m != self.mood {
            self.mood = m;
            if self.trace {
                log::info!("music {}", m.name());
            }
            self.send(Cmd::Mood(m));
        }
    }

    pub fn set_ambience(&mut self, a: Amb) {
        // hours drift continuously: only forward meaningful changes
        if a.place != self.amb.place || (a.hour - self.amb.hour).abs() > 0.05 {
            self.amb = a;
            self.send(Cmd::Amb(a));
        }
    }

    /// Lowers the music (1 = normal) while the player reads or decides.
    pub fn set_duck(&mut self, d: f32) {
        self.duck_target = d.clamp(0.0, 1.0);
    }

    /// Pauses / resumes all output (app in background).
    pub fn set_paused(&mut self, p: bool) {
        self.send(Cmd::Paused(p));
    }

    /// Call once per frame.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.duck += (self.duck_target - self.duck) * (1.0 - (-dt * 4.0).exp());
        let s = settings();
        let (m, f) = if s.muted { (0.0, 0.0) } else { (s.music * s.music * self.duck, s.sfx * s.sfx) };
        if (m - self.sent.0).abs() > 0.002 || (f - self.sent.1).abs() > 0.002 {
            self.sent = (m, f);
            self.send(Cmd::Volumes(m, f));
        }
        if PREVIEW.swap(false, Ordering::Relaxed) {
            self.play(Sfx::Select);
        }
        if let Some(b) = &mut self.backend {
            b.update();
        }
    }
}

// ------------------------------------------------------------------ offline mixer (native thread, WAV dump)

/// Mono effect voice in the software mixer.
#[cfg(not(target_arch = "wasm32"))]
struct Voice {
    buf: std::sync::Arc<[f32]>,
    pos: f32,
    rate: f32,
    gl: f32,
    gr: f32,
}

/// Software mixer used by the native backend: music + ambience + effects.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct Mixer {
    sr: f32,
    pub music: music::MusicEngine,
    pub amb: ambience::AmbEngine,
    bank: std::collections::HashMap<(Sfx, u8), std::sync::Arc<[f32]>>,
    voices: Vec<Voice>,
    ml: Vec<f32>,
    mr: Vec<f32>,
    al: Vec<f32>,
    ar: Vec<f32>,
    vol: (f32, f32),
    vol_target: (f32, f32),
    limiter: dsp::Limiter,
}

#[cfg(not(target_arch = "wasm32"))]
impl Mixer {
    pub fn new(sr: f32) -> Mixer {
        Mixer {
            sr,
            music: music::MusicEngine::new(sr),
            amb: ambience::AmbEngine::new(sr),
            bank: std::collections::HashMap::new(),
            voices: Vec::with_capacity(32),
            ml: Vec::new(),
            mr: Vec::new(),
            al: Vec::new(),
            ar: Vec::new(),
            vol: (0.0, 0.0),
            vol_target: (0.5, 0.64),
            limiter: dsp::Limiter::new(sr, 0.92),
        }
    }

    /// Renders every effect up front so none is synthesized mid-game.
    pub fn prewarm(&mut self) {
        for s in Sfx::ALL {
            for v in 0..s.variants() {
                let sr = self.sr;
                self.bank.entry((s, v)).or_insert_with(|| sfx::render(s, v, sr).into());
            }
        }
    }

    pub fn handle(&mut self, c: Cmd) {
        match c {
            Cmd::Play { sfx, variant, gain, pan, rate } => {
                if self.voices.len() >= 28 {
                    return;
                }
                let sr = self.sr;
                let buf = self.bank.entry((sfx, variant)).or_insert_with(|| sfx::render(sfx, variant, sr).into()).clone();
                let (gl, gr) = dsp::pan_gains(pan);
                self.voices.push(Voice {
                    buf,
                    pos: 0.0,
                    rate,
                    gl: gl * gain * 1.414,
                    gr: gr * gain * 1.414,
                });
            }
            Cmd::Mood(m) => self.music.set_mood(m),
            Cmd::Amb(a) => self.amb.set(a),
            Cmd::Volumes(m, f) => self.vol_target = (m, f),
            Cmd::Paused(_) => {}
        }
    }

    /// Renders interleaved stereo.
    pub fn render(&mut self, out: &mut [f32]) {
        let n = out.len() / 2;
        for v in [&mut self.ml, &mut self.mr, &mut self.al, &mut self.ar] {
            if v.len() < n {
                v.resize(n, 0.0);
            }
        }
        self.music.render(&mut self.ml[..n], &mut self.mr[..n]);
        self.amb.render(&mut self.al[..n], &mut self.ar[..n]);
        let (m0, f0) = self.vol;
        let (m1, f1) = self.vol_target;
        self.vol = self.vol_target;
        let inv = 1.0 / n.max(1) as f32;
        for i in 0..n {
            let t = i as f32 * inv;
            let m = m0 + (m1 - m0) * t;
            let f = f0 + (f1 - f0) * t;
            out[i * 2] = self.ml[i] * m + self.al[i] * f;
            out[i * 2 + 1] = self.mr[i] * m + self.ar[i] * f;
        }
        let mut k = 0;
        while k < self.voices.len() {
            let v = &mut self.voices[k];
            let len = v.buf.len();
            let mut done = false;
            for i in 0..n {
                let ip = v.pos as usize;
                if ip + 1 >= len {
                    done = true;
                    break;
                }
                let fr = v.pos - ip as f32;
                let s = v.buf[ip] + (v.buf[ip + 1] - v.buf[ip]) * fr;
                let t = i as f32 * inv;
                let f = f0 + (f1 - f0) * t;
                out[i * 2] += s * v.gl * f;
                out[i * 2 + 1] += s * v.gr * f;
                v.pos += v.rate;
            }
            if done {
                self.voices.swap_remove(k);
            } else {
                k += 1;
            }
        }
        for i in 0..n {
            let (l, r) = self.limiter.run(out[i * 2], out[i * 2 + 1]);
            out[i * 2] = l;
            out[i * 2 + 1] = r;
        }
    }
}

// ------------------------------------------------------------------ debug tools

/// Plays a scripted scene on the real output device for `secs` seconds and
/// reports callback / underrun counters (debug: `--audio-test <secs>`).
#[cfg(not(target_arch = "wasm32"))]
pub fn device_test(secs: f32) {
    let mut a = Audio::new(true, Some("/nonexistent-finakids-audio-test"));
    if !a.enabled() {
        println!("audio-test: no hay dispositivo de salida");
        return;
    }
    store(Settings {
        music: 0.7,
        sfx: 0.8,
        muted: false,
    });
    a.set_music(Mood::Plaza);
    a.set_ambience(Amb { place: Place::Plaza, hour: 15.0 });
    let t0 = std::time::Instant::now();
    let mut next = 1.0f32;
    let mut k = 0;
    let dt = 1.0 / 60.0;
    while t0.elapsed().as_secs_f32() < secs {
        let t = t0.elapsed().as_secs_f32();
        if t >= next {
            a.play(Sfx::ALL[k % Sfx::ALL.len()]);
            k += 1;
            next += 0.7;
        }
        if (t * 2.2) as u32 != ((t - dt) * 2.2) as u32 {
            a.play_at(Sfx::StepStone, 0.6, 0.2);
        }
        a.update(dt);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    if let Some(b) = &a.backend {
        let (cb, under, need, fill) = b.stats();
        println!("audio-test: {secs:.0}s, callbacks {cb}, underruns {under}, bloque del dispositivo {need} frames, en cola {fill} frames");
    }
}

/// Renders every mood, ambience and effect to WAV files in `dir` and prints
/// level statistics (peak, RMS, clipping, DC) plus synthesis cost.
#[cfg(not(target_arch = "wasm32"))]
pub fn dump(dir: &str) {
    use std::io::Write;
    let sr = 48_000.0f32;
    let _ = std::fs::create_dir_all(dir);
    let write = |name: &str, data: &[f32], ch: u16| {
        let path = std::path::Path::new(dir).join(name);
        let mut f = std::io::BufWriter::new(std::fs::File::create(&path).expect("wav"));
        let bytes = (data.len() * 2) as u32;
        let rate = sr as u32;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + bytes).to_le_bytes()).unwrap();
        f.write_all(b"WAVEfmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&ch.to_le_bytes()).unwrap();
        f.write_all(&rate.to_le_bytes()).unwrap();
        f.write_all(&(rate * ch as u32 * 2).to_le_bytes()).unwrap();
        f.write_all(&(ch * 2).to_le_bytes()).unwrap();
        f.write_all(&16u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&bytes.to_le_bytes()).unwrap();
        for s in data {
            f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).unwrap();
        }
    };
    let stats = |name: &str, data: &[f32], secs: f32, cost: f32| {
        let n = data.len().max(1) as f32;
        let peak = data.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        let rms = (data.iter().map(|v| v * v).sum::<f32>() / n).sqrt();
        let dc = data.iter().sum::<f32>() / n;
        let clip = data.iter().filter(|v| v.abs() > 0.999).count();
        let bad = data.iter().filter(|v| !v.is_finite()).count();
        let zc = data.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count() as f32 / n;
        println!(
            "{name:<22} {secs:5.1}s  peak {peak:.3} ({:+.1} dB)  rms {rms:.4} ({:+.1} dB)  dc {dc:+.4}  clip {clip}  nan {bad}  zcr {zc:.3}  cost {cost:.2} ms/s",
            20.0 * peak.max(1e-9).log10(),
            20.0 * rms.max(1e-9).log10(),
        );
    };
    // music
    for mood in Mood::ALL {
        let secs = 48.0;
        let n = (secs * sr) as usize;
        let mut eng = music::MusicEngine::new(sr);
        eng.set_mood(mood);
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        let t0 = std::time::Instant::now();
        for (a, b) in l.chunks_mut(2048).zip(r.chunks_mut(2048)) {
            eng.render(a, b);
        }
        let cost = t0.elapsed().as_secs_f32() * 1000.0 / secs;
        let inter: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        stats(&format!("music_{}", mood.name()), &inter, secs, cost);
        write(&format!("music_{}.wav", mood.name()), &inter, 2);
    }
    // mood crossfade
    {
        let secs = 20.0;
        let n = (secs * sr) as usize;
        let mut eng = music::MusicEngine::new(sr);
        eng.set_mood(Mood::Morning);
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (i, (a, b)) in l.chunks_mut(2048).zip(r.chunks_mut(2048)).enumerate() {
            if i == (8.0 * sr / 2048.0) as usize {
                eng.set_mood(Mood::Night);
            }
            eng.render(a, b);
        }
        let inter: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        stats("music_crossfade", &inter, secs, 0.0);
        write("music_crossfade.wav", &inter, 2);
    }
    // ambience
    for (name, a) in [
        ("amb_indoor_morning", Amb { place: Place::Indoor, hour: 8.0 }),
        ("amb_indoor_night", Amb { place: Place::Indoor, hour: 22.0 }),
        ("amb_plaza_morning", Amb { place: Place::Plaza, hour: 8.5 }),
        ("amb_plaza_afternoon", Amb { place: Place::Plaza, hour: 14.5 }),
        ("amb_plaza_night", Amb { place: Place::Plaza, hour: 22.0 }),
    ] {
        let secs = 16.0;
        let n = (secs * sr) as usize;
        let mut eng = ambience::AmbEngine::new(sr);
        eng.set(a);
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        let t0 = std::time::Instant::now();
        for (x, y) in l.chunks_mut(2048).zip(r.chunks_mut(2048)) {
            eng.render(x, y);
        }
        let cost = t0.elapsed().as_secs_f32() * 1000.0 / secs;
        let inter: Vec<f32> = l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect();
        stats(name, &inter[(4.0 * sr) as usize * 2..], secs, cost);
        write(&format!("{name}.wav"), &inter, 2);
    }
    // effects
    for s in Sfx::ALL {
        for v in 0..s.variants() {
            let t0 = std::time::Instant::now();
            let d = sfx::render(s, v, sr);
            let ms = t0.elapsed().as_secs_f32() * 1000.0;
            let secs = d.len() as f32 / sr;
            stats(&format!("sfx_{}_{v}", s.name()), &d, secs, ms / secs.max(1e-3));
            write(&format!("sfx_{}_{v}.wav", s.name()), &d, 1);
        }
    }
    // full mix through the native mixer: a short scripted "scene"
    {
        let secs = 30.0;
        let n = (secs * sr) as usize;
        let mut mix = Mixer::new(sr);
        mix.handle(Cmd::Mood(Mood::Plaza));
        mix.handle(Cmd::Amb(Amb { place: Place::Plaza, hour: 15.0 }));
        mix.handle(Cmd::Volumes(0.49, 0.64));
        let mut out = vec![0.0f32; n * 2];
        let seq = [
            (4.0, Sfx::Click),
            (5.0, Sfx::Open),
            (6.0, Sfx::Coin),
            (8.0, Sfx::Notify),
            (10.0, Sfx::Select),
            (12.0, Sfx::Spend),
            (14.0, Sfx::Success),
            (17.0, Sfx::Bell),
            (19.0, Sfx::Wrong),
            (21.0, Sfx::Door),
            (24.0, Sfx::Warn),
            (26.0, Sfx::Sleep),
        ];
        let t0 = std::time::Instant::now();
        for (i, chunk) in out.chunks_mut(512).enumerate() {
            let t = i as f32 * 256.0 / sr;
            let t1 = (i + 1) as f32 * 256.0 / sr;
            for (at, s) in seq {
                if at >= t && at < t1 {
                    mix.handle(Cmd::Play {
                        sfx: s,
                        variant: 0,
                        gain: s.level(),
                        pan: 0.0,
                        rate: 1.0,
                    });
                }
            }
            // footsteps every 0.45 s
            if (t / 0.45) as u32 != (t1 / 0.45) as u32 {
                mix.handle(Cmd::Play {
                    sfx: Sfx::StepStone,
                    variant: (i % 4) as u8,
                    gain: 0.4,
                    pan: 0.0,
                    rate: 1.0,
                });
            }
            mix.render(chunk);
        }
        let cost = t0.elapsed().as_secs_f32() * 1000.0 / secs;
        stats("mix_scene", &out, secs, cost);
        write("mix_scene.wav", &out, 2);
    }
    println!("WAV escritos en {dir}");
}
