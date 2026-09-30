//! Procedural character construction: SDF body, clothing, head, hands and hair
//! are meshed with surface nets and skinned to the skeleton.

use super::appearance::{Appearance, Bottom, Hair, Top};
use super::skeleton::*;
use crate::gfx::mesh::{kind, Mat, MeshData, SkinMeshData, SkinVertex};
use crate::math::{lerp, smoothstep};
use crate::sdf::{surface_nets, BoneSpec, MeshOut, Op, Prim, Sdf, Shape};
use glam::{Mat3, Quat, Vec3};

pub const M_SKIN: u8 = 0;
pub const M_TOP: u8 = 1;
pub const M_TOP2: u8 = 2;
pub const M_BOTTOM: u8 = 3;
pub const M_SHOE: u8 = 4;
pub const M_SOLE: u8 = 5;
pub const M_HAIR: u8 = 6;
pub const M_LIP: u8 = 7;
pub const M_BELT: u8 = 8;
pub const M_WHITE: u8 = 9;
pub const M_METAL: u8 = 10;
pub const M_TIE: u8 = 11;
pub const M_LEGS: u8 = 12;
pub const M_DARK: u8 = 13;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Quality {
    High,
    Low,
}

/// Facial landmarks relative to the head bone (bind space offset from HEAD joint).
#[derive(Clone, Debug, Default)]
pub struct FaceLayout {
    pub eye: [Vec3; 2],
    pub eye_r: f32,
    pub brow: [[Vec3; 4]; 2],
    pub mouth: Vec3,
    pub mouth_w: f32,
    pub face_curve: f32,
    pub nose_tip: Vec3,
    pub head_center: Vec3,
}

pub struct CharacterMeshes {
    pub skin: SkinMeshData,
    pub face: FaceLayout,
    pub mats: Vec<Mat>,
}

pub fn materials(a: &Appearance) -> Vec<Mat> {
    let top_kind = match a.top {
        Top::Sweater => kind::KNIT,
        _ => kind::FABRIC,
    };
    let bottom_kind = match a.bottom {
        Bottom::Jeans => kind::DENIM,
        _ => kind::FABRIC,
    };
    let skin = crate::math::rgb(a.skin);
    let lip = [
        (skin[0] as f32 * 0.9) as u8,
        (skin[1] as f32 * 0.66) as u8,
        (skin[2] as f32 * 0.66) as u8,
        255,
    ];
    let lip_hex = ((lip[0] as u32) << 16) | ((lip[1] as u32) << 8) | lip[2] as u32;
    let legs = if a.bottom == Bottom::Skirt { 0x3a3032 } else { a.skin };
    vec![
        Mat::new(a.skin, 0.6).kind(kind::SKIN),
        Mat::new(a.top_color, if a.top == Top::Jacket { 0.65 } else { 0.9 }).kind(top_kind),
        Mat::new(a.top_color2, 0.85).kind(top_kind),
        Mat::new(a.bottom_color, 0.9).kind(bottom_kind),
        Mat::new(a.shoe_color, 0.55).kind(kind::LEATHER),
        Mat::new(a.sole_color, 0.8),
        Mat::new(a.hair_color, 0.5).kind(kind::HAIR),
        Mat::new(lip_hex, 0.35).kind(kind::SKIN),
        Mat::new(0x2a2320, 0.45).kind(kind::LEATHER),
        Mat::new(0xefebe4, 0.8).kind(kind::FABRIC),
        Mat::new(0x9a9aa0, 0.3).metal(1.0),
        Mat::new(0x222226, 0.6),
        Mat::new(legs, if a.bottom == Bottom::Skirt { 0.7 } else { 0.52 }).kind(if a.bottom == Bottom::Skirt {
            kind::FABRIC
        } else {
            kind::SKIN
        }),
        Mat::new(0x1a1414, 0.9),
    ]
}

pub const HEAD_SCALE: f32 = 1.08;
pub const HEAD_PIVOT: f32 = 1.47;

struct P {
    k: f32,
    hs: f32,
}

impl P {
    fn body(k: f32) -> P {
        P { k, hs: 1.0 }
    }
    fn head(k: f32) -> P {
        P { k, hs: HEAD_SCALE }
    }
    fn v(&self, x: f32, y: f32, z: f32) -> Vec3 {
        let piv = Vec3::new(0.0, HEAD_PIVOT, 0.0);
        (piv + (Vec3::new(x, y, z) - piv) * self.hs) * self.k
    }
    fn r(&self, r: f32) -> f32 {
        r * self.k * self.hs
    }
}

fn rot_y_to(dir: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::Y, dir.normalize())
}

fn ell(c: Vec3, r: Vec3, rot: Quat) -> Shape {
    Shape::Ellipsoid { c, r, inv: rot.inverse() }
}

fn rbox(c: Vec3, half: Vec3, r: f32, rot: Quat) -> Shape {
    Shape::RoundBox { c, half, r, inv: rot.inverse() }
}

fn torus(c: Vec3, big: f32, small: f32, axis: Vec3) -> Shape {
    let rot = rot_y_to(axis);
    Shape::Torus { c, big, small, inv: rot.inverse() }
}

fn cone(a: Vec3, b: Vec3, ra: f32, rb: f32) -> Shape {
    Shape::RoundCone { a, b, ra, rb }
}

/// Inflates a shape by `t` (used for clothing layers).
fn inflate(s: Shape, t: f32) -> Shape {
    match s {
        Shape::Sphere { c, r } => Shape::Sphere { c, r: r + t },
        Shape::Ellipsoid { c, r, inv } => Shape::Ellipsoid { c, r: r + Vec3::splat(t), inv },
        Shape::Capsule { a, b, r } => Shape::Capsule { a, b, r: r + t },
        Shape::RoundCone { a, b, ra, rb } => Shape::RoundCone { a, b, ra: ra + t, rb: rb + t },
        Shape::RoundBox { c, half, r, inv } => Shape::RoundBox {
            c,
            half: half + Vec3::splat(t),
            r: r + t,
            inv,
        },
        Shape::Torus { c, big, small, inv } => Shape::Torus { c, big, small: small + t, inv },
        other => other,
    }
}

struct BodyParts {
    torso: Vec<(Shape, BoneSpec)>,
    neck: Vec<(Shape, BoneSpec)>,
    chest: Vec<(Shape, BoneSpec)>,
    pelvis: Vec<(Shape, BoneSpec)>,
    legs: [Vec<(Shape, BoneSpec)>; 2],
    upper_arm: [Vec<(Shape, BoneSpec)>; 2],
    forearm: [Vec<(Shape, BoneSpec)>; 2],
}

fn body_parts(a: &Appearance, sk: &Skeleton) -> BodyParts {
    let p = P::body(a.height / 1.65);
    let fem = 1.0 - a.masc;
    let thick = 0.9 + a.build * 0.25 + a.age.min(1.2) * 0.04;
    let hipw = 1.0 + fem * 0.06 - a.masc * 0.04;
    let chestw = 1.0 + a.masc * 0.1;
    let t = |x: f32| p.r(x) * thick;

    let mut torso = Vec::new();
    let mut pelvis = Vec::new();
    // pelvis & glutes
    pelvis.push((
        ell(p.v(0.0, 0.905, -0.005), Vec3::new(t(0.152) * hipw, t(0.115), t(0.1)), Quat::IDENTITY),
        BoneSpec::blend(HIPS, SPINE, p.v(0.0, 0.93, 0.0), p.v(0.0, 1.08, 0.0), 0.5, 1.2),
    ));
    for side in 0..2 {
        let s = sx(side);
        pelvis.push((
            ell(p.v(0.066 * s * hipw, 0.855, -0.056), Vec3::new(t(0.077), t(0.086), t(0.066)), Quat::IDENTITY),
            BoneSpec::blend(HIPS, THIGH[side], p.v(0.0, 0.98, 0.0), p.v(0.0, 0.75, 0.0), 0.55, 1.05),
        ));
    }
    // abdomen, ribcage, shoulders
    torso.push((
        ell(p.v(0.0, 1.035, 0.004), Vec3::new(t(0.118) * (1.0 + a.masc * 0.05), t(0.13), t(0.088)), Quat::IDENTITY),
        BoneSpec::blend(HIPS, SPINE, p.v(0.0, 0.95, 0.0), p.v(0.0, 1.11, 0.0), 0.2, 0.9),
    ));
    torso.push((
        ell(p.v(0.0, 1.2, -0.012), Vec3::new(t(0.136) * chestw, t(0.155), t(0.097)), Quat::IDENTITY),
        BoneSpec::blend(SPINE, CHEST, p.v(0.0, 1.07, 0.0), p.v(0.0, 1.25, 0.0), 0.25, 0.9),
    ));
    torso.push((
        Shape::Capsule {
            a: p.v(-0.115 * chestw, 1.31, -0.028),
            b: p.v(0.115 * chestw, 1.31, -0.028),
            r: t(0.05),
        },
        BoneSpec::single(CHEST as u8),
    ));
    let mut chest = Vec::new();
    if fem > 0.5 {
        for side in 0..2 {
            let s = sx(side);
            chest.push((
                ell(p.v(0.052 * s, 1.18, 0.052), Vec3::new(t(0.046), t(0.041), t(0.034)), Quat::IDENTITY),
                BoneSpec::single(CHEST as u8),
            ));
        }
    }
    // neck base
    let neck = vec![(
        cone(p.v(0.0, 1.3, -0.022), p.v(0.0, 1.47, -0.01), t(0.056) * (1.0 + a.masc * 0.12), t(0.05) * (1.0 + a.masc * 0.12)),
        BoneSpec::blend(CHEST, NECK, p.v(0.0, 1.36, 0.0), p.v(0.0, 1.45, 0.0), 0.0, 0.7),
    )];

    let mut legs: [Vec<(Shape, BoneSpec)>; 2] = [Vec::new(), Vec::new()];
    let mut upper_arm: [Vec<(Shape, BoneSpec)>; 2] = [Vec::new(), Vec::new()];
    let mut forearm: [Vec<(Shape, BoneSpec)>; 2] = [Vec::new(), Vec::new()];
    for side in 0..2 {
        let hip = sk.bind[THIGH[side]];
        let knee = sk.bind[SHIN[side]];
        let ankle = sk.bind[FOOT[side]];
        let mid_thigh = hip.lerp(knee, 0.5);
        let th = THIGH[side] as u8;
        let sh = SHIN[side] as u8;
        let ft = FOOT[side] as u8;
        let l = &mut legs[side];
        l.push((
            cone(hip + p.v(0.0, 0.03, -0.004), mid_thigh, t(0.088) * hipw.sqrt(), t(0.072)),
            BoneSpec::blend(HIPS as u8, th, p.v(0.0, 1.0, 0.0), hip + p.v(0.0, -0.12, 0.0), 0.45, 1.0),
        ));
        l.push((
            cone(mid_thigh, knee, t(0.071), t(0.05)),
            BoneSpec::blend(th, sh, hip, knee, 0.82, 1.08),
        ));
        l.push((
            Shape::Sphere { c: knee + p.v(0.0, 0.0, 0.016), r: t(0.036) },
            BoneSpec::blend(th, sh, hip, knee, 0.85, 1.12),
        ));
        let mid_shin = knee.lerp(ankle, 0.5);
        l.push((
            cone(knee, mid_shin, t(0.05), t(0.041)),
            BoneSpec::blend(th, sh, knee, ankle, -0.14, 0.18),
        ));
        l.push((
            cone(mid_shin, ankle, t(0.041), t(0.03)),
            BoneSpec::blend(sh, ft, knee, ankle, 0.86, 1.03),
        ));
        l.push((
            ell(knee.lerp(ankle, 0.38) + p.v(0.0, 0.0, -0.022), Vec3::new(t(0.046), t(0.095), t(0.045)), Quat::IDENTITY),
            BoneSpec::single(sh),
        ));

        let shoulder = sk.bind[UPARM[side]];
        let elbow = sk.bind[FOREARM[side]];
        let wrist = sk.bind[HAND[side]];
        let dir = (elbow - shoulder).normalize();
        let fdir = (wrist - elbow).normalize();
        let chest_c = p.v(0.0, 1.33, -0.02);
        let ua = UPARM[side] as u8;
        let fa = FOREARM[side] as u8;
        let hd = HAND[side] as u8;
        let arm_t = 1.0 + a.masc * 0.1;
        let u = &mut upper_arm[side];
        u.push((
            Shape::Sphere { c: shoulder + Vec3::new(0.004 * sx(side), -0.01, 0.0) * p.k, r: t(0.04) * arm_t },
            BoneSpec::blend(CHEST as u8, ua, chest_c, shoulder, 0.55, 1.12),
        ));
        u.push((
            cone(shoulder, shoulder.lerp(elbow, 0.55), t(0.043) * arm_t, t(0.039) * arm_t),
            BoneSpec::blend(CHEST as u8, ua, chest_c, shoulder, 0.6, 1.05),
        ));
        u.push((
            cone(shoulder.lerp(elbow, 0.45), elbow, t(0.04) * arm_t, t(0.034) * arm_t),
            BoneSpec::blend(ua, fa, shoulder, elbow, 0.86, 1.08),
        ));
        u.push((
            ell(
                shoulder.lerp(elbow, 0.5) + p.v(0.0, 0.0, 0.01),
                Vec3::new(t(0.034) * arm_t, t(0.07), t(0.034) * arm_t),
                rot_y_to(dir),
            ),
            BoneSpec::single(ua),
        ));
        let f = &mut forearm[side];
        f.push((
            cone(elbow, elbow.lerp(wrist, 0.5), t(0.035) * arm_t, t(0.03) * arm_t),
            BoneSpec::blend(ua, fa, elbow, wrist, -0.12, 0.12),
        ));
        f.push((
            cone(elbow.lerp(wrist, 0.45), wrist, t(0.032) * arm_t, t(0.025) * arm_t),
            BoneSpec::blend(fa, hd, elbow, wrist, 0.92, 1.06),
        ));
        f.push((
            ell(
                elbow + fdir * p.r(0.07),
                Vec3::new(t(0.036) * arm_t, t(0.065), t(0.031) * arm_t),
                rot_y_to(fdir),
            ),
            BoneSpec::single(fa),
        ));
    }
    BodyParts {
        torso,
        neck,
        chest,
        pelvis,
        legs,
        upper_arm,
        forearm,
    }
}

fn add_all(sdf: &mut Sdf, parts: &[(Shape, BoneSpec)], op: Op, mat: u8, infl: f32) {
    for (s, b) in parts {
        sdf.add(Prim::new(inflate(*s, infl), op, mat, *b));
    }
}

fn add_clipped(sdf: &mut Sdf, parts: &[(Shape, BoneSpec)], op: Op, mat: u8, infl: f32, clip: (Vec3, f32), disp: f32) {
    for (s, b) in parts {
        let mut pr = Prim::new(inflate(*s, infl), op, mat, *b).clip(clip.0, clip.1);
        if disp > 0.0 {
            pr = pr.with_disp(disp, 55.0);
        }
        sdf.add(pr);
    }
}

pub fn body_sdf(a: &Appearance, sk: &Skeleton) -> Sdf {
    let p = P::body(a.height / 1.65);
    let parts = body_parts(a, sk);
    let mut sdf = Sdf::new();
    let sm = Op::Smooth(p.r(0.03));
    let sm2 = Op::Smooth(p.r(0.018));
    // skin
    add_all(&mut sdf, &parts.pelvis, sm, M_SKIN, 0.0);
    add_all(&mut sdf, &parts.torso, sm, M_SKIN, 0.0);
    add_all(&mut sdf, &parts.neck, sm, M_SKIN, 0.0);
    add_all(&mut sdf, &parts.chest, sm, M_SKIN, 0.0);
    for side in 0..2 {
        add_all(&mut sdf, &parts.legs[side], sm2, M_LEGS, 0.0);
        add_all(&mut sdf, &parts.upper_arm[side], sm2, M_SKIN, 0.0);
        add_all(&mut sdf, &parts.forearm[side], sm2, M_SKIN, 0.0);
    }

    // ---- bottoms
    let waist_y = p.r(1.0);
    let small = Op::Smooth(p.r(0.006));
    match a.bottom {
        Bottom::Jeans | Bottom::Pants => {
            let infl = if a.bottom == Bottom::Jeans { p.r(0.006) } else { p.r(0.009) };
            add_clipped(&mut sdf, &parts.pelvis, small, M_BOTTOM, infl + p.r(0.002), (Vec3::Y, waist_y), 0.0);
            add_clipped(&mut sdf, &parts.torso[..1], small, M_BOTTOM, infl + p.r(0.002), (Vec3::Y, waist_y), 0.0);
            for side in 0..2 {
                add_clipped(&mut sdf, &parts.legs[side], small, M_BOTTOM, infl, (Vec3::NEG_Y, -p.r(0.1)), p.r(0.0012));
                let ankle = sk.bind[FOOT[side]];
                sdf.add(Prim::new(
                    torus(Vec3::new(ankle.x, p.r(0.108), ankle.z + p.r(0.004)), p.r(0.036), p.r(0.009), Vec3::Y),
                    small,
                    M_BOTTOM,
                    BoneSpec::single(SHIN[side] as u8),
                ));
            }
            // belt
            sdf.add(
                Prim::new(
                    inflate(parts.torso[0].0, infl + p.r(0.006)),
                    Op::Union,
                    M_BELT,
                    BoneSpec::single(HIPS as u8),
                )
                .clip(Vec3::Y, waist_y)
                .clip(Vec3::NEG_Y, -(waist_y - p.r(0.028))),
            );
        }
        Bottom::Skirt => {
            sdf.add(
                Prim::new(
                    cone(p.v(0.0, 1.0, -0.005), p.v(0.0, 0.52, 0.0), p.r(0.158), p.r(0.235)),
                    small,
                    M_BOTTOM,
                    BoneSpec::blend(HIPS as u8, HEM as u8, p.v(0.0, 0.95, 0.0), p.v(0.0, 0.55, 0.0), 0.2, 1.0),
                )
                .clip(Vec3::NEG_Y, -p.r(0.54))
                .with_disp(p.r(0.003), 45.0),
            );
        }
    }

    // ---- tops
    let (infl, hem_y, sleeve, disp) = match a.top {
        Top::Hoodie => (p.r(0.009), p.r(0.9), 1.0, p.r(0.0011)),
        Top::Tee => (p.r(0.005), p.r(0.93), 0.42, p.r(0.0012)),
        Top::Blouse => (p.r(0.006), p.r(0.95), 0.7, p.r(0.0012)),
        Top::Sweater => (p.r(0.012), p.r(0.9), 1.0, p.r(0.0015)),
        Top::Apron => (p.r(0.005), p.r(0.95), 0.9, p.r(0.0012)),
        Top::Jacket => (p.r(0.014), p.r(0.87), 1.0, p.r(0.0018)),
    };
    let hem_clip = (Vec3::NEG_Y, -hem_y);
    add_clipped(&mut sdf, &parts.torso, small, M_TOP, infl, hem_clip, disp);
    let loose = matches!(a.top, Top::Hoodie | Top::Sweater | Top::Jacket);
    add_clipped(&mut sdf, &parts.chest, small, M_TOP, if loose { infl * 0.5 } else { infl }, hem_clip, disp);
    add_clipped(&mut sdf, &parts.pelvis[..1], small, M_TOP, infl + p.r(0.004), hem_clip, disp);
    for side in 0..2 {
        let shoulder = sk.bind[UPARM[side]];
        let elbow = sk.bind[FOREARM[side]];
        let wrist = sk.bind[HAND[side]];
        let dir = (elbow - shoulder).normalize();
        let fdir = (wrist - elbow).normalize();
        let sleeve_inf = infl * 0.7;
        if sleeve <= 0.5 {
            // short sleeves end on the upper arm
            let end = shoulder.lerp(elbow, sleeve / 0.5 * 0.5);
            add_clipped(&mut sdf, &parts.upper_arm[side], small, M_TOP, sleeve_inf + p.r(0.004), (dir, end.dot(dir)), disp);
            sdf.add(Prim::new(
                torus(end - dir * p.r(0.01), p.r(0.043), p.r(0.006), dir),
                small,
                M_TOP,
                BoneSpec::single(UPARM[side] as u8),
            ));
        } else {
            add_clipped(&mut sdf, &parts.upper_arm[side], small, M_TOP, sleeve_inf, (dir, 1e3), disp);
            let end = if sleeve >= 0.95 {
                wrist - fdir * p.r(0.012)
            } else {
                elbow.lerp(wrist, (sleeve - 0.5) / 0.5)
            };
            add_clipped(&mut sdf, &parts.forearm[side], small, M_TOP, sleeve_inf, (fdir, end.dot(fdir)), disp);
            let cuff_r = if sleeve >= 0.95 { p.r(0.027) } else { p.r(0.033) };
            sdf.add(Prim::new(
                torus(end - fdir * p.r(0.012), cuff_r + sleeve_inf, p.r(0.008), fdir),
                small,
                if a.top == Top::Hoodie { M_TOP2 } else { M_TOP },
                BoneSpec::blend(FOREARM[side] as u8, HAND[side] as u8, elbow, wrist, 0.95, 1.05),
            ));
        }
    }
    // hem band
    sdf.add(
        Prim::new(inflate(parts.pelvis[0].0, infl + p.r(0.008)), small, if a.top == Top::Hoodie { M_TOP2 } else { M_TOP }, BoneSpec::blend(HIPS as u8, SPINE as u8, p.v(0.0, 0.95, 0.0), p.v(0.0, 1.1, 0.0), 0.5, 1.2))
            .clip(Vec3::NEG_Y, -hem_y)
            .clip(Vec3::Y, hem_y + p.r(0.035)),
    );
    // collar / hood / details
    let collar_c = p.v(0.0, 1.375, -0.012);
    match a.top {
        Top::Hoodie => {
            sdf.add(Prim::new(
                ell(p.v(0.0, 1.37, -0.09), Vec3::new(p.r(0.088), p.r(0.05), p.r(0.042)), Quat::from_rotation_x(-0.35)),
                Op::Smooth(p.r(0.02)),
                M_TOP,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.2),
            ));
            sdf.add(Prim::new(
                torus(collar_c + Vec3::new(0.0, -0.016, 0.0) * p.k, p.r(0.068), p.r(0.016), Vec3::new(0.0, 1.0, -0.35)),
                Op::Smooth(p.r(0.012)),
                M_TOP,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.0),
            ));
            for side in 0..2 {
                let s = sx(side);
                sdf.add(Prim::new(
                    Shape::Capsule {
                        a: p.v(0.024 * s, 1.345, 0.078),
                        b: p.v(0.03 * s, 1.215, 0.112),
                        r: p.r(0.0045),
                    },
                    Op::Union,
                    M_WHITE,
                    BoneSpec::single(CHEST as u8),
                ));
            }
        }
        Top::Tee | Top::Apron | Top::Sweater => {
            sdf.add(Prim::new(
                torus(collar_c, p.r(0.064), p.r(0.009), Vec3::new(0.0, 1.0, -0.25)),
                Op::Smooth(p.r(0.008)),
                M_TOP,
                BoneSpec::single(CHEST as u8),
            ));
        }
        Top::Blouse => {
            sdf.add(Prim::new(
                torus(collar_c + Vec3::new(0.0, 0.004, 0.004) * p.k, p.r(0.066), p.r(0.0075), Vec3::new(0.0, 1.0, -0.3)),
                Op::Smooth(p.r(0.004)),
                M_TOP2,
                BoneSpec::single(CHEST as u8),
            ));
        }
        Top::Jacket => {
            sdf.add(Prim::new(
                torus(collar_c + p.v(0.0, 0.01, 0.0), p.r(0.07), p.r(0.018), Vec3::new(0.0, 1.0, -0.3)),
                Op::Smooth(p.r(0.01)),
                M_TOP,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.0),
            ));
            // shirt collar peeking
            sdf.add(Prim::new(
                torus(collar_c + p.v(0.0, 0.03, 0.01), p.r(0.055), p.r(0.008), Vec3::new(0.0, 1.0, -0.3)),
                Op::Smooth(p.r(0.004)),
                M_TOP2,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.0),
            ));
            sdf.add(Prim::new(
                Shape::Capsule {
                    a: p.v(0.0, 1.33, 0.1),
                    b: p.v(0.0, 0.9, 0.13),
                    r: p.r(0.004),
                },
                Op::Smooth(p.r(0.004)),
                M_METAL,
                BoneSpec::blend(HIPS as u8, CHEST as u8, p.v(0.0, 0.95, 0.0), p.v(0.0, 1.25, 0.0), 0.0, 1.0),
            ));
        }
    }
    if a.top == Top::Apron {
        sdf.add(Prim::new(
            rbox(p.v(0.0, 1.0, 0.108), Vec3::new(p.r(0.13), p.r(0.26), p.r(0.006)), p.r(0.004), Quat::IDENTITY),
            Op::Smooth(p.r(0.02)),
            M_TOP2,
            BoneSpec::blend(HIPS as u8, SPINE as u8, p.v(0.0, 0.92, 0.0), p.v(0.0, 1.12, 0.0), 0.0, 1.0),
        ).clip(Vec3::NEG_Y, -p.r(0.78)));
    }

    // ---- shoes
    let dress = matches!(a.top, Top::Blouse | Top::Jacket | Top::Sweater);
    for side in 0..2 {
        let s = sx(side);
        let ankle = sk.bind[FOOT[side]];
        let yaw = Quat::from_rotation_y(0.1 * s);
        let ft = FOOT[side] as u8;
        let (h, w) = if dress { (p.r(0.034), p.r(0.04)) } else { (p.r(0.041), p.r(0.044)) };
        sdf.add(Prim::new(
            rbox(Vec3::new(ankle.x + p.r(0.004) * s, h, p.r(0.052)), Vec3::new(w, h, p.r(0.124)), p.r(0.038).min(h * 0.95), yaw),
            Op::Smooth(p.r(0.01)),
            M_SHOE,
            BoneSpec::blend(SHIN[side] as u8, ft, ankle + p.v(0.0, 0.06, 0.0), ankle, 0.4, 1.0),
        ));
        sdf.add(Prim::new(
            rbox(Vec3::new(ankle.x + p.r(0.004) * s, p.r(0.013), p.r(0.052)), Vec3::new(w + p.r(0.004), p.r(0.013), p.r(0.13)), p.r(0.011), yaw),
            Op::Union,
            M_SOLE,
            BoneSpec::single(ft),
        ));
        if !dress {
            sdf.add(Prim::new(
                torus(Vec3::new(ankle.x, p.r(0.1), ankle.z - p.r(0.006)), p.r(0.038), p.r(0.012), Vec3::new(0.0, 1.0, 0.25)),
                Op::Smooth(p.r(0.01)),
                M_SHOE,
                BoneSpec::blend(SHIN[side] as u8, ft, ankle + p.v(0.0, 0.06, 0.0), ankle, 0.3, 1.0),
            ));
            // laces
            for i in 0..3 {
                let z = p.r(0.05 + i as f32 * 0.025);
                let y = h * 2.0 - p.r(0.004) - i as f32 * p.r(0.008);
                sdf.add(Prim::new(
                    Shape::Capsule {
                        a: Vec3::new(ankle.x - p.r(0.018), y, z),
                        b: Vec3::new(ankle.x + p.r(0.018), y, z),
                        r: p.r(0.0035),
                    },
                    Op::Union,
                    M_SOLE,
                    BoneSpec::single(ft),
                ));
            }
        }
    }
    sdf
}

// ------------------------------------------------------------------ head

pub fn head_sdf(a: &Appearance) -> Sdf {
    let p = P::head(a.height / 1.65);
    let m = a.masc;
    let fem = 1.0 - m;
    let mut sdf = Sdf::new();
    let head = BoneSpec::single(HEAD as u8);
    let sm = |x: f32| Op::Smooth(p.r(x));
    let adult = a.age.min(1.0);
    let jaw_w = 0.93 + m * 0.17 + (a.age - 1.0).max(0.0) * 0.04;
    let nk = 0.92 + m * 0.14 + adult * 0.04;
    // shorter lower face for feminine / young faces
    let lower = 0.0045 * fem + 0.003 * (1.0 - adult);
    // upper neck (overlaps the body mesh neck)
    sdf.add(Prim::new(
        cone(p.v(0.0, 1.41, -0.02), p.v(0.0, 1.51, -0.012), p.r(0.048) * (1.0 + m * 0.14), p.r(0.045) * (1.0 + m * 0.14)),
        Op::Union,
        M_SKIN,
        BoneSpec::blend(NECK as u8, HEAD as u8, p.v(0.0, 1.44, 0.0), p.v(0.0, 1.5, 0.0), 0.0, 1.0),
    ));
    // cranium
    sdf.add(Prim::new(ell(p.v(0.0, 1.572, -0.012), Vec3::new(p.r(0.073), p.r(0.082), p.r(0.094)), Quat::IDENTITY), sm(0.03), M_SKIN, head));
    sdf.add(Prim::new(ell(p.v(0.0, 1.557, -0.045), Vec3::new(p.r(0.066), p.r(0.07), p.r(0.064)), Quat::IDENTITY), sm(0.02), M_SKIN, head));
    // forehead
    sdf.add(Prim::new(ell(p.v(0.0, 1.598, 0.03), Vec3::new(p.r(0.062), p.r(0.052), p.r(0.057)), Quat::IDENTITY), sm(0.02), M_SKIN, head));
    // face mass
    sdf.add(Prim::new(ell(p.v(0.0, 1.535 + lower * 0.5, 0.018), Vec3::new(p.r(0.054) * jaw_w, p.r(0.061 - lower * 0.5), p.r(0.061)), Quat::IDENTITY), sm(0.03), M_SKIN, head));
    for side in 0..2 {
        let s = sx(side);
        // jaw line
        sdf.add(Prim::new(
            Shape::Capsule {
                a: p.v(0.048 * s * jaw_w, 1.526, -0.012),
                b: p.v(0.014 * s * jaw_w, 1.474 + lower, 0.054),
                r: p.r(0.015 + m * 0.004),
            },
            sm(0.028),
            M_SKIN,
            head,
        ));
        // cheeks (higher and rounder for feminine faces)
        sdf.add(Prim::new(Shape::Sphere { c: p.v(0.036 * s, 1.541 + 0.004 * fem, 0.052), r: p.r(0.02 + 0.002 * fem) }, sm(0.022), M_SKIN, head));
        sdf.add(Prim::new(
            ell(p.v(0.049 * s, 1.553, 0.048), Vec3::new(p.r(0.022), p.r(0.012), p.r(0.018)), Quat::IDENTITY),
            sm(0.015),
            M_SKIN,
            head,
        ));
        // ears
        sdf.add(Prim::new(
            ell(p.v(0.071 * s, 1.553, -0.013), Vec3::new(p.r(0.0095), p.r(0.025), p.r(0.017)), Quat::from_rotation_y(-0.2 * s) * Quat::from_rotation_x(0.15)),
            sm(0.006),
            M_SKIN,
            head,
        ));
        sdf.add(Prim::new(
            ell(p.v(0.079 * s, 1.556, -0.01), Vec3::new(p.r(0.005), p.r(0.016), p.r(0.01)), Quat::from_rotation_y(-0.2 * s)),
            Op::Subtract(p.r(0.004)),
            M_SKIN,
            head,
        ));
    }
    // chin
    sdf.add(Prim::new(
        ell(p.v(0.0, 1.472 + lower, 0.064), Vec3::new(p.r(0.02) * (1.0 + m * 0.3), p.r(0.018), p.r(0.018)), Quat::IDENTITY),
        sm(0.02),
        M_SKIN,
        head,
    ));
    // brow ridge
    sdf.add(Prim::new(
        Shape::Capsule {
            a: p.v(-0.036, 1.584, 0.074),
            b: p.v(0.036, 1.584, 0.074),
            r: p.r(0.009 + m * 0.006),
        },
        sm(0.022),
        M_SKIN,
        head,
    ));
    // nose
    sdf.add(Prim::new(
        Shape::Capsule {
            a: p.v(0.0, 1.574, 0.082),
            b: p.v(0.0, 1.539, 0.092 * nk.sqrt()),
            r: p.r(0.0072) * nk,
        },
        sm(0.012),
        M_SKIN,
        head,
    ));
    sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0, 1.533, 0.0905 + 0.004 * (nk - 1.0)), r: p.r(0.0105) * nk }, sm(0.01), M_SKIN, head));
    for side in 0..2 {
        let s = sx(side);
        sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0115 * s * nk, 1.5305, 0.084), r: p.r(0.0078) * nk }, sm(0.008), M_SKIN, head));
        sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0066 * s, 1.5228, 0.0878), r: p.r(0.0025) }, Op::Subtract(p.r(0.003)), M_SKIN, head));
    }
    // lips volume (the animated mouth overlay sits on top)
    let lip_full = 1.0 + fem * 0.12;
    sdf.add(Prim::new(
        ell(p.v(0.0, 1.507 + lower * 0.6, 0.074), Vec3::new(p.r(0.021), p.r(0.0078) * lip_full, p.r(0.0102)), Quat::IDENTITY),
        sm(0.012),
        M_SKIN,
        head,
    ));
    sdf.add(Prim::new(
        ell(p.v(0.0, 1.4935 + lower * 0.6, 0.071), Vec3::new(p.r(0.018), p.r(0.0076) * lip_full, p.r(0.0098)), Quat::IDENTITY),
        sm(0.012),
        M_SKIN,
        head,
    ));
    // eye openings (almond)
    for side in 0..2 {
        let s = sx(side);
        sdf.add(Prim::new(
            ell(p.v(0.031 * s, 1.5585, 0.0815), Vec3::new(p.r(0.0158 + 0.0006 * fem), p.r(0.0088 + 0.0006 * fem), p.r(0.0095)), Quat::from_rotation_z(0.06 * s)),
            Op::Subtract(p.r(0.0045)),
            M_SKIN,
            head,
        ));
    }
    sdf
}

pub fn hand_sdf(a: &Appearance, sk: &Skeleton, side: usize) -> Sdf {
    let p = P::body(a.height / 1.65);
    let hk = 1.0 + a.masc * 0.1;
    let mut sdf = Sdf::new();
    let wrist = sk.bind[HAND[side]];
    let elbow = sk.bind[FOREARM[side]];
    let f = (wrist - elbow).normalize();
    let n = f.cross(Vec3::Z).normalize() * sx(side);
    let t = (Vec3::Z - f * Vec3::Z.dot(f) - n * Vec3::Z.dot(n)).normalize();
    let hd = HAND[side] as u8;
    let fg = FINGERS[side] as u8;
    let fa = FOREARM[side] as u8;
    let r = |x: f32| p.r(x) * hk;
    // wrist stub (inside the forearm)
    sdf.add(Prim::new(
        cone(wrist - f * p.r(0.035), wrist + f * p.r(0.018), r(0.023), r(0.021)),
        Op::Union,
        M_SKIN,
        BoneSpec::blend(fa, hd, wrist - f * p.r(0.035), wrist + f * p.r(0.01), 0.3, 1.0),
    ));
    // palm
    let y_axis = f.cross(t);
    let rot = Quat::from_mat3(&Mat3::from_cols(t, y_axis, f));
    sdf.add(Prim::new(
        rbox(wrist + f * r(0.05) - n * p.r(0.001), Vec3::new(r(0.039), r(0.0115), r(0.044)), r(0.011), rot),
        Op::Smooth(p.r(0.012)),
        M_SKIN,
        BoneSpec::single(hd),
    ));
    // thenar
    sdf.add(Prim::new(
        ell(wrist + f * r(0.035) + t * r(0.022) + n * r(0.006), Vec3::new(r(0.017), r(0.012), r(0.03)), rot),
        Op::Smooth(p.r(0.01)),
        M_SKIN,
        BoneSpec::single(hd),
    ));
    // fingers
    let offs = [0.027, 0.009, -0.009, -0.026];
    let lens = [0.074, 0.081, 0.076, 0.061];
    for i in 0..4 {
        let kn = wrist + f * r(0.092) + t * r(offs[i]) - n * p.r(0.002);
        let spread = t * r(offs[i]) * 0.12;
        let l = r(lens[i]);
        let m1 = kn + (f + spread) * l * 0.55 + n * p.r(0.006);
        let tip = m1 + (f + spread * 1.5) * l * 0.45 + n * p.r(0.013);
        let rr = r(0.0084) * if i == 3 { 0.88 } else { 1.0 };
        sdf.add(Prim::new(
            Shape::Sphere { c: kn, r: rr * 1.08 },
            Op::Smooth(p.r(0.008)),
            M_SKIN,
            BoneSpec::blend(hd, fg, wrist, kn, 0.85, 1.1),
        ));
        sdf.add(Prim::new(
            cone(kn, m1, rr, rr * 0.93),
            Op::Smooth(p.r(0.005)),
            M_SKIN,
            BoneSpec::blend(hd, fg, wrist, kn, 0.9, 1.05),
        ));
        sdf.add(Prim::new(cone(m1, tip, rr * 0.93, rr * 0.8), Op::Smooth(p.r(0.004)), M_SKIN, BoneSpec::single(fg)));
    }
    // thumb
    let tb = wrist + f * r(0.022) + t * r(0.03) + n * r(0.008);
    let tm = tb + f * r(0.033) + t * r(0.022) + n * r(0.012);
    let tt = tm + f * r(0.028) + t * r(0.008) + n * r(0.014);
    sdf.add(Prim::new(cone(tb, tm, r(0.0118), r(0.0102)), Op::Smooth(p.r(0.01)), M_SKIN, BoneSpec::single(hd)));
    sdf.add(Prim::new(cone(tm, tt, r(0.0102), r(0.0086)), Op::Smooth(p.r(0.004)), M_SKIN, BoneSpec::single(hd)));
    sdf
}

pub fn hair_sdf(a: &Appearance) -> Sdf {
    let p = P::head(a.height / 1.65);
    let mut sdf = Sdf::new();
    let head = BoneSpec::single(HEAD as u8);
    let cr_c = p.v(0.0, 1.575, -0.014);
    let cr_r = Vec3::new(p.r(0.073), p.r(0.082), p.r(0.094));
    let cap = |extra: f32| ell(cr_c + Vec3::new(0.0, extra * 0.4, -extra * 0.3), cr_r + Vec3::splat(extra), Quat::IDENTITY);
    let face_cut = |sdf: &mut Sdf, top: f32| {
        sdf.add(Prim::new(
            ell(p.v(0.0, top - 0.098, 0.088), Vec3::new(p.r(0.067), p.r(0.098), p.r(0.075)), Quat::from_rotation_x(-0.22)),
            Op::Subtract(p.r(0.01)),
            M_HAIR,
            head,
        ));
    };
    let ear_cut = |sdf: &mut Sdf| {
        for side in 0..2 {
            let s = sx(side);
            sdf.add(Prim::new(
                ell(p.v(0.078 * s, 1.535, 0.008), Vec3::new(p.r(0.03), p.r(0.047), p.r(0.036)), Quat::IDENTITY),
                Op::Subtract(p.r(0.01)),
                M_HAIR,
                head,
            ));
        }
    };
    // back of the head coverage
    let back_extra = if a.hair == Hair::Buzz { 0.004 } else { 0.007 };
    sdf.add(
        Prim::new(
            ell(p.v(0.0, 1.548, -0.047), Vec3::new(p.r(0.066 + back_extra), p.r(0.07 + back_extra), p.r(0.064 + back_extra)), Quat::IDENTITY),
            Op::Union,
            M_HAIR,
            head,
        )
        .clip(Vec3::NEG_Y, -p.v(0.0, 1.482, 0.0).y)
        .clip(Vec3::Z, p.v(0.0, 0.0, -0.02).z),
    );
    match a.hair {
        Hair::Ponytail => {
            sdf.add(Prim::new(cap(p.r(0.008)), Op::Union, M_HAIR, head).with_grooves(p.r(0.0011), 55.0, cr_c - Vec3::Y * p.r(0.02)));
            face_cut(&mut sdf, 1.612);
            ear_cut(&mut sdf);
            // soft volume on top and a centre parting
            sdf.add(Prim::new(ell(p.v(0.0, 1.625, 0.0), Vec3::new(p.r(0.066), p.r(0.04), p.r(0.08)), Quat::IDENTITY), Op::Smooth(p.r(0.02)), M_HAIR, head).with_grooves(p.r(0.0009), 55.0, cr_c - Vec3::Y * p.r(0.02)));
            sdf.add(Prim::new(
                Shape::Capsule {
                    a: p.v(0.004, 1.676, 0.06),
                    b: p.v(0.004, 1.672, -0.02),
                    r: p.r(0.0035),
                },
                Op::Subtract(p.r(0.006)),
                M_HAIR,
                head,
            ));
            // hair tie
            sdf.add(Prim::new(torus(p.v(0.0, 1.585, -0.112), p.r(0.016), p.r(0.0065), Vec3::new(0.0, 0.35, -1.0)), Op::Union, M_TIE, head));
            sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0, 1.585, -0.113), r: p.r(0.018) }, Op::Smooth(p.r(0.012)), M_HAIR, head));
        }
        Hair::Short => {
            sdf.add(Prim::new(cap(p.r(0.01)), Op::Union, M_HAIR, head).with_grooves(p.r(0.0015), 70.0, cr_c));
            sdf.add(Prim::new(ell(p.v(0.0, 1.628, 0.01), Vec3::new(p.r(0.07), p.r(0.042), p.r(0.085)), Quat::IDENTITY), Op::Smooth(p.r(0.02)), M_HAIR, head).with_grooves(p.r(0.0018), 60.0, cr_c));
            face_cut(&mut sdf, 1.625);
            ear_cut(&mut sdf);
        }
        Hair::Curly => {
            sdf.add(Prim::new(cap(p.r(0.016)), Op::Union, M_HAIR, head).with_disp(p.r(0.005), 150.0));
            let mut rng = crate::math::Rng::new(a.seed * 31);
            for _ in 0..16 {
                let th = rng.range(-2.4, 2.4);
                let ph = rng.range(0.15, 1.2);
                let dir = Vec3::new(th.sin() * ph.sin(), ph.cos(), th.cos() * ph.sin() * 1.1 - 0.15);
                let c = cr_c + Vec3::new(dir.x * cr_r.x, dir.y * cr_r.y, dir.z * cr_r.z) * 1.08;
                sdf.add(Prim::new(Shape::Sphere { c, r: p.r(rng.range(0.022, 0.03)) }, Op::Smooth(p.r(0.015)), M_HAIR, head).with_disp(p.r(0.003), 180.0));
            }
            face_cut(&mut sdf, 1.63);
            ear_cut(&mut sdf);
        }
        Hair::Bob => {
            sdf.add(Prim::new(cap(p.r(0.01)), Op::Union, M_HAIR, head).with_falling(p.r(0.0012), 70.0, cr_c));
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.548, -0.016), Vec3::new(p.r(0.09), p.r(0.105), p.r(0.105)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.02)),
                    M_HAIR,
                    BoneSpec::blend(HEAD as u8, HAIR as u8, p.v(0.0, 1.58, 0.0), p.v(0.0, 1.47, 0.0), 0.0, 1.0),
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.468, 0.0).y)
                .with_falling(p.r(0.0014), 70.0, cr_c),
            );
            face_cut(&mut sdf, 1.585);
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.615, 0.062), Vec3::new(p.r(0.066), p.r(0.034), p.r(0.026)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.016)),
                    M_HAIR,
                    head,
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.59, 0.0).y)
                .with_falling(p.r(0.001), 70.0, cr_c),
            );
        }
        Hair::Bun => {
            sdf.add(Prim::new(cap(p.r(0.007)), Op::Union, M_HAIR, head).with_grooves(p.r(0.001), 60.0, cr_c));
            face_cut(&mut sdf, 1.618);
            ear_cut(&mut sdf);
            sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0, 1.635, -0.098), r: p.r(0.036) }, Op::Smooth(p.r(0.012)), M_HAIR, head).with_disp(p.r(0.002), 160.0));
        }
        Hair::Buzz => {
            sdf.add(Prim::new(cap(p.r(0.0045)), Op::Union, M_HAIR, head).with_grooves(p.r(0.0005), 90.0, cr_c));
            face_cut(&mut sdf, 1.635);
            ear_cut(&mut sdf);
        }
        Hair::Long => {
            sdf.add(Prim::new(cap(p.r(0.01)), Op::Union, M_HAIR, head).with_falling(p.r(0.0012), 70.0, cr_c));
            let sway = BoneSpec::blend(HEAD as u8, HAIR as u8, p.v(0.0, 1.56, 0.0), p.v(0.0, 1.3, 0.0), 0.0, 1.0);
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.48, -0.03), Vec3::new(p.r(0.095), p.r(0.2), p.r(0.105)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.03)),
                    M_HAIR,
                    sway,
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.3, 0.0).y)
                .with_falling(p.r(0.0016), 70.0, cr_c),
            );
            face_cut(&mut sdf, 1.605);
            // keep the neck and shoulders free
            sdf.add(Prim::new(
                cone(p.v(0.0, 1.3, 0.02), p.v(0.0, 1.5, 0.03), p.r(0.075), p.r(0.066)),
                Op::Subtract(p.r(0.02)),
                M_HAIR,
                head,
            ));
            sdf.add(Prim::new(
                rbox(p.v(0.0, 1.36, 0.06), Vec3::new(p.r(0.2), p.r(0.08), p.r(0.07)), p.r(0.02), Quat::IDENTITY),
                Op::Subtract(p.r(0.02)),
                M_HAIR,
                head,
            ));
        }
    }
    if a.mustache {
        sdf.add(Prim::new(
            ell(p.v(0.0, 1.508, 0.097), Vec3::new(p.r(0.023), p.r(0.0055), p.r(0.008)), Quat::IDENTITY),
            Op::Union,
            M_HAIR,
            head,
        ).with_disp(p.r(0.0008), 400.0));
    }
    if a.glasses {
        for side in 0..2 {
            let s = sx(side);
            sdf.add(Prim::new(torus(p.v(0.032 * s, 1.566, 0.096), p.r(0.017), p.r(0.0017), Vec3::Z), Op::Union, M_METAL, head));
            sdf.add(Prim::new(
                Shape::Capsule {
                    a: p.v(0.049 * s, 1.568, 0.094),
                    b: p.v(0.074 * s, 1.568, 0.0),
                    r: p.r(0.0015),
                },
                Op::Union,
                M_METAL,
                head,
            ));
        }
        sdf.add(Prim::new(
            Shape::Capsule {
                a: p.v(-0.015, 1.57, 0.1),
                b: p.v(0.015, 1.57, 0.1),
                r: p.r(0.0017),
            },
            Op::Union,
            M_METAL,
            head,
        ));
    }
    sdf
}

/// Ponytail tube skinned to the tail bones.
fn ponytail(a: &Appearance, sk: &Skeleton, mats: &[Mat]) -> SkinMeshData {
    let p = P::head(a.height / 1.65);
    let mut path = Vec::new();
    let mut radii = Vec::new();
    let n = 16;
    let start = p.v(0.0, 1.585, -0.118);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let seg = (t * (TAIL_N as f32 - 1.0)).min(TAIL_N as f32 - 1.001);
        let i0 = seg.floor() as usize;
        let f = seg - i0 as f32;
        let a0 = sk.bind[TAIL0 + i0];
        let a1 = sk.bind[TAIL0 + i0 + 1];
        let mut pt = a0.lerp(a1, f);
        if i == 0 {
            pt = start;
        }
        pt.z -= (t * std::f32::consts::PI).sin() * p.r(0.018);
        path.push(pt);
        let r = lerp(p.r(0.026), p.r(0.005), t.powf(1.1)) * (1.0 + 0.3 * (t * std::f32::consts::PI).sin());
        radii.push(r);
    }
    let mut md = MeshData::new();
    md.tube(&path, &radii, 12, &mats[M_HAIR as usize], true);
    let mut out = SkinMeshData::default();
    for v in &md.verts {
        let pos = glam::Vec3::from(v.pos);
        // find the chain parameter
        let mut best = (f32::MAX, 0usize, 0.0f32);
        for i in 0..TAIL_N - 1 {
            let a0 = sk.bind[TAIL0 + i];
            let a1 = sk.bind[TAIL0 + i + 1];
            let ab = a1 - a0;
            let t = ((pos - a0).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            let d = (a0 + ab * t).distance(pos);
            if d < best.0 {
                best = (d, i, t);
            }
        }
        let (_, i, t) = best;
        let b0 = (TAIL0 + i) as u8;
        let b1 = (TAIL0 + i + 1) as u8;
        let w1 = smoothstep(0.3, 1.0, t);
        let mut color = v.color;
        color[3] = (200.0 + 55.0 * (1.0 - t)) as u8;
        out.verts.push(SkinVertex {
            pos: v.pos,
            nrm: v.nrm,
            color,
            mat: v.mat,
            joints: [b0, b1, 0, 0],
            weights: [((1.0 - w1) * 255.0) as u8, (w1 * 255.0) as u8, 0, 0],
        });
    }
    out.indices = md.indices.clone();
    out
}

fn to_skin(mesh: &MeshOut, sdf: &Sdf, mats: &[Mat], sigma: f32, ao_scale: f32, adjust: &dyn Fn(Vec3, u8, &mut [(u8, f32); 4])) -> SkinMeshData {
    let mut out = SkinMeshData::default();
    out.verts.reserve(mesh.pos.len());
    for (i, p) in mesh.pos.iter().enumerate() {
        let n = mesh.nrm[i];
        let (mat, mut w) = sdf.attributes(*p, sigma);
        adjust(*p, mat, &mut w);
        // SDF ambient occlusion
        let mut occ = 0.0;
        let mut wsum = 0.0;
        for (k, h) in [0.012f32, 0.028, 0.05].iter().enumerate() {
            let h = h * ao_scale;
            let d = sdf.eval(*p + n * h);
            let wk = 1.0 / (k as f32 + 1.0);
            occ += ((h - d) / h).clamp(0.0, 1.0) * wk;
            wsum += wk;
        }
        let ao = (1.0 - occ / wsum * 0.85).clamp(0.25, 1.0);
        let m = &mats[mat as usize];
        let mut joints = [0u8; 4];
        let mut weights = [0u8; 4];
        let mut total = 0u32;
        for k in 0..4 {
            joints[k] = w[k].0;
            weights[k] = (w[k].1 * 255.0).round() as u8;
            total += weights[k] as u32;
        }
        if total != 255 {
            let diff = 255i32 - total as i32;
            weights[0] = (weights[0] as i32 + diff).clamp(0, 255) as u8;
        }
        out.verts.push(SkinVertex {
            pos: p.to_array(),
            nrm: n.to_array(),
            color: m.color_ao(ao),
            mat: m.packed(),
            joints,
            weights,
        });
    }
    out.indices = mesh.idx.clone();
    out
}

fn append_skin(dst: &mut SkinMeshData, src: &SkinMeshData) {
    let base = dst.verts.len() as u32;
    dst.verts.extend_from_slice(&src.verts);
    dst.indices.extend(src.indices.iter().map(|i| i + base));
}

/// Ray-march an SDF along -Z to find the front surface at (x, y).
fn surface_z(sdf: &Sdf, x: f32, y: f32, z_start: f32) -> f32 {
    let mut z = z_start;
    for _ in 0..96 {
        let d = sdf.eval(Vec3::new(x, y, z));
        if d < 0.0002 {
            break;
        }
        z -= d.max(0.0005);
    }
    z
}

pub fn build(a: &Appearance, sk: &Skeleton, q: Quality) -> CharacterMeshes {
    let k = a.height / 1.65;
    let mats = materials(a);
    let (c_body, c_head, c_hand, c_hair) = match q {
        Quality::High => (0.0072, 0.0027, 0.0028, 0.0036),
        Quality::Low => (0.0095, 0.0036, 0.0036, 0.0046),
    };
    let mut skin = SkinMeshData::default();

    // body
    let body = body_sdf(a, sk);
    let lo = Vec3::new(-0.7, -0.01, -0.22) * k;
    let hi = Vec3::new(0.7, 1.47, 0.26) * k;
    let bm = surface_nets(&body, lo, hi, c_body * k);
    let hem_y = 1.0 * k;
    let is_skirt = a.bottom == Bottom::Skirt;
    let adjust_body = move |p: Vec3, mat: u8, w: &mut [(u8, f32); 4]| {
        // loose clothing sways with the hem bone
        let loose = if mat == M_TOP || mat == M_TOP2 {
            smoothstep(hem_y, hem_y - 0.12 * k, p.y) * 0.5
        } else if is_skirt && mat == M_BOTTOM {
            0.0
        } else {
            0.0
        };
        if loose > 0.0 {
            for e in w.iter_mut() {
                e.1 *= 1.0 - loose;
            }
            // replace the weakest influence with the hem bone
            w[3] = (HEM as u8, loose);
            let s: f32 = w.iter().map(|e| e.1).sum();
            for e in w.iter_mut() {
                e.1 /= s.max(1e-6);
            }
        }
    };
    append_skin(&mut skin, &to_skin(&bm, &body, &mats, 0.012 * k, k, &adjust_body));

    // head
    let hs = head_sdf(a);
    let hlo = Vec3::new(-0.115, 1.395, -0.135) * k;
    let hhi = Vec3::new(0.115, 1.7, 0.145) * k;
    let hm = surface_nets(&hs, hlo, hhi, c_head * k);
    append_skin(&mut skin, &to_skin(&hm, &hs, &mats, 0.01 * k, k * 0.6, &|_, _, _| {}));

    // hands
    for side in 0..2 {
        let hsdf = hand_sdf(a, sk, side);
        let w = sk.bind[HAND[side]];
        let lo = w - Vec3::splat(0.2 * k);
        let hi = w + Vec3::splat(0.2 * k);
        let m = surface_nets(&hsdf, lo, hi, c_hand * k);
        append_skin(&mut skin, &to_skin(&m, &hsdf, &mats, 0.008 * k, k * 0.5, &|_, _, _| {}));
    }

    // hair
    let hair = hair_sdf(a);
    let ylo = if a.hair == Hair::Long { 1.26 } else { 1.42 };
    let hrlo = Vec3::new(-0.14, ylo, -0.21) * k;
    let hrhi = Vec3::new(0.14, 1.75, 0.16) * k;
    let hrm = surface_nets(&hair, hrlo, hrhi, c_hair * k);
    let long = matches!(a.hair, Hair::Long | Hair::Bob);
    let adjust_hair = move |p: Vec3, _mat: u8, w: &mut [(u8, f32); 4]| {
        if long {
            let s = smoothstep(1.58 * k, 1.42 * k, p.y) * 0.8;
            if s > 0.0 && w[0].0 == HEAD as u8 {
                *w = [(HEAD as u8, 1.0 - s), (HAIR as u8, s), (0, 0.0), (0, 0.0)];
            }
        }
    };
    let mut hair_mesh = to_skin(&hrm, &hair, &mats, 0.01 * k, k * 0.5, &adjust_hair);
    // hair is darker in the crevices; keep AO but brighten the tips slightly
    for v in &mut hair_mesh.verts {
        v.color[3] = ((v.color[3] as f32) * 0.9 + 20.0).min(255.0) as u8;
    }
    append_skin(&mut skin, &hair_mesh);
    if a.hair == Hair::Ponytail {
        append_skin(&mut skin, &ponytail(a, sk, &mats));
    }

    // face layout (relative to the head bone)
    let hp = P::head(k);
    let head_b = sk.bind[HEAD];
    let mut face = FaceLayout {
        eye_r: hp.r(0.0122),
        ..Default::default()
    };
    for side in 0..2 {
        let s = sx(side);
        face.eye[side] = hp.v(0.031 * s, 1.559, 0.0655) - head_b;
        let brow_y = [1.5785f32, 1.5815, 1.5815, 1.5785];
        for (i, x) in [0.012f32, 0.024, 0.036, 0.046].iter().enumerate() {
            let q = hp.v(x * s, brow_y[i], 0.0);
            let z = surface_z(&hs, q.x, q.y, 0.25 * k);
            face.brow[side][i] = Vec3::new(q.x, q.y, z) - head_b;
        }
    }
    let lower = 0.0045 * (1.0 - a.masc) + 0.003 * (1.0 - a.age.min(1.0));
    let mq = hp.v(0.0, 1.5005 + lower * 0.6, 0.0);
    let mz = surface_z(&hs, 0.0, mq.y, 0.25 * k);
    let side_x = hp.r(0.02);
    let mz_side = surface_z(&hs, side_x, mq.y, 0.25 * k);
    face.mouth = Vec3::new(0.0, mq.y, mz) - head_b;
    face.mouth_w = hp.r(0.0205) * (1.0 + a.masc * 0.08);
    face.face_curve = (mz - mz_side).max(0.001) / (side_x * side_x);
    let nq = hp.v(0.0, 1.533, 0.0);
    face.nose_tip = Vec3::new(0.0, nq.y, surface_z(&hs, 0.0, nq.y, 0.25 * k)) - head_b;
    face.head_center = hp.v(0.0, 1.56, 0.0) - head_b;

    CharacterMeshes { skin, face, mats }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hair_probe() {
        let a = Appearance::sofia();
        let sdf = hair_sdf(&a);
        let k = a.height / 1.65;
        for (name, p) in [("chin", Vec3::new(0.0, 1.45, 0.07)), ("jaw", Vec3::new(0.05, 1.48, 0.03)), ("nape", Vec3::new(0.0, 1.5, -0.09)), ("top", Vec3::new(0.0, 1.66, 0.0))] {
            let q = p * k;
            let mut d = 1e9f32;
            println!("--- {name} total {:.4}", sdf.eval(q));
            for (i, pr) in sdf.prims.iter().enumerate() {
                let pd = pr.dist(q);
                println!("prim {i} op {:?} dist {:.4}", pr.op, pd);
                d = d.min(pd);
            }
        }
    }
}
