//! WebAudio output.
//!
//! Music and ambience are synthesized on the main thread in small blocks and
//! scheduled ahead of the playhead with `AudioBufferSourceNode.start(when)`;
//! effects are cached `AudioBuffer`s played on demand. Synthesis is budgeted
//! per frame so it never stalls rendering.

use super::ambience::AmbEngine;
use super::music::MusicEngine;
use super::{sfx, Cmd, Sfx};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioBuffer, AudioContext, AudioContextState, GainNode};

/// Frames per scheduled block.
const BLOCK: usize = 2048;
/// How far ahead of the playhead we keep audio scheduled (seconds).
const LOOKAHEAD: f64 = 0.9;
/// Synthesis budget per frame (milliseconds) at normal frame rates. When
/// frames are long the budget grows with them (up to 8% of the frame), so slow
/// devices still receive audio as fast as it plays.
const BUDGET_MS: f64 = 5.0;

pub struct Web {
    ctx: AudioContext,
    sr: f32,
    music_gain: GainNode,
    sfx_gain: GainNode,
    music: MusicEngine,
    amb: AmbEngine,
    until: f64,
    bufs: HashMap<(Sfx, u8), AudioBuffer>,
    l: Vec<f32>,
    r: Vec<f32>,
    vol: (f32, f32),
    vol_applied: bool,
    last_ms: f64,
    paused: bool,
    unlocked: Rc<Cell<bool>>,
    panner_ok: bool,
}

/// Debug (browser console: `(await import('./pkg/finakids.js')).finakids_audio_bench(8)`):
/// milliseconds of main-thread time needed per second of music and of ambience.
#[wasm_bindgen]
pub fn finakids_audio_bench(seconds: f32) -> String {
    let sr = 48_000.0f32;
    let n = (seconds.clamp(1.0, 60.0) * sr) as usize / BLOCK;
    let (mut l, mut r) = (vec![0.0f32; BLOCK], vec![0.0f32; BLOCK]);
    let secs = (n * BLOCK) as f64 / sr as f64;
    let mut out = String::new();
    for mood in [super::Mood::Title, super::Mood::Afternoon, super::Mood::Plaza] {
        let mut m = MusicEngine::new(sr);
        m.set_mood(mood);
        let t0 = now_ms();
        let mut worst = 0.0f64;
        for _ in 0..n {
            let b0 = now_ms();
            m.render(&mut l, &mut r);
            worst = worst.max(now_ms() - b0);
        }
        out += &format!("music {} {:.2} ms/s (peor bloque {:.2} ms); ", mood.name(), (now_ms() - t0) / secs, worst);
    }
    for (name, place) in [("interior", super::Place::Indoor), ("plaza", super::Place::Plaza)] {
        let mut a = AmbEngine::new(sr);
        a.set(super::Amb { place, hour: 15.0 });
        let t0 = now_ms();
        for _ in 0..n {
            a.render(&mut l, &mut r);
        }
        out += &format!("ambiente {} {:.2} ms/s; ", name, (now_ms() - t0) / secs);
    }
    let t0 = now_ms();
    let mut total = 0usize;
    for s in Sfx::ALL {
        for v in 0..s.variants() {
            total += sfx::render(s, v, sr).len();
        }
    }
    out += &format!("efectos: {:.1} ms para {:.1} s de audio", now_ms() - t0, total as f32 / sr);
    out
}

/// Raw synthesis benchmark with no JS dependencies (timed from outside, e.g. Node):
/// kind 0..=7 music moods, 8 indoor ambience, 9 plaza ambience, 10 every effect.
/// Renders `blocks` blocks of 2048 frames at 48 kHz and returns a checksum.
#[no_mangle]
pub extern "C" fn finakids_bench_raw(kind: u32, blocks: u32) -> f32 {
    let sr = 48_000.0f32;
    let (mut l, mut r) = (vec![0.0f32; BLOCK], vec![0.0f32; BLOCK]);
    let mut sum = 0.0f32;
    match kind {
        0..=7 => {
            let mut m = MusicEngine::new(sr);
            m.set_mood(super::Mood::ALL[kind as usize]);
            for _ in 0..blocks {
                m.render(&mut l, &mut r);
                sum += l.iter().map(|v| v.abs()).sum::<f32>();
            }
        }
        8 | 9 => {
            let mut a = AmbEngine::new(sr);
            a.set(super::Amb {
                place: if kind == 8 { super::Place::Indoor } else { super::Place::Plaza },
                hour: 15.0,
            });
            for _ in 0..blocks {
                a.render(&mut l, &mut r);
                sum += l.iter().map(|v| v.abs()).sum::<f32>();
            }
        }
        _ => {
            for s in Sfx::ALL {
                for v in 0..s.variants() {
                    sum += sfx::render(s, v, sr).iter().map(|v| v.abs()).sum::<f32>();
                }
            }
        }
    }
    sum
}

fn now_ms() -> f64 {
    web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0)
}

impl Web {
    pub fn new() -> Option<Web> {
        let ctx = AudioContext::new().ok()?;
        let sr = ctx.sample_rate();
        let dest = ctx.destination();
        let music_gain = ctx.create_gain().ok()?;
        let sfx_gain = ctx.create_gain().ok()?;
        music_gain.gain().set_value(0.0);
        sfx_gain.gain().set_value(0.0);
        music_gain.connect_with_audio_node(&dest).ok()?;
        sfx_gain.connect_with_audio_node(&dest).ok()?;
        let unlocked = Rc::new(Cell::new(ctx.state() == AudioContextState::Running));

        // Browsers only start audio from inside a real user gesture.
        let doc = web_sys::window()?.document()?;
        {
            let ctx = ctx.clone();
            let unlocked = unlocked.clone();
            let cb = Closure::<dyn FnMut()>::new(move || {
                if ctx.state() != AudioContextState::Running {
                    let _ = ctx.resume();
                }
                if !unlocked.get() {
                    unlocked.set(true);
                    // iOS: a (silent) buffer started inside the gesture unlocks output
                    if let (Ok(buf), Ok(src)) = (ctx.create_buffer(1, 1, ctx.sample_rate()), ctx.create_buffer_source()) {
                        src.set_buffer(Some(&buf));
                        let _ = src.connect_with_audio_node(&ctx.destination());
                        let _ = src.start();
                    }
                }
            });
            for ev in ["pointerdown", "pointerup", "touchend", "click", "keydown"] {
                let _ = doc.add_event_listener_with_callback_and_bool(ev, cb.as_ref().unchecked_ref(), true);
            }
            cb.forget();
        }
        // Silence the game when the tab is hidden.
        {
            let ctx = ctx.clone();
            let unlocked = unlocked.clone();
            let d = doc.clone();
            let cb = Closure::<dyn FnMut()>::new(move || {
                if d.hidden() {
                    let _ = ctx.suspend();
                } else if unlocked.get() {
                    let _ = ctx.resume();
                }
            });
            let _ = doc.add_event_listener_with_callback("visibilitychange", cb.as_ref().unchecked_ref());
            cb.forget();
        }
        let panner_ok = ctx.create_stereo_panner().is_ok();
        // handy when debugging from the browser console
        if let Some(w) = web_sys::window() {
            let _ = js_sys::Reflect::set(&w, &JsValue::from_str("__finakidsAudio"), &ctx);
        }
        log::info!("audio: WebAudio {} Hz ({:?})", sr, ctx.state());
        Some(Web {
            sr,
            music_gain,
            sfx_gain,
            music: MusicEngine::new(sr),
            amb: AmbEngine::new(sr),
            until: 0.0,
            bufs: {
                // rendered while the loading screen is still up, so no effect
                // ever hitches the first time it plays
                let mut bufs = HashMap::new();
                for s in Sfx::ALL {
                    for v in 0..s.variants() {
                        let data = sfx::render(s, v, sr);
                        if let Ok(buf) = ctx.create_buffer(1, data.len().max(1) as u32, sr) {
                            if buf.copy_to_channel(&data, 0).is_ok() {
                                bufs.insert((s, v), buf);
                            }
                        }
                    }
                }
                bufs
            },
            l: vec![0.0; BLOCK],
            r: vec![0.0; BLOCK],
            vol: (0.0, 0.0),
            vol_applied: false,
            last_ms: 0.0,
            paused: false,
            unlocked,
            panner_ok,
            ctx,
        })
    }

    fn running(&self) -> bool {
        !self.paused && self.ctx.state() == AudioContextState::Running
    }

    fn buffer(&mut self, s: Sfx, variant: u8) -> Option<AudioBuffer> {
        if let Some(b) = self.bufs.get(&(s, variant)) {
            return Some(b.clone());
        }
        let data = sfx::render(s, variant, self.sr);
        let buf = self.ctx.create_buffer(1, data.len().max(1) as u32, self.sr).ok()?;
        buf.copy_to_channel(&data, 0).ok()?;
        self.bufs.insert((s, variant), buf.clone());
        Some(buf)
    }

    fn play(&mut self, s: Sfx, variant: u8, gain: f32, pan: f32, rate: f32) {
        if !self.running() {
            return;
        }
        let Some(buf) = self.buffer(s, variant) else {
            return;
        };
        let (Ok(src), Ok(g)) = (self.ctx.create_buffer_source(), self.ctx.create_gain()) else {
            return;
        };
        src.set_buffer(Some(&buf));
        src.playback_rate().set_value(rate);
        g.gain().set_value(gain);
        let _ = src.connect_with_audio_node(&g);
        let mut panned = false;
        if self.panner_ok && pan.abs() > 0.02 {
            if let Ok(p) = self.ctx.create_stereo_panner() {
                p.pan().set_value(pan);
                let _ = g.connect_with_audio_node(&p);
                let _ = p.connect_with_audio_node(&self.sfx_gain);
                panned = true;
            }
        }
        if !panned {
            let _ = g.connect_with_audio_node(&self.sfx_gain);
        }
        let _ = src.start();
    }

    pub fn send(&mut self, c: Cmd) {
        match c {
            Cmd::Play { sfx, variant, gain, pan, rate } => self.play(sfx, variant, gain, pan, rate),
            Cmd::Mood(m) => self.music.set_mood(m),
            Cmd::Amb(a) => self.amb.set(a),
            Cmd::Volumes(m, f) => {
                self.vol = (m, f);
                if self.vol_applied && self.running() {
                    let t = self.ctx.current_time();
                    let _ = self.music_gain.gain().set_target_at_time(m, t, 0.06);
                    let _ = self.sfx_gain.gain().set_target_at_time(f, t, 0.04);
                }
            }
            Cmd::Paused(p) => {
                self.paused = p;
                if p {
                    let _ = self.ctx.suspend();
                } else if self.unlocked.get() {
                    let _ = self.ctx.resume();
                }
            }
        }
    }

    fn schedule(&self, gain: &GainNode, at: f64) {
        let Ok(buf) = self.ctx.create_buffer(2, BLOCK as u32, self.sr) else {
            return;
        };
        if buf.copy_to_channel(&self.l, 0).is_err() || buf.copy_to_channel(&self.r, 1).is_err() {
            return;
        }
        if let Ok(src) = self.ctx.create_buffer_source() {
            src.set_buffer(Some(&buf));
            let _ = src.connect_with_audio_node(gain);
            let _ = src.start_with_when(at);
        }
    }

    /// Keeps music and ambience scheduled ahead of the playhead.
    pub fn update(&mut self) {
        let t0 = now_ms();
        let frame_ms = if self.last_ms > 0.0 { (t0 - self.last_ms).clamp(0.0, 1000.0) } else { 16.0 };
        self.last_ms = t0;
        if !self.running() {
            // the context clock is frozen while suspended, so what is already
            // scheduled stays valid: keep `until` and just wait
            self.vol_applied = false;
            return;
        }
        let now = self.ctx.current_time();
        if !self.vol_applied {
            // (re)started: set the bus gains outright; automation scheduled while
            // the context was suspended is not reliable across browsers
            self.vol_applied = true;
            for (g, v) in [(&self.music_gain, self.vol.0), (&self.sfx_gain, self.vol.1)] {
                let _ = g.gain().cancel_scheduled_values(0.0);
                let _ = g.gain().set_value_at_time(v, now);
            }
        }
        if self.until < now + 0.03 {
            // first block, or we fell behind (stalled tab): restart just ahead
            self.until = now + 0.08;
        }
        let budget = (frame_ms * 0.08).clamp(BUDGET_MS, 40.0);
        let dur = BLOCK as f64 / self.sr as f64;
        let mut blocks = 0;
        while self.until < now + LOOKAHEAD && blocks < 16 && now_ms() - t0 < budget {
            if !self.music.is_idle() {
                let (mut l, mut r) = (std::mem::take(&mut self.l), std::mem::take(&mut self.r));
                self.music.render(&mut l, &mut r);
                self.l = l;
                self.r = r;
                self.schedule(&self.music_gain, self.until);
            }
            if !self.amb.is_silent() {
                let (mut l, mut r) = (std::mem::take(&mut self.l), std::mem::take(&mut self.r));
                self.amb.render(&mut l, &mut r);
                self.l = l;
                self.r = r;
                self.schedule(&self.sfx_gain, self.until);
            }
            self.until += dur;
            blocks += 1;
        }
    }
}
