//! Facial expressions and face-part meshes (eyes, lids, brows, animated mouth).

use super::build::FaceLayout;
use crate::gfx::mesh::{kind, Mat, MeshData};
use crate::math::{damp, lerp, smoothstep};
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
    /// Upper lash line: index = side + 2 * (thin as usize).
    pub lash: [MeshData; 4],
}

/// Lid shell radius relative to the eyeball.
pub const LID_SCALE: f32 = 1.07;
/// Outward yaw of each eye's lid frame so the opening follows the curve of the face.
pub const EYE_YAW: f32 = 0.2;
/// Inner radius of the lid shell (unit sphere units).
const LID_INNER: f32 = 0.93;

impl FaceMeshes {
    pub fn new() -> FaceMeshes {
        let white = Mat::new(0xffffff, 0.06).kind(kind::EYE);
        let mut eyeball = MeshData::new();
        eyeball.sphere(Mat4::IDENTITY, 1.0, 28, 18, &white);

        // Lids are hemispherical shells that rotate about the eye's horizontal axis:
        // both edges pass through the eye corners, so the opening is an almond.
        let skin = Mat::new(0xffffff, 0.58).kind(kind::SKIN);
        let a0 = (-25f32).to_radians();
        let a1 = 205f32.to_radians();
        let steps = 10;
        let half = std::f32::consts::FRAC_PI_2;
        // upper lid (pole +Y): inner surface pole -> edge, margin, outer surface edge -> pole
        let mut prof_u = Vec::new();
        for i in 0..=steps {
            let ph = i as f32 / steps as f32 * half;
            prof_u.push((LID_INNER * ph.sin(), LID_INNER * ph.cos()));
        }
        prof_u.push((0.97, -0.012));
        for i in (0..=steps).rev() {
            let ph = i as f32 / steps as f32 * half;
            prof_u.push((ph.sin(), ph.cos()));
        }
        let mut up = MeshData::new();
        up.lathe_arc(Mat4::IDENTITY, &prof_u, 24, a0, a1, &skin);
        // soft shading toward the lash line
        for v in &mut up.verts {
            let y = v.pos[1];
            v.color[3] = (255.0 * (0.84 + 0.16 * smoothstep(0.0, 0.4, y))) as u8;
        }
        // lower lid (pole -Y): outer surface pole -> edge, margin, inner surface edge -> pole
        let mut prof_l = Vec::new();
        for i in 0..=steps {
            let ph = std::f32::consts::PI - i as f32 / steps as f32 * half;
            prof_l.push((ph.sin(), ph.cos()));
        }
        prof_l.push((0.97, 0.012));
        for i in (0..=steps).rev() {
            let ph = std::f32::consts::PI - i as f32 / steps as f32 * half;
            prof_l.push((LID_INNER * ph.sin(), LID_INNER * ph.cos()));
        }
        let mut low = MeshData::new();
        low.lathe_arc(Mat4::IDENTITY, &prof_l, 24, a0, a1, &skin);
        for v in &mut low.verts {
            let y = -v.pos[1];
            v.color[3] = (255.0 * (0.92 + 0.08 * smoothstep(0.0, 0.3, y))) as u8;
        }

        // lash line along the upper lid edge, thicker toward the outer corner
        let dark = Mat::new(0x17100e, 0.75);
        let lash = |side: usize, thin: bool| -> MeshData {
            let mut path = Vec::new();
            let mut radii = Vec::new();
            let n = 18;
            for i in 0..=n {
                // t: 0 = inner corner, 1 = outer corner
                let t = i as f32 / n as f32;
                let deg: f32 = 166.0 - 154.0 * t;
                let a = if side == 0 { deg } else { 180.0 - deg }.to_radians();
                let wing = smoothstep(0.8, 1.0, t);
                let r = 1.012 + 0.03 * wing;
                path.push(Vec3::new(r * a.cos(), -0.012 + 0.045 * wing * wing, r * a.sin()));
                let body = (t * std::f32::consts::PI).sin().max(0.0).powf(0.6);
                let w = if thin {
                    0.012 + 0.014 * body
                } else {
                    0.014 + 0.026 * body + 0.02 * smoothstep(0.45, 0.9, t)
                };
                radii.push(w * (1.0 - 0.65 * smoothstep(0.9, 1.0, t)));
            }
            let mut md = MeshData::new();
            md.tube(&path, &radii, 6, &dark, true);
            md
        };
        FaceMeshes {
            eyeball,
            lid_upper: up,
            lid_lower: low,
            lash: [lash(0, false), lash(1, false), lash(0, true), lash(1, true)],
        }
    }
}

impl Default for FaceMeshes {
    fn default() -> Self {
        Self::new()
    }
}

/// Lid angles (radians) for the current face state: elevation of the front edge of the
/// upper lid above the eye centre, and of the lower lid below it.
pub fn lid_angles(f: &FaceState, blink: f32, gaze_pitch: f32) -> (f32, f32) {
    let open = (f.lid_open * (1.0 - blink)).clamp(0.0, 1.35);
    let pitch = gaze_pitch.to_degrees().clamp(-20.0, 25.0);
    let closed = -7.0;
    let neutral = 25.5;
    let mut up = closed + (neutral - closed) * open;
    // lids follow the gaze
    up -= pitch * 0.75 * open.min(1.0);
    let base_lo = 27.5 - f.lid_lower * 9.0 + pitch * 0.4;
    let shut = (1.0 - open.min(1.0)).powi(2);
    let lo = base_lo + (5.5 - base_lo) * shut;
    (up.clamp(closed, 40.0).to_radians(), lo.clamp(5.0, 40.0).to_radians())
}

fn catmull(p: &[Vec3; 4], t: f32) -> Vec3 {
    // uniform Catmull-Rom through all four points (t in 0..1 spans p0..p3)
    let seg = (t * 3.0).min(2.9999);
    let i = seg.floor() as usize;
    let f = seg - i as f32;
    let p0 = p[i.saturating_sub(1)];
    let p1 = p[i];
    let p2 = p[i + 1];
    let p3 = p[(i + 2).min(3)];
    0.5 * ((2.0 * p1) + (p2 - p0) * f + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f + (3.0 * p1 - p0 - 3.0 * p2 + p3) * f * f * f)
}

/// Builds the brow mesh for one side in head-local space: a soft tapered ribbon that
/// follows the brow arc on the skin.
pub fn brow_mesh(layout: &FaceLayout, side: usize, color: u32, masc: f32) -> MeshData {
    let mut core = Mat::new(color, 0.88).kind(kind::HAIR);
    // brows are never pure black: lift very dark hair toward a soft brown
    let lift = [30.0, 20.0, 14.0];
    for i in 0..3 {
        core.color[i] = (core.color[i] as f32 * 0.92 + lift[i]).min(255.0) as u8;
    }
    // lighter, softer edges read as sparse hair instead of a painted bar
    let mut edge = core;
    for i in 0..3 {
        edge.color[i] = (core.color[i] as f32 * 1.15 + 16.0).min(255.0) as u8;
    }
    let sc = layout.eye_r / 0.0122;
    let pts = layout.brow[side];
    let h_max = (0.0025 + 0.0012 * masc) * sc;
    let n = 16;
    let mut md = MeshData::new();
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let p = catmull(&pts, t);
        let p_next = catmull(&pts, (t + 0.02).min(1.0));
        let p_prev = catmull(&pts, (t - 0.02).max(0.0));
        let tan = (p_next - p_prev).normalize_or(Vec3::X);
        let nrm = ((p - layout.head_center) * Vec3::new(1.0, 0.15, 1.0)).normalize_or(Vec3::Z);
        let mut up = nrm.cross(tan).normalize_or(Vec3::Y);
        if up.y < 0.0 {
            up = -up;
        }
        // thickness: rounded inner end, fullest at one third, thin pointed tail
        let h = h_max * (0.4 + 0.6 * smoothstep(0.0, 0.14, t)) * (1.0 - 0.88 * smoothstep(0.22, 1.0, t).powf(1.1));
        let lift = 0.0005 * sc;
        let base = p + nrm * 0.0004 * sc;
        md.vert(base - up * h, nrm - up * 0.7, &edge, 1.0);
        md.vert(base - up * h * 0.35 + nrm * lift, nrm - up * 0.15, &core, 1.0);
        md.vert(base + up * h * 0.4 + nrm * lift, nrm + up * 0.15, &core, 1.0);
        md.vert(base + up * h, nrm + up * 0.7, &edge, 1.0);
    }
    for i in 0..n as u32 {
        for r in 0..3u32 {
            let a = i * 4 + r;
            let b = a + 1;
            let c = a + 4;
            let d = c + 1;
            if side == 0 {
                md.indices.extend_from_slice(&[a, c, d, a, d, b]);
            } else {
                md.indices.extend_from_slice(&[a, d, c, a, b, d]);
            }
        }
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
    let n = 18;
    let m = layout.mouth;
    let sc = layout.eye_r / 0.0122 / 1.08;
    let w = layout.mouth_w * (1.0 + f.wide * 0.2 + f.smile.max(0.0) * 0.12 - f.wide.min(0.0) * -0.15);
    let curve = layout.face_curve;
    let open = f.open.clamp(0.0, 1.0);
    let lip_m = Mat::new(0, 0.42).kind(kind::SKIN);
    let inner_m = Mat::new(0, 0.6);
    let teeth_m = Mat::new(0, 0.25);
    let mut lip_mat = lip_m;
    lip_mat.color = lip;
    // the vermilion border blends into the skin instead of ending in a hard edge
    let mut border = lip_m;
    border.rough = 0.55;
    for i in 0..3 {
        border.color[i] = ((lip[i] as f32 * 0.55 + skin[i] as f32 * 0.45) as u8).min(255);
    }
    border.color[3] = 255;
    let mut skin_mat = lip_m;
    skin_mat.rough = 0.6;
    skin_mat.color = skin;
    let mut inner = inner_m;
    inner.color = [62, 20, 24, 255];
    let mut teeth = teeth_m;
    teeth.color = [232, 228, 218, 255];

    let rows = 8;
    // rows (top to bottom): 0 skin, 1 border, 2 upper lip body, 3 upper lip inner edge,
    // 4 lower lip inner edge, 5 lower lip body, 6 border, 7 skin
    let mut grid: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); rows];
    for i in 0..=n {
        let u = i as f32 / n as f32 * 2.0 - 1.0;
        let x = u * w;
        let prof = (1.0 - u * u).max(0.0);
        let side = if u >= 0.0 { 1.0 } else { -1.0 };
        // corners move up with a smile; the centre of the upper lip dips a little
        let corner = (f.smile + f.asym * side * 0.6) * 0.0052 * sc * u.abs().powf(1.6);
        let z_face = m.z - curve * x * x;
        let bulge = (0.0017 * prof.powf(0.6) + f.wide.min(0.0).abs() * 0.004 * prof) * sc;
        let y_line = m.y + corner - 0.0006 * sc * prof * (1.0 - f.smile.max(0.0));
        let gap = open * 0.017 * sc * prof.powf(0.75);
        // cupid's bow: a dip at the centre flanked by two soft peaks
        let bow = 1.0 - 0.24 * (-(u * 6.5).powi(2)).exp() + 0.1 * (-((u.abs() - 0.27) * 7.0).powi(2)).exp();
        let stretch = 1.0 - 0.18 * f.smile.max(0.0);
        let h_up = 0.0056 * sc * prof.powf(0.62) * bow * stretch;
        let h_lo = 0.0073 * sc * prof.powf(0.5) * (1.0 - 0.15 * open) * stretch;
        let y_top = y_line + h_up;
        let y_bot = y_line - gap - h_lo;
        let nx = 2.0 * curve * x;
        let b = 0.0014 * sc * (0.4 + 0.6 * prof);
        let pts = [
            (Vec3::new(x * 1.06, y_top + b * 1.9, z_face + 0.0002), Vec3::new(nx, 0.15, 1.0)),
            (Vec3::new(x * 1.02, y_top + b * 0.5, z_face + bulge * 0.25 + 0.0004), Vec3::new(nx, 0.45, 1.0)),
            (Vec3::new(x, y_top - h_up * 0.45, z_face + bulge * 0.95 + 0.0005), Vec3::new(nx, 0.3, 1.0)),
            (Vec3::new(x * 0.985, y_line, z_face + bulge * 0.8 + 0.0003), Vec3::new(nx, -0.55, 1.0)),
            (Vec3::new(x * 0.985, y_line - gap, z_face + bulge * 0.85 + 0.0003), Vec3::new(nx, 0.55, 1.0)),
            (Vec3::new(x, y_bot + h_lo * 0.45, z_face + bulge * 1.15 + 0.0005), Vec3::new(nx, -0.2, 1.0)),
            (Vec3::new(x * 1.02, y_bot - b * 0.5, z_face + bulge * 0.3 + 0.0004), Vec3::new(nx, -0.5, 1.0)),
            (Vec3::new(x * 1.05, y_bot - b * 2.2, z_face + 0.0002), Vec3::new(nx, -0.15, 1.0)),
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
    band(out, 0, 1, &skin_mat, &border, &grid);
    band(out, 1, 2, &border, &lip_mat, &grid);
    band(out, 2, 3, &lip_mat, &lip_mat, &grid);
    if open > 0.02 {
        // mouth interior slightly recessed, with a strip of teeth
        let mut inner_rows: Vec<Vec<(Vec3, Vec3)>> = vec![Vec::new(); 3];
        for i in 0..=n {
            let (a, _) = grid[3][i];
            let (b, _) = grid[4][i];
            let tz = -0.003;
            let teeth_h = (a.y - b.y).min(0.0042);
            inner_rows[0].push((a + Vec3::new(0.0, 0.0, tz * 0.3), Vec3::Z));
            inner_rows[1].push((Vec3::new(a.x, a.y - teeth_h, a.z + tz * 0.5), Vec3::Z));
            inner_rows[2].push((b + Vec3::new(0.0, 0.0, tz), Vec3::Z));
        }
        band(out, 0, 1, &teeth, &teeth, &inner_rows);
        band(out, 1, 2, &inner, &inner, &inner_rows);
    }
    band(out, 4, 5, &lip_mat, &lip_mat, &grid);
    band(out, 5, 6, &lip_mat, &border, &grid);
    band(out, 6, 7, &border, &skin_mat, &grid);
    if open <= 0.02 {
        // closed lip line: a soft dark seam, thinner toward the corners
        let base = out.verts.len() as u32;
        let mut dark = lip_mat;
        dark.color = [
            (lip[0] as f32 * 0.5) as u8,
            (lip[1] as f32 * 0.4) as u8,
            (lip[2] as f32 * 0.4) as u8,
            255,
        ];
        for i in 0..=n {
            let (p, _) = grid[3][i];
            let t = (i as f32 / n as f32 * 2.0 - 1.0).abs();
            let hw = 0.00055 * sc * (1.0 - t * 0.6);
            out.vert(p + Vec3::new(0.0, hw, 0.0003), Vec3::Z, &dark, 1.0);
            out.vert(p + Vec3::new(0.0, -hw, 0.0003), Vec3::Z, &dark, 1.0);
        }
        for i in 0..n as u32 {
            let a = base + i * 2;
            out.indices.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    }
}
