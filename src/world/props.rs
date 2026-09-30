//! Procedural furniture and props. Every function builds geometry in local
//! space (origin on the floor, facing +Z) and returns MeshData.

use crate::gfx::mesh::{kind, m4, m4r, m4s, Mat, MeshData};
use crate::math::{fbm3, noise3, Rng};
use glam::{Mat4, Quat, Vec2, Vec3};

pub const WOOD: Mat = Mat::new(0xa87b52, 0.55).kind(kind::WOOD);
pub const WOOD_LIGHT: Mat = Mat::new(0xd2b48c, 0.5).kind(kind::WOOD);
pub const WOOD_DARK: Mat = Mat::new(0x5c3d2a, 0.45).kind(kind::WOOD);
pub const WHITE_PAINT: Mat = Mat::new(0xefece6, 0.6);
pub const METAL_DARK: Mat = Mat::new(0x2a2b30, 0.35).metal(0.9);
pub const METAL: Mat = Mat::new(0xb8bcc2, 0.3).metal(1.0).kind(kind::BRUSHED);
pub const PLASTIC_DARK: Mat = Mat::new(0x202226, 0.4);
pub const CERAMIC: Mat = Mat::new(0xf1ede6, 0.2);

fn quat_y(a: f32) -> Quat {
    Quat::from_rotation_y(a)
}

/// Rumples the top of a mesh with noise (duvets, cushions).
fn rumple(m: &mut MeshData, from: usize, amp: f32, freq: f32, seed: f32) {
    for v in &mut m.verts[from..] {
        let p = Vec3::from(v.pos);
        let n = Vec3::from(v.nrm);
        let d = fbm3(p * freq + Vec3::splat(seed), 3) * amp;
        let q = p + n * d;
        v.pos = q.to_array();
    }
}

pub fn bed(duvet: Mat, pillow: Mat) -> MeshData {
    // bed along X, head at +X; footprint 2.0 x 0.95
    let mut m = MeshData::new();
    let frame = WOOD_LIGHT;
    m.rbox(m4(Vec3::new(0.0, 0.2, 0.0)), Vec3::new(1.0, 0.1, 0.475), 0.02, 2, &frame);
    for (x, z) in [(-0.95, -0.42), (-0.95, 0.42), (0.95, -0.42), (0.95, 0.42)] {
        m.rbox(m4(Vec3::new(x, 0.06, z)), Vec3::new(0.035, 0.06, 0.035), 0.01, 1, &frame);
    }
    // headboard
    m.rbox(m4(Vec3::new(0.99, 0.62, 0.0)), Vec3::new(0.03, 0.42, 0.48), 0.03, 3, &frame);
    m.rbox(m4(Vec3::new(0.965, 0.68, 0.0)), Vec3::new(0.012, 0.28, 0.4), 0.01, 2, &Mat::new(0xd9cfc2, 0.9).kind(kind::FABRIC));
    // mattress
    m.rbox(m4(Vec3::new(-0.02, 0.4, 0.0)), Vec3::new(0.96, 0.1, 0.45), 0.06, 3, &Mat::new(0xf4f1ea, 0.85).kind(kind::FABRIC));
    // duvet (rumpled, draped)
    let start = m.verts.len();
    m.rbox(m4(Vec3::new(-0.2, 0.5, 0.0)), Vec3::new(0.8, 0.055, 0.5), 0.05, 5, &duvet);
    rumple(&mut m, start, 0.025, 3.5, 1.3);
    // folded top edge of the duvet
    let s2 = m.verts.len();
    m.rbox(m4r(Vec3::new(0.52, 0.56, 0.0), Quat::from_rotation_z(0.05)), Vec3::new(0.09, 0.04, 0.5), 0.04, 4, &Mat::new(0xf4f1ea, 0.85).kind(kind::FABRIC));
    rumple(&mut m, s2, 0.012, 5.0, 2.1);
    // pillows
    for (i, z) in [-0.2f32, 0.2].iter().enumerate() {
        let s3 = m.verts.len();
        m.rbox(
            m4r(Vec3::new(0.78, 0.58, *z), Quat::from_rotation_z(-0.25 + i as f32 * 0.05) * quat_y(0.06 * i as f32)),
            Vec3::new(0.1, 0.07, 0.24),
            0.07,
            4,
            &pillow,
        );
        rumple(&mut m, s3, 0.012, 6.0, i as f32 * 3.0);
    }
    m.floor_ao(0.0, 0.25, 0.5);
    m
}

pub fn nightstand() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.29, 0.0)), Vec3::new(0.22, 0.26, 0.2), 0.012, 2, &WHITE_PAINT);
    m.rbox(m4(Vec3::new(0.0, 0.565, 0.0)), Vec3::new(0.235, 0.018, 0.215), 0.008, 2, &WOOD_LIGHT);
    m.rbox(m4(Vec3::new(0.0, 0.36, 0.2)), Vec3::new(0.19, 0.1, 0.008), 0.004, 1, &WHITE_PAINT);
    m.rbox(m4(Vec3::new(0.0, 0.36, 0.212)), Vec3::new(0.04, 0.008, 0.008), 0.004, 1, &METAL);
    for (x, z) in [(-0.18, -0.16), (-0.18, 0.16), (0.18, -0.16), (0.18, 0.16)] {
        m.cylinder(m4(Vec3::new(x, 0.0, z)), 0.012, 0.04, 8, &WOOD_DARK);
    }
    m.floor_ao(0.0, 0.2, 0.4);
    m
}

pub fn desk() -> MeshData {
    // 1.4 x 0.62, top at 0.75, faces +Z (user side)
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.735, 0.0)), Vec3::new(0.7, 0.018, 0.31), 0.006, 2, &WOOD_LIGHT);
    for x in [-0.66f32, 0.66] {
        m.rbox(m4(Vec3::new(x, 0.36, 0.0)), Vec3::new(0.018, 0.36, 0.28), 0.005, 1, &Mat::new(0xe8e5df, 0.5));
    }
    // drawer unit
    m.rbox(m4(Vec3::new(0.46, 0.36, 0.02)), Vec3::new(0.2, 0.34, 0.27), 0.01, 2, &Mat::new(0xe8e5df, 0.5));
    for i in 0..3 {
        let y = 0.14 + i as f32 * 0.22;
        m.rbox(m4(Vec3::new(0.46, y, 0.292)), Vec3::new(0.18, 0.095, 0.006), 0.004, 1, &WOOD_LIGHT);
        m.rbox(m4(Vec3::new(0.46, y + 0.05, 0.302)), Vec3::new(0.05, 0.006, 0.006), 0.003, 1, &METAL);
    }
    m.rbox(m4(Vec3::new(0.0, 0.45, -0.29)), Vec3::new(0.66, 0.2, 0.01), 0.005, 1, &Mat::new(0xe8e5df, 0.5));
    m.floor_ao(0.0, 0.3, 0.4);
    m
}

pub fn office_chair(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let fabric = Mat::new(color, 0.85).kind(kind::FABRIC);
    // star base
    for i in 0..5 {
        let a = i as f32 / 5.0 * std::f32::consts::TAU;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        m.capsule(Vec3::new(0.0, 0.08, 0.0), d * 0.28 + Vec3::Y * 0.07, 0.016, 8, &PLASTIC_DARK);
        m.sphere(m4(d * 0.28 + Vec3::Y * 0.03), 0.028, 8, 6, &PLASTIC_DARK);
    }
    m.cylinder(m4(Vec3::new(0.0, 0.07, 0.0)), 0.025, 0.36, 10, &METAL);
    // seat
    m.rbox(m4(Vec3::new(0.0, 0.47, 0.02)), Vec3::new(0.24, 0.045, 0.24), 0.04, 3, &fabric);
    // backrest
    m.rbox(m4r(Vec3::new(0.0, 0.8, -0.22), Quat::from_rotation_x(-0.12)), Vec3::new(0.22, 0.25, 0.035), 0.04, 3, &fabric);
    m.capsule(Vec3::new(0.0, 0.45, -0.2), Vec3::new(0.0, 0.62, -0.25), 0.018, 8, &PLASTIC_DARK);
    m.floor_ao(0.0, 0.15, 0.3);
    m
}

pub fn laptop_base() -> MeshData {
    let mut m = MeshData::new();
    let alu = Mat::new(0xc4c7cc, 0.28).metal(1.0).kind(kind::BRUSHED);
    m.rbox(m4(Vec3::new(0.0, 0.008, 0.0)), Vec3::new(0.165, 0.008, 0.115), 0.006, 2, &alu);
    m.rbox(m4(Vec3::new(0.0, 0.0165, -0.02)), Vec3::new(0.14, 0.0015, 0.06), 0.001, 1, &Mat::new(0x1e1f22, 0.6));
    m.rbox(m4(Vec3::new(0.0, 0.0165, 0.07)), Vec3::new(0.05, 0.0012, 0.032), 0.002, 1, &Mat::new(0xaeb2b8, 0.2).metal(1.0));
    // lid (hinged at back, tilted)
    let lid = m4(Vec3::new(0.0, 0.015, -0.112)) * Mat4::from_rotation_x(-0.25);
    m.rbox(lid * m4(Vec3::new(0.0, 0.11, -0.004)), Vec3::new(0.165, 0.11, 0.004), 0.005, 2, &alu);
    m.rbox(lid * m4(Vec3::new(0.0, 0.11, 0.001)), Vec3::new(0.158, 0.104, 0.0012), 0.001, 1, &Mat::new(0x0b0c0f, 0.2));
    m
}

/// Screen quad for the laptop (separate for emissive toggling). Local xy spans the screen.
pub fn laptop_screen() -> MeshData {
    let mut m = MeshData::new();
    let lid = m4(Vec3::new(0.0, 0.015, -0.112)) * Mat4::from_rotation_x(-0.25);
    let xf = lid * m4(Vec3::new(0.0, 0.11, 0.0027));
    let hw = 0.148;
    let hh = 0.094;
    let mat = Mat::new(0xffffff, 0.1).kind(kind::SCREEN).emit(0.6);
    let p = [
        xf.transform_point3(Vec3::new(-hw, -hh, 0.0)),
        xf.transform_point3(Vec3::new(hw, -hh, 0.0)),
        xf.transform_point3(Vec3::new(hw, hh, 0.0)),
        xf.transform_point3(Vec3::new(-hw, hh, 0.0)),
    ];
    m.quad(p, &mat);
    m
}

pub fn desk_lamp(shade: u32) -> MeshData {
    let mut m = MeshData::new();
    let metal = Mat::new(shade, 0.35).metal(0.6);
    m.rcylinder(m4(Vec3::ZERO), 0.075, 0.02, 0.006, 16, &metal);
    m.capsule(Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.03, 0.26, -0.02), 0.009, 8, &metal);
    m.capsule(Vec3::new(0.03, 0.26, -0.02), Vec3::new(0.08, 0.38, 0.1), 0.009, 8, &metal);
    m.sphere(m4(Vec3::new(0.03, 0.26, -0.02)), 0.015, 8, 6, &metal);
    m
}

/// Lamp head (shade) separately so it can glow.
pub fn lamp_shade_cone(r0: f32, r1: f32, h: f32, color: u32) -> MeshData {
    let mut m = MeshData::new();
    let mat = Mat::new(color, 0.8).kind(kind::SHADE);
    m.lathe(Mat4::IDENTITY, &[(r0, 0.0), (r1, h), (r1 - 0.004, h), (r0 - 0.004, 0.0)], 20, &mat);
    m
}

pub fn wardrobe(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let body = Mat::new(color, 0.5);
    m.rbox(m4(Vec3::new(0.0, 1.05, 0.0)), Vec3::new(0.6, 1.05, 0.3), 0.01, 2, &body);
    for x in [-0.3f32, 0.3] {
        m.rbox(m4(Vec3::new(x, 1.07, 0.302)), Vec3::new(0.29, 0.98, 0.008), 0.004, 1, &body);
        m.rbox(m4(Vec3::new(x * 0.12, 1.1, 0.316)), Vec3::new(0.008, 0.12, 0.008), 0.004, 1, &METAL);
    }
    m.rbox(m4(Vec3::new(0.0, 0.035, 0.0)), Vec3::new(0.58, 0.035, 0.28), 0.005, 1, &WOOD_DARK);
    m.floor_ao(0.0, 0.2, 0.4);
    m
}

pub fn shelf(len: f32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::ZERO), Vec3::new(len * 0.5, 0.015, 0.11), 0.004, 1, &WOOD);
    for x in [-len * 0.4, len * 0.4] {
        m.rbox(m4(Vec3::new(x, -0.05, -0.09)), Vec3::new(0.01, 0.05, 0.02), 0.003, 1, &METAL_DARK);
    }
    m
}

/// Row of books standing on a surface, local x along the row.
pub fn books(len: f32, seed: u64) -> MeshData {
    let mut m = MeshData::new();
    let mut r = Rng::new(seed);
    let mut x = -len * 0.5;
    let pal = [0x7a2e2e, 0x2e4a7a, 0xd9b44a, 0x2e6a4e, 0xe6e0d4, 0x4a3a6a, 0xc9724a, 0x333338];
    while x < len * 0.5 - 0.02 {
        let w = r.range(0.018, 0.04);
        let h = r.range(0.17, 0.26);
        let d = r.range(0.13, 0.17);
        let c = *r.pick(&pal);
        let tilt = if r.chance(0.08) { 0.12 } else { 0.0 };
        m.rbox(
            m4r(Vec3::new(x + w * 0.5, h * 0.5, 0.0), Quat::from_rotation_z(tilt)),
            Vec3::new(w * 0.5, h * 0.5, d * 0.5),
            0.003,
            1,
            &Mat::new(c, 0.7).kind(kind::FABRIC),
        );
        // spine band
        if r.chance(0.6) {
            m.rbox(
                m4(Vec3::new(x + w * 0.5, h * r.range(0.6, 0.8), d * 0.5 + 0.001)),
                Vec3::new(w * 0.45, 0.008, 0.001),
                0.0,
                1,
                &Mat::new(0xe8d9a8, 0.4).metal(0.3),
            );
        }
        x += w + 0.002;
    }
    m
}

pub fn plant(seed: u64, pot_color: u32, size: f32) -> MeshData {
    let mut m = MeshData::new();
    let mut r = Rng::new(seed);
    let pot = Mat::new(pot_color, 0.45);
    m.lathe(
        m4s(Vec3::ZERO, Vec3::splat(size)),
        &[(0.0, 0.0), (0.075, 0.0), (0.095, 0.16), (0.1, 0.17), (0.092, 0.17), (0.0, 0.15)],
        18,
        &pot,
    );
    m.disc(m4(Vec3::new(0.0, 0.155 * size, 0.0)), 0.088 * size, 14, &Mat::new(0x3a2a1e, 1.0));
    let leaf = Mat::new(0x3f7a3a, 0.6).kind(kind::FOLIAGE);
    let n = 9 + r.int(6);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU + r.range(-0.3, 0.3);
        let tilt = r.range(0.35, 0.9);
        let len = r.range(0.16, 0.26) * size;
        let base = Vec3::new(0.0, 0.16 * size, 0.0);
        let dir = Vec3::new(a.cos() * tilt.sin(), tilt.cos(), a.sin() * tilt.sin());
        let tip = base + dir * len;
        m.capsule(base, base + dir * len * 0.5, 0.004 * size, 5, &Mat::new(0x4a6a2a, 0.7));
        let q = Quat::from_rotation_arc(Vec3::Y, dir) * Quat::from_rotation_y(r.range(0.0, 3.0));
        let mut l = leaf;
        l.color = crate::math::scale_rgb(leaf.color, r.range(0.8, 1.2));
        m.sphere(Mat4::from_scale_rotation_translation(Vec3::new(0.045, 0.1, 0.012) * size, q, (base + tip) * 0.5 + dir * len * 0.2), 1.0, 10, 6, &l);
    }
    m
}

pub fn rug(w: f32, d: f32, color: u32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.006, 0.0)), Vec3::new(w * 0.5, 0.006, d * 0.5), 0.005, 1, &Mat::new(color, 1.0).kind(kind::CARPET));
    m
}

/// Window frame with glass and sill. Local: centred, in the XY plane, facing +Z (into the room).
pub fn window_frame(w: f32, h: f32, depth: f32) -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    let f = WHITE_PAINT;
    let t = 0.05;
    m.rbox(m4(Vec3::new(0.0, h * 0.5 - t * 0.5, 0.0)), Vec3::new(w * 0.5, t * 0.5, depth * 0.5), 0.005, 1, &f);
    m.rbox(m4(Vec3::new(0.0, -h * 0.5 + t * 0.5, 0.0)), Vec3::new(w * 0.5, t * 0.5, depth * 0.5), 0.005, 1, &f);
    m.rbox(m4(Vec3::new(w * 0.5 - t * 0.5, 0.0, 0.0)), Vec3::new(t * 0.5, h * 0.5, depth * 0.5), 0.005, 1, &f);
    m.rbox(m4(Vec3::new(-w * 0.5 + t * 0.5, 0.0, 0.0)), Vec3::new(t * 0.5, h * 0.5, depth * 0.5), 0.005, 1, &f);
    m.rbox(m4(Vec3::new(0.0, 0.0, 0.0)), Vec3::new(0.02, h * 0.5, 0.025), 0.004, 1, &f);
    m.rbox(m4(Vec3::new(0.0, h * 0.12, 0.0)), Vec3::new(w * 0.5, 0.018, 0.022), 0.004, 1, &f);
    // sill
    m.rbox(m4(Vec3::new(0.0, -h * 0.5 - 0.02, depth * 0.5 + 0.05)), Vec3::new(w * 0.5 + 0.08, 0.02, 0.1), 0.006, 1, &f);
    let mut glass = MeshData::new();
    glass.cube(m4(Vec3::ZERO), Vec3::new(w * 0.5 - t, h * 0.5 - t, 0.004), &Mat::new(0xdfeaf2, 0.05).kind(kind::GLASS));
    (m, glass)
}

/// Curtain panel hanging from `top` downwards, wavy along X.
pub fn curtain(w: f32, h: f32, color: u32, gather: f32) -> MeshData {
    let mut m = MeshData::new();
    let mat = Mat::new(color, 0.9).kind(kind::FABRIC);
    let nx = 24;
    let ny = 8;
    let base = m.verts.len() as u32;
    for j in 0..=ny {
        for i in 0..=nx {
            let u = i as f32 / nx as f32;
            let v = j as f32 / ny as f32;
            let x = (u - 0.5) * w * (1.0 - gather * v * 0.3);
            let wave = (u * std::f32::consts::TAU * 5.0).sin() * 0.03;
            let y = -v * h;
            let z = wave + 0.02;
            let dwave = (u * std::f32::consts::TAU * 5.0).cos();
            let n = Vec3::new(-dwave * 0.9, 0.0, 1.0).normalize();
            m.vert(Vec3::new(x, y, z), n, &mat, 0.85 + 0.15 * (1.0 - dwave.abs()));
        }
    }
    let row = nx + 1;
    for j in 0..ny {
        for i in 0..nx {
            let a = base + j * row + i;
            let b = a + 1;
            let c = a + row;
            let d = c + 1;
            m.indices.extend_from_slice(&[a, c, d, a, d, b]);
            // back face
            m.indices.extend_from_slice(&[a, d, c, a, b, d]);
        }
    }
    m
}

pub fn fairy_lights(len: f32, sag: f32, bulbs: usize) -> (MeshData, MeshData) {
    let mut wire = MeshData::new();
    let mut pts = Vec::new();
    for i in 0..=40 {
        let t = i as f32 / 40.0;
        let x = (t - 0.5) * len;
        let y = -sag * (1.0 - (2.0 * t - 1.0).powi(2)) - 0.03 * (t * 30.0).sin().abs();
        pts.push(Vec3::new(x, y, 0.0));
    }
    let radii = vec![0.0018; pts.len()];
    wire.tube(&pts, &radii, 4, &Mat::new(0x303030, 0.5), false);
    let mut b = MeshData::new();
    let bulb = Mat::new(0xffd89a, 0.3).emit(1.0);
    for i in 0..bulbs {
        let t = (i as f32 + 0.5) / bulbs as f32;
        let idx = (t * 40.0) as usize;
        b.sphere(m4(pts[idx] + Vec3::new(0.0, -0.018, 0.0)), 0.011, 8, 6, &bulb);
    }
    (wire, b)
}

pub fn pendant_lamp(color: u32) -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    m.cylinder(m4(Vec3::new(0.0, -0.6, 0.0)), 0.004, 0.6, 6, &PLASTIC_DARK);
    m.rcylinder(m4(Vec3::new(0.0, -0.03, 0.0)), 0.05, 0.03, 0.01, 12, &PLASTIC_DARK);
    let shade = {
        let mut s = MeshData::new();
        let mat = Mat::new(color, 0.8).kind(kind::SHADE);
        s.lathe(
            m4(Vec3::new(0.0, -0.82, 0.0)),
            &[(0.2, 0.0), (0.16, 0.12), (0.06, 0.22), (0.02, 0.23), (0.0, 0.23)],
            24,
            &mat,
        );
        s.lathe(
            m4(Vec3::new(0.0, -0.82, 0.0)),
            &[(0.0, 0.22), (0.058, 0.215), (0.157, 0.115), (0.197, 0.0)],
            24,
            &Mat::new(0xfff4e0, 0.9).kind(kind::SHADE),
        );
        s
    };
    (m, shade)
}

pub fn door(color: u32) -> MeshData {
    // 0.9 x 2.05 door in XY plane, facing +Z, hinge side at -X
    let mut m = MeshData::new();
    let frame = WHITE_PAINT;
    m.rbox(m4(Vec3::new(-0.49, 1.05, 0.0)), Vec3::new(0.04, 1.05, 0.07), 0.006, 1, &frame);
    m.rbox(m4(Vec3::new(0.49, 1.05, 0.0)), Vec3::new(0.04, 1.05, 0.07), 0.006, 1, &frame);
    m.rbox(m4(Vec3::new(0.0, 2.1, 0.0)), Vec3::new(0.53, 0.04, 0.07), 0.006, 1, &frame);
    let panel = Mat::new(color, 0.5);
    m.rbox(m4(Vec3::new(0.0, 1.03, 0.0)), Vec3::new(0.45, 1.02, 0.02), 0.006, 2, &panel);
    for y in [0.55f32, 1.45] {
        m.rbox(m4(Vec3::new(0.0, y, 0.021)), Vec3::new(0.34, 0.33, 0.004), 0.006, 2, &panel);
    }
    m.rbox(m4(Vec3::new(0.36, 1.0, 0.035)), Vec3::new(0.05, 0.008, 0.008), 0.004, 1, &METAL);
    m.rbox(m4(Vec3::new(0.36, 1.0, -0.035)), Vec3::new(0.05, 0.008, 0.008), 0.004, 1, &METAL);
    m
}

pub fn corkboard(w: f32, h: f32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::ZERO), Vec3::new(w * 0.5, h * 0.5, 0.012), 0.004, 1, &WOOD_LIGHT);
    m.rbox(m4(Vec3::new(0.0, 0.0, 0.01)), Vec3::new(w * 0.5 - 0.025, h * 0.5 - 0.025, 0.006), 0.002, 1, &Mat::new(0xb88a5a, 1.0).kind(kind::CARPET));
    m
}

/// Paper note with a pin (for the corkboard).
pub fn note(color: u32, w: f32, h: f32, pin: u32) -> MeshData {
    let mut m = MeshData::new();
    m.cube(m4(Vec3::ZERO), Vec3::new(w * 0.5, h * 0.5, 0.0015), &Mat::new(color, 0.85));
    m.sphere(m4(Vec3::new(0.0, h * 0.38, 0.008)), 0.006, 8, 6, &Mat::new(pin, 0.3));
    m
}

pub fn piggy_bank() -> MeshData {
    let mut m = MeshData::new();
    let pink = Mat::new(0xf2a0b0, 0.18);
    m.sphere(m4s(Vec3::new(0.0, 0.07, 0.0), Vec3::new(0.08, 0.062, 0.06)), 1.0, 20, 14, &pink);
    m.rcylinder(m4r(Vec3::new(0.078, 0.075, 0.0), Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)), 0.022, 0.02, 0.006, 12, &pink);
    m.disc(m4r(Vec3::new(0.0985, 0.075, 0.0), Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)), 0.018, 12, &Mat::new(0xe07a8f, 0.3));
    for z in [-0.028f32, 0.028] {
        m.cone(m4r(Vec3::new(0.045, 0.118, z), Quat::from_rotation_z(-0.3)), 0.016, 0.004, 0.03, 8, &pink);
        m.sphere(m4(Vec3::new(0.07, 0.09, z * 0.8)), 0.006, 8, 6, &Mat::new(0x1a1414, 0.2));
    }
    for (x, z) in [(-0.04, -0.03), (-0.04, 0.03), (0.04, -0.03), (0.04, 0.03)] {
        m.cylinder(m4(Vec3::new(x, 0.0, z)), 0.012, 0.03, 8, &pink);
    }
    m.rbox(m4(Vec3::new(-0.01, 0.131, 0.0)), Vec3::new(0.018, 0.003, 0.004), 0.002, 1, &Mat::new(0x301820, 0.5));
    m.torus(m4r(Vec3::new(-0.085, 0.09, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.012, 0.003, 10, 5, &pink);
    m
}

pub fn alarm_clock() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.04, 0.0)), Vec3::new(0.06, 0.04, 0.03), 0.015, 2, &Mat::new(0xf2e8d8, 0.4));
    m.rbox(m4(Vec3::new(0.0, 0.043, 0.0305)), Vec3::new(0.045, 0.022, 0.001), 0.003, 1, &Mat::new(0x10141a, 0.1).emit(0.0));
    m
}

pub fn mug(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let c = Mat::new(color, 0.25);
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.038, 0.0), (0.04, 0.095), (0.036, 0.095), (0.034, 0.02), (0.0, 0.02)], 16, &c);
    m.torus(m4r(Vec3::new(0.045, 0.05, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.022, 0.006, 10, 6, &c);
    m
}

pub fn headphones(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let c = Mat::new(color, 0.35);
    let mut pts = Vec::new();
    for i in 0..=16 {
        let a = std::f32::consts::PI * i as f32 / 16.0;
        pts.push(Vec3::new(a.cos() * 0.085, a.sin() * 0.09 + 0.05, 0.0));
    }
    let radii = vec![0.011; pts.len()];
    m.tube(&pts, &radii, 8, &c, false);
    for x in [-0.085f32, 0.085] {
        m.rcylinder(m4r(Vec3::new(x, 0.035, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.042, 0.035, 0.012, 16, &c);
        m.rcylinder(m4r(Vec3::new(x * 0.82, 0.035, 0.0), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.036, 0.012, 0.01, 16, &Mat::new(0x222226, 0.8).kind(kind::LEATHER));
    }
    m
}

pub fn shoebox(color: u32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.06, 0.0)), Vec3::new(0.18, 0.06, 0.11), 0.004, 1, &Mat::new(color, 0.7));
    m.rbox(m4(Vec3::new(0.0, 0.125, 0.0)), Vec3::new(0.185, 0.012, 0.115), 0.004, 1, &Mat::new(0xf2f0ea, 0.7));
    m
}

pub fn guitar() -> MeshData {
    let mut m = MeshData::new();
    let body = Mat::new(0xc47a3a, 0.3).kind(kind::WOOD);
    // body is built lying in XY plane (upright), front +Z
    m.sphere(m4s(Vec3::new(0.0, 0.22, 0.0), Vec3::new(0.19, 0.19, 0.05)), 1.0, 20, 10, &body);
    m.sphere(m4s(Vec3::new(0.0, 0.46, 0.0), Vec3::new(0.145, 0.14, 0.05)), 1.0, 20, 10, &body);
    m.disc(m4r(Vec3::new(0.0, 0.33, 0.051), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.045, 16, &Mat::new(0x1a1410, 0.6));
    m.rbox(m4(Vec3::new(0.0, 0.8, 0.02)), Vec3::new(0.025, 0.3, 0.012), 0.006, 1, &WOOD_DARK);
    m.rbox(m4(Vec3::new(0.0, 1.14, 0.015)), Vec3::new(0.04, 0.06, 0.012), 0.008, 1, &WOOD_DARK);
    m.rbox(m4(Vec3::new(0.0, 0.2, 0.052)), Vec3::new(0.06, 0.008, 0.006), 0.003, 1, &WOOD_DARK);
    m
}

pub fn skateboard() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.1, 0.0)), Vec3::new(0.1, 0.008, 0.4), 0.008, 2, &Mat::new(0x2a8a7a, 0.5));
    for z in [-0.28f32, 0.28] {
        m.rbox(m4(Vec3::new(0.0, 0.075, z)), Vec3::new(0.08, 0.012, 0.02), 0.005, 1, &METAL);
        for x in [-0.08f32, 0.08] {
            m.rcylinder(m4r(Vec3::new(x, 0.03, z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.028, 0.03, 0.01, 12, &Mat::new(0xf2d64a, 0.4));
        }
    }
    m
}

pub fn bicycle(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let frame = Mat::new(color, 0.3).metal(0.3);
    let tire = Mat::new(0x1a1a1c, 0.8);
    let wheel = |m: &mut MeshData, z: f32| {
        m.torus(m4r(Vec3::new(0.0, 0.34, z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.32, 0.02, 32, 8, &tire);
        m.torus(m4r(Vec3::new(0.0, 0.34, z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.29, 0.008, 32, 6, &METAL);
        for i in 0..12 {
            let a = i as f32 / 12.0 * std::f32::consts::TAU;
            m.capsule(Vec3::new(0.0, 0.34, z), Vec3::new(0.0, 0.34 + a.sin() * 0.29, z + a.cos() * 0.29), 0.0015, 3, &METAL);
        }
    };
    wheel(&mut m, -0.52);
    wheel(&mut m, 0.52);
    let bb = Vec3::new(0.0, 0.32, -0.02);
    let seat = Vec3::new(0.0, 0.8, -0.2);
    let head = Vec3::new(0.0, 0.78, 0.4);
    m.capsule(bb, seat, 0.017, 8, &frame);
    m.capsule(bb, head, 0.019, 8, &frame);
    m.capsule(seat + Vec3::new(0.0, -0.08, 0.0), head, 0.016, 8, &frame);
    m.capsule(bb, Vec3::new(0.0, 0.34, -0.52), 0.012, 8, &frame);
    m.capsule(seat + Vec3::new(0.0, -0.08, 0.0), Vec3::new(0.0, 0.34, -0.52), 0.011, 8, &frame);
    m.capsule(head, Vec3::new(0.0, 0.34, 0.52), 0.014, 8, &frame);
    m.capsule(head, head + Vec3::new(0.0, 0.14, -0.04), 0.014, 8, &frame);
    m.capsule(head + Vec3::new(-0.24, 0.15, -0.06), head + Vec3::new(0.24, 0.15, -0.06), 0.012, 8, &PLASTIC_DARK);
    m.rbox(m4(seat + Vec3::new(0.0, 0.03, 0.0)), Vec3::new(0.06, 0.02, 0.12), 0.02, 2, &Mat::new(0x1a1a1c, 0.6).kind(kind::LEATHER));
    m
}

pub fn beanbag(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let s = m.verts.len();
    m.sphere(m4s(Vec3::new(0.0, 0.25, 0.0), Vec3::new(0.42, 0.28, 0.42)), 1.0, 20, 12, &Mat::new(color, 0.9).kind(kind::FABRIC));
    for v in &mut m.verts[s..] {
        if v.pos[1] < 0.02 {
            v.pos[1] = 0.02;
        }
        let p = Vec3::from(v.pos);
        let d = noise3(p * 5.0) * 0.02 - (p.y - 0.4).max(0.0) * 0.4 * (p.x * 0.5 + 0.5).max(0.0);
        v.pos = (p + Vec3::from(v.nrm) * d).to_array();
    }
    m.floor_ao(0.0, 0.2, 0.5);
    m
}

// ---------------------------------------------------------------- living room & kitchen

pub fn sofa(color: u32) -> MeshData {
    // 2.0 wide along X, facing +Z
    let mut m = MeshData::new();
    let fab = Mat::new(color, 0.9).kind(kind::FABRIC);
    m.rbox(m4(Vec3::new(0.0, 0.25, 0.0)), Vec3::new(1.0, 0.13, 0.45), 0.05, 3, &fab);
    m.rbox(m4(Vec3::new(0.0, 0.58, -0.36)), Vec3::new(1.0, 0.3, 0.1), 0.08, 3, &fab);
    for x in [-0.93f32, 0.93] {
        m.rbox(m4(Vec3::new(x, 0.45, 0.02)), Vec3::new(0.08, 0.2, 0.44), 0.07, 3, &fab);
    }
    for x in [-0.44f32, 0.44] {
        let s = m.verts.len();
        m.rbox(m4(Vec3::new(x, 0.44, 0.04)), Vec3::new(0.43, 0.07, 0.38), 0.07, 4, &fab);
        rumple(&mut m, s, 0.01, 5.0, x);
        let s = m.verts.len();
        m.rbox(m4r(Vec3::new(x, 0.7, -0.24), Quat::from_rotation_x(-0.15)), Vec3::new(0.42, 0.2, 0.07), 0.07, 4, &fab);
        rumple(&mut m, s, 0.012, 4.0, x + 3.0);
    }
    // throw pillow
    m.rbox(m4r(Vec3::new(0.7, 0.62, -0.1), Quat::from_rotation_z(0.3) * Quat::from_rotation_x(-0.3)), Vec3::new(0.18, 0.18, 0.06), 0.06, 3, &Mat::new(0xe0b04a, 0.9).kind(kind::KNIT));
    for (x, z) in [(-0.9, -0.38), (-0.9, 0.38), (0.9, -0.38), (0.9, 0.38)] {
        m.cone(m4(Vec3::new(x, 0.0, z)), 0.02, 0.025, 0.12, 8, &WOOD_DARK);
    }
    m.floor_ao(0.0, 0.25, 0.5);
    m
}

pub fn coffee_table() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.4, 0.0)), Vec3::new(0.55, 0.025, 0.3), 0.02, 2, &WOOD);
    m.rbox(m4(Vec3::new(0.0, 0.12, 0.0)), Vec3::new(0.5, 0.012, 0.26), 0.01, 1, &WOOD);
    for (x, z) in [(-0.5, -0.26), (-0.5, 0.26), (0.5, -0.26), (0.5, 0.26)] {
        m.rbox(m4(Vec3::new(x, 0.2, z)), Vec3::new(0.02, 0.2, 0.02), 0.006, 1, &WOOD_DARK);
    }
    m.floor_ao(0.0, 0.2, 0.3);
    m
}

pub fn tv(width: f32) -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    let h = width * 0.5625;
    m.rbox(m4(Vec3::ZERO), Vec3::new(width * 0.5, h * 0.5, 0.025), 0.01, 2, &PLASTIC_DARK);
    let mut s = MeshData::new();
    s.quad(
        [
            Vec3::new(-width * 0.48, -h * 0.47, 0.026),
            Vec3::new(width * 0.48, -h * 0.47, 0.026),
            Vec3::new(width * 0.48, h * 0.47, 0.026),
            Vec3::new(-width * 0.48, h * 0.47, 0.026),
        ],
        &Mat::new(0xffffff, 0.1).kind(kind::SCREEN).emit(0.45),
    );
    (m, s)
}

pub fn tv_stand() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.25, 0.0)), Vec3::new(0.8, 0.25, 0.2), 0.01, 2, &WOOD_DARK);
    for x in [-0.4f32, 0.4] {
        m.rbox(m4(Vec3::new(x, 0.25, 0.201)), Vec3::new(0.38, 0.22, 0.005), 0.004, 1, &WOOD);
    }
    m.floor_ao(0.0, 0.2, 0.4);
    m
}

pub fn dining_table() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.75, 0.0)), Vec3::new(0.8, 0.025, 0.45), 0.012, 2, &WOOD);
    for (x, z) in [(-0.72, -0.38), (-0.72, 0.38), (0.72, -0.38), (0.72, 0.38)] {
        m.rbox(m4(Vec3::new(x, 0.37, z)), Vec3::new(0.03, 0.37, 0.03), 0.008, 1, &WOOD_DARK);
    }
    m.floor_ao(0.0, 0.3, 0.4);
    m
}

pub fn dining_chair() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.45, 0.0)), Vec3::new(0.21, 0.02, 0.21), 0.01, 2, &WOOD);
    for (x, z) in [(-0.18, -0.18), (-0.18, 0.18), (0.18, -0.18), (0.18, 0.18)] {
        m.rbox(m4(Vec3::new(x, 0.225, z)), Vec3::new(0.018, 0.225, 0.018), 0.006, 1, &WOOD_DARK);
    }
    for x in [-0.18f32, 0.18] {
        m.rbox(m4(Vec3::new(x, 0.7, -0.19)), Vec3::new(0.018, 0.25, 0.018), 0.006, 1, &WOOD_DARK);
    }
    m.rbox(m4(Vec3::new(0.0, 0.85, -0.19)), Vec3::new(0.2, 0.08, 0.015), 0.01, 2, &WOOD);
    m.floor_ao(0.0, 0.2, 0.3);
    m
}

pub fn kitchen_counter(len: f32) -> MeshData {
    // along X, back at -Z, depth 0.62
    let mut m = MeshData::new();
    let cab = Mat::new(0x5e7a6e, 0.45);
    m.rbox(m4(Vec3::new(0.0, 0.43, 0.0)), Vec3::new(len * 0.5, 0.39, 0.29), 0.008, 1, &cab);
    m.rbox(m4(Vec3::new(0.0, 0.05, 0.02)), Vec3::new(len * 0.5, 0.05, 0.27), 0.004, 1, &Mat::new(0x2a2b2e, 0.6));
    m.rbox(m4(Vec3::new(0.0, 0.89, 0.01)), Vec3::new(len * 0.5 + 0.01, 0.02, 0.31), 0.004, 1, &Mat::new(0xe8e4dc, 0.15).kind(kind::CONCRETE));
    let n = (len / 0.6).round().max(1.0) as i32;
    for i in 0..n {
        let x = -len * 0.5 + (i as f32 + 0.5) * len / n as f32;
        m.rbox(m4(Vec3::new(x, 0.47, 0.292)), Vec3::new(len / n as f32 * 0.5 - 0.008, 0.34, 0.008), 0.004, 1, &cab);
        m.rbox(m4(Vec3::new(x, 0.74, 0.305)), Vec3::new(0.07, 0.006, 0.006), 0.003, 1, &METAL);
    }
    m.floor_ao(0.0, 0.2, 0.35);
    m
}

pub fn upper_cabinets(len: f32) -> MeshData {
    let mut m = MeshData::new();
    let cab = Mat::new(0xe9e6df, 0.45);
    m.rbox(m4(Vec3::ZERO), Vec3::new(len * 0.5, 0.35, 0.17), 0.006, 1, &cab);
    let n = (len / 0.6).round().max(1.0) as i32;
    for i in 0..n {
        let x = -len * 0.5 + (i as f32 + 0.5) * len / n as f32;
        m.rbox(m4(Vec3::new(x, 0.0, 0.172)), Vec3::new(len / n as f32 * 0.5 - 0.008, 0.33, 0.006), 0.004, 1, &cab);
        m.rbox(m4(Vec3::new(x, -0.25, 0.185)), Vec3::new(0.06, 0.006, 0.006), 0.003, 1, &METAL);
    }
    m
}

pub fn fridge() -> MeshData {
    let mut m = MeshData::new();
    let steel = Mat::new(0xd8dadc, 0.25).metal(0.8).kind(kind::BRUSHED);
    m.rbox(m4(Vec3::new(0.0, 0.92, 0.0)), Vec3::new(0.34, 0.92, 0.33), 0.03, 2, &steel);
    m.rbox(m4(Vec3::new(0.0, 1.25, 0.332)), Vec3::new(0.33, 0.005, 0.003), 0.0, 1, &PLASTIC_DARK);
    for y in [0.9f32, 1.5] {
        m.rbox(m4(Vec3::new(0.27, y, 0.35)), Vec3::new(0.01, 0.18, 0.015), 0.008, 1, &METAL);
    }
    // magnets / notes
    m.rbox(m4(Vec3::new(-0.1, 1.55, 0.334)), Vec3::new(0.06, 0.08, 0.002), 0.0, 1, &Mat::new(0xfff2a8, 0.8));
    m.rbox(m4(Vec3::new(0.06, 1.62, 0.334)), Vec3::new(0.05, 0.06, 0.002), 0.0, 1, &Mat::new(0xa8e0ff, 0.8));
    m.floor_ao(0.0, 0.2, 0.3);
    m
}

pub fn stove() -> MeshData {
    let mut m = MeshData::new();
    for (x, z) in [(-0.15, -0.1), (0.15, -0.1), (-0.15, 0.12), (0.15, 0.12)] {
        m.torus(m4(Vec3::new(x, 0.915, z)), 0.06, 0.008, 16, 6, &METAL_DARK);
    }
    m.rbox(m4(Vec3::new(0.0, 0.905, 0.0)), Vec3::new(0.3, 0.008, 0.28), 0.004, 1, &Mat::new(0x151518, 0.1));
    m
}

pub fn sink() -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.905, 0.0)), Vec3::new(0.25, 0.01, 0.2), 0.02, 2, &METAL);
    m.capsule(Vec3::new(0.0, 0.91, -0.2), Vec3::new(0.0, 1.15, -0.2), 0.012, 8, &METAL);
    m.capsule(Vec3::new(0.0, 1.15, -0.2), Vec3::new(0.0, 1.15, -0.05), 0.012, 8, &METAL);
    m
}

pub fn fruit_bowl() -> MeshData {
    let mut m = MeshData::new();
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.06, 0.0), (0.13, 0.07), (0.125, 0.075), (0.055, 0.008), (0.0, 0.008)], 20, &CERAMIC);
    let fr = [(0xe0402a, 0.04), (0xf0a020, 0.042), (0x8fbf3a, 0.038), (0xe0402a, 0.036)];
    for (i, (c, r)) in fr.iter().enumerate() {
        let a = i as f32 * 1.7;
        m.sphere(m4(Vec3::new(a.cos() * 0.05, 0.05 + r * 0.5, a.sin() * 0.05)), *r, 12, 8, &Mat::new(*c, 0.35));
    }
    m
}

pub fn picture_frame(w: f32, h: f32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::ZERO), Vec3::new(w * 0.5, h * 0.5, 0.012), 0.004, 1, &WOOD_DARK);
    m.rbox(m4(Vec3::new(0.0, 0.0, 0.009)), Vec3::new(w * 0.5 - 0.03, h * 0.5 - 0.03, 0.005), 0.0, 1, &Mat::new(0xf4f1ea, 0.8));
    m
}

pub fn poster_panel(w: f32, h: f32) -> MeshData {
    let mut m = MeshData::new();
    // local xy in [-0.5*w, 0.5*w]; poster pattern uses local coordinates
    let mat = Mat::new(0xffffff, 0.6).kind(kind::POSTER);
    m.quad(
        [
            Vec3::new(-w * 0.5, -h * 0.5, 0.0),
            Vec3::new(w * 0.5, -h * 0.5, 0.0),
            Vec3::new(w * 0.5, h * 0.5, 0.0),
            Vec3::new(-w * 0.5, h * 0.5, 0.0),
        ],
        &mat,
    );
    m
}

pub fn floor_lamp() -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    m.rcylinder(m4(Vec3::ZERO), 0.15, 0.025, 0.01, 18, &METAL_DARK);
    m.cylinder(m4(Vec3::new(0.0, 0.02, 0.0)), 0.012, 1.4, 8, &METAL_DARK);
    let shade = lamp_shade_cone(0.2, 0.15, 0.28, 0xf2e6cf);
    let mut s = MeshData::new();
    s.append_xf(&shade, m4(Vec3::new(0.0, 1.32, 0.0)));
    (m, s)
}

// ---------------------------------------------------------------- exterior

pub fn tree(seed: u64, height: f32) -> MeshData {
    let mut m = MeshData::new();
    let mut r = Rng::new(seed);
    let bark = Mat::new(0x5a4636, 0.9).kind(kind::WOOD);
    let trunk_h = height * 0.45;
    let mut pts = Vec::new();
    let mut radii = Vec::new();
    for i in 0..=6 {
        let t = i as f32 / 6.0;
        pts.push(Vec3::new((t * 3.0).sin() * 0.06, t * trunk_h, (t * 2.3).cos() * 0.04 - 0.04));
        radii.push(0.14 * (1.0 - t * 0.55) * height / 6.0 + 0.04);
    }
    m.tube(&pts, &radii, 10, &bark, false);
    // branches
    for i in 0..5 {
        let a = i as f32 * 1.3 + r.range(0.0, 0.5);
        let base = pts[4 + i % 2];
        let dir = Vec3::new(a.cos(), r.range(0.8, 1.4), a.sin()).normalize();
        m.capsule(base, base + dir * height * 0.18, 0.035 * height / 6.0 + 0.015, 6, &bark);
    }
    let leaf = Mat::new(0x4a7a34, 0.75).kind(kind::FOLIAGE);
    let n = 14;
    for i in 0..n {
        let a = r.range(0.0, std::f32::consts::TAU);
        let rr = r.range(0.2, 1.0);
        let y = trunk_h + r.range(0.1, height * 0.5);
        let rad = height * r.range(0.13, 0.2);
        let c = Vec3::new(a.cos() * rr * height * 0.22, y, a.sin() * rr * height * 0.22);
        let mut l = leaf;
        l.color = crate::math::scale_rgb(leaf.color, r.range(0.8, 1.15));
        let s = m.verts.len();
        m.sphere(m4s(c, Vec3::new(1.0, 0.8, 1.0) * rad), 1.0, 12, 8, &l);
        for v in &mut m.verts[s..] {
            let p = Vec3::from(v.pos);
            let d = fbm3(p * 1.8 + Vec3::splat(i as f32), 3) * rad * 0.35;
            v.pos = (p + Vec3::from(v.nrm) * d).to_array();
            // darker inside
            let ao = ((p.y - trunk_h) / (height * 0.7)).clamp(0.3, 1.0);
            v.color[3] = (ao * 255.0) as u8;
        }
    }
    m
}

pub fn bench() -> MeshData {
    let mut m = MeshData::new();
    for i in 0..4 {
        m.rbox(m4(Vec3::new(0.0, 0.45, -0.18 + i as f32 * 0.11)), Vec3::new(0.9, 0.018, 0.045), 0.008, 1, &WOOD);
    }
    for i in 0..3 {
        m.rbox(m4r(Vec3::new(0.0, 0.72 + i as f32 * 0.1, -0.26 - i as f32 * 0.02), Quat::from_rotation_x(-0.2)), Vec3::new(0.9, 0.035, 0.015), 0.008, 1, &WOOD);
    }
    for x in [-0.75f32, 0.75] {
        m.rbox(m4(Vec3::new(x, 0.22, 0.0)), Vec3::new(0.025, 0.22, 0.2), 0.01, 1, &METAL_DARK);
        m.rbox(m4r(Vec3::new(x, 0.7, -0.28), Quat::from_rotation_x(-0.2)), Vec3::new(0.025, 0.25, 0.02), 0.01, 1, &METAL_DARK);
    }
    m.floor_ao(0.0, 0.3, 0.35);
    m
}

pub fn lamppost() -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    let pole = Mat::new(0x2a2d32, 0.4).metal(0.7);
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.12, 0.0), (0.12, 0.08), (0.08, 0.12), (0.06, 0.5), (0.045, 3.6), (0.0, 3.6)], 12, &pole);
    m.capsule(Vec3::new(0.0, 3.55, 0.0), Vec3::new(0.0, 3.75, 0.25), 0.03, 8, &pole);
    m.lathe(m4(Vec3::new(0.0, 3.62, 0.35)), &[(0.02, 0.2), (0.16, 0.08), (0.18, 0.0), (0.0, 0.0)], 16, &pole);
    let mut glow = MeshData::new();
    glow.lathe(m4(Vec3::new(0.0, 3.6, 0.35)), &[(0.0, 0.0), (0.12, 0.0), (0.1, 0.03), (0.0, 0.04)], 16, &Mat::new(0xffe2b0, 0.3).emit(1.0));
    (m, glow)
}

pub fn planter(r: f32) -> MeshData {
    let mut m = MeshData::new();
    m.rcylinder(m4(Vec3::ZERO), r, 0.45, 0.04, 24, &Mat::new(0xb8b2a6, 0.8).kind(kind::CONCRETE));
    m.disc(m4(Vec3::new(0.0, 0.44, 0.0)), r - 0.04, 20, &Mat::new(0x3a2e22, 1.0).kind(kind::GRASS));
    m
}

pub fn fountain() -> (MeshData, MeshData) {
    let mut m = MeshData::new();
    let stone = Mat::new(0xc8c0b2, 0.7).kind(kind::CONCRETE);
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.05), (1.5, 0.05), (1.62, 0.0), (1.7, 0.0), (1.7, 0.5), (1.55, 0.55), (1.45, 0.5), (1.45, 0.2), (0.0, 0.2)], 48, &stone);
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.2), (0.25, 0.2), (0.18, 0.9), (0.55, 1.05), (0.6, 1.15), (0.5, 1.15), (0.12, 1.2), (0.08, 1.6), (0.0, 1.65)], 32, &stone);
    let mut water = MeshData::new();
    water.disc(m4(Vec3::new(0.0, 0.42, 0.0)), 1.46, 48, &Mat::new(0x4a7a8a, 0.05).kind(kind::WATER));
    water.disc(m4(Vec3::new(0.0, 1.1, 0.0)), 0.52, 24, &Mat::new(0x4a7a8a, 0.05).kind(kind::WATER));
    (m, water)
}

pub fn market_stall(awning: u32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::new(0.0, 0.85, 0.0)), Vec3::new(0.9, 0.03, 0.45), 0.01, 1, &WOOD);
    m.rbox(m4(Vec3::new(0.0, 0.45, 0.4)), Vec3::new(0.88, 0.38, 0.02), 0.01, 1, &Mat::new(0xefe6d6, 0.9).kind(kind::FABRIC));
    for (x, z) in [(-0.85, -0.4), (0.85, -0.4), (-0.85, 0.4), (0.85, 0.4)] {
        m.cylinder(m4(Vec3::new(x, 0.0, z)), 0.025, 2.2, 8, &WOOD_DARK);
    }
    // striped awning
    for i in 0..8 {
        let x0 = -1.0 + i as f32 * 0.25;
        let c = if i % 2 == 0 { awning } else { 0xf4efe6 };
        m.rbox(m4r(Vec3::new(x0 + 0.125, 2.2, 0.05), Quat::from_rotation_x(0.25)), Vec3::new(0.125, 0.012, 0.6), 0.004, 1, &Mat::new(c, 0.9).kind(kind::FABRIC));
        m.rbox(m4(Vec3::new(x0 + 0.125, 2.02, 0.63)), Vec3::new(0.125, 0.08, 0.008), 0.004, 1, &Mat::new(c, 0.9).kind(kind::FABRIC));
    }
    m.floor_ao(0.0, 0.3, 0.3);
    m
}

pub fn cafe_table() -> MeshData {
    let mut m = MeshData::new();
    m.rcylinder(m4(Vec3::new(0.0, 0.72, 0.0)), 0.35, 0.025, 0.008, 24, &Mat::new(0xefece6, 0.3));
    m.cylinder(m4(Vec3::new(0.0, 0.02, 0.0)), 0.03, 0.7, 8, &METAL_DARK);
    m.rcylinder(m4(Vec3::ZERO), 0.22, 0.03, 0.01, 16, &METAL_DARK);
    m
}

pub fn cafe_chair() -> MeshData {
    let mut m = MeshData::new();
    let met = Mat::new(0x2f4f45, 0.4).metal(0.5);
    m.rcylinder(m4(Vec3::new(0.0, 0.44, 0.0)), 0.2, 0.025, 0.008, 18, &met);
    for i in 0..4 {
        let a = i as f32 / 4.0 * std::f32::consts::TAU + 0.78;
        m.capsule(Vec3::new(a.cos() * 0.15, 0.44, a.sin() * 0.15), Vec3::new(a.cos() * 0.19, 0.0, a.sin() * 0.19), 0.01, 6, &met);
    }
    let mut pts = Vec::new();
    for i in 0..=10 {
        let a = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 10.0);
        pts.push(Vec3::new(a.cos() * 0.19, 0.8, -a.sin() * 0.19));
    }
    m.tube(&pts, &[0.012; 11], 6, &met, false);
    m.capsule(Vec3::new(0.15, 0.45, -0.1), Vec3::new(0.16, 0.8, -0.11), 0.01, 6, &met);
    m.capsule(Vec3::new(-0.15, 0.45, -0.1), Vec3::new(-0.16, 0.8, -0.11), 0.01, 6, &met);
    m
}

pub fn car(color: u32) -> MeshData {
    let mut m = MeshData::new();
    let paint = Mat::new(color, 0.25).metal(0.4);
    m.rbox(m4(Vec3::new(0.0, 0.55, 0.0)), Vec3::new(0.85, 0.28, 2.1), 0.2, 4, &paint);
    m.rbox(m4(Vec3::new(0.0, 1.0, -0.15)), Vec3::new(0.75, 0.25, 1.1), 0.22, 4, &paint);
    m.rbox(m4(Vec3::new(0.0, 1.02, -0.15)), Vec3::new(0.76, 0.2, 1.0), 0.18, 3, &Mat::new(0x1a2430, 0.05).metal(0.2));
    for (x, z) in [(-0.8, -1.35), (0.8, -1.35), (-0.8, 1.35), (0.8, 1.35)] {
        m.rcylinder(m4r(Vec3::new(x, 0.33, z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.33, 0.24, 0.08, 18, &Mat::new(0x151517, 0.8));
        m.rcylinder(m4r(Vec3::new(x * 1.01, 0.33, z), Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)), 0.2, 0.245, 0.03, 12, &METAL);
    }
    for x in [-0.55f32, 0.55] {
        m.rbox(m4(Vec3::new(x, 0.62, 2.1)), Vec3::new(0.16, 0.06, 0.02), 0.03, 1, &Mat::new(0xfff6e0, 0.1).emit(0.2));
        m.rbox(m4(Vec3::new(x, 0.66, -2.1)), Vec3::new(0.16, 0.05, 0.02), 0.03, 1, &Mat::new(0xc02020, 0.2).emit(0.15));
    }
    m.floor_ao(0.0, 0.4, 0.5);
    m
}

pub fn trash_bin() -> MeshData {
    let mut m = MeshData::new();
    m.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.2, 0.0), (0.23, 0.9), (0.24, 0.92), (0.0, 0.92)], 16, &Mat::new(0x3e5e4e, 0.5).metal(0.3));
    m
}

/// Simple coloured storefront sign icon plate.
pub fn sign_plate(w: f32, h: f32, color: u32) -> MeshData {
    let mut m = MeshData::new();
    m.rbox(m4(Vec3::ZERO), Vec3::new(w * 0.5, h * 0.5, 0.05), 0.03, 2, &Mat::new(color, 0.3).kind(kind::SIGN).emit(0.25));
    m
}

/// Icon on a sign: 0 = chip (tech), 1 = cup (café), 2 = columns (bank), 3 = house
pub fn sign_icon(icon: u32, size: f32) -> MeshData {
    let mut m = MeshData::new();
    let w = Mat::new(0xfff8ec, 0.3).kind(kind::SIGN).emit(0.35);
    let s = size;
    match icon {
        0 => {
            m.rbox(m4(Vec3::ZERO), Vec3::new(0.18, 0.18, 0.02) * s, 0.02 * s, 1, &w);
            for i in 0..3 {
                let o = (i as f32 - 1.0) * 0.1 * s;
                m.rbox(m4(Vec3::new(o, 0.25 * s, 0.0)), Vec3::new(0.02, 0.06, 0.02) * s, 0.0, 1, &w);
                m.rbox(m4(Vec3::new(o, -0.25 * s, 0.0)), Vec3::new(0.02, 0.06, 0.02) * s, 0.0, 1, &w);
                m.rbox(m4(Vec3::new(0.25 * s, o, 0.0)), Vec3::new(0.06, 0.02, 0.02) * s, 0.0, 1, &w);
                m.rbox(m4(Vec3::new(-0.25 * s, o, 0.0)), Vec3::new(0.06, 0.02, 0.02) * s, 0.0, 1, &w);
            }
        }
        1 => {
            m.lathe(m4r(Vec3::new(0.0, -0.18 * s, 0.0), Quat::IDENTITY), &[(0.0, 0.0), (0.14 * s, 0.0), (0.18 * s, 0.3 * s), (0.0, 0.3 * s)], 16, &w);
            m.torus(m4r(Vec3::new(0.2 * s, -0.03 * s, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.07 * s, 0.02 * s, 10, 6, &w);
            for i in 0..3 {
                m.capsule(Vec3::new((i as f32 - 1.0) * 0.07 * s, 0.17 * s, 0.0), Vec3::new((i as f32 - 1.0) * 0.07 * s + 0.02 * s, 0.3 * s, 0.0), 0.012 * s, 6, &w);
            }
        }
        2 => {
            m.rbox(m4(Vec3::new(0.0, 0.2 * s, 0.0)), Vec3::new(0.3, 0.03, 0.02) * s, 0.0, 1, &w);
            m.rbox(m4(Vec3::new(0.0, -0.22 * s, 0.0)), Vec3::new(0.3, 0.03, 0.02) * s, 0.0, 1, &w);
            for i in 0..4 {
                let x = (i as f32 - 1.5) * 0.15 * s;
                m.rbox(m4(Vec3::new(x, 0.0, 0.0)), Vec3::new(0.03, 0.18, 0.02) * s, 0.0, 1, &w);
            }
            m.extrude(m4(Vec3::new(0.0, 0.23 * s, -0.02 * s)), &[Vec2::new(-0.32 * s, 0.0), Vec2::new(0.32 * s, 0.0), Vec2::new(0.0, 0.15 * s)], 0.04 * s, &w);
        }
        _ => {
            m.rbox(m4(Vec3::new(0.0, -0.08 * s, 0.0)), Vec3::new(0.18, 0.14, 0.02) * s, 0.0, 1, &w);
            m.extrude(m4(Vec3::new(0.0, 0.06 * s, -0.02 * s)), &[Vec2::new(-0.25 * s, 0.0), Vec2::new(0.25 * s, 0.0), Vec2::new(0.0, 0.2 * s)], 0.04 * s, &w);
        }
    }
    m
}
