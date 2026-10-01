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
    // natural lip tone: a rosier, slightly darker version of the skin (subtler on men)
    let lf = lerp(1.0, 0.55, a.masc);
    let lip = [
        (skin[0] as f32 * (1.0 - 0.06 * lf)) as u8,
        (skin[1] as f32 * (1.0 - 0.27 * lf)) as u8,
        (skin[2] as f32 * (1.0 - 0.22 * lf)) as u8,
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
        Mat::new(
            a.hair_color,
            match a.hair {
                Hair::Curly => 0.74,
                Hair::Buzz => 0.8,
                Hair::Bun | Hair::Short => 0.6,
                _ => 0.52,
            },
        )
        .kind(kind::HAIR),
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

/// The head is modelled at realistic size and scaled up about eye level for a slightly
/// stylized, more expressive proportion (the chin comes down, shortening the neck).
pub const HEAD_SCALE: f32 = 1.17;
pub const HEAD_PIVOT: f32 = 1.60;

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
    // the base of the neck is covered up to the collar plane (higher at the nape, lower
    // at the throat), so no skin pokes through the shoulders of the garment
    let collar_c = p.v(0.0, 1.375, -0.012);
    let collar_n = if a.top == Top::Hoodie { Vec3::Y } else { Vec3::new(0.0, 1.0, 0.26).normalize() };
    let (cover_up, cover_mat) = match a.top {
        Top::Hoodie => (0.017, M_TOP),
        Top::Jacket => (0.02, M_TOP),
        Top::Sweater => (0.008, M_TOP),
        _ => (0.0, M_TOP),
    };
    add_clipped(
        &mut sdf,
        &parts.neck,
        small,
        cover_mat,
        infl * 0.6 + p.r(0.002),
        (collar_n, (collar_c + Vec3::Y * p.r(cover_up)).dot(collar_n)),
        0.0,
    );
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
    match a.top {
        Top::Hoodie => {
            sdf.add(Prim::new(
                ell(p.v(0.0, 1.37, -0.09), Vec3::new(p.r(0.088), p.r(0.05), p.r(0.042)), Quat::from_rotation_x(-0.35)),
                Op::Smooth(p.r(0.02)),
                M_TOP,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.2),
            ));
            sdf.add(Prim::new(
                torus(collar_c + Vec3::new(0.0, -0.006, 0.0) * p.k, p.r(0.067), p.r(0.019), Vec3::new(0.0, 1.0, -0.35)),
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
            let thick = if a.top == Top::Sweater { 0.013 } else { 0.0085 };
            sdf.add(Prim::new(
                torus(collar_c + Vec3::Y * p.r(cover_up), p.r(0.061), p.r(thick), collar_n),
                Op::Smooth(p.r(0.008)),
                M_TOP,
                BoneSpec::single(CHEST as u8),
            ));
        }
        Top::Blouse => {
            sdf.add(Prim::new(
                torus(collar_c + Vec3::new(0.0, 0.001, 0.0) * p.k, p.r(0.062), p.r(0.0095), collar_n),
                Op::Smooth(p.r(0.004)),
                M_TOP2,
                BoneSpec::single(CHEST as u8),
            ));
        }
        Top::Jacket => {
            sdf.add(Prim::new(
                torus(collar_c + p.v(0.0, 0.006, 0.0), p.r(0.07), p.r(0.018), collar_n),
                Op::Smooth(p.r(0.01)),
                M_TOP,
                BoneSpec::blend(CHEST as u8, NECK as u8, p.v(0.0, 1.3, 0.0), p.v(0.0, 1.45, 0.0), 0.3, 1.0),
            ));
            // shirt collar peeking
            sdf.add(Prim::new(
                torus(collar_c + p.v(0.0, 0.026, 0.0), p.r(0.057), p.r(0.008), collar_n),
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
    // upper neck (overlaps the body mesh neck; body-space so it matches the torso)
    let pb = P::body(a.height / 1.65);
    let neck_t = (0.9 + a.build * 0.25 + a.age.min(1.2) * 0.04) * (1.0 + m * 0.12);
    sdf.add(Prim::new(
        cone(pb.v(0.0, 1.42, -0.0185), pb.v(0.0, 1.5, -0.014), pb.r(0.0524) * neck_t, pb.r(0.0505) * neck_t),
        Op::Union,
        M_SKIN,
        BoneSpec::blend(NECK as u8, HEAD as u8, pb.v(0.0, 1.43, 0.0), pb.v(0.0, 1.49, 0.0), 0.0, 1.0),
    ));
    // cranium
    sdf.add(Prim::new(ell(p.v(0.0, 1.572, -0.012), Vec3::new(p.r(0.069), p.r(0.082), p.r(0.094)), Quat::IDENTITY), sm(0.03), M_SKIN, head));
    sdf.add(Prim::new(ell(p.v(0.0, 1.557, -0.045), Vec3::new(p.r(0.064), p.r(0.07), p.r(0.064)), Quat::IDENTITY), sm(0.02), M_SKIN, head));
    // forehead
    sdf.add(Prim::new(ell(p.v(0.0, 1.598, 0.03), Vec3::new(p.r(0.06), p.r(0.052), p.r(0.057)), Quat::IDENTITY), sm(0.02), M_SKIN, head));
    // face mass: an egg that narrows toward the chin
    sdf.add(Prim::new(
        ell(p.v(0.0, 1.537 + lower * 0.5, 0.02), Vec3::new(p.r(0.0495) * jaw_w, p.r(0.062 - lower * 0.5), p.r(0.0595)), Quat::IDENTITY),
        sm(0.03),
        M_SKIN,
        head,
    ));
    for side in 0..2 {
        let s = sx(side);
        // jaw line: from the angle below the ear to the side of the chin
        sdf.add(Prim::new(
            Shape::Capsule {
                a: p.v(0.0435 * s * jaw_w, 1.507 + lower * 0.5, -0.004),
                b: p.v(0.017 * s * jaw_w, 1.4755 + lower, 0.053),
                r: p.r(0.0122 + m * 0.0045),
            },
            sm(0.034),
            M_SKIN,
            head,
        ));
        // masseter: fills the hollow between cheekbone and jaw so the cheek falls in one plane
        sdf.add(Prim::new(
            ell(p.v(0.031 * s * jaw_w, 1.512 + lower * 0.5, 0.034), Vec3::new(p.r(0.015), p.r(0.025), p.r(0.027)), Quat::from_rotation_x(-0.4)),
            sm(0.03),
            M_SKIN,
            head,
        ));
        // cheeks: soft apples in front, a subtle cheekbone toward the ear
        sdf.add(Prim::new(Shape::Sphere { c: p.v(0.033 * s, 1.5385 + 0.003 * fem, 0.0565), r: p.r(0.0168 + 0.0015 * fem) }, sm(0.022), M_SKIN, head));
        sdf.add(Prim::new(
            ell(p.v(0.0475 * s, 1.552, 0.04), Vec3::new(p.r(0.017), p.r(0.0105), p.r(0.017)), Quat::IDENTITY),
            sm(0.016),
            M_SKIN,
            head,
        ));
        // ears
        sdf.add(Prim::new(
            ell(p.v(0.0675 * s, 1.5505, -0.012), Vec3::new(p.r(0.0085), p.r(0.0225), p.r(0.0155)), Quat::from_rotation_y(-0.25 * s) * Quat::from_rotation_x(0.15)),
            sm(0.006),
            M_SKIN,
            head,
        ));
        sdf.add(Prim::new(
            ell(p.v(0.0748 * s, 1.553, -0.0095), Vec3::new(p.r(0.0045), p.r(0.0145), p.r(0.009)), Quat::from_rotation_y(-0.25 * s)),
            Op::Subtract(p.r(0.004)),
            M_SKIN,
            head,
        ));
    }
    // chin
    sdf.add(Prim::new(
        ell(p.v(0.0, 1.4725 + lower, 0.0675), Vec3::new(p.r(0.0195) * (1.0 + m * 0.3), p.r(0.0175), p.r(0.0185)), Quat::IDENTITY),
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
    // nose: slim bridge, small rounded tip, soft wings
    sdf.add(Prim::new(
        Shape::RoundCone {
            a: p.v(0.0, 1.5715, 0.0835),
            b: p.v(0.0, 1.540, 0.0935 * nk.sqrt()),
            ra: p.r(0.0052) * nk,
            rb: p.r(0.0062) * nk,
        },
        sm(0.011),
        M_SKIN,
        head,
    ));
    sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0, 1.5342, 0.0935 + 0.004 * (nk - 1.0)), r: p.r(0.008) * nk }, sm(0.008), M_SKIN, head));
    for side in 0..2 {
        let s = sx(side);
        sdf.add(Prim::new(Shape::Sphere { c: p.v(0.0094 * s * nk, 1.5305, 0.0858), r: p.r(0.0056) * nk }, sm(0.0065), M_SKIN, head));
        sdf.add(Prim::new(
            ell(p.v(0.0058 * s, 1.5238, 0.0888), Vec3::new(p.r(0.0028), p.r(0.0017), p.r(0.0034)), Quat::from_rotation_y(0.5 * s)),
            Op::Subtract(p.r(0.0025)),
            M_SKIN,
            head,
        ));
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
    // eye sockets: a shallow carve just outside the lid shells, wider than tall so the
    // corners of the eye stay visible
    for side in 0..2 {
        let s = sx(side);
        let c = eye_center(s);
        let er = EYE_R * super::face::LID_SCALE;
        // The carve stays inside the lid shell (radius `er`), so its rim is always hidden
        // behind the lids; the wide soft blend makes the surrounding skin dip toward the eye.
        sdf.add(Prim::new(
            ell(
                p.v(c.x + 0.001 * s, c.y + er * 0.05, c.z),
                Vec3::new(p.r(er * 1.3), p.r(er * 0.66), p.r(er * 1.2)),
                Quat::from_rotation_y(super::face::EYE_YAW * s) * Quat::from_rotation_z(0.07 * s),
            ),
            Op::Subtract(p.r(0.004)),
            M_SKIN,
            head,
        ));
    }
    sdf
}

/// Eyeball radius and centre in unscaled head units.
const EYE_R: f32 = 0.0122;
fn eye_center(s: f32) -> Vec3 {
    Vec3::new(0.0312 * s, 1.5588, 0.0702)
}

pub fn hand_sdf(a: &Appearance, sk: &Skeleton, side: usize) -> Sdf {
    let p = P::body(a.height / 1.65);
    let mut sdf = Sdf::new();
    let wrist = sk.bind[HAND[side]];
    let hf = hand_frame(side);
    let (f, t, n) = (hf.f, hf.t, hf.n);
    let hd = HAND[side] as u8;
    let fa = FOREARM[side] as u8;
    // hand-space helpers: lengths scale with the body and the build's hand size
    let hs = p.k * sk.hand_scale;
    let r = |x: f32| x * hs;
    let at = |ff: f32, tt: f32, nn: f32| wrist + (f * ff + t * tt + n * nn) * hs;
    // wrist (slightly flattened, runs inside the forearm / cuff)
    sdf.add(Prim::new(
        cone(wrist - f * p.r(0.035), wrist + f * r(0.02), r(0.0235), r(0.0225)),
        Op::Union,
        M_SKIN,
        BoneSpec::blend(fa, hd, wrist - f * p.r(0.035), wrist + f * p.r(0.01), 0.3, 1.0),
    ));
    // palm: a rounded slab, wider at the knuckles than at the wrist
    let rot = Quat::from_mat3(&Mat3::from_cols(t, f.cross(t), f));
    sdf.add(Prim::new(
        rbox(at(0.05, 0.001, -0.0015), Vec3::new(r(0.036), r(0.0105), r(0.041)), r(0.0105), rot),
        Op::Smooth(r(0.012)),
        M_SKIN,
        BoneSpec::single(hd),
    ));
    // knuckle ridge on the back of the hand
    sdf.add(Prim::new(
        Shape::Capsule { a: at(0.088, 0.027, -0.004), b: at(0.081, -0.026, -0.004), r: r(0.0088) },
        Op::Smooth(r(0.01)),
        M_SKIN,
        BoneSpec::single(hd),
    ));
    // heel pads: thenar (thumb side) and hypothenar (little-finger side)
    sdf.add(Prim::new(
        ell(at(0.034, 0.02, 0.0065), Vec3::new(r(0.0165), r(0.011), r(0.028)), rot * Quat::from_rotation_y(-0.35 * sx(side))),
        Op::Smooth(r(0.01)),
        M_SKIN,
        BoneSpec::blend(hd, THUMB[side] as u8, at(0.0, 0.01, 0.0), at(0.03, 0.03, 0.0), 0.4, 1.3),
    ));
    sdf.add(Prim::new(
        ell(at(0.036, -0.024, 0.004), Vec3::new(r(0.012), r(0.01), r(0.03)), rot),
        Op::Smooth(r(0.01)),
        M_SKIN,
        BoneSpec::single(hd),
    ));
    // fingers: three phalanges each, gently curved toward the palm even at rest
    let (p1, p2) = hand::PHALANX;
    for i in 0..4 {
        let (kf, kt) = hand::KNUCKLE[i];
        let len = hand::LENGTH[i];
        let (prox, dist) = if i == 0 { (INDEX[side] as u8, INDEX2[side] as u8) } else { (FINGERS[side] as u8, FINGERS2[side] as u8) };
        // slight fan: fingers converge a little toward the middle finger tip
        let fan = -kt * 0.06;
        let kn = at(kf, kt, -0.001);
        let pip = at(kf + len * p1, kt + fan * p1, 0.0015);
        let dip = at(kf + len * (p1 + p2), kt + fan * (p1 + p2), 0.0045);
        let tip = at(kf + len - 0.006, kt + fan, 0.008);
        let rr = r(if i == 3 { 0.0072 } else { 0.0082 });
        // web between palm and finger
        sdf.add(Prim::new(
            Shape::Sphere { c: kn, r: rr * 1.1 },
            Op::Smooth(r(0.007)),
            M_SKIN,
            BoneSpec::blend(hd, prox, at(kf - 0.03, kt, 0.0), kn, 0.75, 1.25),
        ));
        sdf.add(Prim::new(
            cone(kn, pip, rr * 1.02, rr * 0.95),
            Op::Smooth(r(0.004)),
            M_SKIN,
            BoneSpec::blend(hd, prox, at(kf - 0.03, kt, 0.0), kn, 0.82, 1.18),
        ));
        sdf.add(Prim::new(
            cone(pip, dip, rr * 0.96, rr * 0.88),
            Op::Smooth(r(0.003)),
            M_SKIN,
            BoneSpec::blend(prox, dist, kn, pip, 0.8, 1.2),
        ));
        sdf.add(Prim::new(cone(dip, tip, rr * 0.88, rr * 0.78), Op::Smooth(r(0.003)), M_SKIN, BoneSpec::single(dist)));
    }
    // thumb: metacarpal (in the thenar), two phalanges
    let th = THUMB[side] as u8;
    let th2 = THUMB2[side] as u8;
    let tb = hand::THUMB_BASE;
    let td = Vec3::new(hand::THUMB_DIR.0, hand::THUMB_DIR.1, hand::THUMB_DIR.2).normalize();
    let td2 = Vec3::new(hand::THUMB_DIR2.0, hand::THUMB_DIR2.1, hand::THUMB_DIR2.2).normalize();
    let (l0, l1, l2) = hand::THUMB_LEN;
    let mc = Vec3::new(tb.0, tb.1, tb.2) + td * l0;
    let along = |d: f32| {
        let q = mc + td2 * d;
        at(q.x, q.y, q.z)
    };
    let cmc = at(tb.0, tb.1, tb.2);
    let mcp = along(0.0);
    let ip = along(l1);
    let tt = along(l1 + l2 - 0.007);
    sdf.add(Prim::new(
        cone(cmc, mcp, r(0.0125), r(0.0108)),
        Op::Smooth(r(0.01)),
        M_SKIN,
        BoneSpec::blend(hd, th, at(tb.0 - 0.02, tb.1 - 0.012, tb.2), cmc, 0.7, 1.4),
    ));
    sdf.add(Prim::new(cone(mcp, ip, r(0.0104), r(0.0096)), Op::Smooth(r(0.004)), M_SKIN, BoneSpec::blend(th, th2, cmc, mcp, 0.82, 1.18)));
    sdf.add(Prim::new(cone(ip, tt, r(0.0098), r(0.0086)), Op::Smooth(r(0.003)), M_SKIN, BoneSpec::single(th2)));
    sdf
}

pub fn hair_sdf(a: &Appearance) -> Sdf {
    let p = P::head(a.height / 1.65);
    let mut sdf = Sdf::new();
    let head = BoneSpec::single(HEAD as u8);
    let cr_c = p.v(0.0, 1.575, -0.014);
    let cr_r = Vec3::new(p.r(0.0705), p.r(0.082), p.r(0.094));
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
                ell(p.v(0.0755 * s, 1.535, 0.013), Vec3::new(p.r(0.029), p.r(0.045), p.r(0.039)), Quat::IDENTITY),
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
            sdf.add(Prim::new(cap(p.r(0.008)), Op::Union, M_HAIR, head).with_grooves(p.r(0.00055), 64.0, cr_c - Vec3::Y * p.r(0.02)));
            face_cut(&mut sdf, 1.604);
            ear_cut(&mut sdf);
            // soft volume on top and a centre parting
            sdf.add(Prim::new(ell(p.v(0.0, 1.627, 0.004), Vec3::new(p.r(0.066), p.r(0.041), p.r(0.082)), Quat::IDENTITY), Op::Smooth(p.r(0.02)), M_HAIR, head).with_grooves(p.r(0.0005), 64.0, cr_c - Vec3::Y * p.r(0.02)));
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
            sdf.add(Prim::new(cap(p.r(0.01)), Op::Union, M_HAIR, head).with_falling(p.r(0.0007), 70.0, cr_c));
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.548, -0.016), Vec3::new(p.r(0.088), p.r(0.105), p.r(0.105)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.02)),
                    M_HAIR,
                    BoneSpec::blend(HEAD as u8, HAIR as u8, p.v(0.0, 1.58, 0.0), p.v(0.0, 1.47, 0.0), 0.0, 1.0),
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.468, 0.0).y)
                .with_falling(p.r(0.0008), 70.0, cr_c),
            );
            face_cut(&mut sdf, 1.585);
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.615, 0.062), Vec3::new(p.r(0.066), p.r(0.034), p.r(0.026)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.016)),
                    M_HAIR,
                    head,
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.5915, 0.0).y)
                .with_falling(p.r(0.0004), 70.0, cr_c),
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
            sdf.add(Prim::new(cap(p.r(0.01)), Op::Union, M_HAIR, head).with_falling(p.r(0.0007), 70.0, cr_c));
            let sway = BoneSpec::blend(HEAD as u8, HAIR as u8, p.v(0.0, 1.56, 0.0), p.v(0.0, 1.3, 0.0), 0.0, 1.0);
            sdf.add(
                Prim::new(
                    ell(p.v(0.0, 1.48, -0.03), Vec3::new(p.r(0.095), p.r(0.2), p.r(0.105)), Quat::IDENTITY),
                    Op::Smooth(p.r(0.03)),
                    M_HAIR,
                    sway,
                )
                .clip(Vec3::NEG_Y, -p.v(0.0, 1.3, 0.0).y)
                .with_falling(p.r(0.0009), 70.0, cr_c),
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
            ell(p.v(0.0, 1.5115, 0.084), Vec3::new(p.r(0.021), p.r(0.0052), p.r(0.0062)), Quat::IDENTITY),
            Op::Union,
            M_HAIR,
            head,
        ).with_disp(p.r(0.0006), 400.0));
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
        let list = mesh.list_for(i);
        let (mat, mut w) = sdf.attributes_in(*p, sigma, list);
        adjust(*p, mat, &mut w);
        // SDF ambient occlusion (two samples along the normal)
        let mut occ = 0.0;
        let mut wsum = 0.0;
        for (k, h) in [0.016f32, 0.042].iter().enumerate() {
            let h = h * ao_scale;
            let d = sdf.eval_in(*p + n * h, list);
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

/// Paints subtle colour zones into the face skin (blush, warm nose and ears, depth around
/// the eyes, lip base, beard shadow) so it stops reading as one flat plastic tone.
fn tint_face(mesh: &mut SkinMeshData, a: &Appearance, k: f32, mats: &[Mat]) {
    let hs = HEAD_SCALE;
    let fem = 1.0 - a.masc;
    let young = 1.0 - ((a.age - 1.0).max(0.0) * 0.6).min(0.6);
    let lower = 0.0045 * fem + 0.003 * (1.0 - a.age.min(1.0));
    let lip = mats[M_LIP as usize].color;
    let g = |d2: f32, r: f32| (-d2 / (r * r)).exp();
    for v in &mut mesh.verts {
        let ux = v.pos[0] / (k * hs);
        let uy = (v.pos[1] / k - HEAD_PIVOT) / hs + HEAD_PIVOT;
        let uz = v.pos[2] / (k * hs);
        if uy < 1.455 {
            continue;
        }
        let ax = ux.abs();
        let mut c = [v.color[0] as f32, v.color[1] as f32, v.color[2] as f32];
        let mul = |c: &mut [f32; 3], m: [f32; 3], w: f32| {
            for i in 0..3 {
                c[i] *= 1.0 + (m[i] - 1.0) * w;
            }
        };
        // blush on the apples of the cheeks
        let d2 = (ax - 0.037).powi(2) + (uy - 1.532).powi(2) * 1.4 + (uz - 0.06).powi(2) * 0.5;
        mul(&mut c, [1.04, 0.9, 0.9], g(d2, 0.019) * (0.55 + 0.45 * fem) * young);
        // nose tip and wings
        let d2 = ux * ux + (uy - 1.533).powi(2) + (uz - 0.095).powi(2);
        mul(&mut c, [1.03, 0.91, 0.9], g(d2, 0.012) * 0.7);
        // ears
        mul(&mut c, [1.03, 0.9, 0.89], smoothstep(0.066, 0.075, ax) * smoothstep(0.03, 0.0, uz) * 0.8);
        // depth around the eyes
        let d2 = (ax - 0.0312).powi(2) + (uy - 1.561).powi(2) * 2.2 + (uz - 0.078).powi(2) * 0.6;
        mul(&mut c, [0.95, 0.9, 0.9], g(d2, 0.0165) * 0.7);
        // beard shadow
        if a.masc > 0.5 && a.age >= 1.0 {
            let w = smoothstep(1.527, 1.508, uy) * smoothstep(-0.015, 0.03, uz) * smoothstep(1.462, 1.475, uy);
            let upper_lip = g(ux * ux + (uy - 1.513).powi(2) * 6.0, 0.02) * smoothstep(0.07, 0.085, uz);
            mul(&mut c, [0.84, 0.86, 0.9], (w.max(upper_lip) * a.masc * 0.55).min(1.0));
        }
        // lip base under the animated mouth
        let d2 = (ux / 0.0185).powi(2) + ((uy - (1.5005 + lower * 0.6)) / 0.0085).powi(2);
        let w = smoothstep(1.0, 0.55, d2) * smoothstep(0.065, 0.078, uz);
        for i in 0..3 {
            c[i] += (lip[i] as f32 - c[i]) * w * 0.85;
            v.color[i] = c[i].clamp(0.0, 255.0) as u8;
        }
    }
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
        Quality::High => (0.0072, 0.003, 0.003, 0.0039),
        Quality::Low => (0.0095, 0.004, 0.0038, 0.005),
    };
    let mut skin = SkinMeshData::default();
    let t0 = web_time::Instant::now();

    // body
    let body = body_sdf(a, sk);
    let lo = Vec3::new(-0.7, -0.01, -0.22) * k;
    let hi = Vec3::new(0.7, 1.47, 0.26) * k;
    let bm = surface_nets(&body, lo, hi, c_body * k);
    let hem_y = 1.0 * k;
    let is_skirt = a.bottom == Bottom::Skirt;
    let adjust_body = move |p: Vec3, mat: u8, w: &mut [(u8, f32); 4]| {
        // loose clothing sways with the hem bone (never the sleeves: cuffs hang at hem
        // height in the bind pose but must follow the arms)
        let on_arm = w.iter().any(|e| e.1 > 0.15 && ((CLAV_L..=FINGERS_R).contains(&(e.0 as usize)) || e.0 as usize > HEM));
        let loose = if on_arm {
            0.0
        } else if mat == M_TOP || mat == M_TOP2 {
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
    let t_body = t0.elapsed().as_secs_f32();

    // head
    let hs = head_sdf(a);
    let hlo = Vec3::new(-0.115, 1.4, -0.14) * k;
    let hhi = Vec3::new(0.115, 1.7, 0.145) * k;
    let hm = surface_nets(&hs, hlo, hhi, c_head * k);
    let mut head_mesh = to_skin(&hm, &hs, &mats, 0.01 * k, k * 0.6, &|_, _, _| {});
    tint_face(&mut head_mesh, a, k, &mats);
    append_skin(&mut skin, &head_mesh);
    let t_head = t0.elapsed().as_secs_f32();

    // hands
    for side in 0..2 {
        let hsdf = hand_sdf(a, sk, side);
        let w = sk.bind[HAND[side]];
        let f = (sk.bind[HAND[side]] - sk.bind[FOREARM[side]]).normalize();
        let c = w + f * 0.075 * k;
        let lo = c - Vec3::splat(0.125 * k);
        let hi = c + Vec3::splat(0.125 * k);
        let m = surface_nets(&hsdf, lo, hi, c_hand * k);
        append_skin(&mut skin, &to_skin(&m, &hsdf, &mats, 0.008 * k, k * 0.5, &|_, _, _| {}));
    }

    let t_hands = t0.elapsed().as_secs_f32();
    // hair
    let hair = hair_sdf(a);
    let ylo = if a.hair == Hair::Long { 1.23 } else { 1.42 };
    let hrlo = Vec3::new(-0.14, ylo, -0.21) * k;
    let hrhi = Vec3::new(0.14, 1.76, 0.16) * k;
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

    log::info!(
        "{}: body {:.2}s head {:.2}s hands {:.2}s hair {:.2}s, {} verts",
        a.name,
        t_body,
        t_head - t_body,
        t_hands - t_head,
        t0.elapsed().as_secs_f32() - t_hands,
        skin.verts.len()
    );
    // face layout (relative to the head bone)
    let hp = P::head(k);
    let head_b = sk.bind[HEAD];
    let mut face = FaceLayout {
        eye_r: hp.r(EYE_R),
        ..Default::default()
    };
    for side in 0..2 {
        let s = sx(side);
        let ec = eye_center(s);
        face.eye[side] = hp.v(ec.x, ec.y, ec.z) - head_b;
        // brow arc: starts low near the nose, peaks at two thirds, short descending tail
        let arch = 1.0 - a.masc * 0.55;
        let drop = a.masc * 0.0012;
        let brow_y = [1.5802f32 - drop, 1.5816 - drop + 0.0008 * arch, 1.5822 - drop + 0.0016 * arch, 1.5792 - drop + 0.0004 * arch];
        for (i, x) in [0.0118f32, 0.0235, 0.0365, 0.0475].iter().enumerate() {
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
mod probe {
    use super::*;
    #[test]
    /// Everything near a hand must be skinned to that arm only: a stray weight on a body
    /// bone (the hem, the other arm) tears the mesh as soon as the arm leaves the hip.
    fn hands_follow_their_arm() {
        assert!(BONE_COUNT <= crate::gfx::renderer::MAX_BONES);
        let a = Appearance::sofia();
        let sk = Skeleton::new(a.height, 1.0, 1.05);
        let m = build(&a, &sk, Quality::Low);
        for side in 0..2 {
            let wrist = sk.bind[HAND[side]];
            let f = hand_frame(side).f;
            let allowed = [FOREARM[side], HAND[side], FINGERS[side], FINGERS2[side], INDEX[side], INDEX2[side], THUMB[side], THUMB2[side]];
            let mut seen = 0;
            for v in &m.skin.verts {
                let p = Vec3::from(v.pos);
                // the hand itself and the cuff around the wrist
                let along = (p - wrist).dot(f);
                if p.distance(wrist + f * 0.08) < 0.11 && along > -0.03 {
                    seen += 1;
                    for k in 0..4 {
                        if v.weights[k] > 8 {
                            assert!(allowed.contains(&(v.joints[k] as usize)), "side {side}: bone {} weight {} at {p:?}", v.joints[k], v.weights[k]);
                        }
                    }
                }
            }
            assert!(seen > 500, "hand mesh missing on side {side}");
        }
    }

    #[test]
    /// A relaxed hand hangs with gently curled fingers and the thumb beside the index
    /// (no claw): every digit stays within ~60 degrees of the hand direction.
    fn relaxed_hand_is_not_a_claw() {
        let a = Appearance::sofia();
        let sk = Skeleton::new(a.height, 1.0, 1.05);
        let mut an = crate::character::anim::Animator::new(sk.clone(), Vec3::ZERO, 0.0, 1);
        for _ in 0..120 {
            an.update(1.0 / 60.0);
        }
        for side in 0..2 {
            let hf = hand_frame(side);
            let f = an.fk.rot[HAND[side]] * hf.f;
            for (bone, dir) in [(FINGERS2[side], hf.f), (INDEX2[side], hf.f), (THUMB2[side], sk.bone_dir(THUMB2[side]))] {
                let d = an.fk.rot[bone] * dir;
                assert!(d.dot(f) > 0.5, "side {side} bone {bone}: {d:?} vs hand {f:?}");
            }
        }
    }
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
