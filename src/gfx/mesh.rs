//! Vertex formats, CPU mesh building and procedural primitives.

use crate::math::rgb;
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    /// Albedo in sRGB, alpha used as ambient occlusion.
    pub color: [u8; 4],
    /// roughness, metallic, emissive, kind
    pub mat: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct SkinVertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub color: [u8; 4],
    pub mat: [u8; 4],
    pub joints: [u8; 4],
    pub weights: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    pub tint: [f32; 4],
    pub params: [f32; 4],
}

/// Surface kinds understood by the PBR shader (procedural patterns).
pub mod kind {
    pub const PLAIN: u8 = 0;
    pub const SKIN: u8 = 1;
    pub const FABRIC: u8 = 2;
    pub const DENIM: u8 = 3;
    pub const HAIR: u8 = 4;
    pub const WOOD: u8 = 5;
    pub const PLANKS: u8 = 6;
    pub const TILES: u8 = 7;
    pub const PLASTER: u8 = 8;
    pub const CARPET: u8 = 9;
    pub const EYE: u8 = 10;
    pub const SCREEN: u8 = 11;
    pub const FOLIAGE: u8 = 12;
    pub const ASPHALT: u8 = 13;
    pub const PAVERS: u8 = 14;
    pub const BRICK: u8 = 15;
    pub const GRASS: u8 = 16;
    pub const BRUSHED: u8 = 17;
    pub const GLASS: u8 = 18;
    pub const POSTER: u8 = 19;
    pub const BOOKS: u8 = 20;
    pub const PLAID: u8 = 21;
    pub const SHADE: u8 = 22;
    pub const LEATHER: u8 = 23;
    pub const SOFT: u8 = 24; // particles / soft round sprites
    pub const BEAM: u8 = 25; // volumetric light beam
    pub const PHONE: u8 = 26; // phone screen
    pub const STRIPES: u8 = 27;
    pub const CONCRETE: u8 = 28;
    pub const SIGN: u8 = 29;
    pub const WATER: u8 = 30;
    pub const KNIT: u8 = 31;
}

#[derive(Clone, Copy, Debug)]
pub struct Mat {
    pub color: [u8; 4],
    pub rough: f32,
    pub metal: f32,
    pub emissive: f32,
    pub kind: u8,
}

impl Mat {
    pub const fn new(hex: u32, rough: f32) -> Self {
        Mat {
            color: rgb(hex),
            rough,
            metal: 0.0,
            emissive: 0.0,
            kind: kind::PLAIN,
        }
    }
    pub const fn kind(mut self, k: u8) -> Self {
        self.kind = k;
        self
    }
    pub const fn metal(mut self, m: f32) -> Self {
        self.metal = m;
        self
    }
    pub const fn emit(mut self, e: f32) -> Self {
        self.emissive = e;
        self
    }
    pub fn packed(&self) -> [u8; 4] {
        [
            (self.rough.clamp(0.0, 1.0) * 255.0) as u8,
            (self.metal.clamp(0.0, 1.0) * 255.0) as u8,
            (self.emissive.clamp(0.0, 1.0) * 255.0) as u8,
            self.kind,
        ]
    }
    pub fn color_ao(&self, ao: f32) -> [u8; 4] {
        [
            self.color[0],
            self.color[1],
            self.color[2],
            (ao.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }
}

#[derive(Clone, Default, Debug)]
pub struct MeshData {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u32>,
}

fn normal_matrix(xf: &Mat4) -> Mat3 {
    Mat3::from_mat4(*xf).inverse().transpose()
}

impl MeshData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn vert(&mut self, p: Vec3, n: Vec3, m: &Mat, ao: f32) -> u32 {
        let i = self.verts.len() as u32;
        self.verts.push(Vertex {
            pos: p.to_array(),
            nrm: n.normalize_or(Vec3::Y).to_array(),
            color: m.color_ao(ao),
            mat: m.packed(),
        });
        i
    }

    pub fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    pub fn quad_idx(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.indices.extend_from_slice(&[a, b, c, a, c, d]);
    }

    pub fn append(&mut self, other: &MeshData) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&other.verts);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    pub fn append_xf(&mut self, other: &MeshData, xf: Mat4) {
        let nm = normal_matrix(&xf);
        let base = self.verts.len() as u32;
        for v in &other.verts {
            let mut v2 = *v;
            v2.pos = xf.transform_point3(Vec3::from(v.pos)).to_array();
            v2.nrm = (nm * Vec3::from(v.nrm)).normalize_or(Vec3::Y).to_array();
            self.verts.push(v2);
        }
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Flat shaded quad with explicit corners (counter-clockwise seen from the front).
    pub fn quad(&mut self, p: [Vec3; 4], m: &Mat) {
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or(Vec3::Y);
        let a = self.vert(p[0], n, m, 1.0);
        let b = self.vert(p[1], n, m, 1.0);
        let c = self.vert(p[2], n, m, 1.0);
        let d = self.vert(p[3], n, m, 1.0);
        self.quad_idx(a, b, c, d);
    }

    /// Quad with per-corner ambient occlusion.
    pub fn quad_ao(&mut self, p: [Vec3; 4], ao: [f32; 4], m: &Mat) {
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or(Vec3::Y);
        let a = self.vert(p[0], n, m, ao[0]);
        let b = self.vert(p[1], n, m, ao[1]);
        let c = self.vert(p[2], n, m, ao[2]);
        let d = self.vert(p[3], n, m, ao[3]);
        self.quad_idx(a, b, c, d);
    }

    /// Subdivided flat grid on local XZ plane, facing +Y, centred at origin.
    pub fn plane(&mut self, xf: Mat4, size: Vec2, div: (u32, u32), m: &Mat) {
        let nm = normal_matrix(&xf);
        let n = (nm * Vec3::Y).normalize();
        let base = self.verts.len() as u32;
        let (dx, dz) = (div.0.max(1), div.1.max(1));
        for j in 0..=dz {
            for i in 0..=dx {
                let p = Vec3::new(
                    (i as f32 / dx as f32 - 0.5) * size.x,
                    0.0,
                    (j as f32 / dz as f32 - 0.5) * size.y,
                );
                self.vert(xf.transform_point3(p), n, m, 1.0);
            }
        }
        for j in 0..dz {
            for i in 0..dx {
                let a = base + j * (dx + 1) + i;
                let b = a + 1;
                let c = a + dx + 1;
                let d = c + 1;
                self.indices.extend_from_slice(&[a, c, d, a, d, b]);
            }
        }
    }

    /// Axis-aligned box (flat normals) centred at origin in local space.
    pub fn cube(&mut self, xf: Mat4, half: Vec3, m: &Mat) {
        let h = half;
        let c = [
            Vec3::new(-h.x, -h.y, -h.z),
            Vec3::new(h.x, -h.y, -h.z),
            Vec3::new(h.x, h.y, -h.z),
            Vec3::new(-h.x, h.y, -h.z),
            Vec3::new(-h.x, -h.y, h.z),
            Vec3::new(h.x, -h.y, h.z),
            Vec3::new(h.x, h.y, h.z),
            Vec3::new(-h.x, h.y, h.z),
        ];
        let faces = [
            [4, 5, 6, 7], // +z
            [1, 0, 3, 2], // -z
            [5, 1, 2, 6], // +x
            [0, 4, 7, 3], // -x
            [7, 6, 2, 3], // +y
            [0, 1, 5, 4], // -y
        ];
        for f in faces {
            let p = [
                xf.transform_point3(c[f[0]]),
                xf.transform_point3(c[f[1]]),
                xf.transform_point3(c[f[2]]),
                xf.transform_point3(c[f[3]]),
            ];
            self.quad(p, m);
        }
    }

    /// Box with rounded edges and smooth normals.
    pub fn rbox(&mut self, xf: Mat4, half: Vec3, radius: f32, segs: u32, m: &Mat) {
        let r = radius.min(half.min_element() * 0.999).max(0.0);
        if r <= 1e-5 {
            self.cube(xf, half, m);
            return;
        }
        let segs = segs.max(1);
        let coords = |h: f32| -> Vec<f32> {
            let mut v = Vec::new();
            for i in 0..=segs {
                let a = i as f32 / segs as f32 * std::f32::consts::FRAC_PI_2;
                v.push(-h + r * (1.0 - a.cos()));
            }
            if h - r > -h + r + 1e-5 {
                for i in 0..=segs {
                    let a = i as f32 / segs as f32 * std::f32::consts::FRAC_PI_2;
                    v.push(h - r + r * a.sin());
                }
            } else {
                for i in 1..=segs {
                    let a = i as f32 / segs as f32 * std::f32::consts::FRAC_PI_2;
                    v.push(h - r + r * a.sin());
                }
            }
            v
        };
        let cx = coords(half.x);
        let cy = coords(half.y);
        let cz = coords(half.z);
        let inner = half - Vec3::splat(r);
        let nm = normal_matrix(&xf);
        // (axis u, axis v, fixed axis, sign)
        let faces: [(usize, usize, usize, f32); 6] = [
            (0, 1, 2, 1.0),
            (1, 0, 2, -1.0),
            (1, 2, 0, 1.0),
            (2, 1, 0, -1.0),
            (2, 0, 1, 1.0),
            (0, 2, 1, -1.0),
        ];
        let all = [&cx, &cy, &cz];
        for (ua, va, fa, sign) in faces {
            let us = all[ua];
            let vs = all[va];
            let base = self.verts.len() as u32;
            for &vv in vs.iter() {
                for &uu in us.iter() {
                    let mut p = Vec3::ZERO;
                    p[ua] = uu;
                    p[va] = vv;
                    p[fa] = half[fa] * sign;
                    let q = p.clamp(-inner, inner);
                    let mut n = p - q;
                    if n.length_squared() < 1e-12 {
                        n = Vec3::ZERO;
                        n[fa] = sign;
                    }
                    let n = n.normalize();
                    let pos = q + n * r;
                    let wn = (nm * n).normalize();
                    self.vert(xf.transform_point3(pos), wn, m, 1.0);
                }
            }
            let nu = us.len() as u32;
            let nv = vs.len() as u32;
            for j in 0..nv - 1 {
                for i in 0..nu - 1 {
                    let a = base + j * nu + i;
                    let b = a + 1;
                    let c = a + nu;
                    let d = c + 1;
                    self.indices.extend_from_slice(&[a, b, d, a, d, c]);
                }
            }
        }
    }

    /// Surface of revolution around local Y. Profile is (radius, y) from bottom to top.
    /// Sharp corners are created where the profile turns more than ~35 degrees.
    pub fn lathe(&mut self, xf: Mat4, profile: &[(f32, f32)], segs: u32, m: &Mat) {
        self.lathe_arc(xf, profile, segs, 0.0, std::f32::consts::TAU, m);
    }

    pub fn lathe_arc(
        &mut self,
        xf: Mat4,
        profile: &[(f32, f32)],
        segs: u32,
        a0: f32,
        a1: f32,
        m: &Mat,
    ) {
        if profile.len() < 2 {
            return;
        }
        let nm = normal_matrix(&xf);
        let seg_n: Vec<Vec2> = profile
            .windows(2)
            .map(|w| {
                let d = Vec2::new(w[1].0 - w[0].0, w[1].1 - w[0].1);
                Vec2::new(d.y, -d.x).normalize_or(Vec2::X)
            })
            .collect();
        let cos_lim = (35f32).to_radians().cos();
        for (s, w) in profile.windows(2).enumerate() {
            let n_here = seg_n[s];
            let n0 = if s > 0 && seg_n[s - 1].dot(n_here) > cos_lim {
                (seg_n[s - 1] + n_here).normalize()
            } else {
                n_here
            };
            let n1 = if s + 1 < seg_n.len() && seg_n[s + 1].dot(n_here) > cos_lim {
                (seg_n[s + 1] + n_here).normalize()
            } else {
                n_here
            };
            let base = self.verts.len() as u32;
            for i in 0..=segs {
                let a = a0 + (a1 - a0) * i as f32 / segs as f32;
                let (sn, cs) = a.sin_cos();
                for (k, (r, y)) in [(w[0].0, w[0].1), (w[1].0, w[1].1)].iter().enumerate() {
                    let nn = if k == 0 { n0 } else { n1 };
                    let p = Vec3::new(r * cs, *y, r * sn);
                    let n = Vec3::new(nn.x * cs, nn.y, nn.x * sn);
                    self.vert(xf.transform_point3(p), (nm * n).normalize_or(Vec3::Y), m, 1.0);
                }
            }
            for i in 0..segs {
                let a = base + i * 2;
                let b = a + 1;
                let c = a + 2;
                let d = a + 3;
                self.indices.extend_from_slice(&[a, b, d, a, d, c]);
            }
        }
    }

    /// Cylinder along +Y from y=0 to y=h.
    pub fn cylinder(&mut self, xf: Mat4, r: f32, h: f32, segs: u32, m: &Mat) {
        self.lathe(xf, &[(0.0, 0.0), (r, 0.0), (r, h), (0.0, h)], segs, m);
    }

    /// Cylinder with softly rounded rims.
    pub fn rcylinder(&mut self, xf: Mat4, r: f32, h: f32, bevel: f32, segs: u32, m: &Mat) {
        let b = bevel.min(r * 0.5).min(h * 0.5);
        let mut prof = vec![(0.0, 0.0)];
        for i in 0..=3 {
            let a = i as f32 / 3.0 * std::f32::consts::FRAC_PI_2;
            prof.push((r - b + b * a.sin(), b - b * a.cos()));
        }
        for i in 0..=3 {
            let a = i as f32 / 3.0 * std::f32::consts::FRAC_PI_2;
            prof.push((r - b + b * a.cos(), h - b + b * a.sin()));
        }
        prof.push((0.0, h));
        self.lathe(xf, &prof, segs, m);
    }

    pub fn cone(&mut self, xf: Mat4, r0: f32, r1: f32, h: f32, segs: u32, m: &Mat) {
        self.lathe(xf, &[(0.0, 0.0), (r0, 0.0), (r1, h), (0.0, h)], segs, m);
    }

    pub fn sphere(&mut self, xf: Mat4, r: f32, segs: u32, rings: u32, m: &Mat) {
        let mut prof = Vec::new();
        for i in 0..=rings {
            let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / rings as f32;
            prof.push((r * a.cos(), r * a.sin()));
        }
        self.lathe_smooth(xf, &prof, segs, m);
    }

    /// Lathe with all-smooth normals (no crease detection).
    pub fn lathe_smooth(&mut self, xf: Mat4, profile: &[(f32, f32)], segs: u32, m: &Mat) {
        let nm = normal_matrix(&xf);
        let n = profile.len();
        let normals: Vec<Vec2> = (0..n)
            .map(|i| {
                let p0 = if i > 0 { profile[i - 1] } else { profile[i] };
                let p1 = if i + 1 < n { profile[i + 1] } else { profile[i] };
                let d = Vec2::new(p1.0 - p0.0, p1.1 - p0.1);
                Vec2::new(d.y, -d.x).normalize_or(Vec2::Y)
            })
            .collect();
        let base = self.verts.len() as u32;
        for i in 0..=segs {
            let a = std::f32::consts::TAU * i as f32 / segs as f32;
            let (sn, cs) = a.sin_cos();
            for (k, (r, y)) in profile.iter().enumerate() {
                let nn = normals[k];
                let p = Vec3::new(r * cs, *y, r * sn);
                let nv = Vec3::new(nn.x * cs, nn.y, nn.x * sn);
                self.vert(xf.transform_point3(p), (nm * nv).normalize_or(Vec3::Y), m, 1.0);
            }
        }
        let n = n as u32;
        for i in 0..segs {
            for k in 0..n - 1 {
                let a = base + i * n + k;
                let b = a + 1;
                let c = a + n;
                let d = c + 1;
                self.indices.extend_from_slice(&[a, b, d, a, d, c]);
            }
        }
    }

    pub fn torus(&mut self, xf: Mat4, big_r: f32, r: f32, segs: u32, sides: u32, m: &Mat) {
        let mut prof = Vec::new();
        for i in 0..=sides {
            let a = -std::f32::consts::PI + std::f32::consts::TAU * i as f32 / sides as f32;
            prof.push((big_r + r * a.cos(), r * a.sin()));
        }
        // lathe expects bottom-to-top; torus profile loops, smooth normals handle it
        let nm = normal_matrix(&xf);
        let base = self.verts.len() as u32;
        for i in 0..=segs {
            let a = std::f32::consts::TAU * i as f32 / segs as f32;
            let (sn, cs) = a.sin_cos();
            for k in 0..=sides {
                let b = -std::f32::consts::PI + std::f32::consts::TAU * k as f32 / sides as f32;
                let (bs, bc) = b.sin_cos();
                let rr = big_r + r * bc;
                let p = Vec3::new(rr * cs, r * bs, rr * sn);
                let n = Vec3::new(bc * cs, bs, bc * sn);
                self.vert(xf.transform_point3(p), (nm * n).normalize_or(Vec3::Y), m, 1.0);
            }
        }
        let n = sides + 1;
        for i in 0..segs {
            for k in 0..sides {
                let a = base + i * n + k;
                let b = a + 1;
                let c = a + n;
                let d = c + 1;
                self.indices.extend_from_slice(&[a, b, d, a, d, c]);
            }
        }
        let _ = prof;
    }

    /// Capsule between two points.
    pub fn capsule(&mut self, a: Vec3, b: Vec3, r: f32, segs: u32, m: &Mat) {
        let d = b - a;
        let len = d.length();
        let rot = glam::Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y));
        let xf = Mat4::from_rotation_translation(rot, a);
        let rings = (segs / 2).max(3);
        let mut prof = Vec::new();
        for i in 0..=rings {
            let t = -std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_2 * i as f32 / rings as f32;
            prof.push((r * t.cos(), r * t.sin()));
        }
        for i in 0..=rings {
            let t = std::f32::consts::FRAC_PI_2 * i as f32 / rings as f32;
            prof.push((r * t.cos(), len + r * t.sin()));
        }
        self.lathe_smooth(xf, &prof, segs, m);
    }

    /// Tube along a polyline with per-point radius.
    pub fn tube(&mut self, path: &[Vec3], radii: &[f32], segs: u32, m: &Mat, cap: bool) {
        if path.len() < 2 {
            return;
        }
        let base = self.verts.len() as u32;
        let mut prev_side = {
            let d = (path[1] - path[0]).normalize_or(Vec3::Y);
            d.any_orthonormal_vector()
        };
        for (i, p) in path.iter().enumerate() {
            let d = if i + 1 < path.len() {
                path[i + 1] - path[i]
            } else {
                path[i] - path[i - 1]
            }
            .normalize_or(Vec3::Y);
            let side = (prev_side - d * prev_side.dot(d)).normalize_or(d.any_orthonormal_vector());
            prev_side = side;
            let up = d.cross(side);
            let r = radii[i.min(radii.len() - 1)];
            for s in 0..=segs {
                let a = std::f32::consts::TAU * s as f32 / segs as f32;
                let n = side * a.cos() + up * a.sin();
                self.vert(*p + n * r, n, m, 1.0);
            }
        }
        let ring = segs + 1;
        for i in 0..path.len() as u32 - 1 {
            for s in 0..segs {
                let a = base + i * ring + s;
                let b = a + 1;
                let c = a + ring;
                let d = c + 1;
                self.indices.extend_from_slice(&[a, d, c, a, b, d]);
            }
        }
        if cap {
            let last = path.len() - 1;
            let d = (path[last] - path[last - 1]).normalize_or(Vec3::Y);
            let center = self.vert(path[last] + d * radii[radii.len() - 1] * 0.5, d, m, 1.0);
            let start = base + last as u32 * ring;
            for s in 0..segs {
                self.tri(start + s, start + s + 1, center);
            }
        }
    }

    /// Flat disc facing +Y.
    pub fn disc(&mut self, xf: Mat4, r: f32, segs: u32, m: &Mat) {
        let nm = normal_matrix(&xf);
        let n = (nm * Vec3::Y).normalize();
        let c = self.vert(xf.transform_point3(Vec3::ZERO), n, m, 1.0);
        let base = self.verts.len() as u32;
        for i in 0..=segs {
            let a = std::f32::consts::TAU * i as f32 / segs as f32;
            self.vert(xf.transform_point3(Vec3::new(r * a.cos(), 0.0, r * a.sin())), n, m, 1.0);
        }
        for i in 0..segs {
            self.tri(c, base + i + 1, base + i);
        }
    }

    /// Extrudes a 2D polygon (XY, counter-clockwise) along +Z by `depth`.
    pub fn extrude(&mut self, xf: Mat4, poly: &[Vec2], depth: f32, m: &Mat) {
        let n = poly.len();
        if n < 3 {
            return;
        }
        let nm = normal_matrix(&xf);
        // caps (fan triangulation; fine for convex shapes)
        for (z, nz) in [(depth, 1.0f32), (0.0, -1.0)] {
            let nn = (nm * Vec3::Z * nz).normalize();
            let base = self.verts.len() as u32;
            for p in poly {
                self.vert(xf.transform_point3(Vec3::new(p.x, p.y, z)), nn, m, 1.0);
            }
            for i in 1..n as u32 - 1 {
                if nz > 0.0 {
                    self.tri(base, base + i, base + i + 1);
                } else {
                    self.tri(base, base + i + 1, base + i);
                }
            }
        }
        for i in 0..n {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let p = [
                xf.transform_point3(Vec3::new(a.x, a.y, 0.0)),
                xf.transform_point3(Vec3::new(b.x, b.y, 0.0)),
                xf.transform_point3(Vec3::new(b.x, b.y, depth)),
                xf.transform_point3(Vec3::new(a.x, a.y, depth)),
            ];
            self.quad(p, m);
        }
    }

    /// Recolour all vertices (keeps AO).
    pub fn set_material(&mut self, m: &Mat) {
        for v in &mut self.verts {
            let ao = v.color[3];
            v.color = m.color;
            v.color[3] = ao;
            v.mat = m.packed();
        }
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for v in &self.verts {
            let p = Vec3::from(v.pos);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        (lo, hi)
    }

    /// Darkens vertices close to the floor (cheap contact AO).
    pub fn floor_ao(&mut self, floor_y: f32, reach: f32, strength: f32) {
        for v in &mut self.verts {
            let h = (v.pos[1] - floor_y).max(0.0);
            let t = (h / reach).clamp(0.0, 1.0);
            let ao = 1.0 - strength * (1.0 - t) * (1.0 - t);
            v.color[3] = ((v.color[3] as f32) * ao) as u8;
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct SkinMeshData {
    pub verts: Vec<SkinVertex>,
    pub indices: Vec<u32>,
}

impl SkinMeshData {
    /// Adds a rigid static mesh attached to a single bone.
    pub fn append_rigid(&mut self, m: &MeshData, bone: u8) {
        let base = self.verts.len() as u32;
        for v in &m.verts {
            self.verts.push(SkinVertex {
                pos: v.pos,
                nrm: v.nrm,
                color: v.color,
                mat: v.mat,
                joints: [bone, 0, 0, 0],
                weights: [255, 0, 0, 0],
            });
        }
        self.indices.extend(m.indices.iter().map(|i| i + base));
    }
}

pub fn m4(t: Vec3) -> Mat4 {
    Mat4::from_translation(t)
}

pub fn m4s(t: Vec3, s: Vec3) -> Mat4 {
    Mat4::from_scale_rotation_translation(s, glam::Quat::IDENTITY, t)
}

pub fn m4ry(t: Vec3, yaw: f32) -> Mat4 {
    Mat4::from_rotation_translation(glam::Quat::from_rotation_y(yaw), t)
}

pub fn m4r(t: Vec3, q: glam::Quat) -> Mat4 {
    Mat4::from_rotation_translation(q, t)
}
