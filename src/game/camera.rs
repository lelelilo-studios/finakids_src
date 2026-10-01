//! Cinematic camera: third-person follow with smoothing, scripted shots, blends and handheld motion.

use crate::gfx::Camera;
use crate::math::*;
use crate::world::CamRig;
use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub from_pos: Vec3,
    pub from_target: Vec3,
    pub to_pos: Vec3,
    pub to_target: Vec3,
    pub fov: f32,
    pub dur: f32,
    pub ease: Ease,
    /// Seconds to blend in from the previous camera (0 = hard cut).
    pub blend: f32,
    pub handheld: f32,
}

impl Shot {
    pub fn fixed(pos: Vec3, target: Vec3, fov: f32) -> Shot {
        Shot {
            from_pos: pos,
            from_target: target,
            to_pos: pos,
            to_target: target,
            fov,
            dur: 1.0,
            ease: Ease::InOut,
            blend: 0.0,
            handheld: 0.35,
        }
    }
    pub fn dolly(from: Vec3, to: Vec3, target: Vec3, fov: f32, dur: f32) -> Shot {
        Shot {
            from_pos: from,
            from_target: target,
            to_pos: to,
            to_target: target,
            fov,
            dur,
            ease: Ease::Sine,
            blend: 0.0,
            handheld: 0.3,
        }
    }
    pub fn blend(mut self, b: f32) -> Shot {
        self.blend = b;
        self
    }
    pub fn pan_to(mut self, target: Vec3) -> Shot {
        self.to_target = target;
        self
    }
}

pub struct CamCtl {
    pub cam: Camera,
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    yaw_target: f32,
    dist_target: f32,
    focus: Vec3,
    shot: Option<Shot>,
    shot_t: f32,
    blend_t: f32,
    blend_from: Camera,
    release_t: f32,
    release_from: Camera,
    time: f32,
    pub zoom_bias: f32,
    limits: Option<(f32, f32, f32, f32)>,
    /// Outdoor areas: the camera stays inside this (min x, min z, max x, max z) box.
    area: Option<(f32, f32, f32, f32)>,
}

impl CamCtl {
    pub fn new() -> CamCtl {
        CamCtl {
            cam: Camera::default(),
            yaw: 0.0,
            pitch: 0.5,
            dist: 5.0,
            yaw_target: 0.0,
            dist_target: 5.0,
            focus: Vec3::ZERO,
            shot: None,
            shot_t: 0.0,
            blend_t: 1.0,
            blend_from: Camera::default(),
            release_t: 1.0,
            release_from: Camera::default(),
            time: 0.0,
            zoom_bias: 0.0,
            limits: None,
            area: None,
        }
    }

    pub fn reset_to(&mut self, rig: &CamRig, focus: Vec3) {
        self.yaw = rig.yaw;
        self.yaw_target = rig.yaw;
        self.pitch = rig.pitch;
        self.dist = rig.dist;
        self.dist_target = rig.dist;
        self.focus = focus;
        self.shot = None;
        self.release_t = 1.0;
        self.cam = self.follow_cam(rig);
    }

    pub fn rotate(&mut self, d: f32) {
        self.yaw_target += d;
    }

    pub fn zoom(&mut self, d: f32, rig: &CamRig) {
        self.dist_target = (self.dist_target * (1.0 - d * 0.12)).clamp(rig.min_dist, rig.max_dist);
    }

    pub fn play(&mut self, shot: Shot) {
        self.blend_from = self.cam;
        self.blend_t = if shot.blend > 0.0 { 0.0 } else { 1.0 };
        self.shot = Some(shot);
        self.shot_t = 0.0;
    }

    pub fn in_shot(&self) -> bool {
        self.shot.is_some()
    }

    /// Returns to the follow camera with a blend.
    pub fn release(&mut self, blend: f32) {
        if self.shot.is_some() {
            self.shot = None;
            self.release_from = self.cam;
            self.release_t = if blend > 0.0 { 0.0 } else { 1.0 };
            // keep the follow yaw close to the current view to avoid spins
            let fwd = self.cam.forward();
            let yaw = (-fwd.x).atan2(-fwd.z);
            self.yaw = self.yaw + wrap_angle(yaw - self.yaw);
            self.yaw_target = self.yaw;
        }
    }

    fn follow_cam(&self, rig: &CamRig) -> Camera {
        let dir = glam::Vec3::new(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos());
        let pos = self.focus + dir * self.dist;
        Camera {
            pos,
            target: self.focus,
            fov: rig.fov,
            ..Camera::default()
        }
    }

    /// Interior limits: (min_x, max_x, yaw_min, yaw_max)
    pub fn set_limits(&mut self, lim: Option<(f32, f32, f32, f32)>) {
        self.limits = lim;
    }

    /// Keeps the follow camera in front of building facades in outdoor areas.
    pub fn set_area(&mut self, area: Option<(f32, f32, f32, f32)>) {
        self.area = area;
    }

    pub fn update(&mut self, dt: f32, focus: Vec3, rig: &CamRig) {
        if let Some((_, _, y0, y1)) = self.limits {
            self.yaw_target = self.yaw_target.clamp(y0, y1);
        }
        self.time += dt;
        self.focus = damp_v3(self.focus, focus, 6.0, dt);
        self.yaw = damp(self.yaw, self.yaw_target, 8.0, dt);
        self.dist = damp(self.dist, (self.dist_target + self.zoom_bias).clamp(rig.min_dist * 0.8, rig.max_dist * 1.2), 5.0, dt);
        self.pitch = damp(self.pitch, rig.pitch, 3.0, dt);
        let mut follow = self.follow_cam(rig);
        if let Some((x0, x1, _, _)) = self.limits {
            follow.pos.x = follow.pos.x.clamp(x0, x1);
        }
        if let Some((x0, z0, x1, z1)) = self.area {
            // pull the camera in along its view ray until it is back inside the area
            let d = follow.pos - follow.target;
            let mut k = 1.0f32;
            if d.x < 0.0 && follow.pos.x < x0 {
                k = k.min((x0 - follow.target.x) / d.x);
            }
            if d.x > 0.0 && follow.pos.x > x1 {
                k = k.min((x1 - follow.target.x) / d.x);
            }
            if d.z < 0.0 && follow.pos.z < z0 {
                k = k.min((z0 - follow.target.z) / d.z);
            }
            if d.z > 0.0 && follow.pos.z > z1 {
                k = k.min((z1 - follow.target.z) / d.z);
            }
            let min_k = (1.4 / d.length().max(0.1)).min(1.0);
            follow.pos = follow.target + d * k.clamp(min_k, 1.0);
        }
        // gentle breathing motion
        follow.pos += Vec3::new(noise1(self.time * 0.2, 1.0), noise1(self.time * 0.17, 2.0), 0.0) * 0.03;

        if let Some(shot) = self.shot {
            self.shot_t += dt;
            let k = shot.ease.apply(self.shot_t / shot.dur.max(1e-3));
            let mut c = Camera {
                pos: shot.from_pos.lerp(shot.to_pos, k),
                target: shot.from_target.lerp(shot.to_target, k),
                fov: shot.fov,
                ..Camera::default()
            };
            let hh = shot.handheld;
            c.pos += Vec3::new(noise1(self.time * 0.5, 3.0), noise1(self.time * 0.45, 4.0), noise1(self.time * 0.4, 5.0)) * 0.018 * hh;
            c.target += Vec3::new(noise1(self.time * 0.6, 6.0), noise1(self.time * 0.5, 7.0), 0.0) * 0.012 * hh;
            c.roll = noise1(self.time * 0.3, 8.0) * 0.006 * hh;
            if self.blend_t < 1.0 {
                self.blend_t = (self.blend_t + dt / shot.blend.max(1e-3)).min(1.0);
                let b = ease_in_out(self.blend_t);
                c.pos = self.blend_from.pos.lerp(c.pos, b);
                c.target = self.blend_from.target.lerp(c.target, b);
                c.fov = lerp(self.blend_from.fov, c.fov, b);
            }
            self.cam = c;
        } else {
            if self.release_t < 1.0 {
                self.release_t = (self.release_t + dt / 0.9).min(1.0);
                let b = ease_in_out(self.release_t);
                follow.pos = self.release_from.pos.lerp(follow.pos, b);
                follow.target = self.release_from.target.lerp(follow.target, b);
                follow.fov = lerp(self.release_from.fov, follow.fov, b);
            }
            self.cam = follow;
        }
    }
}

impl Default for CamCtl {
    fn default() -> Self {
        Self::new()
    }
}
