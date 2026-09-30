//! Facial expressions and face-part meshes (eyes, lids, brows, animated mouth).

use super::build::FaceLayout;
use crate::gfx::mesh::{kind, Mat, MeshData};
use crate::math::{damp, lerp};
use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Expr {
    Neutral,
    Happy,
    Joy,
    Sad,
    Worried,
    Surprised,
    Thinking,
    Annoyed,
    Smug,
    Tired,
    Excited,
    Focused,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FaceState {
    pub brow_raise: [f32; 2],
    /// + inner brows up (worry), - frown
    pub brow_inner: f32,
    pub lid_open: f32,
    pub lid_lower: f32,
    pub smile: f32,
    pub open: f32,
    pub wide: f32,
    pub asym: f32,
}

impl FaceState {
    pub fn neutral() -> Self {
        FaceState {
            lid_open: 1.0,
            ..Default::default()
        }
    }

    pub fn of(e: Expr) -> Self {
        let n = Self::neutral();
        match e {
            Expr::Neutral => FaceState { smile: 0.08, ..n },
            Expr::Happy => FaceState {
                smile: 0.75,
                lid_lower: 0.45,
                brow_raise: [0.15, 0.15],
                lid_open: 0.95,
                ..n
            },
            Expr::Joy => FaceState {
                smile: 1.0,
                open: 0.45,
                wide: 0.4,
                lid_lower: 0.6,
                brow_raise: [0.4, 0.4],
                lid_open: 0.9,
                ..n
            },
            Expr::Sad => FaceState {
                smile: -0.55,
                brow_inner: 0.8,
                brow_raise: [-0.1, -0.1],
                lid_open: 0.75,
                ..n
            },
            Expr::Worried => FaceState {
                smile: -0.25,
                brow_inner: 0.7,
                brow_raise: [0.2, 0.2],
                lid_open: 1.05,
                wide: -0.1,
                ..n
            },
            Expr::Surprised => FaceState {
                brow_raise: [0.9, 0.9],
                lid_open: 1.3,
                open: 0.5,
                wide: -0.35,
                ..n
            },
            Expr::Thinking => FaceState {
                brow_raise: [0.5, -0.15],
                brow_inner: -0.2,
                smile: -0.05,
                asym: 0.5,
                lid_open: 0.85,
                ..n
            },
            Expr::Annoyed => FaceState {
                brow_inner: -0.8,
                brow_raise: [-0.3, -0.3],
                smile: -0.35,
                lid_open: 0.8,
                ..n
            },
            Expr::Smug => FaceState {
                smile: 0.45,
                asym: 0.7,
                brow_raise: [0.25, -0.05],
                lid_open: 0.8,
                lid_lower: 0.3,
                ..n
            },
            Expr::Tired => FaceState {
                lid_open: 0.55,
                brow_inner: 0.3,
                smile: -0.1,
                ..n
            },
            Expr::Excited => FaceState {
                smile: 0.9,
                open: 0.3,
                wide: 0.3,
                brow_raise: [0.7, 0.7],
                lid_open: 1.15,
                ..n
            },
            Expr::Focused => FaceState {
                brow_inner: -0.35,
                lid_open: 0.85,
                smile: 0.0,
                ..n
            },
        }
    }

    pub fn lerp(&self, b: &FaceState, t: f32) -> FaceState {
        FaceState {
            brow_raise: [lerp(self.brow_raise[0], b.brow_raise[0], t), lerp(self.brow_raise[1], b.brow_raise[1], t)],
            brow_inner: lerp(self.brow_inner, b.brow_inner, t),
            lid_open: lerp(self.lid_open, b.lid_open, t),
            lid_lower: lerp(self.lid_lower, b.lid_lower, t),
            smile: lerp(self.smile, b.smile, t),
            open: lerp(self.open, b.open, t),
            wide: lerp(self.wide, b.wide, t),
            asym: lerp(self.asym, b.asym, t),
        }
    }

    pub fn damp_to(&mut self, b: &FaceState, rate: f32, dt: f32) {
        self.brow_raise[0] = damp(self.brow_raise[0], b.brow_raise[0], rate, dt);
        self.brow_raise[1] = damp(self.brow_raise[1], b.brow_raise[1], rate, dt);
        self.brow_inner = damp(self.brow_inner, b.brow_inner, rate, dt);
        self.lid_open = damp(self.lid_open, b.lid_open, rate, dt);
        self.lid_lower = damp(self.lid_lower, b.lid_lower, rate, dt);
        self.smile = damp(self.smile, b.smile, rate, dt);
        self.open = damp(self.open, b.open, rate * 1.5, dt);
        self.wide = damp(self.wide, b.wide, rate, dt);
        self.asym = damp(self.asym, b.asym, rate, dt);
    }
}

/// Meshes shared by every character's face.
pub struct FaceMeshes {
    pub eyeball: MeshData,
    pub lid_upper: MeshData,
    pub lid_lower: MeshData,
    pub lash: MeshData,
}

impl FaceMeshes {
    pub fn new() -> FaceMeshes {
        let white = Mat::new(0xffffff, 0.06).kind(kind::EYE);
        let mut eyeball = MeshData::new();
        eyeball.sphere(Mat4::IDENTITY, 1.0, 28, 18, &white);

        let skin = Mat::new(0xffffff, 0.5).kind(kind::SKIN);
        let a0 = (-20f32).to_radians();
        let a1 = 200f32.to_radians();
        // upper lid: cap from the top pole to 60 degrees, with thickness
        let mut prof = Vec::new();
        let steps = 8;
        for i in 0..=steps {
            let ph = (i as f32 / steps as f32) * 60f32.to_radians();
            prof.push((1.0 * ph.sin(), 1.0 * ph.cos()));
        }
        let e = 60f32.to_radians();
        prof.push((0.94 * e.sin(), 0.94 * e.cos() - 0.02));
        prof.push((0.9 * e.sin(), 0.9 * e.cos()));
        for i in (0..=steps).rev() {
            let ph = (i as f32 / steps as f32) * 60f32.to_radians();
            prof.push((0.9 * ph.sin(), 0.9 * ph.cos()));
        }
        // lathe expects bottom-to-top; build via reversed order to keep outward normals
        let mut up = MeshData::new();
        let rev: Vec<(f32, f32)> = prof.iter().rev().copied().collect();
        up.lathe_arc(Mat4::IDENTITY, &rev, 20, a0, a1, &skin);
        // lower lid
        let mut prof_l = Vec::new();
        for i in 0..=steps {
            let ph = std::f32::consts::PI - (i as f32 / steps as f32) * 52f32.to_radians();
            prof_l.push((1.0 * ph.sin(), 1.0 * ph.cos()));
        }
        let e2 = std::f32::consts::PI - 52f32.to_radians();
        prof_l.push((0.94 * e2.sin(), 0.94 * e2.cos() + 0.02));
        for i in (0..=steps).rev() {
            let ph = std::f32::consts::PI - (i as f32 / steps as f32) * 52f32.to_radians();
            prof_l.push((0.9 * ph.sin(), 0.9 * ph.cos()));
        }
        let mut low = MeshData::new();
        low.lathe_arc(Mat4::IDENTITY, &prof_l, 20, a0, a1, &skin);

        // lash line along the upper lid edge
        let dark = Mat::new(0x151010, 0.6);
        let mut path = Vec::new();
        let mut radii = Vec::new();
        for i in 0..=14 {
            let a = (15.0 + 150.0 * i as f32 / 14.0f32).to_radians();
            let r = 1.01 * e.sin();
            path.push(Vec3::new(r * a.cos(), 1.01 * e.cos() - 0.01, r * a.sin()));
            let t = i as f32 / 14.0;
            radii.push(0.035 + 0.05 * (1.0 - (t * 2.0 - 1.0).abs()).powf(0.5) * if t < 0.5 { 0.8 } else { 1.0 });
        }
        let mut lash = MeshData::new();
        lash.tube(&path, &radii, 6, &dark, true);
        FaceMeshes {
            eyeball,
            lid_upper: up,
            lid_lower: low,
            lash,
        }
    }
}

impl Default for FaceMeshes {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the brow mesh for one side in head-local space.
pub fn brow_mesh(layout: &FaceLayout, side: usize, color: u32, masc: f32) -> MeshData {
    let m = Mat::new(color, 0.7).kind(kind::HAIR);
    let pts: Vec<Vec3> = layout.brow[side].iter().map(|p| *p + Vec3::new(0.0, 0.0, 0.0022)).collect();
    let th = 1.0 + masc * 0.35;
    let radii = [0.0036 * th, 0.0037 * th, 0.003 * th, 0.0018 * th];
    let mut md = MeshData::new();
    // flatten the tube vertically for a strip-like brow
    md.tube(&pts, &radii, 8, &m, true);
    let c = pts.iter().copied().sum::<Vec3>() / pts.len() as f32;
    for v in &mut md.verts {
        let p = Vec3::from(v.pos) - c;
        let q = Vec3::new(p.x, p.y * 0.62, p.z * 0.4) + c;
        v.pos = q.to_array();
    }
    md
}

/// Brow transform (head-local) for the given face state.
pub fn brow_xf(layout: &FaceLayout, side: usize, f: &FaceState) -> Mat4 {
    let pts = &layout.brow[side];
    let c = (pts[0] + pts[3]) * 0.5;
    let s = if side == 0 { 1.0 } else { -1.0 };
    let raise = f.brow_raise[side] * 0.0035 + f.brow_inner * 0.001;
    // inner-up rotates around the forward axis
    let tilt = f.brow_inner * 0.22 * -s;
    let rot = Quat::from_rotation_z(tilt);
    Mat4::from_translation(c + Vec3::new(0.0, raise, 0.0)) * Mat4::from_quat(rot) * Mat4::from_translation(-c)
}

/// Rebuilds the mouth mesh in head-local space.
pub fn mouth_mesh(layout: &FaceLayout, f: &FaceState, lip: [u8; 4], skin: [u8; 4], out: &mut MeshData) {
    out.verts.clear();
    out.indices.clear();
    let n = 14;
    let m = layout.mouth;
    let w = layout.mouth_w * (1.0 + f.wide * 0.2 + f.smile.max(0.0) * 0.1 - f.wide.min(0.0) * -0.15);
    let curve = layout.face_curve;
    let open = f.open.clamp(0.0, 1.0);
    let lip_m = Mat::new(0, 0.35).kind(kind::SKIN);
    let inner_m = Mat::new(0, 0.6);
    let teeth_m = Mat::new(0, 0.25);
    let mut lip_mat = lip_m;
    lip_mat.color = lip;
    let mut skin_mat = lip_m;
    skin_mat.color = skin;
    let mut inner = inner_m;
    inner.color = [70, 22, 26, 255];
    let mut teeth = teeth_m;
    teeth.color = [236, 232, 222, 255];

    let rows = 6;
    // rows: 0 upper outer (skin blend), 1 upper lip top, 2 upper lip inner, 3 lower lip inner, 4 lower lip bottom, 5 lower outer
    let mut grid: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); rows];
    for i in 0..=n {
        let u = i as f32 / n as f32 * 2.0 - 1.0;
        let x = u * w;
        let prof = (1.0 - u * u).max(0.0);
        let side = if u >= 0.0 { 1.0 } else { -1.0 };
        let corner = (f.smile + f.asym * side * 0.6) * 0.0048 * u * u;
        let z_face = m.z - curve * x * x;
        let bulge = 0.0016 * prof.powf(0.6) + f.wide.min(0.0).abs() * 0.004 * prof;
        let y_line = m.y + corner;
        let gap = open * 0.017 * prof.powf(0.75);
        let h_up = 0.0064 * prof.powf(0.55) * (1.0 - 0.25 * (-(u * 6.0).powi(2)).exp());
        let h_lo = 0.0078 * prof.powf(0.55) * (1.0 - 0.15 * open);
        let y_top = y_line + h_up + 0.0012 * prof;
        let y_bot = y_line - gap - h_lo;
        let pts = [
            (Vec3::new(x * 1.08, y_top + 0.0022, z_face + 0.0003), Vec3::new(2.0 * curve * x, 0.2, 1.0)),
            (Vec3::new(x, y_top, z_face + bulge * 0.8 + 0.0006), Vec3::new(2.0 * curve * x, 0.55, 1.0)),
            (Vec3::new(x * 0.98, y_line, z_face + bulge + 0.0004), Vec3::new(2.0 * curve * x, -0.3, 1.0)),
            (Vec3::new(x * 0.98, y_line - gap, z_face + bulge * 1.1 + 0.0004), Vec3::new(2.0 * curve * x, 0.35, 1.0)),
            (Vec3::new(x, y_bot, z_face + bulge * 0.7 + 0.0006), Vec3::new(2.0 * curve * x, -0.55, 1.0)),
            (Vec3::new(x * 1.06, y_bot - 0.0025, z_face + 0.0003), Vec3::new(2.0 * curve * x, -0.2, 1.0)),
        ];
        for (r, (p, nn)) in pts.iter().enumerate() {
            grid[r].push((*p, nn.normalize()));
        }
    }
    let band = |out: &mut MeshData, r0: usize, r1: usize, m0: &Mat, m1: &Mat, grid: &Vec<Vec<(Vec3, Vec3)>>| {
        let base = out.verts.len() as u32;
        for i in 0..=n {
            out.vert(grid[r0][i].0, grid[r0][i].1, m0, 1.0);
            out.vert(grid[r1][i].0, grid[r1][i].1, m1, 1.0);
        }
        for i in 0..n as u32 {
            let a = base + i * 2;
            // rows go top to bottom; keep the front face towards +z
            out.indices.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    };
    band(out, 0, 1, &skin_mat, &lip_mat, &grid);
    band(out, 1, 2, &lip_mat, &lip_mat, &grid);
    if open > 0.02 {
        // mouth interior slightly recessed, with a strip of teeth
        let mut inner_rows: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); 3];
        for i in 0..=n {
            let (a, _) = grid[2][i];
            let (b, _) = grid[3][i];
            let tz = -0.003;
            let teeth_h = (a.y - b.y).min(0.0042);
            inner_rows[0].push((a + Vec3::new(0.0, 0.0, tz * 0.3), Vec3::Z));
            inner_rows[1].push((Vec3::new(a.x, a.y - teeth_h, a.z + tz * 0.5), Vec3::Z));
            inner_rows[2].push((b + Vec3::new(0.0, 0.0, tz), Vec3::Z));
        }
        band(out, 0, 1, &teeth, &teeth, &inner_rows);
        band(out, 1, 2, &inner, &inner, &inner_rows);
    }
    band(out, 3, 4, &lip_mat, &lip_mat, &grid);
    band(out, 4, 5, &lip_mat, &skin_mat, &grid);
    if open <= 0.02 {
        // closed lip line
        let base = out.verts.len() as u32;
        let mut dark = lip_mat;
        dark.color = [
            (lip[0] as f32 * 0.55) as u8,
            (lip[1] as f32 * 0.45) as u8,
            (lip[2] as f32 * 0.45) as u8,
            255,
        ];
        for i in 0..=n {
            let (p, _) = grid[2][i];
            let t = (i as f32 / n as f32 * 2.0 - 1.0).abs();
            let hw = 0.00065 * (1.0 - t * 0.5);
            out.vert(p + Vec3::new(0.0, hw, 0.0003), Vec3::Z, &dark, 1.0);
            out.vert(p + Vec3::new(0.0, -hw, 0.0003), Vec3::Z, &dark, 1.0);
        }
        for i in 0..n as u32 {
            let a = base + i * 2;
            out.indices.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    }
}
