//! Perspective camera.

use glam::{Mat4, Vec2, Vec3, Vec4};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    /// Vertical field of view in degrees.
    pub fov: f32,
    pub roll: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            pos: Vec3::new(0.0, 1.7, 4.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            roll: 0.0,
            near: 0.05,
            far: 250.0,
        }
    }
}

impl Camera {
    pub fn view(&self) -> Mat4 {
        let fwd = (self.target - self.pos).normalize_or(Vec3::Z);
        let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
        let up0 = right.cross(fwd);
        let up = up0 * self.roll.cos() + right * self.roll.sin();
        Mat4::look_at_rh(self.pos, self.target, up)
    }

    pub fn proj(&self, aspect: f32) -> Mat4 {
        // widen the horizontal coverage on portrait screens
        let fov = if aspect < 1.0 {
            let h = (self.fov.to_radians() * 0.5).tan() / aspect.max(0.4) * 0.8;
            2.0 * h.atan()
        } else {
            self.fov.to_radians()
        };
        Mat4::perspective_rh(fov, aspect.max(0.01), self.near, self.far)
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.pos).normalize_or(Vec3::Z)
    }

    /// World-space ray through a point in normalized device coordinates.
    pub fn ray(&self, ndc: Vec2, aspect: f32) -> (Vec3, Vec3) {
        let inv = (self.proj(aspect) * self.view()).inverse();
        let near = inv * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
        let far = inv * Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        let a = near.truncate() / near.w;
        let b = far.truncate() / far.w;
        (a, (b - a).normalize_or(Vec3::NEG_Z))
    }

    /// Projects a world point to screen pixels. Returns None when behind the camera.
    pub fn project(&self, p: Vec3, w: f32, h: f32) -> Option<Vec2> {
        let clip = self.proj(w / h) * self.view() * p.extend(1.0);
        if clip.w <= 0.01 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new((ndc.x * 0.5 + 0.5) * w, (0.5 - ndc.y * 0.5) * h))
    }
}
