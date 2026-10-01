//! Procedural character animation: stepping with planted feet, two-bone IK,
//! breathing, weight shifts, gaze, blinking, action sequences and secondary motion.

use super::face::{Expr, FaceState};
use super::skeleton::*;
use crate::math::*;
use glam::{Mat4, Quat, Vec3};

// ------------------------------------------------------------------ IK helpers

/// Solves a two-bone chain. Returns the middle joint position and the clamped end.
pub fn two_bone(root: Vec3, target: Vec3, l1: f32, l2: f32, pole: Vec3) -> (Vec3, Vec3) {
    let to = target - root;
    let dist = to.length().clamp((l1 - l2).abs() + 1e-3, (l1 + l2) * 0.9995);
    let dir = to.normalize_or(Vec3::NEG_Y);
    let end = root + dir * dist;
    let a = (l1 * l1 - l2 * l2 + dist * dist) / (2.0 * dist);
    let h = (l1 * l1 - a * a).max(0.0).sqrt();
    let mut pd = pole - dir * pole.dot(dir);
    if pd.length_squared() < 1e-8 {
        pd = dir.any_orthonormal_vector();
    }
    let mid = root + dir * a + pd.normalize() * h;
    (mid, end)
}

/// Local rotation for `bone` so that its bind direction points along `world_dir`,
/// with its bind `ref_axis` rotated toward `world_ref`.
fn aim_local(fk: &Fk, sk: &Skeleton, bone: usize, world_dir: Vec3, ref_axis: Vec3, world_ref: Vec3) -> Quat {
    let bind_dir = sk.bone_dir(bone);
    let world_rot = quat_from_to_basis(bind_dir, ref_axis, world_dir, world_ref);
    let parent_rot = fk.rot[PARENT[bone]];
    (parent_rot.inverse() * world_rot).normalize()
}

// ------------------------------------------------------------------ channels & actions

#[derive(Clone, Copy, Debug)]
pub struct HandChan {
    /// Target in character-local space (relative to root, facing +Z).
    pub pos: Vec3,
    pub w: f32,
    /// Direction the palm faces (local).
    pub palm: Vec3,
    pub grip: f32,
}

impl Default for HandChan {
    fn default() -> Self {
        HandChan {
            pos: Vec3::ZERO,
            w: 0.0,
            palm: Vec3::NEG_X,
            grip: 0.25,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Chans {
    pub hand: [HandChan; 2],
    pub bend: f32,
    pub twist: f32,
    pub side: f32,
    pub head_pitch: f32,
    pub head_roll: f32,
    pub head_yaw: f32,
    pub shrug: f32,
    pub crouch: f32,
    pub look_local: Option<Vec3>,
    pub look_w: f32,
    pub bounce: f32,
}

#[derive(Clone, Debug)]
pub enum Set {
    Hand(usize, Vec3, Vec3, f32),
    HandW(usize, f32),
    Grip(usize, f32),
    Bend(f32),
    Twist(f32),
    Side(f32),
    HeadPitch(f32),
    HeadRoll(f32),
    HeadYaw(f32),
    Shrug(f32),
    Crouch(f32),
    LookLocal(Vec3),
    LookOff,
    Expr(Expr),
    Bounce(f32),
    /// Oscillation on a hand (side, axis, amplitude, frequency).
    Wiggle(usize, Vec3, f32, f32),
    Event(u32),
}

#[derive(Clone, Debug)]
pub struct Phase {
    pub dur: f32,
    pub ease: Ease,
    pub sets: Vec<Set>,
}

pub fn ph(dur: f32, ease: Ease, sets: Vec<Set>) -> Phase {
    Phase { dur, ease, sets }
}

#[derive(Clone, Debug)]
pub struct Action {
    pub name: &'static str,
    pub phases: Vec<Phase>,
    /// Keep the final channel values instead of relaxing to rest.
    pub hold: bool,
}

struct ActionPlayer {
    action: Action,
    idx: usize,
    t: f32,
    start: Chans,
    wiggle: [(Vec3, f32, f32); 2],
    fired: bool,
}

// ------------------------------------------------------------------ animator

#[derive(Clone, Copy, Debug)]
struct Foot {
    planted: Vec3,
    swing: bool,
    from: Vec3,
    to: Vec3,
    t: f32,
    dur: f32,
    yaw: f32,
    from_yaw: f32,
    to_yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stance {
    Stand,
    Sit { seat: Vec3, yaw: f32, feet: f32 },
}

pub struct Animator {
    pub skel: Skeleton,
    pub pos: Vec3,
    pub yaw: f32,
    pub target_yaw: Option<f32>,
    pub vel: Vec3,
    pub desired_vel: Vec3,
    pub stance: Stance,
    sit_amt: f32,
    feet: [Foot; 2],
    time: f32,
    seed: f32,
    // gaze
    pub look_at: Option<Vec3>,
    look_yaw: f32,
    look_pitch: f32,
    eye_yaw: f32,
    eye_pitch: f32,
    sacc: (f32, f32),
    sacc_t: f32,
    // blink
    blink: f32,
    next_blink: f32,
    // face
    pub face: FaceState,
    pub expr: Expr,
    pub talking: bool,
    talk_amt: f32,
    // actions
    player: Option<ActionPlayer>,
    pub ch: Chans,
    rest: Chans,
    holding: bool,
    pub events: Vec<u32>,
    // secondary
    hair: Spring3,
    hem: Spring3,
    tail: [Vec3; TAIL_N],
    tail_prev: [Vec3; TAIL_N],
    tail_init: bool,
    prev_head: Vec3,
    prev_hips: Vec3,
    // outputs
    pub pose: Pose,
    pub fk: Fk,
    pub model: Mat4,
    pub eye_rot: Quat,
    pub blink_amt: f32,
    pub speed: f32,
    pub fidget_t: f32,
    pub hips_world: Vec3,
    pub idle_variant: u32,
}

/// Animator event: a foot was planted after a walking step.
pub const EV_STEP: u32 = 50;

impl Animator {
    pub fn new(skel: Skeleton, pos: Vec3, yaw: f32, seed: u64) -> Animator {
        let rot = Quat::from_rotation_y(yaw);
        let mut feet = [Foot {
            planted: Vec3::ZERO,
            swing: false,
            from: Vec3::ZERO,
            to: Vec3::ZERO,
            t: 0.0,
            dur: 0.3,
            yaw,
            from_yaw: yaw,
            to_yaw: yaw,
        }; 2];
        for (side, f) in feet.iter_mut().enumerate() {
            let a = skel.bind[FOOT[side]];
            f.planted = pos + rot * Vec3::new(a.x * 0.95, 0.0, a.z + if side == 0 { 0.03 } else { -0.02 });
        }
        let mut an = Animator {
            skel,
            pos,
            yaw,
            target_yaw: None,
            vel: Vec3::ZERO,
            desired_vel: Vec3::ZERO,
            stance: Stance::Stand,
            sit_amt: 0.0,
            feet,
            time: 0.0,
            seed: (seed % 1000) as f32 * 0.37,
            look_at: None,
            look_yaw: 0.0,
            look_pitch: 0.0,
            eye_yaw: 0.0,
            eye_pitch: 0.0,
            sacc: (0.0, 0.0),
            sacc_t: 0.0,
            blink: 0.0,
            next_blink: 1.0 + (seed % 7) as f32 * 0.4,
            face: FaceState::of(Expr::Neutral),
            expr: Expr::Neutral,
            talking: false,
            talk_amt: 0.0,
            player: None,
            ch: Chans::default(),
            rest: Chans::default(),
            holding: false,
            events: Vec::new(),
            hair: Spring3::default(),
            hem: Spring3::default(),
            tail: [Vec3::ZERO; TAIL_N],
            tail_prev: [Vec3::ZERO; TAIL_N],
            tail_init: false,
            prev_head: Vec3::ZERO,
            prev_hips: Vec3::ZERO,
            pose: Pose::default(),
            fk: Fk::default(),
            model: Mat4::IDENTITY,
            eye_rot: Quat::IDENTITY,
            blink_amt: 0.0,
            speed: 0.0,
            fidget_t: 6.0 + (seed % 5) as f32,
            hips_world: pos,
            idle_variant: (seed % 3) as u32,
        };
        an.fk = Fk::compute(&an.skel, &an.pose);
        an
    }

    pub fn teleport(&mut self, pos: Vec3, yaw: f32) {
        self.pos = pos;
        self.yaw = yaw;
        self.target_yaw = None;
        self.vel = Vec3::ZERO;
        self.desired_vel = Vec3::ZERO;
        self.stance = Stance::Stand;
        self.sit_amt = 0.0;
        let rot = Quat::from_rotation_y(yaw);
        for side in 0..2 {
            let a = self.skel.bind[FOOT[side]];
            self.feet[side].planted = pos + rot * Vec3::new(a.x * 0.95, 0.0, a.z + if side == 0 { 0.03 } else { -0.02 });
            self.feet[side].swing = false;
            self.feet[side].yaw = yaw;
        }
        self.tail_init = false;
    }

    pub fn sit(&mut self, seat: Vec3, yaw: f32, feet_forward: f32) {
        self.stance = Stance::Sit {
            seat,
            yaw,
            feet: feet_forward,
        };
        self.target_yaw = Some(yaw);
    }

    pub fn stand(&mut self) {
        self.stance = Stance::Stand;
    }

    pub fn is_seated(&self) -> bool {
        matches!(self.stance, Stance::Sit { .. }) && self.sit_amt > 0.95
    }

    pub fn play(&mut self, action: Action) {
        self.holding = false;
        let start = self.ch;
        self.player = Some(ActionPlayer {
            action,
            idx: 0,
            t: 0.0,
            start,
            wiggle: [(Vec3::ZERO, 0.0, 0.0); 2],
            fired: false,
        });
    }

    pub fn busy(&self) -> bool {
        self.player.is_some()
    }

    pub fn action_name(&self) -> Option<&'static str> {
        self.player.as_ref().map(|p| p.action.name)
    }

    pub fn clear_action(&mut self) {
        self.player = None;
        self.holding = false;
    }

    /// Lets a held pose relax back to rest.
    pub fn release(&mut self) {
        self.holding = false;
    }

    pub fn is_holding(&self) -> bool {
        self.holding
    }

    pub fn set_expr(&mut self, e: Expr) {
        self.expr = e;
    }

    /// Converts a world point to character-local space.
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        Quat::from_rotation_y(-self.yaw) * (p - self.pos)
    }

    pub fn to_world(&self, p: Vec3) -> Vec3 {
        self.pos + Quat::from_rotation_y(self.yaw) * p
    }

    pub fn head_world(&self) -> Vec3 {
        self.model.transform_point3(self.fk.pos[HEAD] + Vec3::new(0.0, 0.07, 0.03) * (self.skel.height / 1.65))
    }

    pub fn bone_world(&self, b: usize) -> Mat4 {
        self.model * self.fk.world[b]
    }

    // -------------------------------------------------------------- update

    pub fn update(&mut self, dt: f32) {
        let dt = dt.min(0.05);
        self.time += dt;
        self.events.clear();
        self.update_locomotion(dt);
        self.update_action(dt);
        self.update_face(dt);
        self.solve(dt);
    }

    fn update_locomotion(&mut self, dt: f32) {
        let seated = matches!(self.stance, Stance::Sit { .. });
        let target_sit = if seated { 1.0 } else { 0.0 };
        self.sit_amt = damp(self.sit_amt, target_sit, 3.2, dt);
        if (self.sit_amt - target_sit).abs() < 0.01 {
            self.sit_amt = target_sit;
        }
        let dv = if seated { Vec3::ZERO } else { self.desired_vel };
        // acceleration-limited velocity
        let acc = if dv.length() > self.vel.length() { 5.0 } else { 8.0 };
        let diff = dv - self.vel;
        let step = diff.clamp_length_max(acc * dt);
        self.vel += step;
        self.speed = self.vel.length();
        self.pos += self.vel * dt;
        if let Stance::Sit { seat, .. } = self.stance {
            // slide root under the seat while sitting down
            let tgt = Vec3::new(seat.x, self.pos.y, seat.z);
            self.pos = self.pos.lerp(tgt, damp_factor(4.0, dt) * (self.sit_amt + 0.2).min(1.0));
        }
        // facing
        let face_yaw = if self.speed > 0.15 {
            Some(yaw_of(self.vel))
        } else {
            self.target_yaw
        };
        if let Some(ty) = face_yaw {
            let rate = if self.speed > 0.15 { 7.0 } else { 5.0 };
            self.yaw = damp_angle(self.yaw, ty, rate, dt);
            if self.speed <= 0.15 && wrap_angle(ty - self.yaw).abs() < 0.01 {
                self.target_yaw = None;
                self.yaw = ty;
            }
        }
        self.update_feet(dt);
    }

    fn foot_home(&self, side: usize, lead: f32) -> Vec3 {
        let rot = Quat::from_rotation_y(self.yaw);
        let a = self.skel.bind[FOOT[side]];
        let (x, z) = if let Stance::Sit { feet, .. } = self.stance {
            (a.x * 1.2, feet * self.sit_amt + a.z)
        } else {
            (a.x * 0.95, a.z + if side == 0 { 0.03 } else { -0.02 })
        };
        let mut p = self.pos + rot * Vec3::new(x, 0.0, z) + self.vel * lead;
        p.y = self.pos.y;
        p
    }

    fn update_feet(&mut self, dt: f32) {
        let speed = self.speed;
        let moving = speed > 0.05;
        let step_len = 0.3 + 0.24 * speed.min(2.5);
        let swing_dur = if moving { (0.34 - 0.04 * speed).clamp(0.24, 0.36) } else { 0.28 };
        // progress swings
        for side in 0..2 {
            let f = &mut self.feet[side];
            if f.swing {
                f.t += dt / f.dur;
                // keep retargeting while moving
                if f.t >= 1.0 {
                    f.t = 1.0;
                    f.swing = false;
                    let stride = (f.to - f.from).length();
                    f.planted = f.to;
                    f.yaw = f.to_yaw;
                    // a real step (not a small weight shift): footstep sound
                    if stride > 0.12 && self.sit_amt < 0.3 {
                        self.events.push(EV_STEP);
                    }
                }
            }
        }
        for side in 0..2 {
            if self.feet[side].swing {
                let lead = swing_dur * (1.0 - self.feet[side].t) + 0.5 * step_len / speed.max(0.3) * if moving { 0.5 } else { 0.0 };
                let home = self.foot_home(side, lead);
                let f = &mut self.feet[side];
                f.to = f.to.lerp(home, damp_factor(10.0, dt));
            }
        }
        // decide next step
        let other_busy = |s: usize, feet: &[Foot; 2]| feet[1 - s].swing && feet[1 - s].t < 0.85;
        let mut best: Option<(usize, f32)> = None;
        for side in 0..2 {
            if self.feet[side].swing || other_busy(side, &self.feet) {
                continue;
            }
            let lead = if moving { swing_dur + 0.25 * step_len / speed.max(0.3) } else { 0.0 };
            let home = self.foot_home(side, lead);
            let err = (home - self.feet[side].planted).length();
            let yaw_err = wrap_angle(self.yaw - self.feet[side].yaw).abs();
            let thresh = if moving { step_len * 0.5 } else { 0.1 };
            let score = err + yaw_err * 0.15;
            if err > thresh || yaw_err > 0.5 {
                if best.map(|b| score > b.1).unwrap_or(true) {
                    best = Some((side, score));
                }
            }
        }
        if let Some((side, _)) = best {
            let lead = if moving { swing_dur + 0.25 * step_len / speed.max(0.3) } else { 0.0 };
            let home = self.foot_home(side, lead);
            let yaw = self.yaw;
            let f = &mut self.feet[side];
            f.swing = true;
            f.from = f.planted;
            f.to = home;
            f.t = 0.0;
            f.dur = swing_dur;
            f.from_yaw = f.yaw;
            f.to_yaw = yaw;
        }
    }

    fn foot_pos(&self, side: usize) -> (Vec3, f32, f32) {
        let f = &self.feet[side];
        if f.swing {
            let s = ease_in_out_sine(f.t);
            let lift = (f.t * std::f32::consts::PI).sin() * (0.06 + 0.03 * self.speed.min(1.5)) * (1.0 - self.sit_amt);
            let p = f.from.lerp(f.to, s) + Vec3::Y * lift;
            let yaw = lerp_angle(f.from_yaw, f.to_yaw, s);
            // toe pitch: heel-off early, toe-up before landing
            let pitch = (f.t * std::f32::consts::TAU).sin() * 0.35 * self.speed.min(1.2);
            (p, yaw, pitch)
        } else {
            (f.planted, f.yaw, 0.0)
        }
    }

    fn update_action(&mut self, dt: f32) {
        let mut finished = false;
        let mut target = self.ch;
        if let Some(pl) = self.player.as_mut() {
            if pl.idx < pl.action.phases.len() {
                let phase = &pl.action.phases[pl.idx];
                if !pl.fired {
                    pl.fired = true;
                    for s in &phase.sets {
                        match s {
                            Set::Event(e) => self.events.push(*e),
                            Set::Expr(e) => self.expr = *e,
                            Set::Wiggle(side, axis, amp, freq) => pl.wiggle[*side] = (*axis, *amp, *freq),
                            _ => {}
                        }
                    }
                }
                pl.t += dt;
                let k = phase.ease.apply(pl.t / phase.dur.max(1e-4));
                let st = &pl.start;
                let mut c = *st;
                for s in &phase.sets {
                    match *s {
                        Set::Hand(side, pos, palm, w) => {
                            c.hand[side].pos = st.hand[side].pos.lerp(pos, k);
                            c.hand[side].palm = st.hand[side].palm.lerp(palm, k).normalize_or(palm);
                            c.hand[side].w = lerp(st.hand[side].w, w, k);
                            if st.hand[side].w < 0.01 {
                                // starting from rest: begin at the rest target
                                c.hand[side].pos = self.rest.hand[side].pos.lerp(pos, k);
                            }
                        }
                        Set::HandW(side, w) => c.hand[side].w = lerp(st.hand[side].w, w, k),
                        Set::Grip(side, g) => c.hand[side].grip = lerp(st.hand[side].grip, g, k),
                        Set::Bend(v) => c.bend = lerp(st.bend, v, k),
                        Set::Twist(v) => c.twist = lerp(st.twist, v, k),
                        Set::Side(v) => c.side = lerp(st.side, v, k),
                        Set::HeadPitch(v) => c.head_pitch = lerp(st.head_pitch, v, k),
                        Set::HeadRoll(v) => c.head_roll = lerp(st.head_roll, v, k),
                        Set::HeadYaw(v) => c.head_yaw = lerp(st.head_yaw, v, k),
                        Set::Shrug(v) => c.shrug = lerp(st.shrug, v, k),
                        Set::Crouch(v) => c.crouch = lerp(st.crouch, v, k),
                        Set::Bounce(v) => c.bounce = lerp(st.bounce, v, k),
                        Set::LookLocal(p) => {
                            c.look_local = Some(p);
                            c.look_w = lerp(st.look_w, 1.0, k);
                        }
                        Set::LookOff => c.look_w = lerp(st.look_w, 0.0, k),
                        _ => {}
                    }
                }
                // wiggles
                for side in 0..2 {
                    let (axis, amp, freq) = pl.wiggle[side];
                    if amp > 0.0 {
                        c.hand[side].pos += axis * (self.time * freq * std::f32::consts::TAU).sin() * amp;
                    }
                }
                target = c;
                if pl.t >= phase.dur {
                    pl.idx += 1;
                    pl.t = 0.0;
                    pl.fired = false;
                    pl.start = c;
                    // wiggles only last for the phase that declared them
                    pl.wiggle = [(Vec3::ZERO, 0.0, 0.0); 2];
                    if pl.idx >= pl.action.phases.len() {
                        finished = true;
                    }
                }
            } else {
                finished = true;
            }
            self.ch = target;
        } else if !self.holding {
            // relax toward rest
            let r = 3.5;
            for side in 0..2 {
                self.ch.hand[side].w = damp(self.ch.hand[side].w, 0.0, r, dt);
                self.ch.hand[side].grip = damp(self.ch.hand[side].grip, 0.25, r, dt);
            }
            self.ch.bend = damp(self.ch.bend, 0.0, r, dt);
            self.ch.twist = damp(self.ch.twist, 0.0, r, dt);
            self.ch.side = damp(self.ch.side, 0.0, r, dt);
            self.ch.head_pitch = damp(self.ch.head_pitch, 0.0, r, dt);
            self.ch.head_roll = damp(self.ch.head_roll, 0.0, r, dt);
            self.ch.head_yaw = damp(self.ch.head_yaw, 0.0, r, dt);
            self.ch.shrug = damp(self.ch.shrug, 0.0, r, dt);
            self.ch.crouch = damp(self.ch.crouch, 0.0, r, dt);
            self.ch.look_w = damp(self.ch.look_w, 0.0, r, dt);
            self.ch.bounce = damp(self.ch.bounce, 0.0, r, dt);
        }
        if finished {
            let hold = self.player.as_ref().map(|p| p.action.hold).unwrap_or(false);
            self.player = None;
            self.holding = hold;
        }
        // idle fidgets
        if self.player.is_none() && !self.holding && self.speed < 0.05 && !self.talking && self.sit_amt < 0.05 {
            self.fidget_t -= dt;
            if self.fidget_t <= 0.0 {
                self.fidget_t = 7.0 + hash11(self.time + self.seed) * 9.0;
                let pick = (hash11(self.time * 3.1 + self.seed) * 4.0) as u32;
                let k = self.skel.height / 1.65;
                let a = match pick {
                    0 => super::actions::look_around(k),
                    1 => super::actions::shift_weight(k),
                    2 => super::actions::adjust_sleeve(k),
                    _ => super::actions::stretch_neck(k),
                };
                self.play(a);
            }
        }
    }

    fn update_face(&mut self, dt: f32) {
        let mut target = FaceState::of(self.expr);
        // talking: syllable-like mouth motion
        self.talk_amt = damp(self.talk_amt, if self.talking { 1.0 } else { 0.0 }, 10.0, dt);
        if self.talk_amt > 0.01 {
            let t = self.time * 11.0;
            let syl = (noise1(t, self.seed) * 0.5 + 0.5).powf(1.4);
            let v = (noise1(t * 0.7 + 5.0, self.seed) * 0.5 + 0.5) * 0.6;
            target.open = (target.open + syl * 0.55 * self.talk_amt).min(1.0);
            target.wide += (v - 0.3) * 0.6 * self.talk_amt;
            target.brow_raise[0] += noise1(t * 0.2, self.seed + 3.0) * 0.25 * self.talk_amt;
            target.brow_raise[1] += noise1(t * 0.2, self.seed + 3.0) * 0.22 * self.talk_amt;
        }
        self.face.damp_to(&target, 9.0, dt);
        // blinking
        self.next_blink -= dt;
        if self.next_blink <= 0.0 && self.blink <= 0.0 {
            self.blink = 1.0;
            let r = hash11(self.time * 7.3 + self.seed);
            self.next_blink = 1.8 + r * 4.2;
            if r < 0.15 {
                self.next_blink = 0.25; // double blink
            }
        }
        if self.blink > 0.0 {
            self.blink = (self.blink - dt / 0.17).max(0.0);
        }
        // closing is fast, opening slower
        let b = self.blink;
        self.blink_amt = if b > 0.65 { (1.0 - b) / 0.35 } else { b / 0.65 };
    }

    /// Triggers a blink (e.g. on large gaze shifts).
    pub fn blink_now(&mut self) {
        if self.blink <= 0.0 {
            self.blink = 1.0;
        }
    }

    // -------------------------------------------------------------- pose solving

    fn solve(&mut self, dt: f32) {
        let skel = self.skel.clone();
        let sk = &skel;
        let k = sk.height / 1.65;
        let t = self.time;
        let mut pose = Pose::default();
        let speed = self.speed;
        let walk = smoothstep(0.05, 0.6, speed) * (1.0 - self.sit_amt);
        let idle = 1.0 - walk;

        let root_rot = Quat::from_rotation_y(self.yaw);
        self.model = Mat4::from_rotation_translation(root_rot, self.pos);
        let inv_root = root_rot.inverse();

        // foot targets (character-local)
        let mut foot_local = [Vec3::ZERO; 2];
        let mut foot_yaw = [0.0; 2];
        let mut foot_pitch = [0.0; 2];
        for side in 0..2 {
            let (p, y, pi) = self.foot_pos(side);
            foot_local[side] = inv_root * (p - self.pos);
            foot_yaw[side] = wrap_angle(y - self.yaw);
            foot_pitch[side] = pi;
        }

        // ---- hips
        let breath = (t * std::f32::consts::TAU / 4.2 + self.seed).sin();
        let sway = (t * 0.45 + self.seed).sin();
        let mut hips = Vec3::ZERO;
        // weight shift toward one foot when idle
        hips.x += sway * 0.022 * k * idle;
        hips.z += (t * 0.31 + self.seed * 2.0).sin() * 0.008 * k * idle;
        hips.y -= (0.01 + 0.006 * sway.abs()) * k * idle;
        // walking bob and lateral sway toward stance foot
        let mut swing_s = 0.0;
        for side in 0..2 {
            let f = &self.feet[side];
            if f.swing {
                swing_s = (f.t * std::f32::consts::PI).sin();
                hips.x -= sx(side) * 0.018 * k * swing_s * walk;
            }
        }
        hips.y += (swing_s * 0.022 - 0.02) * k * walk;
        hips.y -= self.ch.crouch * 0.25 * k;
        hips.y += self.ch.bounce * (t * 9.0).sin().abs() * 0.04 * k;
        // leg reach: never over-extend
        let hip_bind = sk.bind[HIPS];
        let leg_len = sk.thigh_len + sk.shin_len;
        let mut hips_off = hips;
        // sitting
        let mut sit_drop = 0.0;
        if let Stance::Sit { seat, .. } = self.stance {
            let seat_local = inv_root * (seat - self.pos);
            let seated = Vec3::new(seat_local.x, seat_local.y + 0.14 * k - hip_bind.y, seat_local.z - 0.02);
            let s = ease_in_out(self.sit_amt);
            hips_off = hips_off.lerp(seated, s);
            sit_drop = seated.y * s;
        } else if self.sit_amt > 0.0 {
            sit_drop = hips_off.y.min(0.0) * self.sit_amt;
        }
        for side in 0..2 {
            let hip_j = sk.bind[THIGH[side]] + hips_off;
            let ankle = foot_local[side] + Vec3::new(0.0, sk.bind[FOOT[side]].y, 0.0);
            let d = hip_j.distance(ankle);
            let max = leg_len * 0.985;
            if d > max {
                let over = d - max;
                hips_off.y -= over;
            }
        }
        pose.hips_offset = hips_off;
        // hip rotation: yaw toward forward foot, roll with stance
        let fwd_diff = foot_local[0].z - foot_local[1].z;
        let hip_yaw = fwd_diff * 0.35 * walk;
        let hip_roll = -hips.x / k * 0.9;
        pose.rot[HIPS] = Quat::from_rotation_y(hip_yaw) * Quat::from_rotation_z(hip_roll);

        // ---- spine
        let lean_fwd = 0.06 * speed.min(2.0) * walk;
        let bend = self.ch.bend + lean_fwd + 0.02 * breath * idle;
        let twist = self.ch.twist - hip_yaw * 1.4;
        let side_bend = self.ch.side - hip_roll * 0.6;
        pose.rot[SPINE] = Quat::from_euler(glam::EulerRot::YXZ, twist * 0.4, bend * 0.45, side_bend * 0.5);
        pose.rot[CHEST] = Quat::from_euler(glam::EulerRot::YXZ, twist * 0.6, bend * 0.4 - 0.015 * breath, side_bend * 0.5);
        pose.scale[CHEST] = Vec3::new(1.0 + 0.008 * breath, 1.0, 1.0 + 0.014 * breath);
        for side in 0..2 {
            let s = sx(side);
            pose.rot[CLAV[side]] = Quat::from_rotation_z(s * (self.ch.shrug * 0.25 + 0.012 * breath));
        }

        // provisional FK for the spine chain
        self.fk = Fk::compute(sk, &pose);

        // ---- head & gaze
        let head_pos_model = self.fk.pos[HEAD] + Vec3::new(0.0, 0.07 * k, 0.0);
        let mut want_yaw = self.ch.head_yaw;
        let mut want_pitch = self.ch.head_pitch;
        let mut have_look = false;
        let look_target_local = if self.ch.look_w > 0.01 {
            self.ch.look_local.map(|p| p + Vec3::Y * sit_drop)
        } else {
            None
        };
        let look_local = look_target_local.or_else(|| self.look_at.map(|w| inv_root * (w - self.pos)));
        if let Some(lp) = look_local {
            let d = lp - head_pos_model;
            let yaw = d.x.atan2(d.z);
            let pitch = (-d.y).atan2((d.x * d.x + d.z * d.z).sqrt());
            let w = if look_target_local.is_some() { self.ch.look_w } else { 1.0 };
            want_yaw += yaw.clamp(-1.25, 1.25) * w;
            want_pitch += pitch.clamp(-0.6, 0.7) * w;
            have_look = true;
        }
        // idle micro motion
        want_yaw += noise1(t * 0.23, self.seed) * 0.06;
        want_pitch += noise1(t * 0.19, self.seed + 9.0) * 0.04 + 0.01 * breath;
        let old_yaw = self.look_yaw;
        self.look_yaw = damp(self.look_yaw, want_yaw, if have_look { 5.0 } else { 3.0 }, dt);
        self.look_pitch = damp(self.look_pitch, want_pitch, 5.0, dt);
        if (want_yaw - old_yaw).abs() > 0.6 {
            self.blink_now();
        }
        // distribute over chest, neck and head
        let (ly, lp) = (self.look_yaw, self.look_pitch);
        pose.rot[CHEST] = pose.rot[CHEST] * Quat::from_rotation_y(ly * 0.12);
        pose.rot[NECK] = Quat::from_euler(glam::EulerRot::YXZ, ly * 0.35, lp * 0.35, 0.0);
        pose.rot[HEAD] = Quat::from_euler(glam::EulerRot::YXZ, ly * 0.45, lp * 0.55, self.ch.head_roll + noise1(t * 0.17, self.seed + 4.0) * 0.03);

        // eyes: lead the head toward the target + saccades
        self.sacc_t -= dt;
        if self.sacc_t <= 0.0 {
            self.sacc_t = 0.4 + hash11(t * 5.1 + self.seed) * 1.8;
            let r1 = hash11(t * 2.3 + self.seed) - 0.5;
            let r2 = hash11(t * 4.7 + self.seed) - 0.5;
            self.sacc = (r1 * 0.16, r2 * 0.08);
        }
        let eye_y = (want_yaw - self.look_yaw) * 0.9 + self.sacc.0;
        let eye_p = (want_pitch - self.look_pitch) * 0.8 + self.sacc.1;
        self.eye_yaw = damp(self.eye_yaw, eye_y.clamp(-0.45, 0.45), 30.0, dt);
        self.eye_pitch = damp(self.eye_pitch, eye_p.clamp(-0.3, 0.3), 30.0, dt);
        // convergence toward near targets is ignored; eyes rotate in head space
        self.eye_rot = Quat::from_euler(glam::EulerRot::YXZ, self.eye_yaw, self.eye_pitch, 0.0);

        self.fk = Fk::compute(sk, &pose);

        // ---- legs (IK)
        for side in 0..2 {
            let hip_j = self.fk.pos[THIGH[side]];
            let ankle = foot_local[side] + Vec3::new(0.0, sk.bind[FOOT[side]].y, 0.0);
            let fwd = Quat::from_rotation_y(foot_yaw[side]) * Vec3::Z;
            let pole = fwd + Vec3::new(sx(side) * 0.08, 0.0, 0.0);
            let (knee, end) = two_bone(hip_j, ankle, sk.thigh_len, sk.shin_len, pole);
            let thigh_dir = (knee - hip_j).normalize();
            pose.rot[THIGH[side]] = aim_local(&self.fk, sk, THIGH[side], thigh_dir, Vec3::Z, fwd);
            self.fk.update_bone(sk, &pose, THIGH[side]);
            let shin_dir = (end - knee).normalize();
            pose.rot[SHIN[side]] = aim_local(&self.fk, sk, SHIN[side], shin_dir, Vec3::Z, fwd);
            self.fk.update_bone(sk, &pose, SHIN[side]);
            // foot flat with pitch
            let foot_world = Quat::from_rotation_y(foot_yaw[side]) * Quat::from_rotation_x(-foot_pitch[side] * 0.5);
            pose.rot[FOOT[side]] = (self.fk.rot[SHIN[side]].inverse() * foot_world).normalize();
            self.fk.update_bone(sk, &pose, FOOT[side]);
        }

        // ---- arms
        for side in 0..2 {
            let s = sx(side);
            let shoulder = self.fk.pos[UPARM[side]];
            // rest target: hanging, swinging with the opposite foot
            let opp = foot_local[1 - side].z - foot_local[side].z;
            let swing = opp * 0.55 * walk;
            let mut rest = Vec3::new(
                s * (sk.bind[UPARM[side]].x.abs() + 0.035 * k),
                shoulder.y - (sk.upper_len + sk.fore_len) * 0.93,
                0.03 * k + swing,
            );
            rest.y += swing.abs() * 0.12;
            // subtle breathing / idle motion
            rest.x += s * 0.004 * breath;
            rest.z += noise1(t * 0.3, self.seed + s) * 0.01;
            let mut sitting_rest = Vec3::new(s * 0.1 * k, hips_off.y + hip_bind.y + 0.1 * k, 0.28 * k);
            if let Stance::Sit { .. } = self.stance {
                sitting_rest.y = self.fk.pos[HIPS].y + 0.12 * k;
                sitting_rest.z = self.fk.pos[HIPS].z + 0.3 * k;
            }
            let rest = rest.lerp(sitting_rest, self.sit_amt);
            self.rest.hand[side].pos = rest;
            let mut hc = self.ch.hand[side];
            // gesture targets are authored standing: follow the body when seated
            hc.pos.y += sit_drop;
            let target = rest.lerp(hc.pos, hc.w);
            let palm_rest = Vec3::new(-s, -0.2, 0.15).normalize();
            let palm_rest = palm_rest.lerp(Vec3::NEG_Y, self.sit_amt).normalize();
            let palm = palm_rest.lerp(hc.palm, hc.w).normalize_or(palm_rest);
            let pole = Vec3::new(s * 0.45, -1.0, -0.5).normalize();
            let (elbow, wrist) = two_bone(shoulder, target, sk.upper_len, sk.fore_len, pole);
            let up_dir = (elbow - shoulder).normalize();
            let fore_dir = (wrist - elbow).normalize();
            let mut bend_ref = fore_dir - up_dir * fore_dir.dot(up_dir);
            if bend_ref.length_squared() < 1e-5 {
                bend_ref = Vec3::Z;
            }
            pose.rot[UPARM[side]] = aim_local(&self.fk, sk, UPARM[side], up_dir, Vec3::Z, bend_ref.normalize());
            self.fk.update_bone(sk, &pose, UPARM[side]);
            // forearm twist follows the palm
            let mut palm_perp = palm - fore_dir * palm.dot(fore_dir);
            if palm_perp.length_squared() < 1e-5 {
                palm_perp = bend_ref;
            }
            let bind_palm = sk.bone_dir(FOREARM[side]).cross(Vec3::Z).normalize() * s;
            pose.rot[FOREARM[side]] = aim_local(&self.fk, sk, FOREARM[side], fore_dir, bind_palm, palm_perp.normalize());
            self.fk.update_bone(sk, &pose, FOREARM[side]);
            // hand continues the forearm, slightly relaxed
            let relax = Quat::from_axis_angle(Vec3::Z, 0.0);
            pose.rot[HAND[side]] = relax;
            self.fk.update_bone(sk, &pose, HAND[side]);
            // finger curl around the palm's side axis
            let grip = hc.grip.max(0.15 + 0.05 * breath.abs());
            let axis = sk.bone_dir(HAND[side]).cross(bind_palm).normalize();
            pose.rot[FINGERS[side]] = Quat::from_axis_angle(axis, grip * 1.4);
            self.fk.update_bone(sk, &pose, FINGERS[side]);
        }

        // ---- secondary motion
        self.fk = Fk::compute(sk, &pose);
        let head_w = self.model.transform_point3(self.fk.pos[HEAD]);
        let hips_w = self.model.transform_point3(self.fk.pos[HIPS]);
        let head_vel = (head_w - self.prev_head) / dt.max(1e-3);
        let hips_vel = (hips_w - self.prev_hips) / dt.max(1e-3);
        self.prev_head = head_w;
        self.prev_hips = hips_w;
        let hv_local = (inv_root * head_vel).clamp_length_max(4.0);
        let target_h = Vec3::new(-hv_local.x, 0.0, -hv_local.z) * 0.06;
        self.hair.update(target_h, 2.2, 0.35, dt);
        let hr = self.hair.x;
        pose.rot[HAIR] = Quat::from_euler(glam::EulerRot::XYZ, hr.z * 0.8, 0.0, -hr.x * 0.8);
        let pv_local = (inv_root * hips_vel).clamp_length_max(4.0);
        let target_m = Vec3::new(-pv_local.x, 0.0, -pv_local.z) * 0.05;
        self.hem.update(target_m, 2.6, 0.3, dt);
        let hm = self.hem.x;
        pose.rot[HEM] = Quat::from_euler(glam::EulerRot::XYZ, hm.z * 0.9, 0.0, -hm.x * 0.9);

        // ponytail verlet chain in model space
        self.fk = Fk::compute(sk, &pose);
        self.solve_tail(dt, &mut pose);
        self.fk = Fk::compute(sk, &pose);
        self.pose = pose;
        self.hips_world = self.model.transform_point3(self.fk.pos[HIPS]);
    }

    fn solve_tail(&mut self, dt: f32, pose: &mut Pose) {
        let skel = self.skel.clone();
        let sk = &skel;
        let world = |p: Vec3, m: &Mat4| m.transform_point3(p);
        let head_m = self.model * self.fk.world[HEAD];
        let anchor = world(sk.bind[TAIL0] - sk.bind[HEAD], &head_m);
        let seg: Vec<f32> = (0..TAIL_N - 1).map(|i| (sk.bind[TAIL0 + i + 1] - sk.bind[TAIL0 + i]).length()).collect();
        if !self.tail_init {
            for i in 0..TAIL_N {
                self.tail[i] = world(sk.bind[TAIL0 + i] - sk.bind[HEAD], &head_m);
                self.tail_prev[i] = self.tail[i];
            }
            self.tail_init = true;
        }
        self.tail[0] = anchor;
        self.tail_prev[0] = anchor;
        let g = Vec3::new(0.0, -9.8, 0.0);
        let damping = 0.9;
        for i in 1..TAIL_N {
            let cur = self.tail[i];
            let vel = (cur - self.tail_prev[i]) * damping;
            self.tail_prev[i] = cur;
            self.tail[i] = cur + vel + g * dt * dt;
        }
        // keep away from the head/neck
        let head_c = world(Vec3::new(0.0, 0.07, 0.0) * (sk.height / 1.65), &head_m);
        let neck_c = self.model.transform_point3(self.fk.pos[NECK]);
        // gently pull the tail toward the head's mid-plane so it hangs down the back
        let side_axis = head_m.transform_vector3(Vec3::X).normalize_or(Vec3::X);
        for i in 1..TAIL_N {
            let off = (self.tail[i] - anchor).dot(side_axis);
            self.tail[i] -= side_axis * off * (1.0 - (-6.0 * dt).exp());
        }
        for _ in 0..3 {
            for i in 1..TAIL_N {
                let a = self.tail[i - 1];
                let b = self.tail[i];
                let d = b - a;
                let l = d.length().max(1e-5);
                self.tail[i] = a + d / l * seg[i - 1];
                // collide
                for (c, r) in [(head_c, 0.098 * sk.height / 1.65), (neck_c, 0.075 * sk.height / 1.65)] {
                    let v = self.tail[i] - c;
                    if v.length() < r {
                        self.tail[i] = c + v.normalize_or(Vec3::NEG_Z) * r;
                    }
                }
            }
        }
        // orient tail bones
        let inv_model = self.model.inverse();
        for i in 0..TAIL_N {
            let b = TAIL0 + i;
            let dir_world = if i + 1 < TAIL_N {
                self.tail[i + 1] - self.tail[i]
            } else {
                self.tail[i] - self.tail[i - 1]
            };
            let dir_model = inv_model.transform_vector3(dir_world).normalize_or(Vec3::NEG_Y);
            let bind_dir = if i + 1 < TAIL_N {
                (sk.bind[b + 1] - sk.bind[b]).normalize()
            } else {
                (sk.bind[b] - sk.bind[b - 1]).normalize()
            };
            let world_rot = Quat::from_rotation_arc(bind_dir, dir_model);
            let parent_rot = self.fk.rot[PARENT[b]];
            pose.rot[b] = (parent_rot.inverse() * world_rot).normalize();
            self.fk.update_bone(sk, pose, b);
        }
    }
}
