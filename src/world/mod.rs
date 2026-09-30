//! Locations of the game world: Sofía's bedroom, the family home and the neighbourhood plaza.

pub mod props;

use crate::gfx::mesh::{kind, m4, m4r, m4ry, Mat, MeshData};
use crate::gfx::{Draw, DrawPass, FrameScene, Gpu, Light, MeshId, Renderer};
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use props::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Loc {
    Bedroom,
    Home,
    Plaza,
}

impl Loc {
    pub fn name(self) -> &'static str {
        match self {
            Loc::Bedroom => "Habitación de Sofía",
            Loc::Home => "Casa",
            Loc::Plaza => "Plaza del barrio",
        }
    }
    pub fn from_str(s: &str) -> Option<Loc> {
        match s {
            "bedroom" | "habitacion" => Some(Loc::Bedroom),
            "home" | "casa" => Some(Loc::Home),
            "plaza" | "barrio" => Some(Loc::Plaza),
            _ => None,
        }
    }
}

pub struct WallGroup {
    pub mesh: MeshId,
    pub n: Vec3,
    pub p: Vec3,
}

#[derive(Clone)]
pub struct Prop {
    pub name: &'static str,
    pub mesh: MeshId,
    pub xf: Mat4,
    pub visible: bool,
    pub tint: Vec4,
    pub params: Vec4,
    pub pass: DrawPass,
    pub shadow: bool,
    /// Emissive strength follows this lamp (index into `lamps`).
    pub glow_lamp: Option<usize>,
    /// When false the lamp level only drives params.y (lamp shades).
    pub glow_tint: bool,
    pub wall: Option<usize>,
    pub highlight: f32,
}

#[derive(Clone, Debug)]
pub struct Lamp {
    pub name: &'static str,
    pub pos: Vec3,
    pub color: Vec3,
    pub radius: f32,
    pub on: bool,
    /// Automatically switched on in the evening.
    pub auto_night: bool,
    /// Daylight fill lights (window glow) scale with daylight instead.
    pub daylight: bool,
    pub level: f32,
}

#[derive(Clone, Debug)]
pub struct Interactable {
    pub id: &'static str,
    pub label: String,
    pub pos: Vec3,
    pub stand: Vec3,
    pub face: Vec3,
    pub radius: f32,
    pub enabled: bool,
    pub prop: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub id: &'static str,
    pub pos: Vec3,
    pub yaw: f32,
    pub feet: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CamRig {
    pub dist: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub fov: f32,
    pub look_height: f32,
    pub min_dist: f32,
    pub max_dist: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowDef {
    pub center: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    /// Normal pointing into the room.
    pub inward: Vec3,
}

pub struct Location {
    pub id: Loc,
    pub base: Vec<MeshId>,
    pub walls: Vec<WallGroup>,
    pub shadow_only: Vec<MeshId>,
    pub props: Vec<Prop>,
    pub lamps: Vec<Lamp>,
    pub colliders: Vec<(Vec2, Vec2)>,
    pub bounds: (Vec2, Vec2),
    pub interior: bool,
    pub spawns: Vec<(&'static str, Vec3, f32)>,
    pub interact: Vec<Interactable>,
    pub seats: Vec<Seat>,
    pub cam: CamRig,
    pub shadow_center: Vec3,
    pub shadow_radius: f32,
    pub windows: Vec<WindowDef>,
    pub beam: MeshId,
    pub ambient_tint: Vec3,
}

impl Location {
    pub fn prop_index(&self, name: &str) -> Option<usize> {
        self.props.iter().position(|p| p.name == name)
    }
    pub fn set_prop_visible(&mut self, name: &str, v: bool) {
        for p in self.props.iter_mut().filter(|p| p.name == name) {
            p.visible = v;
        }
    }
    pub fn lamp_mut(&mut self, name: &str) -> Option<&mut Lamp> {
        self.lamps.iter_mut().find(|l| l.name == name)
    }
    pub fn spawn(&self, name: &str) -> (Vec3, f32) {
        self.spawns
            .iter()
            .find(|s| s.0 == name)
            .map(|s| (s.1, s.2))
            .unwrap_or_else(|| (self.spawns[0].1, self.spawns[0].2))
    }
    pub fn seat(&self, id: &str) -> Option<&Seat> {
        self.seats.iter().find(|s| s.id == id)
    }
    pub fn interactable(&self, id: &str) -> Option<&Interactable> {
        self.interact.iter().find(|i| i.id == id)
    }
    pub fn interactable_mut(&mut self, id: &str) -> Option<&mut Interactable> {
        self.interact.iter_mut().find(|i| i.id == id)
    }

    /// Resolves circle-vs-box collisions and room bounds.
    pub fn collide(&self, mut p: Vec3, r: f32) -> Vec3 {
        let (lo, hi) = self.bounds;
        p.x = p.x.clamp(lo.x + r, hi.x - r);
        p.z = p.z.clamp(lo.y + r, hi.y - r);
        for _ in 0..2 {
            for (a, b) in &self.colliders {
                let q = Vec2::new(p.x.clamp(a.x, b.x), p.z.clamp(a.y, b.y));
                let d = Vec2::new(p.x, p.z) - q;
                let l = d.length();
                if l < r {
                    if l > 1e-5 {
                        let push = d / l * (r - l);
                        p.x += push.x;
                        p.z += push.y;
                    } else {
                        // inside: push out along the smallest axis
                        let dx0 = p.x - a.x;
                        let dx1 = b.x - p.x;
                        let dz0 = p.z - a.y;
                        let dz1 = b.y - p.z;
                        let m = dx0.min(dx1).min(dz0).min(dz1);
                        if m == dx0 {
                            p.x = a.x - r;
                        } else if m == dx1 {
                            p.x = b.x + r;
                        } else if m == dz0 {
                            p.z = a.y - r;
                        } else {
                            p.z = b.y + r;
                        }
                    }
                }
            }
        }
        p
    }

    pub fn blocked(&self, p: Vec3, r: f32) -> bool {
        let (lo, hi) = self.bounds;
        if p.x < lo.x + r || p.x > hi.x - r || p.z < lo.y + r || p.z > hi.y - r {
            return true;
        }
        self.colliders.iter().any(|(a, b)| {
            let q = Vec2::new(p.x.clamp(a.x, b.x), p.z.clamp(a.y, b.y));
            (Vec2::new(p.x, p.z) - q).length() < r
        })
    }

    /// Straight-line path with a simple detour around blocking furniture.
    pub fn path(&self, from: Vec3, to: Vec3, r: f32) -> Vec<Vec3> {
        let clear = |a: Vec3, b: Vec3| -> bool {
            let n = ((b - a).length() / 0.1).ceil() as i32;
            (1..=n).all(|i| !self.blocked(a.lerp(b, i as f32 / n as f32), r * 0.9))
        };
        if clear(from, to) || self.blocked(to, r * 0.9) {
            return vec![to];
        }
        // try waypoints around the midpoint
        let mid = (from + to) * 0.5;
        let dir = (to - from).normalize_or(Vec3::Z);
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        let mut best: Option<(f32, Vec3)> = None;
        for s in [0.6f32, -0.6, 1.0, -1.0, 1.5, -1.5, 2.0, -2.0] {
            let w = mid + side * s;
            if !self.blocked(w, r) && clear(from, w) && clear(w, to) {
                let len = (w - from).length() + (to - w).length();
                if best.map(|b| len < b.0).unwrap_or(true) {
                    best = Some((len, w));
                }
            }
        }
        match best {
            Some((_, w)) => vec![w, to],
            None => vec![to],
        }
    }

    /// Adds this location's geometry to the frame.
    pub fn draw(&self, scene: &mut FrameScene, cam_pos: Vec3, time: f32) {
        for &m in &self.base {
            scene.draws.push(Draw::new(m, Mat4::IDENTITY));
        }
        let mut hidden = vec![false; self.walls.len()];
        for (i, w) in self.walls.iter().enumerate() {
            let behind = (cam_pos - w.p).dot(w.n) < -0.05;
            hidden[i] = behind;
            let mut d = Draw::new(w.mesh, Mat4::IDENTITY);
            if behind {
                d = d.shadow_only();
            }
            scene.draws.push(d);
        }
        for &m in &self.shadow_only {
            scene.draws.push(Draw::new(m, Mat4::IDENTITY).shadow_only());
        }
        for p in &self.props {
            if !p.visible {
                continue;
            }
            let mut tint = p.tint;
            let mut params = p.params;
            if let Some(li) = p.glow_lamp {
                let lvl = self.lamps[li].level;
                if !p.glow_tint {
                    params.y = lvl;
                } else if p.pass == DrawPass::Opaque {
                    tint = Vec4::new(tint.x * lvl.max(0.02), tint.y * lvl.max(0.02), tint.z * lvl.max(0.02), tint.w);
                    params.y = lvl;
                } else {
                    tint.w *= lvl;
                }
            }
            params.y = params.y.max(p.highlight * (0.6 + 0.4 * (time * 3.0).sin()));
            let mut d = Draw::new(p.mesh, p.xf).tint(tint).params(params).pass(p.pass);
            d.shadow = p.shadow && p.pass == DrawPass::Opaque;
            if let Some(w) = p.wall {
                if hidden[w] {
                    d = d.shadow_only();
                    if p.pass != DrawPass::Opaque {
                        continue;
                    }
                }
            }
            scene.draws.push(d);
        }
    }

    pub fn lights(&self, out: &mut Vec<Light>) {
        for l in &self.lamps {
            if l.level > 0.01 {
                out.push(Light {
                    pos: l.pos,
                    radius: l.radius,
                    color: l.color * l.level,
                });
            }
        }
    }

    pub fn update_lamps(&mut self, night: f32, daylight: f32, dt: f32) {
        for l in &mut self.lamps {
            let target = if l.daylight {
                daylight
            } else if l.on || (l.auto_night && night > 0.5) {
                1.0
            } else {
                0.0
            };
            l.level += (target - l.level) * (1.0 - (-6.0 * dt).exp());
        }
    }
}

// ------------------------------------------------------------------ builder

struct Builder {
    base: MeshData,
    walls: Vec<(MeshData, Vec3, Vec3)>,
    shadow: MeshData,
    props: Vec<(Prop, MeshData)>,
    lamps: Vec<Lamp>,
    colliders: Vec<(Vec2, Vec2)>,
    interact: Vec<Interactable>,
    seats: Vec<Seat>,
    spawns: Vec<(&'static str, Vec3, f32)>,
    windows: Vec<WindowDef>,
}

struct Opening {
    at: f32,
    bottom: f32,
    width: f32,
    height: f32,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            base: MeshData::new(),
            walls: Vec::new(),
            shadow: MeshData::new(),
            props: Vec::new(),
            lamps: Vec::new(),
            colliders: Vec::new(),
            interact: Vec::new(),
            seats: Vec::new(),
            spawns: Vec::new(),
            windows: Vec::new(),
        }
    }

    fn collider(&mut self, c: Vec3, half_x: f32, half_z: f32) {
        self.colliders.push((Vec2::new(c.x - half_x, c.z - half_z), Vec2::new(c.x + half_x, c.z + half_z)));
    }

    /// Adds a prop (separate draw) and returns its index.
    fn prop(&mut self, name: &'static str, mesh: MeshData, xf: Mat4) -> usize {
        self.props.push((
            Prop {
                name,
                mesh: MeshId(0),
                xf,
                visible: true,
                tint: Vec4::ONE,
                params: Vec4::ZERO,
                pass: DrawPass::Opaque,
                shadow: true,
                glow_lamp: None,
                glow_tint: true,
                wall: None,
                highlight: 0.0,
            },
            mesh,
        ));
        self.props.len() - 1
    }

    fn lamp(&mut self, name: &'static str, pos: Vec3, color: Vec3, radius: f32, auto_night: bool) -> usize {
        self.lamps.push(Lamp {
            name,
            pos,
            color,
            radius,
            on: false,
            auto_night,
            daylight: false,
            level: 0.0,
        });
        self.lamps.len() - 1
    }

    fn daylight(&mut self, name: &'static str, pos: Vec3, color: Vec3, radius: f32) {
        self.lamps.push(Lamp {
            name,
            pos,
            color,
            radius,
            on: false,
            auto_night: false,
            daylight: true,
            level: 1.0,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn interact(&mut self, id: &'static str, label: &str, pos: Vec3, stand: Vec3, radius: f32, prop: Option<usize>) {
        self.interact.push(Interactable {
            id,
            label: label.to_string(),
            pos,
            stand,
            face: pos,
            radius,
            enabled: true,
            prop,
        });
    }

    /// Wall from `a` along `dir` of length `len`; the interior face lies on the line, `inward` points into the room.
    #[allow(clippy::too_many_arguments)]
    fn wall(&mut self, a: Vec3, dir: Vec3, len: f32, inward: Vec3, h: f32, openings: &[Opening], mat: &Mat, ext: &Mat) -> usize {
        let mut md = MeshData::new();
        let t = 0.14;
        let basis = Mat3::from_cols(dir, Vec3::Y, dir.cross(Vec3::Y));
        let q = Quat::from_mat3(&basis);
        let piece = |md: &mut MeshData, s0: f32, s1: f32, y0: f32, y1: f32| {
            if s1 - s0 < 0.005 || y1 - y0 < 0.005 {
                return;
            }
            let c = a + dir * ((s0 + s1) * 0.5) + Vec3::Y * ((y0 + y1) * 0.5) - inward * (t * 0.5);
            md.rbox(m4r(c, q), Vec3::new((s1 - s0) * 0.5, (y1 - y0) * 0.5, t * 0.5), 0.004, 1, mat);
            // exterior skin
            let ce = a + dir * ((s0 + s1) * 0.5) + Vec3::Y * ((y0 + y1) * 0.5) - inward * (t + 0.005);
            md.rbox(m4r(ce, q), Vec3::new((s1 - s0) * 0.5, (y1 - y0) * 0.5, 0.006), 0.0, 1, ext);
        };
        let mut s = 0.0;
        let mut ops: Vec<&Opening> = openings.iter().collect();
        ops.sort_by(|x, y| x.at.partial_cmp(&y.at).unwrap());
        for o in &ops {
            let o0 = o.at - o.width * 0.5;
            let o1 = o.at + o.width * 0.5;
            piece(&mut md, s, o0, 0.0, h);
            piece(&mut md, o0, o1, 0.0, o.bottom);
            piece(&mut md, o0, o1, o.bottom + o.height, h);
            s = o1;
        }
        piece(&mut md, s, len, 0.0, h);
        // baseboard (skip door openings)
        let skirt = Mat::new(0xf3f0ea, 0.5);
        let mut s = 0.0;
        let mut segs = Vec::new();
        for o in &ops {
            if o.bottom < 0.05 {
                segs.push((s, o.at - o.width * 0.5));
                s = o.at + o.width * 0.5;
            }
        }
        segs.push((s, len));
        for (s0, s1) in segs {
            if s1 - s0 > 0.02 {
                let c = a + dir * ((s0 + s1) * 0.5) + Vec3::Y * 0.045 + inward * 0.008;
                md.rbox(m4r(c, q), Vec3::new((s1 - s0) * 0.5, 0.045, 0.008), 0.003, 1, &skirt);
            }
        }
        let mid = a + dir * (len * 0.5);
        self.walls.push((md, inward, mid));
        self.walls.len() - 1
    }

    fn finish(self, gpu: &Gpu, r: &mut Renderer, id: Loc, bounds: (Vec2, Vec2), interior: bool, cam: CamRig, shadow_center: Vec3, shadow_radius: f32) -> Location {
        let mut base = Vec::new();
        if !self.base.is_empty() {
            base.push(r.upload_mesh(gpu, &self.base));
        }
        let walls = self
            .walls
            .into_iter()
            .map(|(md, n, p)| WallGroup {
                mesh: r.upload_mesh(gpu, &md),
                n,
                p,
            })
            .collect();
        let mut shadow_only = Vec::new();
        if !self.shadow.is_empty() {
            shadow_only.push(r.upload_mesh(gpu, &self.shadow));
        }
        let props = self
            .props
            .into_iter()
            .map(|(mut p, md)| {
                p.mesh = r.upload_mesh(gpu, &md);
                p
            })
            .collect();
        // unit beam box: x,z in [-0.5, 0.5], y in [0, 1]
        let mut beam = MeshData::new();
        let bm = Mat::new(0xfff0d8, 1.0).kind(kind::BEAM);
        let c = [
            Vec3::new(-0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, 0.5),
            Vec3::new(-0.5, 0.0, 0.5),
        ];
        for i in 0..4 {
            let a = c[i];
            let b = c[(i + 1) % 4];
            beam.quad([a, b, b + Vec3::Y, a + Vec3::Y], &bm);
        }
        let beam = r.upload_mesh(gpu, &beam);
        Location {
            id,
            base,
            walls,
            shadow_only,
            props,
            lamps: self.lamps,
            colliders: self.colliders,
            bounds,
            interior,
            spawns: self.spawns,
            interact: self.interact,
            seats: self.seats,
            cam,
            shadow_center,
            shadow_radius,
            windows: self.windows,
            beam,
            ambient_tint: Vec3::ONE,
        }
    }
}

fn warm() -> Vec3 {
    Vec3::new(1.0, 0.72, 0.45)
}

// ------------------------------------------------------------------ bedroom

pub fn build_bedroom(gpu: &Gpu, r: &mut Renderer) -> Location {
    let mut b = Builder::new();
    let (x0, x1, z0, z1, h) = (-2.3f32, 2.3f32, -2.0f32, 2.0f32, 2.7f32);
    let wall_mat = Mat::new(0xe6ddd3, 0.92).kind(kind::PLASTER);
    let accent = Mat::new(0xb9c4cf, 0.92).kind(kind::PLASTER);
    let ext = Mat::new(0xd8d0c4, 0.9).kind(kind::PLASTER);
    // floor
    b.base.plane(m4(Vec3::new(0.0, 0.0, 0.0)), Vec2::new(x1 - x0, z1 - z0), (4, 4), &Mat::new(0xb58762, 0.4).kind(kind::PLANKS));
    // ceiling (shadow only)
    b.shadow.cube(m4(Vec3::new(0.0, h + 0.07, 0.0)), Vec3::new(3.0, 0.07, 2.6), &wall_mat);
    // walls
    let win_c = 1.2;
    let back = b.wall(Vec3::new(x0, 0.0, z0), Vec3::X, x1 - x0, Vec3::Z, h, &[Opening { at: win_c - x0, bottom: 0.95, width: 1.3, height: 1.25 }], &accent, &ext);
    let left = b.wall(Vec3::new(x0, 0.0, z1), Vec3::NEG_Z, z1 - z0, Vec3::X, h, &[], &wall_mat, &ext);
    let right = b.wall(Vec3::new(x1, 0.0, z0), Vec3::Z, z1 - z0, Vec3::NEG_X, h, &[Opening { at: 3.1, bottom: 0.0, width: 0.9, height: 2.1 }], &wall_mat, &ext);
    let _front = b.wall(Vec3::new(x1, 0.0, z1), Vec3::NEG_X, x1 - x0, Vec3::NEG_Z, h, &[], &wall_mat, &ext);

    // window
    let wc = Vec3::new(win_c, 0.95 + 0.625, z0);
    let (frame, glass) = window_frame(1.3, 1.25, 0.14);
    let fi = b.prop("window_frame", frame, m4(wc - Vec3::Z * 0.07));
    b.props[fi].0.wall = Some(back);
    let gi = b.prop("window_glass", glass, m4(wc - Vec3::Z * 0.07));
    b.props[gi].0.pass = DrawPass::Transparent;
    b.props[gi].0.tint = Vec4::new(1.0, 1.0, 1.0, 0.08);
    b.props[gi].0.wall = Some(back);
    b.props[gi].0.shadow = false;
    b.windows.push(WindowDef {
        center: wc,
        right: Vec3::X * 0.62,
        up: Vec3::Y * 0.6,
        inward: Vec3::Z,
    });
    // curtains + rod
    let mut rod = MeshData::new();
    rod.capsule(Vec3::new(win_c - 1.0, 2.35, z0 + 0.12), Vec3::new(win_c + 1.0, 2.35, z0 + 0.12), 0.012, 8, &METAL_DARK);
    let ri = b.prop("rod", rod, Mat4::IDENTITY);
    b.props[ri].0.wall = Some(back);
    for (i, x) in [win_c - 0.82, win_c + 0.82].iter().enumerate() {
        let ci = b.prop(if i == 0 { "curtain_l" } else { "curtain_r" }, curtain(0.45, 2.2, 0xe8d8c0, 0.4), m4(Vec3::new(*x, 2.35, z0 + 0.1)));
        b.props[ci].0.wall = Some(back);
    }
    // outside: tree and neighbour facade visible through the window
    b.base.append_xf(&tree(7, 6.5), m4(Vec3::new(2.8, -1.2, -6.5)));
    let mut facade = MeshData::new();
    facade.cube(m4(Vec3::new(-1.0, 2.0, -11.0)), Vec3::new(6.0, 5.0, 0.3), &Mat::new(0xcfb9a0, 0.9).kind(kind::BRICK));
    for i in 0..4 {
        for j in 0..2 {
            facade.cube(
                m4(Vec3::new(-4.0 + i as f32 * 2.2, 2.0 + j as f32 * 2.5, -10.68)),
                Vec3::new(0.5, 0.7, 0.02),
                &Mat::new(0x2a3440, 0.1).emit(0.0),
            );
        }
    }
    b.base.append(&facade);
    b.base.plane(m4(Vec3::new(0.0, -1.2, -8.0)), Vec2::new(30.0, 12.0), (1, 1), &Mat::new(0x5a7a3a, 1.0).kind(kind::GRASS));

    // bed along the back wall, head at +X
    let bed_c = Vec3::new(1.3, 0.0, -1.52);
    b.base.append_xf(&bed(Mat::new(0x7fa7c9, 0.9).kind(kind::PLAID), Mat::new(0xf6f2ea, 0.9).kind(kind::FABRIC)), m4(bed_c));
    b.collider(bed_c, 1.0, 0.48);
    b.seats.push(Seat {
        id: "bed",
        pos: Vec3::new(0.95, 0.47, -1.08),
        yaw: 0.0,
        feet: 0.42,
    });
    // fairy lights over the bed
    let (wire, bulbs) = fairy_lights(2.2, 0.18, 16);
    let wi = b.prop("fairy_wire", wire, m4(Vec3::new(1.2, 2.25, z0 + 0.03)));
    b.props[wi].0.wall = Some(back);
    let fairy_lamp = b.lamp("fairy", Vec3::new(1.2, 2.0, z0 + 0.35), Vec3::new(1.0, 0.7, 0.4) * 1.3, 2.6, true);
    let bi = b.prop("fairy_bulbs", bulbs, m4(Vec3::new(1.2, 2.25, z0 + 0.03)));
    b.props[bi].0.glow_lamp = Some(fairy_lamp);
    b.props[bi].0.wall = Some(back);
    b.props[bi].0.shadow = false;

    // nightstand with alarm clock and phone
    let ns = Vec3::new(2.05, 0.0, -0.78);
    b.base.append_xf(&nightstand(), m4ry(ns, -std::f32::consts::FRAC_PI_2));
    b.base.append_xf(&alarm_clock(), m4ry(ns + Vec3::new(0.02, 0.585, 0.1), -1.9));
    b.collider(ns, 0.22, 0.22);
    let mut phone = MeshData::new();
    phone.rbox(Mat4::IDENTITY, Vec3::new(0.036, 0.0045, 0.074), 0.005, 2, &Mat::new(0x1c1c22, 0.3));
    let pi = b.prop("phone_table", phone, m4ry(ns + Vec3::new(-0.05, 0.59, -0.08), 0.3));
    b.interact("phone", "Revisar el teléfono", ns + Vec3::new(-0.05, 0.6, -0.08), Vec3::new(1.55, 0.0, -0.75), 0.9, Some(pi));
    // lamp on nightstand
    let mut nl = MeshData::new();
    nl.lathe(m4(ns + Vec3::new(0.0, 0.585, -0.1)), &[(0.0, 0.0), (0.06, 0.0), (0.05, 0.02), (0.015, 0.05), (0.012, 0.26), (0.0, 0.26)], 14, &Mat::new(0xd9c4a0, 0.3));
    b.base.append(&nl);
    let nlamp = b.lamp("night_lamp", ns + Vec3::new(0.0, 0.95, -0.1), warm() * 1.6, 2.2, true);
    let si = b.prop("night_shade", lamp_shade_cone(0.11, 0.08, 0.16, 0xf0e2c8), m4(ns + Vec3::new(0.0, 0.8, -0.1)));
    b.props[si].0.glow_lamp = Some(nlamp);
    b.props[si].0.glow_tint = false;
    b.props[si].0.shadow = false;

    // desk along the left wall
    let desk_c = Vec3::new(-1.99, 0.0, -0.5);
    b.base.append_xf(&desk(), m4ry(desk_c, std::f32::consts::FRAC_PI_2));
    b.collider(desk_c, 0.31, 0.7);
    let li = b.prop("laptop", laptop_base(), m4ry(desk_c + Vec3::new(0.02, 0.753, 0.0), std::f32::consts::FRAC_PI_2));
    let _ = li;
    let screen_lamp = b.lamp("laptop_glow", desk_c + Vec3::new(0.3, 0.95, 0.0), Vec3::new(0.55, 0.7, 1.0) * 0.8, 1.6, false);
    let sci = b.prop("laptop_screen", laptop_screen(), m4ry(desk_c + Vec3::new(0.02, 0.753, 0.0), std::f32::consts::FRAC_PI_2));
    b.props[sci].0.glow_lamp = Some(screen_lamp);
    b.props[sci].0.shadow = false;
    b.interact("laptop", "Usar el computador", desk_c + Vec3::new(0.05, 0.85, 0.0), Vec3::new(-1.3, 0.0, -0.5), 0.9, Some(sci));
    // desk lamp
    b.base.append_xf(&desk_lamp(0x2b5d6e), m4ry(desk_c + Vec3::new(-0.12, 0.753, -0.5), 0.9));
    let dlamp = b.lamp("desk_lamp", desk_c + Vec3::new(0.05, 1.05, -0.42), warm() * 2.2, 2.8, true);
    let dsi = b.prop("desk_shade", lamp_shade_cone(0.04, 0.07, 0.09, 0x2b5d6e), m4r(desk_c + Vec3::new(0.0, 1.1, -0.43), Quat::from_rotation_z(0.9)));
    b.props[dsi].0.glow_lamp = Some(dlamp);
    b.props[dsi].0.glow_tint = false;
    b.props[dsi].0.shadow = false;
    // desk items
    b.base.append_xf(&mug(0xe8765a), m4ry(desk_c + Vec3::new(0.1, 0.753, 0.4), 0.4));
    b.base.append_xf(&books(0.25, 3), m4ry(desk_c + Vec3::new(-0.2, 0.753, 0.52), std::f32::consts::FRAC_PI_2));
    b.base.append_xf(&plant(11, 0xf2ede4, 0.8), m4(desk_c + Vec3::new(-0.15, 0.753, -0.6)));
    let pgi = b.prop("piggy", piggy_bank(), m4ry(desk_c + Vec3::new(0.08, 0.753, 0.25), 0.3));
    b.interact("piggy", "Alcancía: ver mis metas", desk_c + Vec3::new(0.08, 0.83, 0.25), Vec3::new(-1.3, 0.0, -0.1), 0.8, Some(pgi));
    // office chair
    let chair_c = Vec3::new(-1.38, 0.0, -0.5);
    b.prop("desk_chair", office_chair(0x3a4a5a), m4ry(chair_c, -std::f32::consts::FRAC_PI_2));
    b.collider(chair_c, 0.22, 0.22);
    b.seats.push(Seat {
        id: "desk_chair",
        pos: Vec3::new(-1.4, 0.49, -0.5),
        yaw: -std::f32::consts::FRAC_PI_2,
        feet: 0.38,
    });
    // shelf above the desk with books & trophies
    let sh = Vec3::new(x0 + 0.12, 1.6, -0.5);
    let mut shelf_md = shelf(1.1);
    shelf_md.append_xf(&books(0.6, 21), m4(Vec3::new(-0.2, 0.015, 0.0)));
    shelf_md.append_xf(&plant(5, 0x6a8a7a, 0.55), m4(Vec3::new(0.35, 0.015, 0.0)));
    let mut trophy = MeshData::new();
    trophy.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.04, 0.0), (0.04, 0.02), (0.012, 0.03), (0.01, 0.09), (0.035, 0.11), (0.04, 0.17), (0.0, 0.17)], 14, &Mat::new(0xd9b24a, 0.25).metal(1.0));
    shelf_md.append_xf(&trophy, m4(Vec3::new(0.18, 0.015, 0.02)));
    let shi = b.prop("shelf", shelf_md, m4ry(sh, std::f32::consts::FRAC_PI_2));
    b.props[shi].0.wall = Some(left);
    // corkboard on the back wall with goal notes
    let cb = Vec3::new(-1.35, 1.55, z0 + 0.02);
    let cbi = b.prop("corkboard", corkboard(0.9, 0.62), m4(cb));
    b.props[cbi].0.wall = Some(back);
    b.interact("corkboard", "Tablero de metas", cb, Vec3::new(-1.35, 0.0, -1.1), 0.9, Some(cbi));
    let notes = [(0xfff3a0, -0.25, 0.1, 0xe04040), (0xa8e0ff, 0.05, 0.14, 0x40a0e0), (0xffc8d8, 0.28, 0.05, 0x40c080), (0xffffff, -0.1, -0.15, 0xe0a040)];
    for (i, (c, x, y, pin)) in notes.iter().enumerate() {
        let name = ["note_0", "note_1", "note_2", "note_3"][i];
        let ni = b.prop(name, note(*c, 0.16, 0.14, *pin), m4r(cb + Vec3::new(*x, *y, 0.022), Quat::from_rotation_z(0.08 * (i as f32 - 1.5))));
        b.props[ni].0.wall = Some(back);
        b.props[ni].0.visible = i == 0;
    }
    // posters
    let p1 = b.prop("poster_1", poster_panel(0.5, 0.7), m4ry(Vec3::new(x1 - 0.012, 1.6, -1.4), -std::f32::consts::FRAC_PI_2));
    b.props[p1].0.params = Vec4::new(3.0, 0.0, 0.0, 0.0);
    b.props[p1].0.wall = Some(right);
    let p2 = b.prop("poster_2", poster_panel(0.42, 0.56), m4ry(Vec3::new(x0 + 0.012, 1.55, 0.75), std::f32::consts::FRAC_PI_2));
    b.props[p2].0.params = Vec4::new(8.0, 0.0, 0.0, 0.0);
    b.props[p2].0.wall = Some(left);
    // wardrobe by the left wall front
    let wd = Vec3::new(-2.0, 0.0, 1.2);
    let wi2 = b.prop("wardrobe", wardrobe(0xe9e4dc), m4ry(wd, std::f32::consts::FRAC_PI_2));
    let _ = wi2;
    b.collider(wd, 0.3, 0.6);
    b.interact("wardrobe", "Clóset", wd + Vec3::new(0.3, 1.1, 0.0), Vec3::new(-1.35, 0.0, 1.2), 0.8, None);
    // rug & beanbag
    b.base.append_xf(&rug(2.0, 1.4, 0xd8b8a0), m4(Vec3::new(0.1, 0.0, 0.25)));
    b.base.append_xf(&beanbag(0xe0a050), m4(Vec3::new(0.9, 0.0, 0.75)));
    b.collider(Vec3::new(0.9, 0.0, 0.75), 0.35, 0.35);
    // door
    let door_c = Vec3::new(x1 - 0.02, 0.0, z0 + 3.1);
    let di = b.prop("door", door(0xefece6), m4ry(door_c, -std::f32::consts::FRAC_PI_2));
    b.props[di].0.wall = Some(right);
    b.interact("door", "Ir a la sala", door_c + Vec3::new(-0.1, 1.0, 0.0), Vec3::new(1.7, 0.0, 1.1), 0.8, Some(di));
    // pendant lamp
    let (cord, shade) = pendant_lamp(0xf0ebe0);
    b.base.append_xf(&cord, m4(Vec3::new(0.1, h, 0.1)));
    let plamp = b.lamp("ceiling", Vec3::new(0.1, h - 1.0, 0.1), warm() * 3.2, 6.5, true);
    let psi = b.prop("pendant_shade", shade, m4(Vec3::new(0.1, h, 0.1)));
    b.props[psi].0.glow_lamp = Some(plamp);
    b.props[psi].0.glow_tint = false;
    b.props[psi].0.shadow = false;
    b.interact("bed", "Dormir", Vec3::new(1.2, 0.6, -1.4), Vec3::new(0.95, 0.0, -0.75), 1.0, None);
    b.interact("window", "Mirar por la ventana", wc, Vec3::new(win_c, 0.0, -0.8), 0.8, None);
    b.interact("lamp", "Encender / apagar luz", ns + Vec3::new(0.0, 0.9, -0.1), Vec3::new(1.55, 0.0, -0.45), 0.7, None);

    // purchasable items (hidden until bought)
    let items: Vec<(&'static str, MeshData, Mat4)> = vec![
        ("item_headphones", headphones(0x2a2a30), m4ry(desk_c + Vec3::new(0.05, 0.765, -0.25), 0.6) * Mat4::from_rotation_x(-1.4)),
        ("item_sneakers", shoebox(0xe07a3a), m4ry(Vec3::new(-1.6, 0.0, 1.85), 0.1)),
        ("item_guitar", guitar(), m4r(Vec3::new(0.05, 0.0, -1.83), Quat::from_rotation_x(-0.2))),
        ("item_skate", skateboard(), m4ry(Vec3::new(1.95, 0.0, 0.55), 0.2)),
        ("item_bike", bicycle(0x2a7fbf), m4ry(Vec3::new(-0.2, 0.0, 1.62), std::f32::consts::FRAC_PI_2)),
        ("item_speaker", {
            let mut m = MeshData::new();
            m.rbox(m4(Vec3::new(0.0, 0.08, 0.0)), Vec3::new(0.06, 0.08, 0.06), 0.02, 2, &Mat::new(0x303038, 0.6).kind(kind::FABRIC));
            m.disc(m4r(Vec3::new(0.0, 0.09, 0.061), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.035, 14, &Mat::new(0x151518, 0.4));
            m
        }, m4ry(desk_c + Vec3::new(-0.15, 0.753, -0.05), 1.3)),
    ];
    for (name, md, xf) in items {
        let i = b.prop(name, md, xf);
        b.props[i].0.visible = false;
    }
    // daylight fill through the window
    b.daylight("sky_fill", Vec3::new(win_c, 1.6, z0 + 0.8), Vec3::new(0.9, 0.95, 1.0) * 1.6, 5.5);
    b.spawns.push(("start", Vec3::new(0.95, 0.0, -0.8), 0.0));
    b.spawns.push(("door", Vec3::new(1.6, 0.0, 1.1), -std::f32::consts::FRAC_PI_2));
    let cam = CamRig {
        dist: 5.2,
        pitch: 0.55,
        yaw: 0.35,
        fov: 42.0,
        look_height: 1.0,
        min_dist: 2.5,
        max_dist: 7.0,
    };
    let mut loc = b.finish(gpu, r, Loc::Bedroom, (Vec2::new(x0, z0), Vec2::new(x1, z1)), true, cam, Vec3::new(0.0, 1.2, 0.0), 4.2);
    loc.ambient_tint = Vec3::new(1.0, 0.95, 0.9);
    loc
}

// ------------------------------------------------------------------ home (living + kitchen)

pub fn build_home(gpu: &Gpu, r: &mut Renderer) -> Location {
    let mut b = Builder::new();
    let (x0, x1, z0, z1, h) = (-3.6f32, 3.6f32, -2.6f32, 2.6f32, 2.75f32);
    let wall_mat = Mat::new(0xefe7dc, 0.92).kind(kind::PLASTER);
    let accent = Mat::new(0xd9c3a8, 0.92).kind(kind::PLASTER);
    let ext = Mat::new(0xd8d0c4, 0.9).kind(kind::PLASTER);
    // floors: planks in the living area, tiles in the kitchen
    b.base.plane(m4(Vec3::new(1.0, 0.0, 0.0)), Vec2::new(5.2, z1 - z0), (4, 4), &Mat::new(0xa47650, 0.4).kind(kind::PLANKS));
    b.base.plane(m4(Vec3::new(-2.6, 0.0, 0.0)), Vec2::new(2.0, z1 - z0), (2, 4), &Mat::new(0xd8d2c8, 0.3).kind(kind::TILES));
    b.shadow.cube(m4(Vec3::new(0.0, h + 0.07, 0.0)), Vec3::new(4.2, 0.07, 3.2), &wall_mat);
    let back = b.wall(Vec3::new(x0, 0.0, z0), Vec3::X, x1 - x0, Vec3::Z, h, &[Opening { at: 1.7, bottom: 1.05, width: 1.4, height: 1.1 }, Opening { at: 6.4, bottom: 0.0, width: 0.9, height: 2.1 }], &wall_mat, &ext);
    let left = b.wall(Vec3::new(x0, 0.0, z1), Vec3::NEG_Z, z1 - z0, Vec3::X, h, &[Opening { at: 1.3, bottom: 0.0, width: 1.0, height: 2.15 }], &accent, &ext);
    let right = b.wall(Vec3::new(x1, 0.0, z0), Vec3::Z, z1 - z0, Vec3::NEG_X, h, &[Opening { at: 3.6, bottom: 0.9, width: 1.5, height: 1.4 }], &wall_mat, &ext);
    let _front = b.wall(Vec3::new(x1, 0.0, z1), Vec3::NEG_X, x1 - x0, Vec3::NEG_Z, h, &[], &wall_mat, &ext);

    // kitchen window (back wall)
    let wc = Vec3::new(x0 + 1.7, 1.05 + 0.55, z0);
    let (frame, glass) = window_frame(1.4, 1.1, 0.14);
    let fi = b.prop("kwin_frame", frame, m4(wc - Vec3::Z * 0.07));
    b.props[fi].0.wall = Some(back);
    let gi = b.prop("kwin_glass", glass, m4(wc - Vec3::Z * 0.07));
    b.props[gi].0.pass = DrawPass::Transparent;
    b.props[gi].0.tint = Vec4::new(1.0, 1.0, 1.0, 0.08);
    b.props[gi].0.wall = Some(back);
    b.windows.push(WindowDef {
        center: wc,
        right: Vec3::X * 0.66,
        up: Vec3::Y * 0.5,
        inward: Vec3::Z,
    });
    // living room window (right wall)
    let wc2 = Vec3::new(x1, 0.9 + 0.7, z0 + 3.6);
    let (frame2, glass2) = window_frame(1.5, 1.4, 0.14);
    let f2 = b.prop("lwin_frame", frame2, m4ry(wc2 + Vec3::X * 0.07, -std::f32::consts::FRAC_PI_2));
    b.props[f2].0.wall = Some(right);
    let g2 = b.prop("lwin_glass", glass2, m4ry(wc2 + Vec3::X * 0.07, -std::f32::consts::FRAC_PI_2));
    b.props[g2].0.pass = DrawPass::Transparent;
    b.props[g2].0.tint = Vec4::new(1.0, 1.0, 1.0, 0.08);
    b.props[g2].0.wall = Some(right);
    for (i, dz) in [-0.95f32, 0.95].iter().enumerate() {
        let ci = b.prop(if i == 0 { "lcurt_a" } else { "lcurt_b" }, curtain(0.5, 2.3, 0x8a9a7a, 0.3), m4ry(Vec3::new(x1 - 0.1, 2.45, wc2.z + dz), -std::f32::consts::FRAC_PI_2));
        b.props[ci].0.wall = Some(right);
    }
    // outside greenery
    b.base.append_xf(&tree(3, 6.0), m4(Vec3::new(-2.0, -1.0, -6.0)));
    b.base.append_xf(&tree(9, 7.0), m4(Vec3::new(7.0, -1.0, 1.0)));
    b.base.plane(m4(Vec3::new(0.0, -1.0, 0.0)), Vec2::new(40.0, 40.0), (1, 1), &Mat::new(0x5a7a3a, 1.0).kind(kind::GRASS));

    // kitchen along the back-left
    let kc = Vec3::new(x0 + 1.75, 0.0, z0 + 0.31);
    b.base.append_xf(&kitchen_counter(2.3), m4(kc));
    b.base.append_xf(&sink(), m4(kc + Vec3::new(0.0, 0.0, 0.02)));
    b.base.append_xf(&stove(), m4(kc + Vec3::new(0.75, 0.0, 0.0)));
    b.base.append_xf(&fruit_bowl(), m4(kc + Vec3::new(-0.7, 0.91, 0.05)));
    let mut kettle = MeshData::new();
    kettle.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.08, 0.0), (0.09, 0.08), (0.06, 0.18), (0.02, 0.2), (0.0, 0.2)], 16, &Mat::new(0xd04a3a, 0.25));
    b.base.append_xf(&kettle, m4(kc + Vec3::new(0.62, 0.92, -0.08)));
    b.collider(kc, 1.15, 0.32);
    let uc = b.prop("upper_cab", upper_cabinets(0.9), m4(Vec3::new(x0 + 0.65, 1.95, z0 + 0.17)));
    b.props[uc].0.wall = Some(back);
    let fr = Vec3::new(x0 + 0.38, 0.0, z0 + 1.1);
    b.base.append_xf(&fridge(), m4ry(fr, std::f32::consts::FRAC_PI_2));
    b.collider(fr, 0.34, 0.34);
    // dining table
    let dt = Vec3::new(-1.6, 0.0, 0.5);
    b.base.append_xf(&dining_table(), m4(dt));
    b.collider(dt, 0.82, 0.47);
    for (i, (dx, dz, yaw)) in [(-0.4, -0.62, 0.0), (0.4, -0.62, 0.0), (-0.4, 0.62, std::f32::consts::PI), (0.4, 0.62, std::f32::consts::PI)].iter().enumerate() {
        let c = dt + Vec3::new(*dx, 0.0, *dz);
        b.base.append_xf(&dining_chair(), m4ry(c, *yaw));
        b.seats.push(Seat {
            id: ["dchair_0", "dchair_1", "dchair_2", "dchair_3"][i],
            pos: c + Vec3::new(0.0, 0.47, 0.0),
            yaw: if *yaw == 0.0 { 0.0 } else { std::f32::consts::PI },
            feet: 0.3,
        });
    }
    b.base.append_xf(&mug(0x4a8ab0), m4(dt + Vec3::new(0.3, 0.775, 0.1)));
    let mut vase = MeshData::new();
    vase.lathe(Mat4::IDENTITY, &[(0.0, 0.0), (0.05, 0.0), (0.07, 0.1), (0.03, 0.22), (0.035, 0.26), (0.0, 0.25)], 16, &Mat::new(0x3a6a8a, 0.15));
    b.base.append_xf(&vase, m4(dt + Vec3::new(-0.2, 0.775, -0.05)));
    let mut flowers = MeshData::new();
    for i in 0..6 {
        let a = i as f32 * 1.05;
        let tip = Vec3::new(a.cos() * 0.08, 0.42 + 0.04 * (i % 2) as f32, a.sin() * 0.08);
        flowers.capsule(Vec3::new(0.0, 0.2, 0.0), tip, 0.003, 4, &Mat::new(0x4a7a3a, 0.7));
        flowers.sphere(m4(tip), 0.028, 8, 6, &Mat::new([0xf2c84a, 0xe86a7a, 0xf2f0ea][i % 3], 0.8).kind(kind::FOLIAGE));
    }
    b.base.append_xf(&flowers, m4(dt + Vec3::new(-0.2, 0.775, -0.05)));

    // living area: sofa facing +X toward the TV on the right wall
    let sofa_c = Vec3::new(1.05, 0.0, -0.1);
    b.base.append_xf(&sofa(0x5f7f9a), m4ry(sofa_c, std::f32::consts::FRAC_PI_2));
    b.collider(sofa_c, 0.48, 1.02);
    b.seats.push(Seat {
        id: "sofa",
        pos: sofa_c + Vec3::new(0.08, 0.4, 0.3),
        yaw: std::f32::consts::FRAC_PI_2,
        feet: 0.4,
    });
    let ct = Vec3::new(2.2, 0.0, -0.1);
    b.base.append_xf(&coffee_table(), m4ry(ct, std::f32::consts::FRAC_PI_2));
    b.collider(ct, 0.3, 0.55);
    b.base.append_xf(&rug(2.6, 1.9, 0xcdb89c), m4ry(Vec3::new(2.0, 0.0, -0.1), std::f32::consts::FRAC_PI_2));
    let tvs = Vec3::new(x1 - 0.22, 0.0, -0.1);
    b.base.append_xf(&tv_stand(), m4ry(tvs, -std::f32::consts::FRAC_PI_2));
    b.collider(tvs, 0.2, 0.8);
    let (tvm, tvscreen) = tv(1.3);
    let tvx = m4ry(Vec3::new(x1 - 0.1, 1.35, -0.1), -std::f32::consts::FRAC_PI_2);
    let ti = b.prop("tv", tvm, tvx);
    b.props[ti].0.wall = Some(right);
    let tv_lamp = b.lamp("tv_glow", Vec3::new(x1 - 0.8, 1.3, -0.1), Vec3::new(0.5, 0.65, 1.0) * 1.2, 2.5, true);
    let tsi = b.prop("tv_screen", tvscreen, tvx);
    b.props[tsi].0.glow_lamp = Some(tv_lamp);
    b.props[tsi].0.wall = Some(right);
    b.props[tsi].0.shadow = false;
    let (fl, fls) = floor_lamp();
    let flc = Vec3::new(0.5, 0.0, -1.95);
    b.base.append_xf(&fl, m4(flc));
    let flamp = b.lamp("floor_lamp", flc + Vec3::new(0.0, 1.45, 0.0), warm() * 2.8, 4.0, true);
    let fsi = b.prop("floor_shade", fls, m4(flc));
    b.props[fsi].0.glow_lamp = Some(flamp);
    b.props[fsi].0.glow_tint = false;
    b.props[fsi].0.shadow = false;
    b.collider(flc, 0.16, 0.16);
    b.base.append_xf(&plant(17, 0xd9cfc2, 1.9), m4(Vec3::new(3.2, 0.0, -2.2)));
    b.collider(Vec3::new(3.2, 0.0, -2.2), 0.2, 0.2);
    b.base.append_xf(&plant(23, 0x5a6a7a, 1.4), m4(Vec3::new(-3.25, 0.0, 2.25)));
    // bookcase on the back wall
    let bc = Vec3::new(2.0, 0.0, z0 + 0.2);
    let mut bookcase = MeshData::new();
    let bc_wood = Mat::new(0x8a6446, 0.55).kind(kind::WOOD);
    bookcase.rbox(m4(Vec3::new(0.0, 0.9, -0.17)), Vec3::new(0.5, 0.9, 0.012), 0.004, 1, &bc_wood);
    for x in [-0.49f32, 0.49] {
        bookcase.rbox(m4(Vec3::new(x, 0.9, 0.0)), Vec3::new(0.014, 0.9, 0.18), 0.004, 1, &bc_wood);
    }
    bookcase.rbox(m4(Vec3::new(0.0, 1.79, 0.0)), Vec3::new(0.5, 0.014, 0.18), 0.004, 1, &bc_wood);
    bookcase.rbox(m4(Vec3::new(0.0, 0.03, 0.0)), Vec3::new(0.5, 0.03, 0.18), 0.004, 1, &bc_wood);
    for i in 0..4 {
        let y = 0.22 + i as f32 * 0.42;
        bookcase.rbox(m4(Vec3::new(0.0, y, 0.0)), Vec3::new(0.48, 0.012, 0.17), 0.003, 1, &bc_wood);
        if i != 2 {
            bookcase.append_xf(&books(if i == 1 { 0.55 } else { 0.8 }, 40 + i), m4(Vec3::new(if i == 1 { -0.18 } else { 0.0 }, y + 0.012, 0.0)));
        }
    }
    bookcase.append_xf(&plant(33, 0xe8e2d6, 0.7), m4(Vec3::new(0.22, 0.652, 0.0)));
    bookcase.append_xf(&picture_frame(0.2, 0.26), m4(Vec3::new(-0.15, 1.2, -0.12)) * Mat4::from_rotation_x(-0.15));
    bookcase.append_xf(&mug(0xd8a040), m4(Vec3::new(0.25, 1.072, 0.0)));
    b.base.append_xf(&bookcase, m4(bc));
    b.collider(bc, 0.5, 0.2);
    // family photos
    for (i, (x, y, w, hh)) in [(0.2, 1.7, 0.35, 0.28), (0.7, 1.55, 0.25, 0.32), (0.55, 1.95, 0.3, 0.22)].iter().enumerate() {
        let pi = b.prop(["photo_0", "photo_1", "photo_2"][i], picture_frame(*w, *hh), m4(Vec3::new(*x, *y, z0 + 0.015)));
        b.props[pi].0.wall = Some(back);
    }
    let art = b.prop("art", poster_panel(0.8, 0.55), m4ry(Vec3::new(x0 + 0.012, 1.65, 0.5), std::f32::consts::FRAC_PI_2));
    b.props[art].0.params = Vec4::new(14.0, 0.0, 0.0, 0.0);
    b.props[art].0.wall = Some(left);
    // doors
    let d1 = Vec3::new(x0 + 0.02, 0.0, z1 - 1.3);
    let di = b.prop("front_door", door(0x6a4a3a), m4ry(d1, std::f32::consts::FRAC_PI_2));
    b.props[di].0.wall = Some(left);
    b.interact("front_door", "Salir a la plaza", d1 + Vec3::new(0.1, 1.0, 0.0), Vec3::new(x0 + 0.7, 0.0, z1 - 1.3), 0.8, Some(di));
    let d2 = Vec3::new(x0 + 6.4, 0.0, z0 + 0.02);
    let d2i = b.prop("hall_door", door(0xefece6), m4(d2));
    b.props[d2i].0.wall = Some(back);
    b.interact("hall_door", "Ir a mi habitación", d2 + Vec3::new(0.0, 1.0, 0.1), Vec3::new(2.8, 0.0, z0 + 0.8), 0.8, Some(d2i));
    b.interact("fridge", "Refrigerador", fr + Vec3::new(0.35, 1.0, 0.0), fr + Vec3::new(0.85, 0.0, 0.0), 0.8, None);
    b.interact("tv", "Ver televisión", Vec3::new(x1 - 0.1, 1.35, -0.1), Vec3::new(1.7, 0.0, 0.6), 1.0, Some(tsi));
    // ceiling lights
    for (name, p) in [("ceil_kitchen", Vec3::new(-1.8, h - 0.9, 0.2)), ("ceil_living", Vec3::new(1.8, h - 0.9, -0.2))] {
        let (cord, shade) = pendant_lamp(0xe8dcc8);
        b.base.append_xf(&cord, m4(Vec3::new(p.x, h, p.z)));
        let li = b.lamp(name, p, warm() * 3.4, 6.5, true);
        let si = b.prop(name, shade, m4(Vec3::new(p.x, h, p.z)));
        b.props[si].0.glow_lamp = Some(li);
        b.props[si].0.glow_tint = false;
        b.props[si].0.shadow = false;
    }
    b.daylight("sky_fill_k", Vec3::new(wc.x, 1.6, z0 + 0.9), Vec3::new(0.9, 0.95, 1.0) * 1.5, 5.0);
    b.daylight("sky_fill_l", Vec3::new(x1 - 0.9, 1.6, wc2.z), Vec3::new(0.9, 0.95, 1.0) * 1.6, 5.5);
    b.spawns.push(("hall", Vec3::new(2.8, 0.0, z0 + 0.8), 0.0));
    b.spawns.push(("front", Vec3::new(x0 + 0.8, 0.0, z1 - 1.3), std::f32::consts::FRAC_PI_2));
    b.spawns.push(("mom", Vec3::new(-2.0, 0.0, -1.55), std::f32::consts::PI));
    let cam = CamRig {
        dist: 6.4,
        pitch: 0.58,
        yaw: 0.25,
        fov: 42.0,
        look_height: 1.0,
        min_dist: 2.8,
        max_dist: 8.5,
    };
    let _ = left;
    let mut loc = b.finish(gpu, r, Loc::Home, (Vec2::new(x0, z0), Vec2::new(x1, z1)), true, cam, Vec3::new(0.0, 1.2, 0.0), 5.2);
    loc.ambient_tint = Vec3::new(1.0, 0.96, 0.9);
    loc
}

// ------------------------------------------------------------------ plaza

#[allow(clippy::too_many_arguments)]
fn facade(md: &mut MeshData, x0: f32, x1: f32, z: f32, h: f32, wall: &Mat, storefront: bool, seed: u64, props_out: &mut Vec<(Vec3, f32, f32)>) {
    let w = x1 - x0;
    let cx = (x0 + x1) * 0.5;
    md.rbox(m4(Vec3::new(cx, h * 0.5, z - 0.3)), Vec3::new(w * 0.5, h * 0.5, 0.3), 0.02, 1, wall);
    // cornice
    md.rbox(m4(Vec3::new(cx, h - 0.1, z + 0.02)), Vec3::new(w * 0.5 + 0.05, 0.1, 0.1), 0.02, 1, &Mat::new(0xefe9df, 0.7));
    let mut rng = crate::math::Rng::new(seed);
    // upper windows
    let n = (w / 2.0).floor().max(1.0) as i32;
    for i in 0..n {
        let x = x0 + (i as f32 + 0.5) * w / n as f32;
        for fl in 1..(h / 3.2) as i32 {
            let y = fl as f32 * 3.1 + 1.2;
            if y + 0.8 > h - 0.3 {
                continue;
            }
            md.rbox(m4(Vec3::new(x, y, z + 0.01)), Vec3::new(0.55, 0.75, 0.03), 0.01, 1, &Mat::new(0xefe9df, 0.6));
            let lit = rng.chance(0.4);
            md.cube(m4(Vec3::new(x, y, z + 0.035)), Vec3::new(0.46, 0.66, 0.01), &Mat::new(if lit { 0x7a6a50 } else { 0x2a3440 }, 0.08));
            md.rbox(m4(Vec3::new(x, y - 0.8, z + 0.1)), Vec3::new(0.62, 0.04, 0.12), 0.01, 1, &Mat::new(0xefe9df, 0.6));
            if lit {
                props_out.push((Vec3::new(x, y, z + 0.05), 0.92, 1.32));
            }
        }
    }
    if storefront {
        // big glass front with door
        md.rbox(m4(Vec3::new(cx, 1.45, z + 0.02)), Vec3::new(w * 0.5 - 0.3, 1.25, 0.04), 0.01, 1, &Mat::new(0x2e3238, 0.3).metal(0.6));
        md.cube(m4(Vec3::new(cx, 1.45, z + 0.065)), Vec3::new(w * 0.5 - 0.4, 1.15, 0.005), &Mat::new(0x3a4a58, 0.05));
        // interior glow panel (emissive back wall seen through glass)
        md.cube(m4(Vec3::new(cx, 1.45, z + 0.06)), Vec3::new(w * 0.5 - 0.45, 1.1, 0.002), &Mat::new(0x8a7a60, 0.5).emit(0.12));
        for k in 0..((w / 1.6) as i32) {
            let x = x0 + 0.4 + k as f32 * 1.6 + 0.8;
            if x < x1 - 0.4 {
                md.rbox(m4(Vec3::new(x, 1.45, z + 0.08)), Vec3::new(0.03, 1.2, 0.03), 0.005, 1, &Mat::new(0x2e3238, 0.3).metal(0.6));
            }
        }
    }
}

pub fn build_plaza(gpu: &Gpu, r: &mut Renderer) -> Location {
    let mut b = Builder::new();
    // ground
    b.base.plane(m4(Vec3::new(0.0, 0.0, -0.5)), Vec2::new(30.0, 15.0), (6, 3), &Mat::new(0xc9bba8, 0.8).kind(kind::PAVERS));
    // street to the south
    b.base.plane(m4(Vec3::new(0.0, 0.0, 9.5)), Vec2::new(60.0, 5.0), (8, 1), &Mat::new(0x3a3b3e, 0.85).kind(kind::ASPHALT));
    for side in [7.0f32, 12.0] {
        b.base.rbox(m4(Vec3::new(0.0, 0.0, side)), Vec3::new(30.0, 0.05, 0.12), 0.02, 1, &Mat::new(0xb8b2a6, 0.8).kind(kind::CONCRETE));
    }
    for i in 0..14 {
        let x = -26.0 + i as f32 * 4.0;
        b.base.cube(m4(Vec3::new(x, 0.003, 9.5)), Vec3::new(0.9, 0.002, 0.07), &Mat::new(0xe8e2d0, 0.7));
    }
    // crosswalk
    for i in 0..7 {
        b.base.cube(m4(Vec3::new(-8.0 + i as f32 * 0.6 - 1.8, 0.003, 9.5)), Vec3::new(0.22, 0.002, 2.2), &Mat::new(0xefe9dc, 0.7));
    }
    // far sidewalk and houses
    b.base.plane(m4(Vec3::new(0.0, 0.0, 14.0)), Vec2::new(60.0, 4.0), (8, 1), &Mat::new(0xc9bba8, 0.8).kind(kind::PAVERS));
    b.base.plane(m4(Vec3::new(0.0, -0.3, 0.0)), Vec2::new(120.0, 120.0), (1, 1), &Mat::new(0x6a7a4a, 1.0).kind(kind::GRASS));
    let mut lit_windows = Vec::new();
    let house_cols = [0xe9d8b8, 0xc9dbe0, 0xe8c4b0, 0xd8e0c4, 0xf0e4d0];
    for (i, x) in [-22.0f32, -14.0, -6.0, 2.0, 10.0, 18.0].iter().enumerate() {
        let mut house = MeshData::new();
        let col = house_cols[i % house_cols.len()];
        let mut hl = Vec::new();
        facade(&mut house, *x - 3.6, *x + 3.6, 16.2, 6.2, &Mat::new(col, 0.9).kind(kind::PLASTER), false, 100 + i as u64, &mut hl);
        for (p, w, h) in hl {
            lit_windows.push((Vec3::new(2.0 * x - p.x, p.y, 32.4 - p.z), w, h));
        }
        // rotate to face north (toward the plaza)
        let mut flipped = MeshData::new();
        flipped.append_xf(&house, m4ry(Vec3::new(2.0 * x, 0.0, 32.4), std::f32::consts::PI));
        b.base.append(&flipped);
        // doors
        b.base.append_xf(&door(0x5a3a2a), m4(Vec3::new(*x + 1.5, 0.0, 16.16)) * Mat4::from_rotation_y(std::f32::consts::PI));
    }
    let home_door = Vec3::new(-6.0 + 1.5, 0.0, 16.0);
    // north facades: store, café, bank
    let store = (-11.5f32, -3.5f32);
    let cafe = (-2.5f32, 4.0f32);
    let bank = (5.0f32, 12.0f32);
    let zf = -7.4;
    let mut fac = MeshData::new();
    facade(&mut fac, store.0, store.1, zf, 7.5, &Mat::new(0xd9d4cc, 0.85).kind(kind::CONCRETE), true, 1, &mut lit_windows);
    facade(&mut fac, cafe.0, cafe.1, zf, 6.8, &Mat::new(0xb8684a, 0.9).kind(kind::BRICK), true, 2, &mut lit_windows);
    facade(&mut fac, bank.0, bank.1, zf, 8.2, &Mat::new(0xe8e2d6, 0.8).kind(kind::PLASTER), true, 3, &mut lit_windows);
    facade(&mut fac, -20.0, store.0, zf, 9.0, &Mat::new(0xc8b8a0, 0.9).kind(kind::BRICK), false, 4, &mut lit_windows);
    facade(&mut fac, store.1, cafe.0, zf, 7.0, &Mat::new(0xa8b8c0, 0.9).kind(kind::PLASTER), false, 5, &mut lit_windows);
    facade(&mut fac, cafe.1, bank.0, zf, 7.4, &Mat::new(0xe0cfa0, 0.9).kind(kind::PLASTER), false, 6, &mut lit_windows);
    facade(&mut fac, bank.1, 20.0, zf, 8.8, &Mat::new(0xb8a898, 0.9).kind(kind::BRICK), false, 7, &mut lit_windows);
    b.base.append(&fac);
    // west & east blocks
    let mut sides = MeshData::new();
    sides.rbox(m4(Vec3::new(-16.5, 4.0, 0.0)), Vec3::new(0.4, 4.0, 7.2), 0.02, 1, &Mat::new(0xc8b8a0, 0.9).kind(kind::BRICK));
    b.base.append(&sides);
    // awnings and signs
    let mut signs = MeshData::new();
    let awning = |md: &mut MeshData, x0: f32, x1: f32, col: u32| {
        let n = ((x1 - x0) / 0.5) as i32;
        for i in 0..n {
            let c = if i % 2 == 0 { col } else { 0xf3eee4 };
            md.rbox(
                m4r(Vec3::new(x0 + 0.25 + i as f32 * 0.5, 3.1, zf + 0.6), Quat::from_rotation_x(0.35)),
                Vec3::new(0.25, 0.015, 0.65),
                0.004,
                1,
                &Mat::new(c, 0.9).kind(kind::FABRIC),
            );
        }
    };
    awning(&mut signs, cafe.0 + 0.3, cafe.1 - 0.3, 0x2e6a52);
    b.base.append(&signs);
    let sign_defs = [(store, 0x2f6fd8, 0u32), (cafe, 0x2e6a52, 1), (bank, 0x2a4a7a, 2)];
    for (i, ((x0, x1), col, icon)) in sign_defs.iter().enumerate() {
        let cx = (x0 + x1) * 0.5;
        let si = b.prop(["sign_store", "sign_cafe", "sign_bank"][i], sign_plate(3.2, 0.7, *col), m4(Vec3::new(cx, 3.55 + if i == 1 { 0.25 } else { 0.0 }, zf + 0.12)));
        b.props[si].0.params = Vec4::new(i as f32 * 2.0, 1.0, 0.0, 0.0);
        let ii = b.prop(["icon_store", "icon_cafe", "icon_bank"][i], sign_icon(*icon, 0.9), m4(Vec3::new(cx - 1.1, 3.55 + if i == 1 { 0.25 } else { 0.0 }, zf + 0.2)));
        b.props[ii].0.params = Vec4::new(i as f32, 1.0, 0.0, 0.0);
    }
    // store window displays: glowing screens
    for k in 0..3 {
        let mut d = MeshData::new();
        d.rbox(m4(Vec3::ZERO), Vec3::new(0.35, 0.22, 0.02), 0.01, 1, &PLASTIC_DARK);
        let di = b.prop("store_display", d, m4(Vec3::new(store.0 + 1.6 + k as f32 * 2.2, 1.5, zf + 0.35)));
        let _ = di;
        let (_, s) = tv(0.66);
        let si = b.prop("store_display_screen", s, m4(Vec3::new(store.0 + 1.6 + k as f32 * 2.2, 1.5, zf + 0.35)));
        b.props[si].0.shadow = false;
    }
    // ATM
    let atm = Vec3::new(bank.0 + 1.3, 0.0, zf + 0.35);
    let mut atm_md = MeshData::new();
    atm_md.rbox(m4(Vec3::new(0.0, 0.8, 0.0)), Vec3::new(0.38, 0.8, 0.28), 0.03, 2, &Mat::new(0x3a5a8a, 0.35).metal(0.4));
    atm_md.rbox(m4r(Vec3::new(0.0, 1.25, 0.26), Quat::from_rotation_x(-0.3)), Vec3::new(0.24, 0.16, 0.02), 0.01, 1, &Mat::new(0x7ab8ff, 0.1).emit(0.5));
    atm_md.rbox(m4(Vec3::new(0.0, 0.95, 0.3)), Vec3::new(0.2, 0.05, 0.06), 0.01, 1, &METAL);
    let atmi = b.prop("atm", atm_md, m4(atm));
    b.collider(atm, 0.4, 0.3);
    b.interact("bank", "Banco Futuro (cajero)", atm + Vec3::new(0.0, 1.2, 0.3), atm + Vec3::new(0.0, 0.0, 1.0), 1.1, Some(atmi));
    b.interact("store", "Entrar a TecnoMundo", Vec3::new((store.0 + store.1) * 0.5, 1.4, zf + 0.1), Vec3::new((store.0 + store.1) * 0.5, 0.0, zf + 1.3), 1.4, None);
    // café counter & outdoor tables
    let counter = Vec3::new((cafe.0 + cafe.1) * 0.5, 0.0, zf + 0.9);
    let mut cm = MeshData::new();
    cm.rbox(m4(Vec3::new(0.0, 0.52, 0.0)), Vec3::new(1.1, 0.52, 0.3), 0.02, 2, &Mat::new(0x6a4a3a, 0.5).kind(kind::WOOD));
    cm.rbox(m4(Vec3::new(0.0, 1.06, 0.0)), Vec3::new(1.15, 0.025, 0.34), 0.01, 1, &Mat::new(0xe8e2d6, 0.2).kind(kind::CONCRETE));
    let mut machine = MeshData::new();
    machine.rbox(m4(Vec3::new(0.0, 0.2, 0.0)), Vec3::new(0.2, 0.2, 0.17), 0.02, 2, &METAL);
    machine.rbox(m4(Vec3::new(0.0, 0.42, 0.0)), Vec3::new(0.21, 0.02, 0.18), 0.01, 1, &PLASTIC_DARK);
    cm.append_xf(&machine, m4(Vec3::new(0.6, 1.085, -0.05)));
    for i in 0..4 {
        cm.append_xf(&mug([0xf2eee6, 0x2e6a52, 0xe8b04a, 0xf2eee6][i]), m4(Vec3::new(-0.7 + i as f32 * 0.15, 1.085, 0.05)));
    }
    b.base.append_xf(&cm, m4(counter));
    b.collider(counter, 1.15, 0.35);
    b.interact("cafe", "Café Aroma", counter + Vec3::new(0.0, 1.2, 0.3), counter + Vec3::new(0.0, 0.0, 1.0), 1.2, None);
    for (i, (x, z)) in [(-1.6, -3.9), (0.4, -3.6), (2.4, -4.0)].iter().enumerate() {
        let c = Vec3::new(*x, 0.0, *z);
        b.base.append_xf(&cafe_table(), m4(c));
        b.collider(c, 0.36, 0.36);
        for (j, a) in [0.3f32, 3.4].iter().enumerate() {
            let cc = c + Vec3::new(a.sin() * 0.62, 0.0, a.cos() * 0.62);
            b.base.append_xf(&cafe_chair(), m4ry(cc, a + std::f32::consts::PI));
            if i == 0 && j == 0 {
                b.seats.push(Seat {
                    id: "cafe_chair",
                    pos: cc + Vec3::new(0.0, 0.46, 0.0),
                    yaw: a + std::f32::consts::PI,
                    feet: 0.3,
                });
            }
        }
        let _ = i;
    }
    // fountain
    let (fnt, water) = fountain();
    b.base.append(&fnt);
    let wi = b.prop("water", water, Mat4::IDENTITY);
    b.props[wi].0.shadow = false;
    b.colliders.push((Vec2::new(-1.75, -1.75), Vec2::new(1.75, 1.75)));
    b.interact("fountain", "Fuente", Vec3::new(0.0, 0.8, 1.6), Vec3::new(0.0, 0.0, 2.3), 1.2, None);
    // trees & planters
    let tree_pos = [(-7.0f32, -2.5f32), (-7.0, 3.5), (7.0, -2.5), (13.5, 3.5), (-13.0, 0.5), (4.5, 5.5)];
    for (i, (x, z)) in tree_pos.iter().enumerate() {
        let c = Vec3::new(*x, 0.0, *z);
        b.base.append_xf(&planter(0.85), m4(c));
        b.base.append_xf(&tree(20 + i as u64, 5.5 + (i % 3) as f32 * 0.8), m4(c + Vec3::Y * 0.4));
        b.collider(c, 0.85, 0.85);
    }
    // benches
    let bench_pos = [(-4.0f32, 2.8f32, std::f32::consts::PI), (4.2, 2.8, std::f32::consts::PI), (-9.5, -2.0, std::f32::consts::FRAC_PI_2)];
    for (i, (x, z, yaw)) in bench_pos.iter().enumerate() {
        let c = Vec3::new(*x, 0.0, *z);
        b.base.append_xf(&bench(), m4ry(c, *yaw));
        let (hx, hz) = if yaw.abs() > 2.0 || yaw.abs() < 0.5 { (0.9, 0.25) } else { (0.25, 0.9) };
        b.collider(c, hx, hz);
        b.seats.push(Seat {
            id: ["bench_0", "bench_1", "bench_2"][i],
            pos: c + Quat::from_rotation_y(*yaw) * Vec3::new(0.3, 0.46, 0.02),
            yaw: *yaw,
            feet: 0.35,
        });
    }
    b.interact("bench", "Sentarse", Vec3::new(-4.0, 0.6, 2.8), Vec3::new(-4.0, 0.0, 2.1), 1.0, None);
    // lampposts
    let mut lamps = Vec::new();
    for (x, z) in [(-10.0f32, 5.5f32), (-3.0, 5.8), (3.0, 5.8), (10.0, 5.5), (-10.0, -5.6), (10.5, -5.6)] {
        let (post, glow) = lamppost();
        let yaw = if z > 0.0 { std::f32::consts::PI } else { 0.0 };
        b.base.append_xf(&post, m4ry(Vec3::new(x, 0.0, z), yaw));
        let li = b.lamp("street", Vec3::new(x, 3.4, z) + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, 0.35), Vec3::new(1.0, 0.8, 0.55) * 11.0, 11.0, true);
        let gi = b.prop("street_glow", glow, m4ry(Vec3::new(x, 0.0, z), yaw));
        b.props[gi].0.glow_lamp = Some(li);
        b.props[gi].0.shadow = false;
        b.collider(Vec3::new(x, 0.0, z), 0.15, 0.15);
        lamps.push(li);
    }
    // store / café interior lights
    b.lamp("store_light", Vec3::new((store.0 + store.1) * 0.5, 2.0, zf + 1.2), Vec3::new(0.7, 0.85, 1.0) * 3.0, 5.0, true);
    b.lamp("cafe_light", Vec3::new((cafe.0 + cafe.1) * 0.5, 2.6, zf + 1.4), warm() * 4.0, 5.5, true);
    // market stall (business)
    let stall = Vec3::new(8.5, 0.0, 1.8);
    let sti = b.prop("stall", market_stall(0xe07a5f), m4ry(stall, -0.3));
    b.props[sti].0.visible = true;
    b.collider(stall, 1.0, 0.55);
    b.interact("stall", "Puesto de la feria", stall + Vec3::new(0.0, 1.0, 0.5), stall + Quat::from_rotation_y(-0.3) * Vec3::new(0.0, 0.0, -0.95), 1.3, Some(sti));
    let mut goods = MeshData::new();
    for i in 0..10 {
        let a = i as f32 * 0.7;
        goods.torus(m4r(Vec3::new(-0.6 + i as f32 * 0.13, 0.9, (a.sin()) * 0.1), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 * 0.9)), 0.035, 0.006, 14, 6, &Mat::new([0xe07a5f, 0x5ed3b3, 0xf2c84a, 0x8f8ad6, 0x6aa8ff][i % 5], 0.5));
    }
    let gi = b.prop("stall_goods", goods, m4ry(stall, -0.3));
    b.props[gi].0.visible = false;
    // trash bins, parked cars
    b.base.append_xf(&trash_bin(), m4(Vec3::new(-5.5, 0.0, 5.8)));
    b.collider(Vec3::new(-5.5, 0.0, 5.8), 0.25, 0.25);
    b.base.append_xf(&car(0xc0392b), m4ry(Vec3::new(6.5, 0.0, 10.6), std::f32::consts::FRAC_PI_2));
    b.base.append_xf(&car(0x2c3e50), m4ry(Vec3::new(-15.0, 0.0, 10.6), std::f32::consts::FRAC_PI_2));
    b.colliders.push((Vec2::new(4.3, 9.7), Vec2::new(8.7, 11.5)));
    // bike rack
    for i in 0..4 {
        let mut rack = MeshData::new();
        rack.torus(m4r(Vec3::new(0.0, 0.0, 0.0), Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)), 0.35, 0.02, 16, 6, &METAL);
        b.base.append_xf(&rack, m4r(Vec3::new(11.0 + i as f32 * 0.5, 0.0, -5.2), Quat::IDENTITY));
    }
    // home door
    b.interact("home", "Volver a casa", home_door + Vec3::new(0.0, 1.0, 0.0), home_door + Vec3::new(0.0, 0.0, -0.9), 1.0, None);
    // lit windows (night)
    let win_lamp = b.lamp("windows", Vec3::new(0.0, -50.0, 0.0), Vec3::ZERO, 0.1, true);
    let mut lw = MeshData::new();
    for (p, w, h) in &lit_windows {
        lw.cube(m4(*p), Vec3::new(w * 0.5, h * 0.5, 0.004), &Mat::new(0xffd89a, 0.4).emit(0.35));
    }
    let lwi = b.prop("lit_windows", lw, Mat4::IDENTITY);
    b.props[lwi].0.glow_lamp = Some(win_lamp);
    b.props[lwi].0.shadow = false;
    // skyline far away
    let mut sky = MeshData::new();
    let mut rng = crate::math::Rng::new(77);
    for i in 0..30 {
        let a = -1.2 + i as f32 * 0.085;
        let d = 60.0 + rng.range(0.0, 20.0);
        let hgt = rng.range(8.0, 28.0);
        let c = Vec3::new(a.sin() * d, hgt * 0.5, -a.cos() * d);
        sky.cube(m4ry(c, -a), Vec3::new(rng.range(3.0, 7.0), hgt * 0.5, 3.0), &Mat::new(0x8a94a0, 1.0));
    }
    b.base.append(&sky);
    b.spawns.push(("home", home_door + Vec3::new(0.0, 0.0, -1.0), std::f32::consts::PI));
    b.spawns.push(("fountain", Vec3::new(0.0, 0.0, 2.6), std::f32::consts::PI));
    b.spawns.push(("tomas", Vec3::new(-3.6, 0.0, 2.2), 0.0));
    b.spawns.push(("vale", counter + Vec3::new(0.0, 0.0, -0.6), 0.0));
    b.spawns.push(("julio", Vec3::new((store.0 + store.1) * 0.5 + 1.5, 0.0, zf + 1.0), 0.0));
    let cam = CamRig {
        dist: 7.8,
        pitch: 0.3,
        yaw: 0.0,
        fov: 44.0,
        look_height: 1.45,
        min_dist: 3.5,
        max_dist: 12.0,
    };
    let mut loc = b.finish(gpu, r, Loc::Plaza, (Vec2::new(-15.5, -6.6), Vec2::new(15.5, 15.3)), false, cam, Vec3::new(0.0, 2.0, 2.0), 17.0);
    loc.ambient_tint = Vec3::ONE;
    loc
}

pub fn beam_draw(loc: &Location, w: &WindowDef, sun: Vec3, strength: f32, scene: &mut FrameScene) {
    if strength <= 0.01 {
        return;
    }
    let dir = -sun.normalize();
    if dir.dot(w.inward) <= 0.05 {
        return;
    }
    // extend until the floor
    let t_floor = w.center.y / (-dir.y).max(0.05);
    let len = t_floor.min(6.0);
    let xf = Mat4::from_cols(
        (w.right * 2.0).extend(0.0),
        (dir * len).extend(0.0),
        (w.up * 2.0).extend(0.0),
        w.center.extend(1.0),
    );
    scene.draws.push(
        Draw::new(loc.beam, xf)
            .tint(Vec4::new(1.0, 0.85, 0.65, 0.05 * strength))
            .pass(DrawPass::Additive),
    );
}
