//! Generative lo-fi score: a bar-by-bar sequencer driving a small synth
//! (electric piano, pad, bass, melody, drum kit) with voice-led chords.
//! Each mood has its own key, tempo, progression and arrangement; moods crossfade.

use super::dsp::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mood {
    Silent,
    /// Title screen: slow, dreamy, no drums.
    Title,
    /// Home in the morning: bright and light.
    Morning,
    /// Home in the afternoon: relaxed groove.
    Afternoon,
    /// Sunset: warm and mellow.
    Evening,
    /// Night: sparse, slow, soft.
    Night,
    /// The plaza by day / the café: more upbeat.
    Plaza,
    /// Overdue debts: minor key, restrained.
    Tense,
    /// Final reflection: gentle and open.
    Reflect,
}

impl Mood {
    pub const ALL: [Mood; 8] = [
        Mood::Title,
        Mood::Morning,
        Mood::Afternoon,
        Mood::Evening,
        Mood::Night,
        Mood::Plaza,
        Mood::Tense,
        Mood::Reflect,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Mood::Silent => "silent",
            Mood::Title => "title",
            Mood::Morning => "morning",
            Mood::Afternoon => "afternoon",
            Mood::Evening => "evening",
            Mood::Night => "night",
            Mood::Plaza => "plaza",
            Mood::Tense => "tense",
            Mood::Reflect => "reflect",
        }
    }
}

/// (root offset from the key in semitones, chord intervals).
type Chord = (i32, &'static [i32]);

const MAJ7: &[i32] = &[0, 4, 7, 11];
const MAJ9: &[i32] = &[0, 4, 7, 11, 14];
const M7: &[i32] = &[0, 3, 7, 10];
const M9: &[i32] = &[0, 3, 7, 10, 14];
const DOM9: &[i32] = &[0, 4, 7, 10, 14];
const SUS7: &[i32] = &[0, 5, 7, 10];
const SIX9: &[i32] = &[0, 4, 7, 9, 14];
const ADD9: &[i32] = &[0, 4, 7, 14];

struct Style {
    bpm: f32,
    /// MIDI note of the tonic (octave 4).
    key: i32,
    minor: bool,
    a: &'static [Chord],
    b: &'static [Chord],
    /// 0 none, 1 soft, 2 lo-fi, 3 upbeat.
    drums: u8,
    ep: f32,
    ep_bright: f32,
    ep_pats: &'static [u8],
    pad: f32,
    pad_cut: f32,
    bass: f32,
    bass_pat: u8,
    mel: f32,
    swing: f32,
    crackle: f32,
    rev: f32,
    /// Output trim so every mood lands at a similar loudness.
    gain: f32,
}

fn style(m: Mood) -> Style {
    match m {
        Mood::Title | Mood::Silent => Style {
            bpm: 66.0,
            key: 64,
            minor: false,
            a: &[(0, MAJ9), (5, MAJ9), (9, M9), (5, MAJ7)],
            b: &[(2, M9), (7, SIX9), (4, M7), (9, M9)],
            drums: 0,
            ep: 0.85,
            ep_bright: 0.85,
            ep_pats: &[4, 1, 4, 2],
            pad: 0.85,
            pad_cut: 1600.0,
            bass: 0.7,
            bass_pat: 1,
            mel: 0.6,
            swing: 0.0,
            crackle: 0.5,
            rev: 1.3,
            gain: 1.3,
        },
        Mood::Morning => Style {
            bpm: 84.0,
            key: 62,
            minor: false,
            a: &[(0, MAJ7), (4, M7), (9, M7), (5, MAJ7)],
            b: &[(5, MAJ9), (7, SUS7), (4, M7), (9, M9)],
            drums: 2,
            ep: 1.0,
            ep_bright: 1.1,
            ep_pats: &[0, 3, 0, 5],
            pad: 0.55,
            pad_cut: 1900.0,
            bass: 0.9,
            bass_pat: 0,
            mel: 0.45,
            swing: 0.22,
            crackle: 0.2,
            rev: 0.9,
            gain: 1.0,
        },
        Mood::Afternoon => Style {
            bpm: 80.0,
            key: 65,
            minor: false,
            a: &[(2, M9), (7, DOM9), (0, MAJ9), (9, M9)],
            b: &[(5, MAJ7), (4, M7), (2, M7), (7, SUS7)],
            drums: 2,
            ep: 1.0,
            ep_bright: 1.0,
            ep_pats: &[0, 2, 3, 0],
            pad: 0.6,
            pad_cut: 1600.0,
            bass: 1.0,
            bass_pat: 2,
            mel: 0.35,
            swing: 0.28,
            crackle: 0.35,
            rev: 0.9,
            gain: 1.0,
        },
        Mood::Evening => Style {
            bpm: 74.0,
            key: 68,
            minor: false,
            a: &[(9, M9), (5, MAJ9), (0, MAJ9), (7, SIX9)],
            b: &[(2, M9), (4, M7), (5, MAJ7), (7, SUS7)],
            drums: 2,
            ep: 0.95,
            ep_bright: 0.85,
            ep_pats: &[2, 0, 1, 3],
            pad: 0.8,
            pad_cut: 1300.0,
            bass: 1.0,
            bass_pat: 0,
            mel: 0.4,
            swing: 0.3,
            crackle: 0.6,
            rev: 1.0,
            gain: 1.0,
        },
        Mood::Night => Style {
            bpm: 62.0,
            key: 61,
            minor: false,
            a: &[(0, MAJ9), (9, M9), (5, MAJ9), (7, SIX9)],
            b: &[(2, M9), (5, MAJ7), (4, M7), (9, M9)],
            drums: 1,
            ep: 0.85,
            ep_bright: 0.75,
            ep_pats: &[1, 4, 2, 1],
            pad: 0.8,
            pad_cut: 1250.0,
            bass: 0.85,
            bass_pat: 1,
            mel: 0.3,
            swing: 0.25,
            crackle: 0.7,
            rev: 1.3,
            gain: 1.3,
        },
        Mood::Plaza => Style {
            bpm: 92.0,
            key: 67,
            minor: false,
            a: &[(0, MAJ7), (9, M7), (2, M7), (7, DOM9)],
            b: &[(5, MAJ7), (7, SUS7), (4, M7), (9, M7)],
            drums: 3,
            ep: 1.0,
            ep_bright: 1.25,
            ep_pats: &[3, 0, 5, 3],
            pad: 0.4,
            pad_cut: 2200.0,
            bass: 1.0,
            bass_pat: 2,
            mel: 0.4,
            swing: 0.18,
            crackle: 0.1,
            rev: 0.8,
            gain: 1.0,
        },
        Mood::Tense => Style {
            bpm: 70.0,
            key: 62,
            minor: true,
            a: &[(0, M9), (8, MAJ7), (10, ADD9), (0, M7)],
            b: &[(5, M7), (8, MAJ7), (7, SUS7), (7, SUS7)],
            drums: 1,
            ep: 0.75,
            ep_bright: 0.7,
            ep_pats: &[2, 1, 5, 2],
            pad: 0.9,
            pad_cut: 900.0,
            bass: 1.0,
            bass_pat: 1,
            mel: 0.15,
            swing: 0.1,
            crackle: 0.3,
            rev: 1.2,
            gain: 1.3,
        },
        Mood::Reflect => Style {
            bpm: 68.0,
            key: 65,
            minor: false,
            a: &[(0, MAJ9), (7, SUS7), (9, M9), (5, MAJ9)],
            b: &[(2, M9), (5, MAJ7), (0, MAJ9), (7, SIX9)],
            drums: 1,
            ep: 0.9,
            ep_bright: 0.8,
            ep_pats: &[4, 1, 2, 4],
            pad: 0.9,
            pad_cut: 1400.0,
            bass: 0.8,
            bass_pat: 1,
            mel: 0.55,
            swing: 0.15,
            crackle: 0.3,
            rev: 1.3,
            gain: 1.25,
        },
    }
}

// ------------------------------------------------------------------ drum kit

pub struct Kit {
    kick: Vec<f32>,
    snare: Vec<f32>,
    rim: Vec<f32>,
    hat: Vec<f32>,
    ohat: Vec<f32>,
    shaker: Vec<f32>,
}

const D_KICK: u8 = 0;
const D_SNARE: u8 = 1;
const D_RIM: u8 = 2;
const D_HAT: u8 = 3;
const D_OHAT: u8 = 4;
const D_SHAKER: u8 = 5;

impl Kit {
    pub fn new(sr: f32) -> Kit {
        let mut rng = Rng::new(77);
        let len = |s: f32| (s * sr) as usize;
        // kick: soft, round, short
        let mut kick = vec![0.0; len(0.32)];
        let mut ph = 0.0f32;
        for (i, s) in kick.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let f = 48.0 + 95.0 * (-t / 0.035).exp();
            ph += f / sr;
            let click = if t < 0.004 { rng.bi() * 0.25 * (1.0 - t / 0.004) } else { 0.0 };
            *s = (sin01(ph) * (-t / 0.11).exp() + click) * (t / 0.0015).min(1.0);
        }
        // snare: dusty noise + body
        let mut snare = vec![0.0; len(0.26)];
        let mut bp = Svf::new(1900.0, 0.9, sr);
        let mut hp = OnePole::new(350.0, sr);
        let mut ph = 0.0f32;
        for (i, s) in snare.iter_mut().enumerate() {
            let t = i as f32 / sr;
            ph += (185.0 + 60.0 * (-t / 0.02).exp()) / sr;
            let n = bp.bp(rng.bi()) * 1.6 * (-t / 0.075).exp();
            let body = sin01(ph) * 0.55 * (-t / 0.05).exp();
            *s = hp.hp(n + body) * (t / 0.001).min(1.0);
        }
        // rim click
        let mut rim = vec![0.0; len(0.09)];
        let mut bp = Svf::new(1700.0, 5.0, sr);
        for (i, s) in rim.iter_mut().enumerate() {
            let t = i as f32 / sr;
            let x = if t < 0.002 { rng.bi() } else { 0.0 };
            *s = bp.bp(x) * 2.0 * (-t / 0.025).exp() + sin01(t * 820.0) * 0.3 * (-t / 0.012).exp();
        }
        // hats: filtered noise
        let hatf = |dec: f32, n: usize, rng: &mut Rng| {
            let mut v = vec![0.0; n];
            let mut hp = Svf::new(7000.0, 0.8, sr);
            let mut lp = OnePole::new(11_000.0, sr);
            for (i, s) in v.iter_mut().enumerate() {
                let t = i as f32 / sr;
                *s = lp.lp(hp.hp(rng.bi())) * (-t / dec).exp() * (t / 0.0008).min(1.0);
            }
            v
        };
        let hat = hatf(0.022, len(0.1), &mut rng);
        let ohat = hatf(0.11, len(0.4), &mut rng);
        // shaker: softer, band-limited, slow attack
        let mut shaker = vec![0.0; len(0.09)];
        let mut bp = Svf::new(5200.0, 1.0, sr);
        for (i, s) in shaker.iter_mut().enumerate() {
            let t = i as f32 / sr;
            *s = bp.bp(rng.bi()) * (t / 0.012).min(1.0) * (-t / 0.03).exp();
        }
        let mut k = Kit {
            kick,
            snare,
            rim,
            hat,
            ohat,
            shaker,
        };
        for v in [&mut k.kick, &mut k.snare, &mut k.rim, &mut k.hat, &mut k.ohat, &mut k.shaker] {
            let m = v.iter().fold(0.0f32, |a, x| a.max(x.abs())).max(1e-6);
            let n = v.len();
            for (i, s) in v.iter_mut().enumerate() {
                let tail = ((n - i) as f32 / (0.004 * sr)).min(1.0);
                *s *= tail / m;
            }
        }
        k
    }

    fn get(&self, k: u8) -> &[f32] {
        match k {
            D_KICK => &self.kick,
            D_SNARE => &self.snare,
            D_RIM => &self.rim,
            D_HAT => &self.hat,
            D_OHAT => &self.ohat,
            _ => &self.shaker,
        }
    }
}

// ------------------------------------------------------------------ voices

#[derive(Clone, Copy)]
struct Tone {
    inc: u32,
    minc: u32,
    ph: u32,
    pm: u32,
    index: f32,
    index_mul: f32,
    env: f32,
    dec_mul: f32,
    rel_mul: f32,
    att: f32,
    att_inc: f32,
    hold: u32,
    gl: f32,
    gr: f32,
    send: f32,
    /// 0 e-piano (FM), 1 bass, 2 hollow melody (FM).
    table: u8,
    /// Ducked by the kick and moved by the tremolo?
    duck: bool,
}

struct PadNote {
    inc1: u32,
    inc2: u32,
    ph1: u32,
    ph2: u32,
    env: f32,
    on: bool,
    midi: i32,
}

struct Hit {
    kind: u8,
    pos: usize,
    gain: f32,
    gl: f32,
    gr: f32,
    send: f32,
}

/// Per-song scratch buffers (one segment long).
#[derive(Default)]
struct Scratch {
    l: Vec<f32>,
    r: Vec<f32>,
    s: Vec<f32>,
    pl: Vec<f32>,
    pr: Vec<f32>,
    dl: Vec<f32>,
    dr: Vec<f32>,
}

#[derive(Clone, Copy)]
enum Ev {
    Ep { midi: i32, vel: f32, dur: u32 },
    Mel { midi: i32, vel: f32, dur: u32 },
    Bass { midi: i32, vel: f32, dur: u32 },
    Drum { kind: u8, vel: f32 },
    Pad { notes: [i32; 5], n: usize },
}

/// EP comping patterns: (eighth-note step, length in steps).
const EP_PATS: [&[(u32, u32)]; 6] = [
    &[(0, 3), (3, 2), (5, 3)],
    &[(0, 8)],
    &[(0, 4), (4, 4)],
    &[(0, 3), (3, 3), (6, 2)],
    &[], // arpeggio (special)
    &[(1, 3), (5, 3)],
];

/// Two-bar melodic rhythms: (sixteenth step, length).
const MEL_RHYTHMS: [&[(u32, u32)]; 5] = [
    &[(0, 4), (6, 2), (8, 6), (16, 4), (22, 2), (24, 8)],
    &[(4, 2), (6, 2), (8, 8), (20, 2), (22, 2), (24, 6)],
    &[(0, 6), (8, 2), (10, 2), (12, 8), (24, 4), (28, 4)],
    &[(2, 2), (4, 4), (10, 6), (18, 2), (20, 8)],
    &[(0, 8), (12, 4), (16, 8)],
];

pub struct Song {
    pub mood: Mood,
    st: Style,
    sr: f32,
    spb: usize,
    pos: usize,
    bar: u32,
    rng: Rng,
    voicing: [i32; 4],
    events: Vec<(u32, Ev)>,
    ev_i: usize,
    tones: Vec<Tone>,
    pads: Vec<PadNote>,
    hits: Vec<Hit>,
    duck: f32,
    duck_mul: f32,
    trem: u32,
    pad_lp: [OnePole; 2],
    pad_lfo: f32,
    scratch: Scratch,
    hiss_lp: OnePole,
    crackle_lp: OnePole,
    mel_last: i32,
    mel_rhythm: usize,
    mel_on: bool,
    mel_notes: Vec<(u32, u32, i32)>,
    pub gain: f32,
    pub gain_target: f32,
    gain_rate: f32,
}

fn voice_chord(prev: &[i32; 4], root_pc: i32, ivs: &[i32], lo: i32, hi: i32) -> [i32; 4] {
    let pcs: [i32; 4] = if ivs.len() >= 5 {
        [ivs[1], ivs[2], ivs[3], ivs[4]]
    } else {
        [ivs[0], ivs[1], ivs[2], ivs[3]]
    }
    .map(|iv| (root_pc + iv).rem_euclid(12));
    const PERMS: [[usize; 4]; 24] = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];
    let mut best = *prev;
    let mut best_cost = i32::MAX;
    for perm in PERMS {
        let mut v = [0i32; 4];
        let mut cost = 0;
        for i in 0..4 {
            let pc = pcs[perm[i]];
            // nearest pitch with this pitch class to the previous voice
            let mut p = prev[i] + (pc - prev[i]).rem_euclid(12);
            if p - prev[i] > 6 {
                p -= 12;
            }
            while p < lo {
                p += 12;
            }
            while p > hi {
                p -= 12;
            }
            cost += (p - prev[i]).abs();
            v[i] = p;
        }
        let mut s = v;
        s.sort_unstable();
        for i in 0..3 {
            let d = s[i + 1] - s[i];
            if d == 0 {
                cost += 12;
            } else if d == 1 {
                cost += 4;
            }
        }
        // keep the voicing from collapsing or spreading too wide
        let span = s[3] - s[0];
        if span < 7 {
            cost += 7 - span;
        }
        if span > 17 {
            cost += (span - 17) * 2;
        }
        if cost < best_cost {
            best_cost = cost;
            best = s;
        }
    }
    best
}

impl Song {
    pub fn new(mood: Mood, sr: f32, seed: u32, fade_in: f32) -> Song {
        let st = style(mood);
        let spb = (sr * 60.0 / st.bpm * 4.0) as usize;
        let k = st.key;
        let mut s = Song {
            mood,
            sr,
            spb,
            pos: 0,
            bar: 0,
            rng: Rng::new(seed),
            voicing: [k - 8, k - 3, k, k + 4],
            events: Vec::with_capacity(96),
            ev_i: 0,
            tones: Vec::with_capacity(48),
            pads: Vec::with_capacity(16),
            hits: Vec::with_capacity(24),
            duck: 0.0,
            duck_mul: decay_mul(0.14, sr),
            trem: 0,
            pad_lp: [OnePole::new(st.pad_cut, sr), OnePole::new(st.pad_cut, sr)],
            pad_lfo: 0.0,
            scratch: Scratch::default(),
            hiss_lp: OnePole::new(5500.0, sr),
            crackle_lp: OnePole::new(3200.0, sr),
            mel_last: k + 16,
            mel_rhythm: 0,
            mel_on: false,
            mel_notes: Vec::with_capacity(16),
            gain: 0.0,
            gain_target: 1.0,
            gain_rate: 1.0 / (fade_in.max(0.05) * sr),
            st,
        };
        s.schedule_bar();
        s
    }

    pub fn fade_out(&mut self, secs: f32) {
        self.gain_target = 0.0;
        self.gain_rate = 1.0 / (secs.max(0.05) * self.sr);
    }

    pub fn finished(&self) -> bool {
        self.gain_target == 0.0 && self.gain <= 0.0
    }

    fn chord_at(&self, bar: u32) -> Chord {
        let phrase = bar / 8;
        let prog = if phrase % 4 == 2 { self.st.b } else { self.st.a };
        prog[(bar as usize) % prog.len()]
    }

    fn scale(&self) -> &'static [i32] {
        if self.st.minor {
            &[0, 3, 5, 7, 10]
        } else {
            &[0, 2, 4, 7, 9]
        }
    }

    /// Builds the melody for a two-bar phrase.
    fn plan_melody(&mut self, bar: u32) {
        self.mel_notes.clear();
        self.mel_on = bar >= 4 && self.rng.chance(self.st.mel);
        if !self.mel_on {
            return;
        }
        // repeat the previous rhythm half the time: motifs feel intentional
        if self.rng.chance(0.5) {
            self.mel_rhythm = self.rng.int(MEL_RHYTHMS.len());
        }
        let scale = self.scale();
        let key = self.st.key;
        let (lo, hi) = (key + 9, key + 26);
        let mut p = self.mel_last;
        let rhythm = MEL_RHYTHMS[self.mel_rhythm];
        for (i, &(step, len)) in rhythm.iter().enumerate() {
            let chord = self.chord_at(bar + step / 16);
            // candidate pitches: pentatonic neighbours of the last note
            let mut cands: Vec<(i32, f32)> = Vec::with_capacity(12);
            for oct in -1..=3 {
                for &d in scale {
                    let q = key + oct * 12 + d;
                    if q < lo || q > hi || (q - p).abs() > 7 {
                        continue;
                    }
                    let pc = (q - key - chord.0).rem_euclid(12);
                    let in_chord = chord.1.iter().any(|iv| iv.rem_euclid(12) == pc);
                    let strong = step % 8 == 0 || i + 1 == rhythm.len();
                    let mut w = 1.0 / (1.0 + (q - p).abs() as f32 * 0.6);
                    if q == p {
                        w *= 0.35;
                    }
                    if in_chord {
                        w *= if strong { 4.0 } else { 1.6 };
                    }
                    cands.push((q, w));
                }
            }
            if cands.is_empty() {
                continue;
            }
            let total: f32 = cands.iter().map(|c| c.1).sum();
            let mut pick = self.rng.f() * total;
            let mut q = cands[0].0;
            for (c, w) in &cands {
                pick -= w;
                q = *c;
                if pick <= 0.0 {
                    break;
                }
            }
            p = q;
            self.mel_notes.push((step, len, q));
        }
        self.mel_last = p;
    }

    fn schedule_bar(&mut self) {
        self.events.clear();
        self.ev_i = 0;
        let bar = self.bar;
        let spb = self.spb as f32;
        let s16 = spb / 16.0;
        let sr = self.sr;
        let swing = self.st.swing;
        let at16 = |step: u32, rng: &mut Rng, human: f32| -> u32 {
            let sw = if step % 2 == 1 { swing * s16 } else { 0.0 };
            let j = rng.bi() * human * sr;
            (step as f32 * s16 + sw + j).clamp(0.0, spb - 1.0) as u32
        };
        let phrase = bar / 8;
        let in_phrase = bar % 8;
        let intro = bar < 2;
        let breakdown = phrase % 4 == 3 && in_phrase < 2;
        let (root, ivs) = self.chord_at(bar);
        let key = self.st.key;
        let root_pc = (key + root).rem_euclid(12);

        // ---- harmony
        let k = key;
        self.voicing = voice_chord(&self.voicing, root_pc, ivs, k - 9, k + 12);
        let v = self.voicing;
        let mut pad_notes = [0i32; 5];
        pad_notes[..4].copy_from_slice(&v);
        // pad doubles the root an octave below the voicing for warmth
        let mut low_root = k - 24 + (root_pc - k.rem_euclid(12)).rem_euclid(12);
        while low_root < k - 19 {
            low_root += 12;
        }
        pad_notes[4] = low_root;
        self.events.push((0, Ev::Pad { notes: pad_notes, n: 5 }));

        // ---- electric piano
        let pat_id = self.st.ep_pats[(bar as usize) % self.st.ep_pats.len()];
        let s8 = s16 * 2.0;
        if pat_id == 4 {
            let order = [0usize, 1, 2, 3, 2, 1, 2, 3];
            for (i, &vi) in order.iter().enumerate() {
                if breakdown && i % 2 == 1 {
                    continue;
                }
                let at = at16(i as u32 * 2, &mut self.rng, 0.004);
                let vel = 0.62 + 0.12 * self.rng.f() + if i == 0 { 0.12 } else { 0.0 };
                self.events.push((
                    at,
                    Ev::Ep {
                        midi: v[vi] + if vi == 3 && i == 7 { 12 } else { 0 },
                        vel,
                        dur: (s8 * 2.6) as u32,
                    },
                ));
            }
        } else {
            for &(step, len) in EP_PATS[pat_id as usize] {
                let base = at16(step * 2, &mut self.rng, 0.003);
                let accent = if step == 0 { 0.1 } else { 0.0 };
                for (j, &note) in v.iter().enumerate() {
                    // strum upward, slightly uneven
                    let strum = (j as f32 * (0.009 + 0.004 * self.rng.f()) * sr) as u32;
                    let vel = 0.6 + accent + 0.12 * self.rng.f() - j as f32 * 0.02;
                    self.events.push((
                        (base + strum).min(self.spb as u32 - 1),
                        Ev::Ep {
                            midi: note,
                            vel,
                            dur: (len as f32 * s8 * 0.92) as u32,
                        },
                    ));
                }
            }
        }

        // ---- bass
        if !breakdown && bar >= 1 {
            let mut b = 33 + (root_pc - 33).rem_euclid(12);
            if b > 43 {
                b -= 12;
            }
            let next_root_pc = (key + self.chord_at(bar + 1).0).rem_euclid(12);
            let mut nb = 33 + (next_root_pc - 33).rem_euclid(12);
            if nb > 43 {
                nb -= 12;
            }
            let approach = if nb > b { nb - 1 } else if nb < b { nb + 1 } else { b + 7 };
            let note = |step: u32, len: u32, midi: i32, vel: f32, rng: &mut Rng| {
                (
                    at16(step, rng, 0.002),
                    Ev::Bass {
                        midi,
                        vel,
                        dur: (len as f32 * s16 * 0.95) as u32,
                    },
                )
            };
            match self.st.bass_pat {
                0 => {
                    self.events.push(note(0, 6, b, 0.9, &mut self.rng));
                    self.events.push(note(8, 5, b, 0.75, &mut self.rng));
                    if self.rng.chance(0.6) {
                        self.events.push(note(14, 2, approach, 0.65, &mut self.rng));
                    }
                }
                1 => {
                    self.events.push(note(0, 13, b, 0.85, &mut self.rng));
                    if in_phrase % 2 == 1 && self.rng.chance(0.5) {
                        self.events.push(note(14, 2, b + 7, 0.55, &mut self.rng));
                    }
                }
                _ => {
                    self.events.push(note(0, 3, b, 0.9, &mut self.rng));
                    self.events.push(note(3, 2, b + 12, 0.55, &mut self.rng));
                    self.events.push(note(6, 2, b, 0.7, &mut self.rng));
                    self.events.push(note(8, 4, b + 7, 0.75, &mut self.rng));
                    self.events.push(note(12, 2, b, 0.7, &mut self.rng));
                    self.events.push(note(14, 2, approach, 0.65, &mut self.rng));
                }
            }
        }

        // ---- drums
        if self.st.drums > 0 && !intro && !breakdown {
            let fill = in_phrase == 7;
            let drum = |step: u32, kind: u8, vel: f32, rng: &mut Rng, ev: &mut Vec<(u32, Ev)>| {
                ev.push((at16(step, rng, 0.0025), Ev::Drum { kind, vel }));
            };
            match self.st.drums {
                1 => {
                    drum(0, D_KICK, 0.7, &mut self.rng, &mut self.events);
                    if bar % 2 == 1 {
                        drum(10, D_KICK, 0.45, &mut self.rng, &mut self.events);
                    }
                    drum(12, D_RIM, 0.5, &mut self.rng, &mut self.events);
                    for s in [2u32, 6, 10, 14] {
                        drum(s, D_SHAKER, 0.35 + 0.1 * self.rng.f(), &mut self.rng, &mut self.events);
                    }
                }
                2 => {
                    drum(0, D_KICK, 0.9, &mut self.rng, &mut self.events);
                    if self.rng.chance(0.6) {
                        drum(7, D_KICK, 0.45, &mut self.rng, &mut self.events);
                    }
                    drum(10, D_KICK, 0.75, &mut self.rng, &mut self.events);
                    drum(4, D_SNARE, 0.8, &mut self.rng, &mut self.events);
                    drum(12, D_SNARE, 0.85, &mut self.rng, &mut self.events);
                    for s in (0..16).step_by(2) {
                        if fill && s >= 13 {
                            continue;
                        }
                        let v = if s % 4 == 0 { 0.5 } else { 0.8 };
                        drum(s, D_HAT, v * (0.8 + 0.2 * self.rng.f()), &mut self.rng, &mut self.events);
                    }
                    if self.rng.chance(0.35) {
                        drum(15, D_HAT, 0.35, &mut self.rng, &mut self.events);
                    }
                    if fill {
                        drum(13, D_SNARE, 0.3, &mut self.rng, &mut self.events);
                        drum(14, D_SNARE, 0.45, &mut self.rng, &mut self.events);
                        drum(15, D_SNARE, 0.6, &mut self.rng, &mut self.events);
                    }
                }
                _ => {
                    for (s, v) in [(0u32, 0.9f32), (6, 0.55), (8, 0.8), (11, 0.5)] {
                        drum(s, D_KICK, v, &mut self.rng, &mut self.events);
                    }
                    drum(4, D_SNARE, 0.8, &mut self.rng, &mut self.events);
                    drum(12, D_SNARE, 0.85, &mut self.rng, &mut self.events);
                    for s in 0..16u32 {
                        if s == 14 {
                            continue;
                        }
                        let v = match s % 4 {
                            0 => 0.55,
                            2 => 0.85,
                            _ => 0.3,
                        };
                        drum(s, D_HAT, v * (0.8 + 0.2 * self.rng.f()), &mut self.rng, &mut self.events);
                    }
                    drum(14, D_OHAT, 0.6, &mut self.rng, &mut self.events);
                    for s in [2u32, 6, 10, 14] {
                        drum(s, D_SHAKER, 0.45, &mut self.rng, &mut self.events);
                    }
                    if fill {
                        drum(14, D_SNARE, 0.4, &mut self.rng, &mut self.events);
                        drum(15, D_SNARE, 0.55, &mut self.rng, &mut self.events);
                    }
                }
            }
        }

        // ---- melody (planned in two-bar phrases)
        if bar % 2 == 0 {
            self.plan_melody(bar);
        }
        if self.mel_on {
            let base = (bar % 2) * 16;
            for i in 0..self.mel_notes.len() {
                let (step, len, midi) = self.mel_notes[i];
                if step >= base && step < base + 16 {
                    let at = at16(step - base, &mut self.rng, 0.006);
                    let vel = 0.6 + 0.25 * self.rng.f();
                    self.events.push((
                        at,
                        Ev::Mel {
                            midi,
                            vel,
                            dur: (len as f32 * s16 * 0.9) as u32,
                        },
                    ));
                }
            }
        }
        self.events.sort_by_key(|e| e.0);
    }

    fn trigger(&mut self, ev: Ev) {
        let sr = self.sr;
        match ev {
            Ev::Ep { midi, vel, dur } => {
                if self.tones.len() >= 44 {
                    return;
                }
                let f = midi_hz(midi as f32);
                // spread the chord around its own centre: low notes slightly left,
                // high notes slightly right, balanced overall
                let centre = self.voicing.iter().sum::<i32>() as f32 * 0.25;
                let pan = ((midi as f32 - centre) / 40.0).clamp(-0.25, 0.25);
                let (gl, gr) = pan_gains(pan);
                // lower notes ring longer, like a real tine piano
                let tau = (1.9 - (midi as f32 - 48.0) * 0.03).clamp(0.6, 2.2);
                let inc = phase_inc(f, sr);
                self.tones.push(Tone {
                    inc,
                    minc: inc,
                    ph: 0,
                    pm: 0,
                    index: (0.06 + 0.2 * vel * vel) * self.st.ep_bright,
                    index_mul: decay_mul(0.5, sr),
                    env: 0.13 * vel * self.st.ep,
                    dec_mul: decay_mul(tau, sr),
                    rel_mul: decay_mul(0.12, sr),
                    att: 0.0,
                    att_inc: 1.0 / (0.004 * sr),
                    hold: dur,
                    gl,
                    gr,
                    send: 0.3 * self.st.rev,
                    table: 0,
                    duck: true,
                });
            }
            Ev::Mel { midi, vel, dur } => {
                if self.tones.len() >= 46 {
                    return;
                }
                let f = midi_hz(midi as f32);
                let (gl, gr) = pan_gains(0.15);
                self.tones.push(Tone {
                    inc: phase_inc(f, sr),
                    minc: phase_inc(f * 3.0, sr),
                    ph: 0,
                    pm: 0,
                    index: 0.05 + 0.07 * vel,
                    index_mul: decay_mul(0.08, sr),
                    env: 0.11 * vel,
                    dec_mul: decay_mul(0.9, sr),
                    rel_mul: decay_mul(0.25, sr),
                    att: 0.0,
                    att_inc: 1.0 / (0.012 * sr),
                    hold: dur,
                    gl,
                    gr,
                    send: 0.55 * self.st.rev,
                    table: 2,
                    duck: false,
                });
            }
            Ev::Bass { midi, vel, dur } => {
                // one bass note at a time: release the previous
                for t in &mut self.tones {
                    if t.table == 1 {
                        t.hold = 0;
                    }
                }
                let f = midi_hz(midi as f32);
                self.tones.push(Tone {
                    inc: phase_inc(f, sr),
                    minc: 0,
                    ph: 0,
                    pm: 0,
                    index: 0.0,
                    index_mul: 1.0,
                    env: 0.2 * vel * self.st.bass,
                    dec_mul: decay_mul(1.6, sr),
                    rel_mul: decay_mul(0.05, sr),
                    att: 0.0,
                    att_inc: 1.0 / (0.008 * sr),
                    hold: dur,
                    gl: 0.707,
                    gr: 0.707,
                    send: 0.0,
                    table: 1,
                    duck: false,
                });
            }
            Ev::Drum { kind, vel } => {
                if self.hits.len() >= 22 {
                    return;
                }
                let (level, pan, send) = match kind {
                    D_KICK => (0.36, 0.0, 0.0),
                    D_SNARE => (0.3, 0.05, 0.22),
                    D_RIM => (0.24, -0.1, 0.3),
                    D_HAT => (0.12, 0.25, 0.04),
                    D_OHAT => (0.09, 0.25, 0.08),
                    _ => (0.09, -0.3, 0.1),
                };
                if kind == D_KICK {
                    self.duck = vel;
                }
                let (gl, gr) = pan_gains(pan);
                self.hits.push(Hit {
                    kind,
                    pos: 0,
                    gain: level * vel,
                    gl,
                    gr,
                    send: send * self.st.rev,
                });
            }
            Ev::Pad { notes, n } => {
                for p in &mut self.pads {
                    p.on = false;
                }
                for &m in &notes[..n] {
                    // keep notes shared with the previous chord sounding
                    if let Some(p) = self.pads.iter_mut().find(|p| p.midi == m) {
                        p.on = true;
                    } else if self.pads.len() < 14 {
                        let f = midi_hz(m as f32);
                        self.pads.push(PadNote {
                            inc1: phase_inc(f * 0.9975, sr),
                            inc2: phase_inc(f * 1.0025, sr),
                            ph1: self.rng.next_u32(),
                            ph2: self.rng.next_u32(),
                            env: 0.0,
                            on: true,
                            midi: m,
                        });
                    }
                }
            }
        }
    }

    /// Renders one event-free stretch of `m` frames into the scratch buffers.
    fn segment(&mut self, kit: &Kit, sc: &mut Scratch, m: usize) {
        let t = tables();
        let sr = self.sr;
        let (l, r, s) = (&mut sc.l[..m], &mut sc.r[..m], &mut sc.s[..m]);
        l.fill(0.0);
        r.fill(0.0);
        s.fill(0.0);

        // kick ducking and e-piano tremolo, shared by the ducked voices
        let (dl, dr) = (&mut sc.dl[..m], &mut sc.dr[..m]);
        let trem_inc = phase_inc(3.2, sr);
        let mut trem = self.trem;
        let mut duck = self.duck;
        for i in 0..m {
            duck *= self.duck_mul;
            trem = trem.wrapping_add(trem_inc);
            let d = 1.0 - 0.3 * duck;
            let tr = 0.12 * lutp(&t.sine, trem);
            dl[i] = d * (1.0 + tr);
            dr[i] = d * (1.0 - tr);
        }
        self.trem = trem;
        self.duck = duck;

        // tones (e-piano, melody, bass)
        for v in &mut self.tones {
            let tab: &Table = match v.table {
                0 => &t.ep,
                1 => &t.bass,
                _ => &t.hollow,
            };
            let fm = v.table != 1;
            for i in 0..m {
                if v.att < 1.0 {
                    v.att = (v.att + v.att_inc).min(1.0);
                }
                let ph = if fm {
                    v.ph.wrapping_add((v.index * lutp(&t.sine, v.pm) * 4_294_967_296.0) as i64 as u32)
                } else {
                    v.ph
                };
                let x = lutp(tab, ph) * v.env * v.att;
                v.ph = v.ph.wrapping_add(v.inc);
                v.pm = v.pm.wrapping_add(v.minc);
                v.index *= v.index_mul;
                if v.hold > 0 {
                    v.hold -= 1;
                    v.env *= v.dec_mul;
                } else {
                    v.env *= v.rel_mul;
                }
                if v.duck {
                    l[i] += x * v.gl * dl[i];
                    r[i] += x * v.gr * dr[i];
                } else {
                    l[i] += x * v.gl;
                    r[i] += x * v.gr;
                }
                s[i] += x * v.send;
            }
        }
        self.tones.retain(|v| v.env >= 1e-4);

        // pad: detuned soft saws, one filter per side
        let (pl, pr) = (&mut sc.pl[..m], &mut sc.pr[..m]);
        pl.fill(0.0);
        pr.fill(0.0);
        let pad_att = 1.0 - decay_mul(0.5, sr);
        let pad_rel = 1.0 - decay_mul(0.5, sr);
        for p in &mut self.pads {
            let (target, k) = if p.on { (1.0, pad_att) } else { (0.0, pad_rel) };
            for i in 0..m {
                p.env += (target - p.env) * k;
                pl[i] += lutp(&t.soft_saw, p.ph1) * p.env;
                pr[i] += lutp(&t.soft_saw, p.ph2) * p.env;
                p.ph1 = p.ph1.wrapping_add(p.inc1);
                p.ph2 = p.ph2.wrapping_add(p.inc2);
            }
        }
        self.pads.retain(|p| p.on || p.env >= 0.005);
        // slow filter movement
        self.pad_lfo += 0.07 * m as f32 / sr;
        if self.pad_lfo >= 1.0 {
            self.pad_lfo -= 1.0;
        }
        let cut = self.st.pad_cut * (1.0 + 0.3 * lut(&t.sine, self.pad_lfo));
        self.pad_lp[0].set(cut, sr);
        self.pad_lp[1].set(cut * 1.06, sr);
        let pad_gain = 0.05 * self.st.pad;
        let pad_send = 0.4 * self.st.rev;
        for i in 0..m {
            // keep some of each side in the other: wide, but mono-safe
            let (a, b) = (pl[i] * 0.78 + pr[i] * 0.22, pr[i] * 0.78 + pl[i] * 0.22);
            let d = (dl[i] + dr[i]) * 0.5;
            let a = self.pad_lp[0].lp(a) * pad_gain * d;
            let b = self.pad_lp[1].lp(b) * pad_gain * d;
            l[i] += a;
            r[i] += b;
            s[i] += (a + b) * 0.5 * pad_send;
        }

        // drums
        for h in &mut self.hits {
            let buf = kit.get(h.kind);
            let n = m.min(buf.len() - h.pos.min(buf.len()));
            let src = &buf[h.pos.min(buf.len())..];
            for i in 0..n {
                let x = src[i] * h.gain;
                l[i] += x * h.gl;
                r[i] += x * h.gr;
                s[i] += x * h.send;
            }
            h.pos += m;
        }
        self.hits.retain(|h| h.pos < kit.get(h.kind).len());

        // vinyl dust
        let crackle = self.st.crackle;
        if crackle > 0.0 {
            let p = crackle * 7.0 / sr;
            for i in 0..m {
                let w = self.rng.bi();
                let hiss = self.hiss_lp.lp(w) * 0.0035 * crackle;
                let tick = if self.rng.f() < p { self.rng.bi() * 0.5 } else { 0.0 };
                let c = self.crackle_lp.lp(tick) * 0.35;
                l[i] += hiss + c;
                r[i] += hiss * 0.8 + c * 0.9;
            }
        }
    }

    /// Adds `l.len()` frames into the buffers.
    pub fn render(&mut self, kit: &Kit, l: &mut [f32], r: &mut [f32], send: &mut [f32]) {
        let n = l.len();
        let mut sc = std::mem::take(&mut self.scratch);
        if sc.l.len() < n {
            for v in [&mut sc.l, &mut sc.r, &mut sc.s, &mut sc.pl, &mut sc.pr, &mut sc.dl, &mut sc.dr] {
                v.resize(n, 0.0);
            }
        }
        let mut i = 0;
        while i < n {
            while self.ev_i < self.events.len() && self.events[self.ev_i].0 as usize <= self.pos {
                let ev = self.events[self.ev_i].1;
                self.ev_i += 1;
                self.trigger(ev);
            }
            let next = if self.ev_i < self.events.len() { self.events[self.ev_i].0 as usize } else { self.spb };
            let m = (n - i).min(next.max(self.pos + 1) - self.pos).min(self.spb - self.pos);
            self.segment(kit, &mut sc, m);
            for k in 0..m {
                if self.gain != self.gain_target {
                    let d = self.gain_target - self.gain;
                    self.gain = if d.abs() <= self.gain_rate { self.gain_target } else { self.gain + self.gain_rate * d.signum() };
                }
                let g = self.gain * self.gain * self.st.gain;
                l[i + k] += sc.l[k] * g;
                r[i + k] += sc.r[k] * g;
                send[i + k] += sc.s[k] * g;
            }
            i += m;
            self.pos += m;
            if self.pos >= self.spb {
                self.pos = 0;
                self.bar += 1;
                self.schedule_bar();
            }
        }
        self.scratch = sc;
    }
}

// ------------------------------------------------------------------ engine

pub struct MusicEngine {
    sr: f32,
    up: Option<Up2>,
    kit: Kit,
    songs: Vec<Song>,
    target: Mood,
    reverb: Reverb,
    limiter: Limiter,
    tone: [OnePole; 2],
    send: Vec<f32>,
    seed: u32,
    /// Smoothed power of the reverb return per side, and the gains that level them.
    wet_pow: [f32; 2],
    wet_gain: [f32; 2],
}

impl MusicEngine {
    /// `out_sr` is the device rate; the synth itself runs at half of it.
    pub fn new(out_sr: f32) -> MusicEngine {
        let (sr, half) = engine_rate(out_sr);
        MusicEngine {
            sr,
            up: if half { Some(Up2::new()) } else { None },
            kit: Kit::new(sr),
            songs: Vec::with_capacity(4),
            target: Mood::Silent,
            reverb: Reverb::new(sr, 0.55, 0.3),
            limiter: Limiter::new(sr, 0.8),
            tone: [OnePole::new(10_000.0, sr), OnePole::new(10_000.0, sr)],
            send: Vec::new(),
            seed: 1,
            wet_pow: [0.0; 2],
            wet_gain: [1.0; 2],
        }
    }

    pub fn mood(&self) -> Mood {
        self.target
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed.max(1);
    }

    pub fn set_mood(&mut self, m: Mood) {
        if m == self.target {
            return;
        }
        self.target = m;
        for s in &mut self.songs {
            s.fade_out(2.2);
        }
        if m != Mood::Silent {
            if self.songs.len() >= 3 {
                self.songs.remove(0);
            }
            self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.songs.push(Song::new(m, self.sr, self.seed ^ (m as u32 * 7919), 1.6));
        }
    }

    pub fn is_idle(&self) -> bool {
        self.songs.is_empty()
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
        let n = l.len();
        l.fill(0.0);
        r.fill(0.0);
        if self.send.len() < n {
            self.send.resize(n, 0.0);
        }
        let send = &mut self.send[..n];
        send.fill(0.0);
        for s in &mut self.songs {
            s.render(&self.kit, l, r, send);
        }
        self.songs.retain(|s| !s.finished());
        // a room excited by sustained chords rings differently on each side;
        // slowly level the two returns so the image stays centred
        let k = 1.0 - decay_mul(1.2, self.sr);
        for i in 0..n {
            let (wl, wr) = self.reverb.run(send[i]);
            self.wet_pow[0] += (wl * wl - self.wet_pow[0]) * k;
            self.wet_pow[1] += (wr * wr - self.wet_pow[1]) * k;
            if i % 32 == 0 {
                let avg = (self.wet_pow[0] + self.wet_pow[1]) * 0.5;
                if avg > 1e-9 {
                    self.wet_gain[0] = (avg / self.wet_pow[0].max(1e-9)).sqrt().clamp(0.6, 1.6);
                    self.wet_gain[1] = (avg / self.wet_pow[1].max(1e-9)).sqrt().clamp(0.6, 1.6);
                }
            }
            let a = self.tone[0].lp(l[i] + wl * self.wet_gain[0]);
            let b = self.tone[1].lp(r[i] + wr * self.wet_gain[1]);
            let (a, b) = self.limiter.run(a * 1.5, b * 1.5);
            l[i] = a;
            r[i] = b;
        }
    }
}
