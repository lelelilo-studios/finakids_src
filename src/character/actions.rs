//! Library of gesture sequences. Every action follows
//! anticipation -> movement -> contact -> action -> recovery.
//! Positions are character-local (x = left, y = up, z = forward) scaled by `k`.

use super::anim::{ph, Action, Set};
use super::face::Expr;
use crate::math::Ease;
use glam::Vec3;

pub const EV_GRAB: u32 = 1;
pub const EV_RELEASE: u32 = 2;
pub const EV_DROP: u32 = 3;
pub const EV_GIVE: u32 = 4;
pub const EV_TAKE: u32 = 5;

const L: usize = 0;
const R: usize = 1;

fn v(k: f32, x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z) * k
}

fn n(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z).normalize()
}

/// Right hand holds the phone in front of the face.
pub fn phone_view_pos(k: f32) -> (Vec3, Vec3) {
    (v(k, -0.05, 1.22, 0.3), n(0.1, 0.55, -0.83))
}

/// Pick the phone up from a surface (local target point).
pub fn pick_up_phone(k: f32, target: Vec3) -> Action {
    let (view, palm) = phone_view_pos(k);
    Action {
        name: "pick_phone",
        hold: true,
        phases: vec![
            // look, slight anticipation
            ph(0.35, Ease::InOut, vec![Set::LookLocal(target), Set::Bend(0.05), Set::Expr(Expr::Focused)]),
            // reach
            ph(
                0.55,
                Ease::InOut,
                vec![Set::Hand(R, target + v(k, 0.0, 0.03, -0.02), n(0.0, -1.0, 0.1), 1.0), Set::Bend(0.18), Set::Grip(R, 0.05)],
            ),
            // contact
            ph(0.18, Ease::Out, vec![Set::Hand(R, target + v(k, 0.0, 0.012, -0.02), n(0.0, -1.0, 0.1), 1.0), Set::Grip(R, 0.6), Set::Event(EV_GRAB)]),
            // lift
            ph(0.6, Ease::InOut, vec![Set::Hand(R, view, palm, 1.0), Set::Bend(0.04), Set::LookLocal(view + v(k, 0.0, 0.02, 0.0))]),
            // settle
            ph(0.3, Ease::Out, vec![Set::HeadPitch(0.05)]),
        ],
    }
}

/// Takes the phone out of the pocket and looks at it.
pub fn phone_from_pocket(k: f32) -> Action {
    let (view, palm) = phone_view_pos(k);
    Action {
        name: "phone_pocket",
        hold: true,
        phases: vec![
            ph(0.22, Ease::InOut, vec![Set::LookLocal(v(k, -0.1, 0.9, 0.3)), Set::HeadPitch(0.1)]),
            ph(0.35, Ease::InOut, vec![Set::Hand(R, v(k, -0.13, 0.93, 0.06), n(-1.0, 0.0, 0.0), 1.0), Set::Grip(R, 0.1)]),
            ph(0.15, Ease::Out, vec![Set::Grip(R, 0.6), Set::Event(EV_GRAB)]),
            ph(0.55, Ease::InOut, vec![Set::Hand(R, view, palm, 1.0), Set::LookLocal(view + v(k, 0.0, 0.02, 0.0))]),
            ph(0.2, Ease::Out, vec![]),
        ],
    }
}

pub fn put_away_phone(k: f32) -> Action {
    Action {
        name: "phone_away",
        hold: false,
        phases: vec![
            ph(0.4, Ease::InOut, vec![Set::Hand(R, v(k, -0.13, 0.93, 0.06), n(-1.0, 0.0, 0.0), 1.0), Set::LookOff]),
            ph(0.12, Ease::Out, vec![Set::Grip(R, 0.1), Set::Event(EV_RELEASE)]),
            ph(0.4, Ease::InOut, vec![Set::HandW(R, 0.0), Set::HeadPitch(0.0)]),
        ],
    }
}

/// Place the held phone down on a surface.
pub fn put_down(k: f32, target: Vec3) -> Action {
    Action {
        name: "put_down",
        hold: false,
        phases: vec![
            ph(0.5, Ease::InOut, vec![Set::Hand(R, target + v(k, 0.0, 0.03, -0.02), n(0.0, -1.0, 0.1), 1.0), Set::Bend(0.15), Set::LookLocal(target)]),
            ph(0.15, Ease::Out, vec![Set::Grip(R, 0.1), Set::Event(EV_RELEASE)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::Bend(0.0), Set::LookOff]),
        ],
    }
}

pub fn drop_phone(k: f32) -> Action {
    Action {
        name: "drop_phone",
        hold: false,
        phases: vec![
            ph(0.12, Ease::In, vec![Set::Grip(R, 0.0), Set::Event(EV_DROP), Set::Expr(Expr::Surprised)]),
            ph(0.25, Ease::OutBack, vec![Set::Hand(R, v(k, -0.1, 1.05, 0.35), n(0.0, -1.0, 0.0), 1.0), Set::Hand(L, v(k, 0.1, 1.05, 0.3), n(0.0, -1.0, 0.0), 1.0), Set::Bend(0.2), Set::LookLocal(v(k, 0.0, 0.0, 0.5))]),
            ph(0.6, Ease::Out, vec![Set::Bend(0.35), Set::Crouch(0.1)]),
            ph(0.5, Ease::InOut, vec![Set::Hand(L, v(k, 0.03, 1.415, 0.155), n(0.0, 0.0, -1.0), 1.0), Set::Grip(L, 0.15), Set::Expr(Expr::Worried), Set::Bend(0.25), Set::Crouch(0.05)]),
            ph(0.8, Ease::InOut, vec![Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::Bend(0.0), Set::Crouch(0.0)]),
        ],
    }
}

pub fn wave(k: f32) -> Action {
    Action {
        name: "wave",
        hold: false,
        phases: vec![
            ph(0.15, Ease::In, vec![Set::Shrug(-0.2)]),
            ph(0.35, Ease::OutBack, vec![Set::Hand(L, v(k, 0.3, 1.62, 0.12), n(0.0, 0.1, 1.0), 1.0), Set::Grip(L, 0.0), Set::Shrug(0.15), Set::Expr(Expr::Happy)]),
            ph(1.0, Ease::Linear, vec![Set::Wiggle(L, Vec3::X * k, 0.06, 2.4), Set::HeadRoll(0.08)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(L, 0.0), Set::Shrug(0.0), Set::HeadRoll(0.0)]),
        ],
    }
}

pub fn stretch(k: f32) -> Action {
    Action {
        name: "stretch",
        hold: false,
        phases: vec![
            ph(0.4, Ease::InOut, vec![Set::Bend(0.1), Set::Crouch(0.05), Set::Expr(Expr::Tired)]),
            ph(0.8, Ease::Out, vec![Set::Hand(L, v(k, 0.14, 2.02, 0.0), n(-1.0, 0.3, 0.0), 1.0), Set::Hand(R, v(k, -0.14, 2.02, 0.0), n(1.0, 0.3, 0.0), 1.0), Set::Bend(-0.18), Set::HeadPitch(-0.35), Set::Crouch(-0.02), Set::Grip(L, 0.1), Set::Grip(R, 0.1)]),
            ph(0.9, Ease::Sine, vec![Set::Side(0.12), Set::Expr(Expr::Joy)]),
            ph(0.5, Ease::Sine, vec![Set::Side(-0.08)]),
            ph(0.8, Ease::InOut, vec![Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::Bend(0.0), Set::Side(0.0), Set::HeadPitch(0.0), Set::Crouch(0.0), Set::Expr(Expr::Neutral)]),
        ],
    }
}

pub fn think(k: f32) -> Action {
    Action {
        name: "think",
        hold: true,
        phases: vec![
            ph(0.3, Ease::InOut, vec![Set::HeadPitch(-0.1), Set::Expr(Expr::Thinking)]),
            // chin resting on a loose fist, the other arm supporting the elbow
            ph(0.6, Ease::InOut, vec![Set::Hand(R, v(k, -0.025, 1.355, 0.135), n(0.2, 0.2, -1.0), 1.0), Set::Grip(R, 0.6), Set::Hand(L, v(k, -0.06, 1.12, 0.16), n(0.0, 1.0, -0.2), 1.0), Set::Grip(L, 0.3), Set::HeadYaw(0.15), Set::HeadPitch(-0.18)]),
            ph(1.2, Ease::Sine, vec![Set::HeadYaw(-0.1), Set::HeadRoll(0.06)]),
        ],
    }
}

pub fn arms_crossed(k: f32) -> Action {
    Action {
        name: "arms_crossed",
        hold: true,
        phases: vec![
            ph(0.2, Ease::In, vec![Set::Shrug(0.1)]),
            ph(0.6, Ease::InOut, vec![Set::Hand(L, v(k, -0.11, 1.16, 0.15), n(0.0, 0.0, -1.0), 1.0), Set::Hand(R, v(k, 0.1, 1.19, 0.13), n(0.0, 0.0, -1.0), 1.0), Set::Grip(L, 0.5), Set::Grip(R, 0.5), Set::Shrug(0.0), Set::Bend(-0.03)]),
        ],
    }
}

pub fn hands_on_hips(k: f32) -> Action {
    Action {
        name: "hands_hips",
        hold: true,
        phases: vec![ph(0.6, Ease::InOut, vec![Set::Hand(L, v(k, 0.18, 0.99, 0.02), n(-1.0, 0.0, 0.0), 1.0), Set::Hand(R, v(k, -0.18, 0.99, 0.02), n(1.0, 0.0, 0.0), 1.0), Set::Grip(L, 0.35), Set::Grip(R, 0.35), Set::Bend(-0.04)])],
    }
}

pub fn shrug(k: f32) -> Action {
    Action {
        name: "shrug",
        hold: false,
        phases: vec![
            ph(0.15, Ease::In, vec![Set::Shrug(-0.1)]),
            ph(0.35, Ease::OutBack, vec![Set::Shrug(0.9), Set::Hand(L, v(k, 0.26, 0.98, 0.2), n(0.0, 1.0, 0.0), 1.0), Set::Hand(R, v(k, -0.26, 0.98, 0.2), n(0.0, 1.0, 0.0), 1.0), Set::HeadRoll(0.12), Set::Grip(L, 0.0), Set::Grip(R, 0.0)]),
            ph(0.5, Ease::Linear, vec![]),
            ph(0.5, Ease::InOut, vec![Set::Shrug(0.0), Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::HeadRoll(0.0)]),
        ],
    }
}

pub fn celebrate(k: f32) -> Action {
    Action {
        name: "celebrate",
        hold: false,
        phases: vec![
            ph(0.2, Ease::In, vec![Set::Crouch(0.12), Set::Bend(0.1), Set::Hand(L, v(k, 0.15, 1.05, 0.2), n(0.0, 0.0, -1.0), 1.0), Set::Hand(R, v(k, -0.15, 1.05, 0.2), n(0.0, 0.0, -1.0), 1.0), Set::Grip(L, 1.0), Set::Grip(R, 1.0)]),
            ph(0.3, Ease::OutBack, vec![Set::Crouch(-0.03), Set::Bend(-0.15), Set::Hand(L, v(k, 0.22, 1.9, 0.12), n(0.0, 0.0, 1.0), 1.0), Set::Hand(R, v(k, -0.22, 1.9, 0.12), n(0.0, 0.0, 1.0), 1.0), Set::Expr(Expr::Joy), Set::HeadPitch(-0.2)]),
            ph(1.1, Ease::Linear, vec![Set::Bounce(1.0), Set::Wiggle(L, Vec3::Y * k, 0.03, 3.0), Set::Wiggle(R, Vec3::Y * k, 0.03, 3.0)]),
            ph(0.6, Ease::InOut, vec![Set::Bounce(0.0), Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::Crouch(0.0), Set::Bend(0.0), Set::HeadPitch(0.0), Set::Expr(Expr::Happy)]),
        ],
    }
}

pub fn sad(k: f32) -> Action {
    let _ = k;
    Action {
        name: "sad",
        hold: false,
        phases: vec![
            ph(0.8, Ease::InOut, vec![Set::Bend(0.2), Set::HeadPitch(0.35), Set::Shrug(-0.3), Set::Expr(Expr::Sad)]),
            ph(1.4, Ease::Linear, vec![]),
            ph(0.9, Ease::InOut, vec![Set::Bend(0.0), Set::HeadPitch(0.0), Set::Shrug(0.0)]),
        ],
    }
}

pub fn facepalm(k: f32) -> Action {
    Action {
        name: "facepalm",
        hold: false,
        phases: vec![
            ph(0.2, Ease::In, vec![Set::HeadPitch(-0.1)]),
            ph(0.35, Ease::Out, vec![Set::Hand(R, v(k, -0.015, 1.47, 0.155), n(0.0, -0.2, -1.0), 1.0), Set::Grip(R, 0.12), Set::HeadPitch(0.25), Set::Expr(Expr::Annoyed)]),
            ph(1.0, Ease::Linear, vec![Set::HeadYaw(0.1)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::HeadPitch(0.0), Set::HeadYaw(0.0)]),
        ],
    }
}

pub fn nod(k: f32) -> Action {
    let _ = k;
    Action {
        name: "nod",
        hold: false,
        phases: vec![
            ph(0.14, Ease::InOut, vec![Set::HeadPitch(0.18)]),
            ph(0.14, Ease::InOut, vec![Set::HeadPitch(-0.04)]),
            ph(0.14, Ease::InOut, vec![Set::HeadPitch(0.14)]),
            ph(0.2, Ease::InOut, vec![Set::HeadPitch(0.0)]),
        ],
    }
}

pub fn shake_head(k: f32) -> Action {
    let _ = k;
    Action {
        name: "shake",
        hold: false,
        phases: vec![
            ph(0.14, Ease::InOut, vec![Set::HeadYaw(0.22)]),
            ph(0.18, Ease::InOut, vec![Set::HeadYaw(-0.22)]),
            ph(0.18, Ease::InOut, vec![Set::HeadYaw(0.16)]),
            ph(0.2, Ease::InOut, vec![Set::HeadYaw(0.0)]),
        ],
    }
}

pub fn give(k: f32) -> Action {
    Action {
        name: "give",
        hold: false,
        phases: vec![
            ph(0.2, Ease::In, vec![Set::Hand(R, v(k, -0.12, 1.05, 0.15), n(0.0, 1.0, 0.0), 0.8)]),
            ph(0.45, Ease::Out, vec![Set::Hand(R, v(k, -0.08, 1.15, 0.42), n(0.0, 1.0, 0.2), 1.0), Set::Bend(0.08), Set::Grip(R, 0.2)]),
            ph(0.3, Ease::Linear, vec![Set::Event(EV_GIVE)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::Bend(0.0)]),
        ],
    }
}

pub fn receive(k: f32) -> Action {
    Action {
        name: "receive",
        hold: false,
        phases: vec![
            ph(0.45, Ease::Out, vec![Set::Hand(R, v(k, -0.08, 1.12, 0.4), n(0.0, 1.0, 0.1), 1.0), Set::Bend(0.06), Set::Grip(R, 0.0)]),
            ph(0.2, Ease::Out, vec![Set::Grip(R, 0.6), Set::Event(EV_TAKE)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::Bend(0.0)]),
        ],
    }
}

pub fn point_at(k: f32, target: Vec3) -> Action {
    let dir = (target - v(k, -0.15, 1.35, 0.0)).normalize_or(Vec3::Z);
    let hand = v(k, -0.15, 1.35, 0.0) + dir * 0.55 * k;
    Action {
        name: "point",
        hold: false,
        phases: vec![
            ph(0.35, Ease::OutBack, vec![Set::Hand(R, hand, n(0.3, -1.0, 0.0), 1.0), Set::Grip(R, 0.3), Set::Point(R, 1.0), Set::LookLocal(target)]),
            ph(1.0, Ease::Linear, vec![]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::Point(R, 0.0), Set::LookOff]),
        ],
    }
}

pub fn type_laptop(k: f32, desk_y: f32) -> Action {
    Action {
        name: "type",
        hold: true,
        phases: vec![
            ph(0.6, Ease::InOut, vec![Set::Hand(L, v(k, 0.12, 0.0, 0.34) + Vec3::Y * (desk_y + 0.03), n(0.0, -1.0, 0.0), 1.0), Set::Hand(R, v(k, -0.12, 0.0, 0.34) + Vec3::Y * (desk_y + 0.03), n(0.0, -1.0, 0.0), 1.0), Set::Bend(0.12), Set::HeadPitch(0.25), Set::Grip(L, 0.3), Set::Grip(R, 0.3), Set::Expr(Expr::Focused)]),
            ph(3.0, Ease::Linear, vec![Set::Wiggle(L, Vec3::Y * k, 0.01, 5.0), Set::Wiggle(R, Vec3::Y * k, 0.01, 6.3)]),
        ],
    }
}

pub fn scratch_head(k: f32) -> Action {
    Action {
        name: "scratch",
        hold: false,
        phases: vec![
            ph(0.45, Ease::InOut, vec![Set::Hand(R, v(k, -0.105, 1.6, -0.03), n(0.5, -0.5, -0.4), 1.0), Set::Grip(R, 0.45), Set::HeadRoll(-0.1), Set::Expr(Expr::Thinking)]),
            ph(0.8, Ease::Linear, vec![Set::Wiggle(R, Vec3::X * k, 0.015, 3.0)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(R, 0.0), Set::HeadRoll(0.0)]),
        ],
    }
}

pub fn talk_gesture(k: f32, seed: u32) -> Action {
    let s = if seed % 2 == 0 { L } else { R };
    let x = if s == L { 0.12 } else { -0.12 };
    let open = seed % 3 == 0;
    let mut phases = vec![ph(
        0.35,
        Ease::OutBack,
        vec![Set::Hand(s, v(k, x, 1.08, 0.3), n(-x * 3.0, 0.8, 0.3), 0.85), Set::Grip(s, if open { 0.05 } else { 0.4 })],
    )];
    if open {
        phases.push(ph(
            0.35,
            Ease::InOut,
            vec![Set::Hand(1 - s, v(k, -x, 1.05, 0.28), n(x * 3.0, 0.8, 0.3), 0.75), Set::Grip(1 - s, 0.05)],
        ));
    }
    phases.push(ph(0.6, Ease::Sine, vec![Set::Wiggle(s, Vec3::new(0.3, 1.0, 0.2) * k, 0.02, 1.3)]));
    phases.push(ph(0.55, Ease::InOut, vec![Set::HandW(L, 0.0), Set::HandW(R, 0.0)]));
    Action {
        name: "talk_gesture",
        hold: false,
        phases,
    }
}

pub fn look_around(k: f32) -> Action {
    let _ = k;
    Action {
        name: "look_around",
        hold: false,
        phases: vec![
            ph(0.6, Ease::InOut, vec![Set::HeadYaw(0.55), Set::HeadPitch(-0.05)]),
            ph(0.9, Ease::Linear, vec![]),
            ph(0.8, Ease::InOut, vec![Set::HeadYaw(-0.4)]),
            ph(0.7, Ease::Linear, vec![]),
            ph(0.6, Ease::InOut, vec![Set::HeadYaw(0.0), Set::HeadPitch(0.0)]),
        ],
    }
}

pub fn shift_weight(k: f32) -> Action {
    let _ = k;
    Action {
        name: "shift_weight",
        hold: false,
        phases: vec![
            ph(1.0, Ease::InOut, vec![Set::Side(0.07), Set::HeadRoll(-0.05)]),
            ph(1.6, Ease::Linear, vec![]),
            ph(1.0, Ease::InOut, vec![Set::Side(0.0), Set::HeadRoll(0.0)]),
        ],
    }
}

pub fn adjust_sleeve(k: f32) -> Action {
    Action {
        name: "adjust_sleeve",
        hold: false,
        phases: vec![
            ph(0.3, Ease::InOut, vec![Set::LookLocal(v(k, -0.1, 1.0, 0.3)), Set::HeadPitch(0.2)]),
            ph(0.5, Ease::InOut, vec![Set::Hand(R, v(k, -0.08, 1.02, 0.22), n(0.0, 0.5, -1.0), 1.0), Set::Hand(L, v(k, -0.03, 1.02, 0.2), n(-1.0, 0.0, 0.0), 1.0), Set::Grip(L, 0.6)]),
            ph(0.6, Ease::Linear, vec![Set::Wiggle(L, Vec3::new(-1.0, 0.0, 0.0) * k, 0.012, 2.0)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::LookOff, Set::HeadPitch(0.0)]),
        ],
    }
}

pub fn stretch_neck(k: f32) -> Action {
    let _ = k;
    Action {
        name: "stretch_neck",
        hold: false,
        phases: vec![
            ph(0.6, Ease::InOut, vec![Set::HeadRoll(0.18)]),
            ph(0.7, Ease::InOut, vec![Set::HeadRoll(-0.15)]),
            ph(0.5, Ease::InOut, vec![Set::HeadRoll(0.0)]),
        ],
    }
}

pub fn yawn(k: f32) -> Action {
    Action {
        name: "yawn",
        hold: false,
        phases: vec![
            ph(0.4, Ease::InOut, vec![Set::Expr(Expr::Tired), Set::HeadPitch(-0.15)]),
            ph(0.5, Ease::Out, vec![Set::Hand(R, v(k, -0.03, 1.4, 0.165), n(0.0, 0.0, -1.0), 1.0), Set::Grip(R, 0.15), Set::Expr(Expr::Surprised), Set::HeadPitch(-0.25)]),
            ph(0.8, Ease::Linear, vec![]),
            ph(0.6, Ease::InOut, vec![Set::HandW(R, 0.0), Set::HeadPitch(0.0), Set::Expr(Expr::Tired)]),
        ],
    }
}

pub fn surprised(k: f32) -> Action {
    Action {
        name: "surprised",
        hold: false,
        phases: vec![
            ph(0.12, Ease::Out, vec![Set::Expr(Expr::Surprised), Set::Shrug(0.4), Set::Bend(-0.08), Set::Hand(L, v(k, 0.05, 1.3, 0.2), n(0.0, 0.0, -1.0), 0.6)]),
            ph(0.8, Ease::Linear, vec![]),
            ph(0.6, Ease::InOut, vec![Set::Shrug(0.0), Set::Bend(0.0), Set::HandW(L, 0.0)]),
        ],
    }
}

pub fn sit_think(k: f32) -> Action {
    Action {
        name: "sit_think",
        hold: true,
        phases: vec![ph(0.8, Ease::InOut, vec![Set::Hand(R, v(k, -0.02, 1.0, 0.25), n(0.0, 0.3, -1.0), 0.8), Set::HeadPitch(0.1), Set::Bend(0.12), Set::Expr(Expr::Thinking)])],
    }
}

pub fn count_money(k: f32) -> Action {
    Action {
        name: "count_money",
        hold: false,
        phases: vec![
            ph(0.5, Ease::InOut, vec![Set::Hand(L, v(k, 0.05, 1.1, 0.3), n(0.0, 1.0, 0.0), 1.0), Set::Hand(R, v(k, -0.05, 1.13, 0.3), n(0.0, -1.0, 0.0), 1.0), Set::LookLocal(v(k, 0.0, 1.1, 0.3)), Set::Bend(0.1)]),
            ph(1.4, Ease::Linear, vec![Set::Wiggle(R, Vec3::X * k, 0.025, 2.5)]),
            ph(0.5, Ease::InOut, vec![Set::HandW(L, 0.0), Set::HandW(R, 0.0), Set::LookOff, Set::Bend(0.0)]),
        ],
    }
}
