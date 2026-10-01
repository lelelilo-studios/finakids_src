//! Humanoid skeleton definition and forward kinematics.

use glam::{Mat4, Quat, Vec3};

pub const ROOT: usize = 0;
pub const HIPS: usize = 1;
pub const SPINE: usize = 2;
pub const CHEST: usize = 3;
pub const NECK: usize = 4;
pub const HEAD: usize = 5;
pub const CLAV_L: usize = 6;
pub const UPARM_L: usize = 7;
pub const FOREARM_L: usize = 8;
pub const HAND_L: usize = 9;
pub const FINGERS_L: usize = 10;
pub const CLAV_R: usize = 11;
pub const UPARM_R: usize = 12;
pub const FOREARM_R: usize = 13;
pub const HAND_R: usize = 14;
pub const FINGERS_R: usize = 15;
pub const THIGH_L: usize = 16;
pub const SHIN_L: usize = 17;
pub const FOOT_L: usize = 18;
pub const THIGH_R: usize = 19;
pub const SHIN_R: usize = 20;
pub const FOOT_R: usize = 21;
pub const HAIR: usize = 22;
pub const TAIL0: usize = 23; // ponytail chain 23..=27
pub const TAIL_N: usize = 5;
pub const HEM: usize = 28;
// Finger articulation. `FINGERS_*` (above) drives the first phalanx of the middle, ring
// and little fingers; these add their outer phalanges, the index finger and the thumb.
pub const FINGERS2_L: usize = 29;
pub const INDEX_L: usize = 30;
pub const INDEX2_L: usize = 31;
pub const THUMB_L: usize = 32;
pub const THUMB2_L: usize = 33;
pub const FINGERS2_R: usize = 34;
pub const INDEX_R: usize = 35;
pub const INDEX2_R: usize = 36;
pub const THUMB_R: usize = 37;
pub const THUMB2_R: usize = 38;
pub const BONE_COUNT: usize = 39;

pub const PARENT: [usize; BONE_COUNT] = [
    usize::MAX, // root
    ROOT,
    HIPS,
    SPINE,
    CHEST,
    NECK,
    CHEST,
    CLAV_L,
    UPARM_L,
    FOREARM_L,
    HAND_L,
    CHEST,
    CLAV_R,
    UPARM_R,
    FOREARM_R,
    HAND_R,
    HIPS,
    THIGH_L,
    SHIN_L,
    HIPS,
    THIGH_R,
    SHIN_R,
    HEAD,
    HEAD,
    TAIL0,
    TAIL0 + 1,
    TAIL0 + 2,
    TAIL0 + 3,
    HIPS,
    FINGERS_L,
    HAND_L,
    INDEX_L,
    HAND_L,
    THUMB_L,
    FINGERS_R,
    HAND_R,
    INDEX_R,
    HAND_R,
    THUMB_R,
];

/// Side-dependent bone ids: index 0 = left, 1 = right.
pub const CLAV: [usize; 2] = [CLAV_L, CLAV_R];
pub const UPARM: [usize; 2] = [UPARM_L, UPARM_R];
pub const FOREARM: [usize; 2] = [FOREARM_L, FOREARM_R];
pub const HAND: [usize; 2] = [HAND_L, HAND_R];
pub const FINGERS: [usize; 2] = [FINGERS_L, FINGERS_R];
pub const FINGERS2: [usize; 2] = [FINGERS2_L, FINGERS2_R];
pub const INDEX: [usize; 2] = [INDEX_L, INDEX_R];
pub const INDEX2: [usize; 2] = [INDEX2_L, INDEX2_R];
pub const THUMB: [usize; 2] = [THUMB_L, THUMB_R];
pub const THUMB2: [usize; 2] = [THUMB2_L, THUMB2_R];
pub const THIGH: [usize; 2] = [THIGH_L, THIGH_R];
pub const SHIN: [usize; 2] = [SHIN_L, SHIN_R];
pub const FOOT: [usize; 2] = [FOOT_L, FOOT_R];

/// Bind pose (A-pose) joint positions in model space; the character faces +Z, left is +X.
#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bind: [Vec3; BONE_COUNT],
    /// Extra reference points: fingertips, toes, head top.
    pub fingertip: [Vec3; 2],
    pub toe: [Vec3; 2],
    pub head_top: Vec3,
    pub upper_len: f32,
    pub fore_len: f32,
    /// Hand size relative to the body scale.
    pub hand_scale: f32,
    pub thigh_len: f32,
    pub shin_len: f32,
    pub height: f32,
    pub hip_width: f32,
}

/// Mirror helper: side 0 = left (+x), 1 = right (-x).
pub fn sx(side: usize) -> f32 {
    if side == 0 {
        1.0
    } else {
        -1.0
    }
}

pub fn arm_dir(side: usize) -> Vec3 {
    Vec3::new(0.643 * sx(side), -0.766, -0.02).normalize()
}

pub fn forearm_dir(side: usize) -> Vec3 {
    Vec3::new(0.62 * sx(side), -0.78, 0.08).normalize()
}

/// Bind-pose hand frame: `f` along the fingers, `t` toward the thumb side, `n` out of the palm.
#[derive(Clone, Copy, Debug)]
pub struct HandFrame {
    pub f: Vec3,
    pub t: Vec3,
    pub n: Vec3,
}

pub fn hand_frame(side: usize) -> HandFrame {
    let f = forearm_dir(side);
    let n = f.cross(Vec3::Z).normalize() * sx(side);
    let t = (Vec3::Z - f * Vec3::Z.dot(f) - n * Vec3::Z.dot(n)).normalize();
    HandFrame { f, t, n }
}

/// Hand landmarks in the hand frame (metres at unit scale, relative to the wrist).
pub mod hand {
    /// Knuckle (MCP) position of index, middle, ring and little finger: (along f, along t).
    pub const KNUCKLE: [(f32, f32); 4] = [(0.091, 0.0268), (0.0945, 0.0088), (0.0905, -0.0088), (0.0825, -0.0258)];
    /// Finger lengths from knuckle to tip.
    pub const LENGTH: [f32; 4] = [0.069, 0.077, 0.072, 0.057];
    /// Fractions of the finger length taken by the first and second phalanx.
    pub const PHALANX: (f32, f32) = (0.44, 0.3);
    /// Thumb base (CMC joint): (f, t, n).
    pub const THUMB_BASE: (f32, f32, f32) = (0.02, 0.027, 0.006);
    /// Resting direction of the thumb metacarpal in (f, t, n) components (normalised by the user).
    pub const THUMB_DIR: (f32, f32, f32) = (0.82, 0.52, 0.22);
    /// Resting direction of the thumb phalanges: they swing back alongside the index.
    pub const THUMB_DIR2: (f32, f32, f32) = (0.94, 0.2, 0.27);
    /// Thumb segment lengths: metacarpal, first phalanx, last phalanx.
    pub const THUMB_LEN: (f32, f32, f32) = (0.038, 0.03, 0.026);
}

impl Skeleton {
    pub fn new(height: f32, shoulder: f32, hips: f32) -> Skeleton {
        let k = height / 1.65;
        let mut b = [Vec3::ZERO; BONE_COUNT];
        let v = |x: f32, y: f32, z: f32| Vec3::new(x, y, z) * k;
        b[ROOT] = Vec3::ZERO;
        b[HIPS] = v(0.0, 0.925, 0.0);
        b[SPINE] = v(0.0, 1.06, -0.005);
        b[CHEST] = v(0.0, 1.195, -0.012);
        b[NECK] = v(0.0, 1.405, -0.022);
        b[HEAD] = v(0.0, 1.495, -0.008);
        let upper_len = 0.28 * k;
        let fore_len = 0.245 * k;
        let hand_scale = 1.0 + (shoulder - 1.0) * 1.25;
        for side in 0..2 {
            let s = sx(side);
            b[CLAV[side]] = v(0.02 * s, 1.37, -0.012);
            let sh = v(0.158 * s * shoulder, 1.35, -0.025);
            b[UPARM[side]] = sh;
            let el = sh + arm_dir(side) * upper_len;
            b[FOREARM[side]] = el;
            let wr = el + forearm_dir(side) * fore_len;
            b[HAND[side]] = wr;
            // finger joints (hands are slightly larger on broad-shouldered builds)
            let hk = k * hand_scale;
            let hf = hand_frame(side);
            let at = |f: f32, t: f32, n: f32| wr + (hf.f * f + hf.t * t + hf.n * n) * hk;
            let (p1, _) = hand::PHALANX;
            let ring = hand::KNUCKLE[2];
            b[FINGERS[side]] = at(ring.0, ring.1 + 0.004, -0.001);
            b[FINGERS2[side]] = at(ring.0 + hand::LENGTH[2] * p1, ring.1 + 0.004, 0.0015);
            let idx = hand::KNUCKLE[0];
            b[INDEX[side]] = at(idx.0, idx.1, -0.001);
            b[INDEX2[side]] = at(idx.0 + hand::LENGTH[0] * p1, idx.1, 0.0015);
            let tb = hand::THUMB_BASE;
            let td = Vec3::new(hand::THUMB_DIR.0, hand::THUMB_DIR.1, hand::THUMB_DIR.2).normalize();
            b[THUMB[side]] = at(tb.0, tb.1, tb.2);
            let l = hand::THUMB_LEN.0;
            b[THUMB2[side]] = at(tb.0 + td.x * l, tb.1 + td.y * l, tb.2 + td.z * l);
            b[THIGH[side]] = v(0.088 * s * hips, 0.87, 0.0);
            b[SHIN[side]] = v(0.092 * s * hips.sqrt(), 0.475, 0.012);
            b[FOOT[side]] = v(0.095 * s * hips.sqrt(), 0.075, -0.02);
        }
        b[HAIR] = b[HEAD] + v(0.0, 0.07, -0.02);
        let hs = super::build::HEAD_SCALE;
        let piv = super::build::HEAD_PIVOT;
        for i in 0..TAIL_N {
            let y = 1.585 - 0.052 * i as f32;
            let z = -0.118 - 0.01 * i as f32;
            b[TAIL0 + i] = Vec3::new(0.0, piv + (y - piv) * hs, z * hs) * k;
        }
        b[HEM] = v(0.0, 1.0, 0.0);
        let tip_len = (hand::KNUCKLE[1].0 + hand::LENGTH[1]) * k * hand_scale;
        let fingertip = [b[HAND_L] + forearm_dir(0) * tip_len, b[HAND_R] + forearm_dir(1) * tip_len];
        let toe = [
            Vec3::new(b[FOOT_L].x + 0.008 * k, 0.02 * k, 0.15 * k),
            Vec3::new(b[FOOT_R].x - 0.008 * k, 0.02 * k, 0.15 * k),
        ];
        Skeleton {
            bind: b,
            fingertip,
            toe,
            head_top: v(0.0, 1.65, 0.0),
            upper_len,
            fore_len,
            hand_scale,
            thigh_len: (b[THIGH_L] - b[SHIN_L]).length(),
            shin_len: (b[SHIN_L] - b[FOOT_L]).length(),
            height,
            hip_width: hips,
        }
    }

    /// Direction from a bone's joint to its main child joint in bind pose.
    pub fn bone_dir(&self, bone: usize) -> Vec3 {
        let child = match bone {
            HIPS => SPINE,
            SPINE => CHEST,
            CHEST => NECK,
            NECK => HEAD,
            HEAD => return Vec3::Y,
            CLAV_L => UPARM_L,
            CLAV_R => UPARM_R,
            UPARM_L => FOREARM_L,
            UPARM_R => FOREARM_R,
            FOREARM_L => HAND_L,
            FOREARM_R => HAND_R,
            HAND_L | FINGERS_L | FINGERS2_L | INDEX_L | INDEX2_L => return forearm_dir(0),
            HAND_R | FINGERS_R | FINGERS2_R | INDEX_R | INDEX2_R => return forearm_dir(1),
            THUMB_L | THUMB2_L | THUMB_R | THUMB2_R => {
                let side = if bone == THUMB_L || bone == THUMB2_L { 0 } else { 1 };
                let hf = hand_frame(side);
                let d = if bone == THUMB_L || bone == THUMB_R { hand::THUMB_DIR } else { hand::THUMB_DIR2 };
                return (hf.f * d.0 + hf.t * d.1 + hf.n * d.2).normalize();
            }
            THIGH_L => SHIN_L,
            THIGH_R => SHIN_R,
            SHIN_L => FOOT_L,
            SHIN_R => FOOT_R,
            FOOT_L => return (self.toe[0] - self.bind[FOOT_L]).normalize(),
            FOOT_R => return (self.toe[1] - self.bind[FOOT_R]).normalize(),
            _ => return Vec3::NEG_Y,
        };
        (self.bind[child] - self.bind[bone]).normalize_or(Vec3::Y)
    }
}

/// Local pose: rotation per bone relative to bind, plus root placement.
#[derive(Clone, Debug)]
pub struct Pose {
    pub rot: [Quat; BONE_COUNT],
    pub scale: [Vec3; BONE_COUNT],
    /// Offset added to the hips joint (model space, before root transform).
    pub hips_offset: Vec3,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            rot: [Quat::IDENTITY; BONE_COUNT],
            scale: [Vec3::ONE; BONE_COUNT],
            hips_offset: Vec3::ZERO,
        }
    }
}

/// World (model-space) transforms of every bone.
#[derive(Clone, Debug)]
pub struct Fk {
    pub world: [Mat4; BONE_COUNT],
    pub rot: [Quat; BONE_COUNT],
    pub pos: [Vec3; BONE_COUNT],
}

impl Default for Fk {
    fn default() -> Self {
        Fk {
            world: [Mat4::IDENTITY; BONE_COUNT],
            rot: [Quat::IDENTITY; BONE_COUNT],
            pos: [Vec3::ZERO; BONE_COUNT],
        }
    }
}

impl Fk {
    /// Full forward kinematics in model space.
    pub fn compute(skel: &Skeleton, pose: &Pose) -> Fk {
        let mut fk = Fk::default();
        for b in 0..BONE_COUNT {
            fk.update_bone(skel, pose, b);
        }
        fk
    }

    pub fn update_bone(&mut self, skel: &Skeleton, pose: &Pose, b: usize) {
        let p = PARENT[b];
        let (ppos, prot, pmat) = if p == usize::MAX {
            (Vec3::ZERO, Quat::IDENTITY, Mat4::IDENTITY)
        } else {
            (self.pos[p], self.rot[p], self.world[p])
        };
        let local_off = if p == usize::MAX {
            skel.bind[b]
        } else {
            skel.bind[b] - skel.bind[p]
        };
        let mut off = local_off;
        if b == HIPS {
            off += pose.hips_offset;
        }
        let pos = if p == usize::MAX { off } else { pmat.transform_point3(off) };
        let rot = (prot * pose.rot[b]).normalize();
        self.pos[b] = pos;
        self.rot[b] = rot;
        // scale is applied locally (not inherited) for breathing
        self.world[b] = Mat4::from_scale_rotation_translation(pose.scale[b], rot, pos);
        let _ = ppos;
    }

    /// Recompute a bone and all of its descendants.
    pub fn update_chain(&mut self, skel: &Skeleton, pose: &Pose, from: usize) {
        self.update_bone(skel, pose, from);
        for b in from + 1..BONE_COUNT {
            if is_descendant(b, from) {
                self.update_bone(skel, pose, b);
            }
        }
    }

    pub fn skin_matrices(&self, skel: &Skeleton, out: &mut [Mat4]) {
        for b in 0..BONE_COUNT {
            out[b] = self.world[b] * Mat4::from_translation(-skel.bind[b]);
        }
    }
}

pub fn is_descendant(mut b: usize, ancestor: usize) -> bool {
    while b != usize::MAX {
        if b == ancestor {
            return true;
        }
        b = PARENT[b];
    }
    false
}
