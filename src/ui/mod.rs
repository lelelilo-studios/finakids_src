//! Immediate-mode UI: SDF shapes, text, frosted glass and simple widgets.

pub mod font;
pub mod icons;

use crate::gfx::renderer::{UiBatch, UiCmd, UiVertex};
use font::Fonts;
use glam::Vec2;
use std::collections::HashMap;

pub use font::{FONT_BOLD, FONT_REGULAR};

pub type Color = [u8; 4];

pub const fn rgba(hex: u32, a: u8) -> Color {
    [((hex >> 16) & 0xff) as u8, ((hex >> 8) & 0xff) as u8, (hex & 0xff) as u8, a]
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    [c[0], c[1], c[2], (c[3] as f32 * a.clamp(0.0, 1.0)) as u8]
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    crate::math::mix_rgb(a, b, t.clamp(0.0, 1.0))
}

/// Palette of the interface.
pub mod pal {
    use super::{rgba, Color};
    pub const WHITE: Color = rgba(0xffffff, 255);
    pub const TEXT: Color = rgba(0xf4f1ec, 255);
    pub const TEXT_DIM: Color = rgba(0xc9c4bd, 255);
    pub const TEXT_MUTED: Color = rgba(0x9a958f, 255);
    pub const INK: Color = rgba(0x16181d, 255);
    pub const PANEL: Color = rgba(0x14161c, 200);
    pub const PANEL_SOLID: Color = rgba(0x1b1e26, 245);
    pub const LINE: Color = rgba(0xffffff, 28);
    pub const ACCENT: Color = rgba(0xffb35c, 255); // warm amber
    pub const ACCENT2: Color = rgba(0x5ed3b3, 255); // mint (savings)
    pub const BLUE: Color = rgba(0x6aa8ff, 255);
    pub const RED: Color = rgba(0xff6b6b, 255);
    pub const GREEN: Color = rgba(0x6fdc8c, 255);
    pub const PURPLE: Color = rgba(0xb28cff, 255);
    pub const YELLOW: Color = rgba(0xffd166, 255);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.x + self.w && p.y <= self.y + self.h
    }
    pub fn shrink(&self, m: f32) -> Rect {
        Rect::new(self.x + m, self.y + m, (self.w - 2.0 * m).max(0.0), (self.h - 2.0 * m).max(0.0))
    }
    pub fn expand(&self, m: f32) -> Rect {
        self.shrink(-m)
    }
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
    pub fn scale_center(&self, s: f32) -> Rect {
        let c = self.center();
        Rect::new(c.x - self.w * s * 0.5, c.y - self.h * s * 0.5, self.w * s, self.h * s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKey {
    Confirm,
    Back,
    Up,
    Down,
    Left,
    Right,
    Num(u8),
    Interact,
    Phone,
    Tab,
}

#[derive(Clone, Debug, Default)]
pub struct UiInput {
    /// Pointer in physical pixels.
    pub pointer: Option<Vec2>,
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
    pub drag_dist: f32,
    pub drag_delta: Vec2,
    pub keys: Vec<UiKey>,
    pub scroll: f32,
    pub touch: bool,
}

pub struct Ui {
    pub fonts: Fonts,
    pub batch: UiBatch,
    pub scale: f32,
    pub width: f32,
    pub height: f32,
    pub phys_w: f32,
    pub phys_h: f32,
    pub time: f32,
    pub dt: f32,
    pub input: UiInput,
    anim: HashMap<u64, f32>,
    anim_touched: HashMap<u64, bool>,
    values: HashMap<u64, f32>,
    pub active: u64,
    pub pointer_over_ui: bool,
    pub click_consumed: bool,
    clip: Option<[f32; 4]>,
    cmd_start: u32,
    pub opacity: f32,
    /// Safe-area insets (logical px): left, top, right, bottom.
    pub safe: [f32; 4],
}

pub fn hash_id(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl Ui {
    pub fn new(atlas_size: u32) -> Ui {
        Ui {
            fonts: Fonts::new(atlas_size),
            batch: UiBatch::default(),
            scale: 1.0,
            width: 1280.0,
            height: 720.0,
            phys_w: 1280.0,
            phys_h: 720.0,
            time: 0.0,
            dt: 0.016,
            input: UiInput::default(),
            anim: HashMap::new(),
            anim_touched: HashMap::new(),
            values: HashMap::new(),
            active: 0,
            pointer_over_ui: false,
            click_consumed: false,
            clip: None,
            cmd_start: 0,
            opacity: 1.0,
            safe: [0.0; 4],
        }
    }

    pub fn begin(&mut self, phys_w: u32, phys_h: u32, input: UiInput, dt: f32, time: f32, user_scale: f32) {
        self.phys_w = phys_w as f32;
        self.phys_h = phys_h as f32;
        let aspect = self.phys_w / self.phys_h.max(1.0);
        let base = if aspect >= 16.0 / 9.0 {
            self.phys_h / 720.0
        } else {
            self.phys_w / 1280.0
        };
        // keep things readable on small portrait screens
        let base = base.max(self.phys_h.min(self.phys_w) / 900.0);
        self.scale = (base * user_scale).max(0.3);
        self.width = self.phys_w / self.scale;
        self.height = self.phys_h / self.scale;
        self.input = input;
        self.dt = dt;
        self.time = time;
        self.batch.verts.clear();
        self.batch.indices.clear();
        self.batch.cmds.clear();
        self.clip = None;
        self.cmd_start = 0;
        self.pointer_over_ui = false;
        self.click_consumed = false;
        self.opacity = 1.0;
        for v in self.anim_touched.values_mut() {
            *v = false;
        }
    }

    pub fn end(&mut self) -> &mut UiBatch {
        self.flush_cmd();
        self.batch.atlas_uploads.append(&mut self.fonts.uploads);
        // forget animations not used this frame
        let touched = &self.anim_touched;
        self.anim.retain(|k, _| touched.get(k).copied().unwrap_or(false));
        self.anim_touched.retain(|_, v| *v);
        &mut self.batch
    }

    fn flush_cmd(&mut self) {
        let n = self.batch.indices.len() as u32;
        if n > self.cmd_start {
            self.batch.cmds.push(UiCmd {
                start: self.cmd_start,
                count: n - self.cmd_start,
                clip: self.clip,
            });
            self.cmd_start = n;
        }
    }

    pub fn set_clip(&mut self, r: Option<Rect>) {
        self.flush_cmd();
        self.clip = r.map(|r| {
            [
                r.x * self.scale,
                r.y * self.scale,
                (r.x + r.w) * self.scale,
                (r.y + r.h) * self.scale,
            ]
        });
    }

    pub fn pointer(&self) -> Option<Vec2> {
        self.input.pointer.map(|p| p / self.scale)
    }

    pub fn key(&self, k: UiKey) -> bool {
        self.input.keys.contains(&k)
    }

    pub fn consume_key(&mut self, k: UiKey) -> bool {
        if let Some(i) = self.input.keys.iter().position(|x| *x == k) {
            self.input.keys.remove(i);
            true
        } else {
            false
        }
    }

    /// Smoothly animated value keyed by id.
    pub fn anim(&mut self, id: u64, target: f32, speed: f32) -> f32 {
        let dt = self.dt;
        let v = self.anim.entry(id).or_insert(target);
        *v += (target - *v) * (1.0 - (-speed * dt).exp());
        if (target - *v).abs() < 0.001 {
            *v = target;
        }
        self.anim_touched.insert(id, true);
        *v
    }

    pub fn anim_from(&mut self, id: u64, start: f32, target: f32, speed: f32) -> f32 {
        if !self.anim.contains_key(&id) {
            self.anim.insert(id, start);
        }
        self.anim(id, target, speed)
    }

    pub fn reset_anim(&mut self, id: u64) {
        self.anim.remove(&id);
    }

    /// Persistent value (scroll offsets and similar).
    pub fn value(&self, id: u64) -> f32 {
        self.values.get(&id).copied().unwrap_or(0.0)
    }

    pub fn set_value(&mut self, id: u64, v: f32) {
        self.values.insert(id, v);
    }

    /// Scrollable region helper: returns the current offset after applying wheel / drag.
    pub fn scroll_area(&mut self, id: u64, r: Rect, content_h: f32) -> f32 {
        let max = (content_h - r.h).max(0.0);
        let mut v = self.value(id);
        if let Some(p) = self.pointer() {
            if r.contains(p) {
                v -= self.input.scroll * 48.0;
                if self.input.down && self.input.drag_dist > 6.0 {
                    v -= self.input.drag_delta.y / self.scale;
                }
            }
        }
        v = v.clamp(0.0, max);
        self.set_value(id, v);
        v
    }

    /// Horizontal slider returning the new value in 0..1 while dragged.
    pub fn slider(&mut self, id: &str, r: Rect, value: f32, color: Color) -> Option<f32> {
        let hid = hash_id(id);
        let hit = r.expand(10.0);
        let (hover, _) = self.hit(hit);
        if self.input.pressed && hover {
            self.active = hid;
        }
        if !self.input.down && self.active == hid {
            self.active = 0;
        }
        let mut out = None;
        let mut v = value.clamp(0.0, 1.0);
        if self.active == hid {
            if let Some(p) = self.pointer() {
                v = ((p.x - r.x) / r.w).clamp(0.0, 1.0);
                out = Some(v);
            }
        }
        let track = Rect::new(r.x, r.y + r.h * 0.5 - 4.0, r.w, 8.0);
        self.rect(track, rgba(0xffffff, 30), 4.0);
        self.rect(Rect::new(track.x, track.y, track.w * v, track.h), color, 4.0);
        let k = Vec2::new(r.x + r.w * v, r.y + r.h * 0.5);
        let grow = self.anim(hid ^ 7, if hover || self.active == hid { 1.0 } else { 0.0 }, 12.0);
        self.circle(k + Vec2::new(0.0, 2.0), 13.0 + grow * 2.0, rgba(0x000000, 70));
        self.circle(k, 12.0 + grow * 2.0, pal::WHITE);
        self.circle(k, 5.0, color);
        out
    }

    // ------------------------------------------------------------ raw drawing

    fn alpha(&self, c: Color) -> [u8; 4] {
        [c[0], c[1], c[2], (c[3] as f32 * self.opacity) as u8]
    }

    #[allow(clippy::too_many_arguments)]
    fn push_quad(
        &mut self,
        corners: [Vec2; 4],
        locals: [Vec2; 4],
        size: Vec2,
        colors: [Color; 4],
        uvs: [[f32; 2]; 4],
        params: [f32; 4],
    ) {
        let base = self.batch.verts.len() as u32;
        for i in 0..4 {
            self.batch.verts.push(UiVertex {
                pos: corners[i].to_array(),
                uv: uvs[i],
                local: locals[i].to_array(),
                size: size.to_array(),
                color: self.alpha(colors[i]),
                params,
            });
        }
        self.batch
            .indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Rounded rect in logical units with optional vertical gradient.
    #[allow(clippy::too_many_arguments)]
    fn sdf_rect(&mut self, r: Rect, top: Color, bottom: Color, radius: f32, border: f32, soft: f32, mode: f32, pad: f32) {
        let s = self.scale;
        let (x0, y0, x1, y1) = (
            (r.x - pad) * s,
            (r.y - pad) * s,
            (r.x + r.w + pad) * s,
            (r.y + r.h + pad) * s,
        );
        let hw = r.w * s * 0.5;
        let hh = r.h * s * 0.5;
        let ph = pad * s;
        let corners = [
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ];
        let locals = [
            Vec2::new(-hw - ph, -hh - ph),
            Vec2::new(hw + ph, -hh - ph),
            Vec2::new(hw + ph, hh + ph),
            Vec2::new(-hw - ph, hh + ph),
        ];
        self.push_quad(
            corners,
            locals,
            Vec2::new(r.w * s, r.h * s),
            [top, top, bottom, bottom],
            [[0.0; 2]; 4],
            [radius * s, border * s, soft * s, mode],
        );
    }

    pub fn rect(&mut self, r: Rect, color: Color, radius: f32) {
        self.sdf_rect(r, color, color, radius, 0.0, 0.0, 0.0, 1.0);
    }

    pub fn rect_grad(&mut self, r: Rect, top: Color, bottom: Color, radius: f32) {
        self.sdf_rect(r, top, bottom, radius, 0.0, 0.0, 0.0, 1.0);
    }

    pub fn border(&mut self, r: Rect, color: Color, radius: f32, width: f32) {
        self.sdf_rect(r, color, color, radius, width, 0.0, 0.0, 1.0);
    }

    pub fn shadow(&mut self, r: Rect, radius: f32, blur: f32, color: Color) {
        self.sdf_rect(r, color, color, radius, 0.0, blur.max(2.0), 0.0, blur * 1.5);
    }

    /// Frosted glass panel (blurred world behind), tinted by `tint` (alpha = tint strength).
    pub fn glass(&mut self, r: Rect, radius: f32, tint: Color) {
        self.sdf_rect(r, tint, tint, radius, 0.0, 0.0, 2.0, 1.0);
    }

    /// Standard panel: soft shadow + glass + hairline border.
    pub fn panel(&mut self, r: Rect, radius: f32) {
        self.shadow(r.offset(0.0, 6.0), radius, 22.0, rgba(0x000000, 90));
        self.glass(r, radius, rgba(0x12141a, 170));
        self.border(r, pal::LINE, radius, 1.0);
    }

    pub fn circle(&mut self, c: Vec2, radius: f32, color: Color) {
        let r = Rect::new(c.x - radius, c.y - radius, radius * 2.0, radius * 2.0);
        self.sdf_rect(r, color, color, 0.0, 0.0, 0.0, 4.0, 1.0);
    }

    /// Progress ring (0..1) drawn clockwise from the top.
    pub fn ring(&mut self, c: Vec2, radius: f32, thickness: f32, progress: f32, color: Color) {
        let s = self.scale;
        let r = Rect::new(c.x - radius, c.y - radius, radius * 2.0, radius * 2.0);
        let (x0, y0, x1, y1) = (r.x * s, r.y * s, r.right() * s, r.bottom() * s);
        let h = radius * s;
        let p = progress.clamp(0.0, 1.0);
        self.push_quad(
            [Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)],
            [Vec2::new(-h, -h), Vec2::new(h, -h), Vec2::new(h, h), Vec2::new(-h, h)],
            Vec2::new(2.0 * h, 2.0 * h),
            [color; 4],
            [[p, 0.0]; 4],
            [thickness * s, 0.0, 0.0, 3.0],
        );
    }

    /// Anti-aliased capsule line.
    pub fn line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        let s = self.scale;
        let (a, b) = (a * s, b * s);
        let d = b - a;
        let len = d.length().max(0.001);
        let dir = d / len;
        let nrm = Vec2::new(-dir.y, dir.x);
        let hw = width * s * 0.5 + 1.0;
        let c = (a + b) * 0.5;
        let hl = len * 0.5 + hw;
        let corners = [
            c - dir * hl - nrm * hw,
            c + dir * hl - nrm * hw,
            c + dir * hl + nrm * hw,
            c - dir * hl + nrm * hw,
        ];
        let locals = [
            Vec2::new(-hl, -hw),
            Vec2::new(hl, -hw),
            Vec2::new(hl, hw),
            Vec2::new(-hl, hw),
        ];
        let size = Vec2::new(len + width * s, width * s);
        self.push_quad(corners, locals, size, [color; 4], [[0.0; 2]; 4], [width * s * 0.5, 0.0, 0.0, 0.0]);
    }

    // ------------------------------------------------------------ text

    fn px(&self, size: f32) -> u32 {
        (size * self.scale).round().max(4.0) as u32
    }

    pub fn measure(&mut self, text: &str, size: f32, font: usize) -> f32 {
        let px = self.px(size);
        self.fonts.measure(font, text, px) / self.scale
    }

    /// Draws a single line; (x, y) is the top-left of the line box. Returns logical width.
    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, font: usize, color: Color) -> f32 {
        let px = self.px(size);
        let s = self.scale;
        let (ascent, _) = self.fonts.line_metrics(font, px);
        let mut pen_x = (x * s).round();
        let base_y = (y * s + ascent).round();
        let start = pen_x;
        let mut prev: Option<char> = None;
        let color = color;
        for ch in text.chars() {
            if let Some(p) = prev {
                pen_x += self.fonts.kern(font, p, ch, px);
            }
            let g = self.fonts.glyph(font, ch, px);
            if g.w > 0.0 {
                let gx = (pen_x + g.xmin).round();
                let gy = (base_y - g.ymin - g.h).round();
                let corners = [
                    Vec2::new(gx, gy),
                    Vec2::new(gx + g.w, gy),
                    Vec2::new(gx + g.w, gy + g.h),
                    Vec2::new(gx, gy + g.h),
                ];
                let uvs = [
                    [g.uv[0], g.uv[1]],
                    [g.uv[2], g.uv[1]],
                    [g.uv[2], g.uv[3]],
                    [g.uv[0], g.uv[3]],
                ];
                self.push_quad(corners, [Vec2::ZERO; 4], Vec2::ZERO, [color; 4], uvs, [0.0, 0.0, 0.0, 1.0]);
            }
            pen_x += g.advance;
            prev = Some(ch);
        }
        (pen_x - start) / s
    }

    pub fn text_shadowed(&mut self, x: f32, y: f32, text: &str, size: f32, font: usize, color: Color) -> f32 {
        let a = color[3] as f32 / 255.0;
        self.text(x, y + 1.5, text, size, font, rgba(0x000000, (140.0 * a) as u8));
        self.text(x, y, text, size, font, color)
    }

    pub fn text_in(&mut self, r: Rect, text: &str, size: f32, font: usize, color: Color, align: Align) {
        let w = self.measure(text, size, font);
        let x = match align {
            Align::Left => r.x,
            Align::Center => r.x + (r.w - w) * 0.5,
            Align::Right => r.right() - w,
        };
        let y = r.y + (r.h - size * 1.2) * 0.5;
        self.text(x, y, text, size, font, color);
    }

    pub fn wrap_lines(&mut self, text: &str, size: f32, font: usize, max_w: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if self.measure(&candidate, size, font) > max_w && !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Wrapped paragraph. `reveal` limits the number of characters drawn (typewriter).
    #[allow(clippy::too_many_arguments)]
    pub fn paragraph(
        &mut self,
        x: f32,
        y: f32,
        max_w: f32,
        text: &str,
        size: f32,
        font: usize,
        color: Color,
        line_h: f32,
        reveal: Option<usize>,
    ) -> f32 {
        let lines = self.wrap_lines(text, size, font, max_w);
        let mut remaining = reveal.unwrap_or(usize::MAX);
        let mut yy = y;
        for l in &lines {
            if remaining == 0 {
                break;
            }
            let n = l.chars().count();
            if remaining >= n {
                self.text(x, yy, l, size, font, color);
                remaining -= n;
                remaining = remaining.saturating_sub(1);
            } else {
                let part: String = l.chars().take(remaining).collect();
                self.text(x, yy, &part, size, font, color);
                remaining = 0;
            }
            yy += size * line_h;
        }
        lines.len() as f32 * size * line_h
    }

    pub fn paragraph_height(&mut self, max_w: f32, text: &str, size: f32, font: usize, line_h: f32) -> f32 {
        self.wrap_lines(text, size, font, max_w).len() as f32 * size * line_h
    }

    // ------------------------------------------------------------ interaction

    /// Returns (hovered, clicked) for a rect and marks the pointer as over UI.
    pub fn hit(&mut self, r: Rect) -> (bool, bool) {
        let Some(p) = self.pointer() else {
            return (false, false);
        };
        if r.contains(p) {
            self.pointer_over_ui = true;
            let clicked = self.input.released && self.input.drag_dist < 14.0 && !self.click_consumed;
            if clicked {
                self.click_consumed = true;
            }
            (true, clicked)
        } else {
            (false, false)
        }
    }

    /// Blocks world interaction under this rect.
    pub fn block(&mut self, r: Rect) {
        if let Some(p) = self.pointer() {
            if r.contains(p) {
                self.pointer_over_ui = true;
            }
        }
    }

    pub fn button(&mut self, id: &str, r: Rect, label: &str, style: ButtonStyle) -> bool {
        let hid = hash_id(id);
        let (hover, clicked) = self.hit(r);
        let pressed = hover && self.input.down;
        let h = self.anim(hid, if hover { 1.0 } else { 0.0 }, 14.0);
        let pr = self.anim(hid ^ 0x55, if pressed { 1.0 } else { 0.0 }, 20.0);
        let rr = r.scale_center(1.0 - 0.03 * pr + 0.015 * h);
        let radius = style.radius.min(rr.h * 0.5);
        match style.kind {
            ButtonKind::Primary => {
                self.shadow(rr.offset(0.0, 4.0), radius, 14.0, rgba(0x000000, 80));
                let top = mix(style.color, pal::WHITE, 0.12 + 0.1 * h);
                self.rect_grad(rr, top, style.color, radius);
                self.text_in(rr, label, style.size, FONT_BOLD, pal::INK, Align::Center);
            }
            ButtonKind::Ghost => {
                self.glass(rr, radius, rgba(0x1a1d24, (150.0 + 40.0 * h) as u8));
                self.border(rr, with_alpha(style.color, 0.35 + 0.5 * h), radius, 1.2);
                self.text_in(rr, label, style.size, FONT_BOLD, mix(pal::TEXT, style.color, h * 0.6), Align::Center);
            }
            ButtonKind::Subtle => {
                self.rect(rr, rgba(0xffffff, (10.0 + 22.0 * h) as u8), radius);
                self.text_in(rr, label, style.size, FONT_REGULAR, pal::TEXT, Align::Center);
            }
        }
        clicked
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Ghost,
    Subtle,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonStyle {
    pub kind: ButtonKind,
    pub color: Color,
    pub size: f32,
    pub radius: f32,
}

impl ButtonStyle {
    pub fn primary() -> Self {
        ButtonStyle {
            kind: ButtonKind::Primary,
            color: pal::ACCENT,
            size: 17.0,
            radius: 14.0,
        }
    }
    pub fn ghost() -> Self {
        ButtonStyle {
            kind: ButtonKind::Ghost,
            color: pal::ACCENT,
            size: 16.0,
            radius: 14.0,
        }
    }
    pub fn subtle() -> Self {
        ButtonStyle {
            kind: ButtonKind::Subtle,
            color: pal::TEXT,
            size: 15.0,
            radius: 10.0,
        }
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}
