//! Characters: appearance, procedural meshes, animation and face rendering.

pub mod actions;
pub mod anim;
pub mod appearance;
pub mod build;
pub mod face;
pub mod skeleton;

use crate::gfx::mesh::{kind, Mat, MeshData};
use crate::gfx::renderer::{FrameScene, MAX_BONES};
use crate::gfx::{Draw, DrawPass, Gpu, MeshId, Renderer};
use crate::math::{color_lin, rgb};
use anim::Animator;
use appearance::Appearance;
use build::{FaceLayout, Quality};
use face::{FaceMeshes, FaceState};
use glam::{Mat4, Quat, Vec3, Vec4};
use skeleton::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldProp {
    Phone,
    Cup,
    Bag,
    Bills,
    Bracelet,
}

/// GPU meshes shared across characters.
pub struct SharedCharMeshes {
    pub eyeball: MeshId,
    pub lid_upper: MeshId,
    pub lid_lower: MeshId,
    pub lash: MeshId,
    pub blob: MeshId,
    pub phone: MeshId,
    pub phone_screen: MeshId,
    pub cup: MeshId,
    pub bag: MeshId,
    pub bills: MeshId,
    pub bracelet: MeshId,
}

impl SharedCharMeshes {
    pub fn new(gpu: &Gpu, r: &mut Renderer) -> SharedCharMeshes {
        let fm = FaceMeshes::new();
        let mut blob = MeshData::new();
        blob.plane(Mat4::IDENTITY, glam::Vec2::new(1.0, 1.0), (1, 1), &Mat::new(0x000000, 1.0).kind(kind::SOFT));
        let mut phone = MeshData::new();
        let body = Mat::new(0x1c1c22, 0.3).metal(0.2);
        phone.rbox(Mat4::IDENTITY, Vec3::new(0.036, 0.074, 0.0045), 0.006, 3, &body);
        let mut screen = MeshData::new();
        screen.plane(
            Mat4::from_rotation_translation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), Vec3::new(0.0, 0.0, 0.0047)),
            glam::Vec2::new(0.066, 0.138),
            (1, 1),
            &Mat::new(0xffffff, 0.1).kind(kind::PHONE).emit(0.5),
        );
        let mut cup = MeshData::new();
        cup.lathe(
            Mat4::IDENTITY,
            &[(0.0, 0.0), (0.032, 0.0), (0.04, 0.1), (0.038, 0.1), (0.0, 0.1)],
            16,
            &Mat::new(0xf2eee6, 0.4),
        );
        cup.cylinder(Mat4::from_translation(Vec3::new(0.0, 0.1, 0.0)), 0.041, 0.012, 16, &Mat::new(0x2e5e4e, 0.5));
        let mut bag = MeshData::new();
        bag.rbox(Mat4::from_translation(Vec3::new(0.0, -0.16, 0.0)), Vec3::new(0.13, 0.15, 0.05), 0.01, 2, &Mat::new(0xe8dcc0, 0.9).kind(kind::FABRIC));
        bag.torus(Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2), 0.05, 0.006, 12, 6, &Mat::new(0xd8c8a8, 0.9));
        let mut bills = MeshData::new();
        for i in 0..3 {
            bills.rbox(
                Mat4::from_rotation_translation(Quat::from_rotation_y(0.1 * i as f32), Vec3::new(0.0, 0.002 * i as f32, 0.0)),
                Vec3::new(0.035, 0.0008, 0.07),
                0.0005,
                1,
                &Mat::new(if i % 2 == 0 { 0x6fa88a } else { 0x8fb86a }, 0.8),
            );
        }
        let mut bracelet = MeshData::new();
        bracelet.torus(Mat4::IDENTITY, 0.03, 0.005, 16, 6, &Mat::new(0xe07a5f, 0.6));
        SharedCharMeshes {
            eyeball: r.upload_mesh(gpu, &fm.eyeball),
            lid_upper: r.upload_mesh(gpu, &fm.lid_upper),
            lid_lower: r.upload_mesh(gpu, &fm.lid_lower),
            lash: r.upload_mesh(gpu, &fm.lash),
            blob: r.upload_mesh(gpu, &blob),
            phone: r.upload_mesh(gpu, &phone),
            phone_screen: r.upload_mesh(gpu, &screen),
            cup: r.upload_mesh(gpu, &cup),
            bag: r.upload_mesh(gpu, &bag),
            bills: r.upload_mesh(gpu, &bills),
            bracelet: r.upload_mesh(gpu, &bracelet),
        }
    }
}

pub struct Character {
    pub id: &'static str,
    pub app: Appearance,
    pub anim: Animator,
    pub mesh: MeshId,
    pub face: FaceLayout,
    pub brow: [MeshId; 2],
    pub mouth: MeshId,
    mouth_data: MeshData,
    last_mouth: FaceState,
    pub mats: Vec<Mat>,
    pub visible: bool,
    pub held: [Option<HeldProp>; 2],
    pub phone_cracked: f32,
    pub phone_glow: f32,
    pub highlight: f32,
    pub path: Vec<Vec3>,
    pub walk_speed: f32,
    pub arrive_yaw: Option<f32>,
}

impl Character {
    pub fn new(gpu: &Gpu, r: &mut Renderer, id: &'static str, app: Appearance, pos: Vec3, yaw: f32, q: Quality) -> Character {
        let (shoulder, hips) = (1.0 + app.masc * 0.08, 1.0 + (1.0 - app.masc) * 0.05);
        let skel = Skeleton::new(app.height, shoulder, hips);
        let meshes = build::build(&app, &skel, q);
        let mesh = r.upload_skinned(gpu, &meshes.skin);
        let brow_col = {
            let c = rgb(app.hair_color);
            let f = if app.age > 1.8 { 0.8 } else { 1.0 };
            ((c[0] as f32 * f) as u32) << 16 | ((c[1] as f32 * f) as u32) << 8 | (c[2] as f32 * f) as u32
        };
        let brow = [
            r.upload_mesh(gpu, &face::brow_mesh(&meshes.face, 0, brow_col, app.masc)),
            r.upload_mesh(gpu, &face::brow_mesh(&meshes.face, 1, brow_col, app.masc)),
        ];
        let mut mouth_data = MeshData::new();
        let fs = FaceState::neutral();
        face::mouth_mesh(&meshes.face, &fs, meshes.mats[build::M_LIP as usize].color, meshes.mats[build::M_SKIN as usize].color, &mut mouth_data);
        let mouth = r.upload_mesh(gpu, &mouth_data);
        let anim = Animator::new(skel, pos, yaw, app.seed);
        Character {
            id,
            app,
            anim,
            mesh,
            face: meshes.face,
            brow,
            mouth,
            mouth_data,
            last_mouth: fs,
            mats: meshes.mats,
            visible: true,
            held: [None, None],
            phone_cracked: 0.0,
            phone_glow: 1.0,
            highlight: 0.0,
            path: Vec::new(),
            walk_speed: 1.25,
            arrive_yaw: None,
        }
    }

    pub fn k(&self) -> f32 {
        self.app.height / 1.65
    }

    pub fn pos(&self) -> Vec3 {
        self.anim.pos
    }

    /// Walks along a path of points.
    pub fn walk_to(&mut self, target: Vec3, arrive_yaw: Option<f32>) {
        self.path = vec![target];
        self.arrive_yaw = arrive_yaw;
        if self.anim.is_seated() || matches!(self.anim.stance, anim::Stance::Sit { .. }) {
            self.anim.stand();
        }
    }

    pub fn walk_path(&mut self, pts: Vec<Vec3>, arrive_yaw: Option<f32>) {
        self.path = pts;
        self.arrive_yaw = arrive_yaw;
        self.anim.stand();
    }

    pub fn is_walking(&self) -> bool {
        !self.path.is_empty() || self.anim.speed > 0.1
    }

    pub fn face_towards(&mut self, p: Vec3) {
        let d = p - self.anim.pos;
        if d.length_squared() > 1e-4 {
            self.anim.target_yaw = Some(crate::math::yaw_of(d));
        }
    }

    pub fn update(&mut self, dt: f32) {
        // path following
        if let Some(&next) = self.path.first() {
            let mut d = next - self.anim.pos;
            d.y = 0.0;
            let dist = d.length();
            let last = self.path.len() == 1;
            if dist < if last { 0.06 } else { 0.35 } {
                self.path.remove(0);
                if self.path.is_empty() {
                    self.anim.desired_vel = Vec3::ZERO;
                    if let Some(y) = self.arrive_yaw.take() {
                        self.anim.target_yaw = Some(y);
                    }
                }
            } else {
                let slow = if last { (dist / 0.6).clamp(0.25, 1.0) } else { 1.0 };
                self.anim.desired_vel = d / dist * self.walk_speed * slow;
            }
        }
        self.anim.update(dt);
        // phone screen glow follows usage
        let target_glow = if self.held[1] == Some(HeldProp::Phone) { 1.0 } else { 0.0 };
        self.phone_glow += (target_glow - self.phone_glow) * (1.0 - (-4.0 * dt).exp());
    }

    pub fn hand_prop_xf(&self, side: usize, p: HeldProp) -> Mat4 {
        let hand = self.anim.bone_world(HAND[side]);
        let k = self.k();
        let s = sx(side);
        let f = self.anim.skel.bone_dir(HAND[side]);
        let n = f.cross(Vec3::Z).normalize() * s;
        let base = self.anim.skel.bind[HAND[side]];
        // prop frame in bind space: y = along the fingers, z = out of the palm (right-handed)
        let x = f.cross(n).normalize();
        let t = x;
        let rot = Quat::from_mat3(&glam::Mat3::from_cols(x, f, n));
        let off = match p {
            HeldProp::Phone => f * 0.075 * k + n * 0.02 * k,
            HeldProp::Cup => f * 0.05 * k + n * 0.045 * k + t * 0.02 * k,
            HeldProp::Bag => f * 0.08 * k,
            HeldProp::Bills => f * 0.07 * k + n * 0.012 * k,
            HeldProp::Bracelet => f * 0.08 * k + n * 0.02 * k,
        };
        let extra = match p {
            HeldProp::Cup => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            HeldProp::Bag => Quat::from_rotation_x(std::f32::consts::PI),
            _ => Quat::IDENTITY,
        };
        let _ = base;
        hand * Mat4::from_translation(off) * Mat4::from_quat(rot * extra)
    }

    pub fn phone_light(&self) -> Option<(Vec3, f32)> {
        if self.held[1] == Some(HeldProp::Phone) && self.phone_glow > 0.05 {
            let xf = self.hand_prop_xf(1, HeldProp::Phone);
            Some((xf.transform_point3(Vec3::new(0.0, 0.0, 0.05)), self.phone_glow))
        } else {
            None
        }
    }

    /// Emits draw calls for this character.
    pub fn draw(&mut self, gpu: &Gpu, r: &mut Renderer, shared: &SharedCharMeshes, scene: &mut FrameScene, show_face: bool) {
        if !self.visible {
            return;
        }
        let mut pal = [Mat4::IDENTITY; MAX_BONES];
        self.anim.fk.skin_matrices(&self.anim.skel, &mut pal[..BONE_COUNT]);
        let pidx = scene.palettes.len() as u32;
        scene.palettes.push(pal);
        let model = self.anim.model;
        let ch = Vec4::new(0.0, self.highlight, 0.0, 1.0);
        let mut body = Draw::new(self.mesh, model).params(ch);
        body.palette = pidx;
        scene.draws.push(body);

        // contact shadow
        let hips = self.anim.hips_world;
        let blob_xf = Mat4::from_scale_rotation_translation(
            Vec3::new(0.62, 1.0, 0.62) * self.k(),
            Quat::from_rotation_y(self.anim.yaw),
            Vec3::new(hips.x, self.anim.pos.y + 0.006, hips.z),
        );
        scene.draws.push(Draw::new(shared.blob, blob_xf).tint(Vec4::new(1.0, 1.0, 1.0, 0.5)).pass(DrawPass::Transparent));

        if show_face {
            self.draw_face(gpu, r, shared, scene);
        }
        // held props
        for side in 0..2 {
            if let Some(p) = self.held[side] {
                let xf = self.hand_prop_xf(side, p);
                let mesh = match p {
                    HeldProp::Phone => shared.phone,
                    HeldProp::Cup => shared.cup,
                    HeldProp::Bag => shared.bag,
                    HeldProp::Bills => shared.bills,
                    HeldProp::Bracelet => shared.bracelet,
                };
                scene.draws.push(Draw::new(mesh, xf).params(Vec4::new(0.0, 0.0, 0.0, 1.0)));
                if p == HeldProp::Phone {
                    scene.draws.push(
                        Draw::new(shared.phone_screen, xf)
                            .tint(Vec4::new(self.phone_glow, self.phone_glow, self.phone_glow, 1.0))
                            .params(Vec4::new(0.0, 0.0, self.phone_cracked, 0.0))
                            .no_shadow(),
                    );
                }
            }
        }
    }

    fn draw_face(&mut self, gpu: &Gpu, r: &mut Renderer, shared: &SharedCharMeshes, scene: &mut FrameScene) {
        let head = self.anim.bone_world(HEAD);
        let hb = self.anim.skel.bind[HEAD];
        // head-local transform: bind offsets are relative to the head joint
        let head_local = head * Mat4::from_translation(Vec3::ZERO);
        let f = self.anim.face;
        let skin_lin = color_lin(self.app.skin);
        let skin_tint = Vec4::new(skin_lin.x, skin_lin.y, skin_lin.z, 1.0);
        let eye_lin = color_lin(self.app.eye_color);
        let char_p = Vec4::new(0.0, 0.0, 0.0, 1.0);
        let er = self.face.eye_r;
        let blink = self.anim.blink_amt;
        for side in 0..2 {
            let c = self.face.eye[side];
            let eye_xf = head_local * Mat4::from_translation(c) * Mat4::from_quat(self.anim.eye_rot) * Mat4::from_scale(Vec3::splat(er));
            scene.draws.push(
                Draw::new(shared.eyeball, eye_xf)
                    .tint(Vec4::new(eye_lin.x, eye_lin.y, eye_lin.z, 1.0))
                    .params(Vec4::new(0.3, 0.0, 0.0, 1.0))
                    .no_shadow(),
            );
            // lids
            let open = (f.lid_open * (1.0 - blink)).clamp(0.0, 1.35);
            let look_down = self.anim.eye_rot.to_euler(glam::EulerRot::YXZ).1.max(0.0);
            let upper_edge = 69.0 + (1.0 - open) * 46.0 + look_down.to_degrees() * 0.6;
            let upper_rot = (upper_edge - 60.0).to_radians();
            let lower_edge = 114.0 - f.lid_lower * 12.0 - (1.0 - open.min(1.0)) * 4.0;
            let lower_rot = -(128.0f32 - lower_edge).to_radians();
            let lid_scale = Vec3::splat(er * 1.07);
            let base = head_local * Mat4::from_translation(c);
            let up_xf = base * Mat4::from_quat(Quat::from_rotation_x(upper_rot)) * Mat4::from_scale(lid_scale);
            scene.draws.push(Draw::new(shared.lid_upper, up_xf).tint(skin_tint).params(char_p).no_shadow());
            scene.draws.push(Draw::new(shared.lash, up_xf).params(char_p).no_shadow());
            let lo_xf = base * Mat4::from_quat(Quat::from_rotation_x(lower_rot)) * Mat4::from_scale(lid_scale);
            scene.draws.push(Draw::new(shared.lid_lower, lo_xf).tint(skin_tint).params(char_p).no_shadow());
            // brow
            let bxf = head_local * face::brow_xf(&self.face, side, &f);
            scene.draws.push(Draw::new(self.brow[side], bxf).params(char_p).no_shadow());
        }
        // mouth (rebuild when the shape changes)
        let changed = (f.smile - self.last_mouth.smile).abs()
            + (f.open - self.last_mouth.open).abs()
            + (f.wide - self.last_mouth.wide).abs()
            + (f.asym - self.last_mouth.asym).abs()
            > 0.004;
        if changed {
            face::mouth_mesh(
                &self.face,
                &f,
                self.mats[build::M_LIP as usize].color,
                self.mats[build::M_SKIN as usize].color,
                &mut self.mouth_data,
            );
            r.update_mesh(gpu, self.mouth, &self.mouth_data);
            self.last_mouth = f;
        }
        scene.draws.push(Draw::new(self.mouth, head_local).params(char_p).no_shadow());
        let _ = hb;
    }
}
