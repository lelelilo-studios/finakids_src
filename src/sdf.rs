//! Signed distance field modelling with a sparse surface-nets mesher.
//! Used to build organic character bodies, heads, hands and hair.

use glam::{Quat, Vec3};

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Sphere { c: Vec3, r: f32 },
    /// Ellipsoid with radii `r`, rotated by `rot` (world = rot * local).
    Ellipsoid { c: Vec3, r: Vec3, inv: Quat },
    Capsule { a: Vec3, b: Vec3, r: f32 },
    /// Cone with rounded ends: radius `ra` at `a`, `rb` at `b`.
    RoundCone { a: Vec3, b: Vec3, ra: f32, rb: f32 },
    RoundBox { c: Vec3, half: Vec3, r: f32, inv: Quat },
    Torus { c: Vec3, big: f32, small: f32, inv: Quat },
    /// Half-space: points with dot(p, n) > d are "inside".
    HalfSpace { n: Vec3, d: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Union,
    Smooth(f32),
    Subtract(f32),
    Intersect(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct BoneSpec {
    pub a: u8,
    pub b: u8,
    pub axis0: Vec3,
    pub axis1: Vec3,
    pub t0: f32,
    pub t1: f32,
}

pub trait BoneIdx {
    fn idx(self) -> u8;
}
impl BoneIdx for u8 {
    fn idx(self) -> u8 {
        self
    }
}
impl BoneIdx for usize {
    fn idx(self) -> u8 {
        self as u8
    }
}

impl BoneSpec {
    pub fn single(b: impl BoneIdx) -> Self {
        let b = b.idx();
        BoneSpec {
            a: b,
            b,
            axis0: Vec3::ZERO,
            axis1: Vec3::Y,
            t0: 0.0,
            t1: 1.0,
        }
    }
    pub fn blend(a: impl BoneIdx, b: impl BoneIdx, axis0: Vec3, axis1: Vec3, t0: f32, t1: f32) -> Self {
        BoneSpec {
            a: a.idx(),
            b: b.idx(),
            axis0,
            axis1,
            t0,
            t1,
        }
    }
    /// Returns (bone_a weight, bone_b weight) at point p.
    pub fn weights(&self, p: Vec3) -> (f32, f32) {
        if self.a == self.b {
            return (1.0, 0.0);
        }
        let ab = self.axis1 - self.axis0;
        let t = (p - self.axis0).dot(ab) / ab.length_squared().max(1e-8);
        let s = crate::math::smoothstep(self.t0, self.t1, t);
        (1.0 - s, s)
    }
}

#[derive(Clone, Debug)]
pub struct Prim {
    pub shape: Shape,
    pub op: Op,
    pub mat: u8,
    pub bones: BoneSpec,
    /// Surface displacement amplitude (wrinkles / hair clumps).
    pub disp: f32,
    pub disp_freq: f32,
    /// 0 = bumpy noise, 1 = combed grooves running front-to-back around `groove_c`
    pub disp_mode: u8,
    pub groove_c: Vec3,
    /// Up to two clip planes (n, d): keeps points where dot(p, n) <= d.
    pub clips: [Option<(Vec3, f32)>; 2],
    lo: Vec3,
    hi: Vec3,
}

impl Prim {
    pub fn new(shape: Shape, op: Op, mat: u8, bones: BoneSpec) -> Prim {
        let (lo, hi) = shape_bounds(&shape);
        Prim {
            shape,
            op,
            mat,
            bones,
            disp: 0.0,
            disp_freq: 0.0,
            disp_mode: 0,
            groove_c: Vec3::ZERO,
            clips: [None, None],
            lo,
            hi,
        }
    }
    /// Keeps only the part of the prim where dot(p, n) <= d.
    pub fn clip(mut self, n: Vec3, d: f32) -> Prim {
        if self.clips[0].is_none() {
            self.clips[0] = Some((n, d));
        } else {
            self.clips[1] = Some((n, d));
        }
        self
    }
    /// Combed-hair grooves: `n` grooves around the axis through `c` along Z.
    pub fn with_grooves(mut self, amp: f32, n: f32, c: Vec3) -> Prim {
        self.disp = amp;
        self.disp_freq = n;
        self.disp_mode = 1;
        self.groove_c = c;
        self.lo -= Vec3::splat(amp);
        self.hi += Vec3::splat(amp);
        self
    }
    /// Falling-hair grooves (vertical strands around the Y axis through `c`).
    pub fn with_falling(mut self, amp: f32, n: f32, c: Vec3) -> Prim {
        self = self.with_grooves(amp, n, c);
        self.disp_mode = 2;
        self
    }
    pub fn with_disp(mut self, amp: f32, freq: f32) -> Prim {
        self.disp = amp;
        self.disp_freq = freq;
        self.lo -= Vec3::splat(amp);
        self.hi += Vec3::splat(amp);
        self
    }

    #[inline]
    fn bound_dist(&self, p: Vec3) -> f32 {
        let q = (self.lo - p).max(p - self.hi).max(Vec3::ZERO);
        q.length()
    }

    #[inline]
    pub fn dist(&self, p: Vec3) -> f32 {
        let mut d = shape_dist(&self.shape, p);
        if self.disp != 0.0 {
            if self.disp_mode >= 1 {
                let q = p - self.groove_c;
                let ang = if self.disp_mode == 1 { q.x.atan2(q.y) } else { q.x.atan2(q.z) };
                let g = (ang * self.disp_freq).sin();
                let g2 = (ang * self.disp_freq * 2.3 + q.z * 40.0).sin();
                d += (g * 0.7 + g2 * 0.3) * self.disp;
            } else {
                let f = self.disp_freq;
                let n = (p.x * f).sin() * (p.y * f * 1.3 + 1.7).sin() * (p.z * f * 0.9 + 0.3).cos();
                d += n * self.disp;
            }
        }
        for (n, dd) in self.clips.iter().flatten() {
            d = smax(d, p.dot(*n) - dd, 0.003);
        }
        d
    }
}

fn shape_bounds(s: &Shape) -> (Vec3, Vec3) {
    match *s {
        Shape::Sphere { c, r } => (c - Vec3::splat(r), c + Vec3::splat(r)),
        Shape::Ellipsoid { c, r, .. } => {
            let m = r.max_element();
            (c - Vec3::splat(m), c + Vec3::splat(m))
        }
        Shape::Capsule { a, b, r } => (a.min(b) - Vec3::splat(r), a.max(b) + Vec3::splat(r)),
        Shape::RoundCone { a, b, ra, rb } => (
            (a - Vec3::splat(ra)).min(b - Vec3::splat(rb)),
            (a + Vec3::splat(ra)).max(b + Vec3::splat(rb)),
        ),
        Shape::RoundBox { c, half, .. } => {
            let m = half.length();
            (c - Vec3::splat(m), c + Vec3::splat(m))
        }
        Shape::Torus { c, big, small, .. } => {
            let m = big + small;
            (c - Vec3::splat(m), c + Vec3::splat(m))
        }
        Shape::HalfSpace { .. } => (Vec3::splat(-1e6), Vec3::splat(1e6)),
    }
}

#[inline]
pub fn shape_dist(s: &Shape, p: Vec3) -> f32 {
    match *s {
        Shape::Sphere { c, r } => (p - c).length() - r,
        Shape::Ellipsoid { c, r, inv } => {
            let q = inv * (p - c);
            let k0 = (q / r).length();
            let k1 = (q / (r * r)).length();
            if k1 < 1e-8 {
                -r.min_element()
            } else {
                k0 * (k0 - 1.0) / k1
            }
        }
        Shape::Capsule { a, b, r } => {
            let pa = p - a;
            let ba = b - a;
            let h = (pa.dot(ba) / ba.length_squared().max(1e-10)).clamp(0.0, 1.0);
            (pa - ba * h).length() - r
        }
        Shape::RoundCone { a, b, ra, rb } => round_cone(p, a, b, ra, rb),
        Shape::RoundBox { c, half, r, inv } => {
            let q = (inv * (p - c)).abs() - half + Vec3::splat(r);
            q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - r
        }
        Shape::Torus { c, big, small, inv } => {
            let q = inv * (p - c);
            let xz = (q.x * q.x + q.z * q.z).sqrt() - big;
            (xz * xz + q.y * q.y).sqrt() - small
        }
        Shape::HalfSpace { n, d } => d - p.dot(n),
    }
}

/// Exact round cone SDF (Inigo Quilez).
fn round_cone(p: Vec3, a: Vec3, b: Vec3, r1: f32, r2: f32) -> f32 {
    let ba = b - a;
    let l2 = ba.dot(ba);
    if l2 < 1e-10 {
        return (p - a).length() - r1.max(r2);
    }
    let rr = r1 - r2;
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;
    let pa = p - a;
    let y = pa.dot(ba);
    let z = y - l2;
    let xv = pa * l2 - ba * y;
    let x2 = xv.dot(xv);
    let y2 = y * y * l2;
    let z2 = z * z * l2;
    let k = rr.signum() * rr * rr * x2;
    if z.signum() * a2 * z2 > k {
        return (x2 + z2).sqrt() * il2 - r2;
    }
    if y.signum() * a2 * y2 < k {
        return (x2 + y2).sqrt() * il2 - r1;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - r1
}

#[inline]
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

#[inline]
pub fn smax(a: f32, b: f32, k: f32) -> f32 {
    -smin(-a, -b, k)
}

#[derive(Clone, Debug, Default)]
pub struct Sdf {
    pub prims: Vec<Prim>,
}

impl Sdf {
    pub fn new() -> Sdf {
        Sdf { prims: Vec::new() }
    }

    pub fn add(&mut self, p: Prim) -> &mut Self {
        self.prims.push(p);
        self
    }

    pub fn eval(&self, p: Vec3) -> f32 {
        let mut d = 1e9f32;
        for pr in &self.prims {
            d = Self::apply(pr, d, p);
        }
        d
    }

    #[inline]
    fn apply(pr: &Prim, d: f32, p: Vec3) -> f32 {
        match pr.op {
            Op::Union => {
                if pr.bound_dist(p) >= d {
                    return d;
                }
                d.min(pr.dist(p))
            }
            Op::Smooth(k) => {
                if pr.bound_dist(p) >= d + k {
                    return d;
                }
                smin(d, pr.dist(p), k)
            }
            Op::Subtract(k) => {
                let bd = pr.bound_dist(p);
                if bd > 0.0 && bd >= k - d {
                    return d;
                }
                let pd = pr.dist(p);
                if k > 0.0 {
                    smax(d, -pd, k)
                } else {
                    d.max(-pd)
                }
            }
            Op::Intersect(k) => {
                let pd = pr.dist(p);
                if k > 0.0 {
                    smax(d, pd, k)
                } else {
                    d.max(pd)
                }
            }
        }
    }

    /// Evaluates only the listed prims (in their original order).
    #[inline]
    pub fn eval_in(&self, p: Vec3, list: &[u16]) -> f32 {
        let mut d = 1e9f32;
        for &i in list {
            d = Self::apply(&self.prims[i as usize], d, p);
        }
        d
    }

    /// Prims that can influence points within the box [lo, hi] grown by `r`.
    pub fn candidates(&self, lo: Vec3, hi: Vec3, r: f32) -> Vec<u16> {
        let mut out = Vec::new();
        for (i, pr) in self.prims.iter().enumerate() {
            if matches!(pr.op, Op::Intersect(_)) || matches!(pr.shape, Shape::HalfSpace { .. }) {
                out.push(i as u16);
                continue;
            }
            let k = match pr.op {
                Op::Smooth(k) | Op::Subtract(k) => k,
                _ => 0.0,
            };
            let gap = (pr.lo - hi).max(lo - pr.hi).max(Vec3::ZERO).length();
            if gap <= r + k {
                out.push(i as u16);
            }
        }
        out
    }

    /// Tetrahedral gradient (4 evaluations).
    pub fn gradient_in(&self, p: Vec3, e: f32, list: &[u16]) -> Vec3 {
        let k0 = Vec3::new(1.0, -1.0, -1.0);
        let k1 = Vec3::new(-1.0, -1.0, 1.0);
        let k2 = Vec3::new(-1.0, 1.0, -1.0);
        let k3 = Vec3::new(1.0, 1.0, 1.0);
        let g = k0 * self.eval_in(p + k0 * e, list)
            + k1 * self.eval_in(p + k1 * e, list)
            + k2 * self.eval_in(p + k2 * e, list)
            + k3 * self.eval_in(p + k3 * e, list);
        g.normalize_or(Vec3::Y)
    }

    pub fn gradient(&self, p: Vec3, e: f32) -> Vec3 {
        let dx = self.eval(p + Vec3::X * e) - self.eval(p - Vec3::X * e);
        let dy = self.eval(p + Vec3::Y * e) - self.eval(p - Vec3::Y * e);
        let dz = self.eval(p + Vec3::Z * e) - self.eval(p - Vec3::Z * e);
        Vec3::new(dx, dy, dz).normalize_or(Vec3::Y)
    }

    /// Material and bone weights at a surface point.
    /// Returns (material of nearest additive prim, second material, blend, weights[(bone, w)]).
    pub fn attributes(&self, p: Vec3, sigma: f32) -> (u8, [(u8, f32); 4]) {
        let all: Vec<u16> = (0..self.prims.len() as u16).collect();
        self.attributes_in(p, sigma, &all)
    }

    pub fn attributes_in(&self, p: Vec3, sigma: f32, list: &[u16]) -> (u8, [(u8, f32); 4]) {
        let mut best = (f32::MAX, 0u8);
        let mut dists: Vec<(f32, usize)> = Vec::with_capacity(16);
        for &li in list {
            let i = li as usize;
            let pr = &self.prims[i];
            if matches!(pr.op, Op::Subtract(_) | Op::Intersect(_)) {
                continue;
            }
            if pr.bound_dist(p) > best.0.max(0.0) + sigma * 6.0 {
                continue;
            }
            let d = pr.dist(p);
            // later prims win ties (clothing layered over skin)
            if d <= best.0 + 0.0008 {
                best = (d, pr.mat);
            }
            dists.push((d.max(0.0), i));
        }
        let dmin = dists.iter().map(|x| x.0).fold(f32::MAX, f32::min);
        let mut acc: Vec<(u8, f32)> = Vec::with_capacity(8);
        let mut add = |b: u8, w: f32| {
            if w <= 0.0 {
                return;
            }
            if let Some(e) = acc.iter_mut().find(|e| e.0 == b) {
                e.1 += w;
            } else {
                acc.push((b, w));
            }
        };
        for (d, i) in dists {
            let w = (-(d - dmin) / sigma).exp();
            if w < 0.01 {
                continue;
            }
            let pr = &self.prims[i];
            let (wa, wb) = pr.bones.weights(p);
            add(pr.bones.a, w * wa);
            add(pr.bones.b, w * wb);
        }
        acc.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let total: f32 = acc.iter().take(4).map(|x| x.1).sum::<f32>().max(1e-6);
        let mut out = [(0u8, 0.0f32); 4];
        for (k, e) in acc.iter().take(4).enumerate() {
            out[k] = (e.0, e.1 / total);
        }
        (best.1, out)
    }
}

pub struct MeshOut {
    pub pos: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub idx: Vec<u32>,
    /// Candidate prim lists per super block, and each vertex's list index.
    pub lists: Vec<Vec<u16>>,
    pub vlist: Vec<u32>,
}

impl MeshOut {
    pub fn list_for(&self, v: usize) -> &[u16] {
        &self.lists[self.vlist[v] as usize]
    }
}

/// Sparse surface nets over the box [lo, hi] with the given cell size.
/// Space is split into super blocks with their own candidate prim lists, and
/// small blocks far from the surface are skipped.
pub fn surface_nets(sdf: &Sdf, lo: Vec3, hi: Vec3, cell: f32) -> MeshOut {
    let size = hi - lo;
    let nx = (size.x / cell).ceil() as usize + 1;
    let ny = (size.y / cell).ceil() as usize + 1;
    let nz = (size.z / cell).ceil() as usize + 1;
    let idx3 = |x: usize, y: usize, z: usize| (z * ny + y) * nx + x;
    let mut vals = vec![f32::NAN; nx * ny * nz];
    let mut exact = vec![false; nx * ny * nz];
    let gp = |x: usize, y: usize, z: usize| lo + Vec3::new(x as f32, y as f32, z as f32) * cell;

    const B: usize = 4;
    const SB: usize = 16;
    let half_diag = (B as f32) * cell * 0.5 * 3f32.sqrt();
    let margin = half_diag + cell * 2.5;
    let max_disp = sdf.prims.iter().map(|p| p.disp).fold(0.0, f32::max);
    let (sbx, sby, sbz) = (nx.div_ceil(SB), ny.div_ceil(SB), nz.div_ceil(SB));
    let mut lists: Vec<Vec<u16>> = Vec::with_capacity(sbx * sby * sbz);
    for zb in 0..sbz {
        for yb in 0..sby {
            for xb in 0..sbx {
                let a = gp(xb * SB, yb * SB, zb * SB);
                let b = gp(((xb + 1) * SB).min(nx - 1), ((yb + 1) * SB).min(ny - 1), ((zb + 1) * SB).min(nz - 1));
                lists.push(sdf.candidates(a, b, margin + cell * 2.0 + max_disp));
            }
        }
    }
    let sb_of = |x: usize, y: usize, z: usize| ((z / SB).min(sbz - 1) * sby + (y / SB).min(sby - 1)) * sbx + (x / SB).min(sbx - 1);

    let mut bz = 0;
    while bz < nz {
        let mut by = 0;
        while by < ny {
            let mut bx = 0;
            while bx < nx {
                let ex = (bx + B).min(nx - 1);
                let ey = (by + B).min(ny - 1);
                let ez = (bz + B).min(nz - 1);
                let list = &lists[sb_of(bx, by, bz)];
                let c = (gp(bx, by, bz) + gp(ex, ey, ez)) * 0.5;
                let d = if list.is_empty() { 1.0 } else { sdf.eval_in(c, list) };
                let far = d.abs() > margin;
                for z in bz..=ez {
                    for y in by..=ey {
                        for x in bx..=ex {
                            let i = idx3(x, y, z);
                            if far {
                                if vals[i].is_nan() {
                                    vals[i] = d;
                                }
                            } else if !exact[i] {
                                // points on block borders may belong to a neighbouring super block
                                let l = &lists[sb_of(x, y, z)];
                                vals[i] = if l.is_empty() { 1.0 } else { sdf.eval_in(gp(x, y, z), l) };
                                exact[i] = true;
                            }
                        }
                    }
                }
                bx += B;
            }
            by += B;
        }
        bz += B;
    }

    // vertices
    let cidx = |x: usize, y: usize, z: usize| (z * (ny - 1) + y) * (nx - 1) + x;
    let mut cell_vert = vec![u32::MAX; (nx - 1) * (ny - 1) * (nz - 1)];
    let mut pos: Vec<Vec3> = Vec::new();
    let mut vlist: Vec<u32> = Vec::new();
    const CORNERS: [(usize, usize, usize); 8] = [
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (1, 1, 0),
        (0, 0, 1),
        (1, 0, 1),
        (0, 1, 1),
        (1, 1, 1),
    ];
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for z in 0..nz - 1 {
        for y in 0..ny - 1 {
            for x in 0..nx - 1 {
                let mut v = [0f32; 8];
                let mut inside = 0;
                for (k, (dx, dy, dz)) in CORNERS.iter().enumerate() {
                    v[k] = vals[idx3(x + dx, y + dy, z + dz)];
                    if v[k] < 0.0 {
                        inside += 1;
                    }
                }
                if inside == 0 || inside == 8 {
                    continue;
                }
                let mut acc = Vec3::ZERO;
                let mut n = 0.0;
                for (a, b) in EDGES {
                    if (v[a] < 0.0) != (v[b] < 0.0) {
                        let t = v[a] / (v[a] - v[b]);
                        let pa = Vec3::new(CORNERS[a].0 as f32, CORNERS[a].1 as f32, CORNERS[a].2 as f32);
                        let pb = Vec3::new(CORNERS[b].0 as f32, CORNERS[b].1 as f32, CORNERS[b].2 as f32);
                        acc += pa + (pb - pa) * t;
                        n += 1.0;
                    }
                }
                let local = acc / n;
                let p = lo + (Vec3::new(x as f32, y as f32, z as f32) + local) * cell;
                cell_vert[cidx(x, y, z)] = pos.len() as u32;
                pos.push(p);
                vlist.push(sb_of(x, y, z) as u32);
            }
        }
    }

    // faces
    let mut idx: Vec<u32> = Vec::new();
    let mut emit = |q: [u32; 4], flip: bool, pos: &Vec<Vec3>| {
        if q.contains(&u32::MAX) {
            return;
        }
        let q = if flip { [q[0], q[3], q[2], q[1]] } else { q };
        let d02 = pos[q[0] as usize].distance_squared(pos[q[2] as usize]);
        let d13 = pos[q[1] as usize].distance_squared(pos[q[3] as usize]);
        if d02 <= d13 {
            idx.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
        } else {
            idx.extend_from_slice(&[q[0], q[1], q[3], q[1], q[2], q[3]]);
        }
    };
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let v0 = vals[idx3(x, y, z)];
                let in0 = v0 < 0.0;
                if x + 1 < nx && y > 0 && z > 0 && y < ny - 1 && z < nz - 1 {
                    let v1 = vals[idx3(x + 1, y, z)];
                    if in0 != (v1 < 0.0) {
                        let q = [
                            cell_vert[cidx(x, y - 1, z - 1)],
                            cell_vert[cidx(x, y, z - 1)],
                            cell_vert[cidx(x, y, z)],
                            cell_vert[cidx(x, y - 1, z)],
                        ];
                        emit(q, !in0, &pos);
                    }
                }
                if y + 1 < ny && x > 0 && z > 0 && x < nx - 1 && z < nz - 1 {
                    let v1 = vals[idx3(x, y + 1, z)];
                    if in0 != (v1 < 0.0) {
                        let q = [
                            cell_vert[cidx(x - 1, y, z - 1)],
                            cell_vert[cidx(x - 1, y, z)],
                            cell_vert[cidx(x, y, z)],
                            cell_vert[cidx(x, y, z - 1)],
                        ];
                        emit(q, !in0, &pos);
                    }
                }
                if z + 1 < nz && x > 0 && y > 0 && x < nx - 1 && y < ny - 1 {
                    let v1 = vals[idx3(x, y, z + 1)];
                    if in0 != (v1 < 0.0) {
                        let q = [
                            cell_vert[cidx(x - 1, y - 1, z)],
                            cell_vert[cidx(x, y - 1, z)],
                            cell_vert[cidx(x, y, z)],
                            cell_vert[cidx(x - 1, y, z)],
                        ];
                        emit(q, !in0, &pos);
                    }
                }
            }
        }
    }

    // project vertices onto the surface and compute normals
    let e = cell * 0.5;
    let mut nrm = Vec::with_capacity(pos.len());
    for (i, p) in pos.iter_mut().enumerate() {
        let list = &lists[vlist[i] as usize];
        let d = sdf.eval_in(*p, list);
        let g = sdf.gradient_in(*p, e, list);
        *p += g * (-d).clamp(-cell * 0.5, cell * 0.5);
        nrm.push(sdf.gradient_in(*p, e * 0.7, list));
    }
    MeshOut {
        pos,
        nrm,
        idx,
        lists,
        vlist,
    }
}
