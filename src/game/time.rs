//! Time of day: sun, sky, ambient and grading keyframes.

use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Sky {
    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub sun_disc: f32,
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub clouds: f32,
    pub stars: f32,
    pub amb_up: Vec3,
    pub amb_down: Vec3,
    pub exposure: f32,
    pub night: f32,
    pub daylight: f32,
    pub grade: Vec3,
    pub fog: Vec3,
}

struct Key {
    h: f32,
    dir: [f32; 3],
    sun: [f32; 3],
    zen: [f32; 3],
    hor: [f32; 3],
    up: [f32; 3],
    down: [f32; 3],
    stars: f32,
    exp: f32,
    night: f32,
    grade: [f32; 3],
}

const KEYS: &[Key] = &[
    Key { h: 0.0, dir: [0.35, 0.6, -0.7], sun: [0.16, 0.2, 0.34], zen: [0.012, 0.016, 0.04], hor: [0.04, 0.05, 0.085], up: [0.05, 0.065, 0.11], down: [0.025, 0.025, 0.035], stars: 1.0, exp: 1.8, night: 1.0, grade: [0.9, 0.95, 1.1] },
    Key { h: 5.5, dir: [0.35, 0.6, -0.7], sun: [0.16, 0.2, 0.34], zen: [0.02, 0.03, 0.07], hor: [0.08, 0.08, 0.12], up: [0.06, 0.07, 0.12], down: [0.03, 0.03, 0.04], stars: 0.8, exp: 1.7, night: 1.0, grade: [0.92, 0.95, 1.08] },
    Key { h: 6.5, dir: [0.75, 0.05, -0.66], sun: [1.2, 0.5, 0.28], zen: [0.12, 0.17, 0.32], hor: [0.9, 0.52, 0.36], up: [0.25, 0.28, 0.4], down: [0.16, 0.12, 0.1], stars: 0.15, exp: 1.3, night: 0.4, grade: [1.06, 0.97, 0.9] },
    Key { h: 8.0, dir: [0.45, 0.33, -0.83], sun: [3.3, 2.55, 1.8], zen: [0.24, 0.44, 0.8], hor: [0.88, 0.82, 0.74], up: [0.42, 0.52, 0.72], down: [0.3, 0.26, 0.2], stars: 0.0, exp: 1.0, night: 0.0, grade: [1.04, 1.0, 0.95] },
    Key { h: 13.0, dir: [0.12, 0.92, -0.37], sun: [4.2, 4.0, 3.75], zen: [0.15, 0.35, 0.78], hor: [0.64, 0.77, 0.92], up: [0.5, 0.62, 0.86], down: [0.32, 0.3, 0.26], stars: 0.0, exp: 0.88, night: 0.0, grade: [1.0, 1.0, 1.0] },
    Key { h: 17.0, dir: [-0.45, 0.45, -0.77], sun: [3.7, 3.15, 2.45], zen: [0.19, 0.39, 0.78], hor: [0.84, 0.79, 0.72], up: [0.46, 0.54, 0.74], down: [0.32, 0.27, 0.22], stars: 0.0, exp: 0.95, night: 0.0, grade: [1.03, 1.0, 0.96] },
    Key { h: 19.3, dir: [-0.76, 0.1, -0.64], sun: [3.1, 1.45, 0.6], zen: [0.2, 0.24, 0.48], hor: [1.0, 0.5, 0.28], up: [0.38, 0.33, 0.42], down: [0.3, 0.18, 0.12], stars: 0.0, exp: 1.08, night: 0.15, grade: [1.08, 0.97, 0.88] },
    Key { h: 20.6, dir: [-0.82, 0.02, -0.57], sun: [0.5, 0.25, 0.22], zen: [0.05, 0.07, 0.16], hor: [0.3, 0.18, 0.2], up: [0.12, 0.12, 0.2], down: [0.06, 0.05, 0.06], stars: 0.45, exp: 1.45, night: 0.8, grade: [0.96, 0.95, 1.05] },
    Key { h: 22.0, dir: [0.35, 0.6, -0.7], sun: [0.16, 0.2, 0.34], zen: [0.012, 0.016, 0.04], hor: [0.04, 0.05, 0.085], up: [0.05, 0.065, 0.11], down: [0.025, 0.025, 0.035], stars: 1.0, exp: 1.8, night: 1.0, grade: [0.9, 0.95, 1.1] },
    Key { h: 24.0, dir: [0.35, 0.6, -0.7], sun: [0.16, 0.2, 0.34], zen: [0.012, 0.016, 0.04], hor: [0.04, 0.05, 0.085], up: [0.05, 0.065, 0.11], down: [0.025, 0.025, 0.035], stars: 1.0, exp: 1.8, night: 1.0, grade: [0.9, 0.95, 1.1] },
];

fn v(a: [f32; 3]) -> Vec3 {
    Vec3::from(a)
}

pub fn sky_at(hour: f32, clouds: f32) -> Sky {
    let h = hour.rem_euclid(24.0);
    let mut i = 0;
    while i + 1 < KEYS.len() && KEYS[i + 1].h < h {
        i += 1;
    }
    let a = &KEYS[i];
    let b = &KEYS[(i + 1).min(KEYS.len() - 1)];
    let t = if b.h > a.h { ((h - a.h) / (b.h - a.h)).clamp(0.0, 1.0) } else { 0.0 };
    let t = t * t * (3.0 - 2.0 * t);
    let l = |x: [f32; 3], y: [f32; 3]| v(x).lerp(v(y), t);
    let f = |x: f32, y: f32| x + (y - x) * t;
    let night = f(a.night, b.night);
    let sun_dir = l(a.dir, b.dir).normalize();
    Sky {
        sun_dir,
        sun_color: l(a.sun, b.sun),
        sun_disc: (1.0 - night).clamp(0.0, 1.0),
        zenith: l(a.zen, b.zen),
        horizon: l(a.hor, b.hor),
        clouds,
        stars: f(a.stars, b.stars),
        amb_up: l(a.up, b.up),
        amb_down: l(a.down, b.down),
        exposure: f(a.exp, b.exp),
        night,
        daylight: (1.0 - night).clamp(0.0, 1.0) * (sun_dir.y * 3.0).clamp(0.25, 1.0),
        grade: l(a.grade, b.grade),
        fog: l(a.hor, b.hor) * 0.9,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl Phase {
    pub fn hour(self) -> f32 {
        match self {
            Phase::Morning => 8.2,
            Phase::Afternoon => 14.5,
            Phase::Evening => 19.2,
            Phase::Night => 22.0,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Phase::Morning => "Mañana",
            Phase::Afternoon => "Tarde",
            Phase::Evening => "Atardecer",
            Phase::Night => "Noche",
        }
    }
    pub fn next(self) -> Option<Phase> {
        match self {
            Phase::Morning => Some(Phase::Afternoon),
            Phase::Afternoon => Some(Phase::Evening),
            Phase::Evening => Some(Phase::Night),
            Phase::Night => None,
        }
    }
    pub fn index(self) -> u32 {
        match self {
            Phase::Morning => 0,
            Phase::Afternoon => 1,
            Phase::Evening => 2,
            Phase::Night => 3,
        }
    }
    pub fn from_index(i: u32) -> Phase {
        match i {
            0 => Phase::Morning,
            1 => Phase::Afternoon,
            2 => Phase::Evening,
            _ => Phase::Night,
        }
    }
}
