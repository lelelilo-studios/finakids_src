//! Small vector icons composed from SDF primitives.

use super::{rgba, Color, Rect, Ui};
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Coin,
    Piggy,
    Target,
    Card,
    Chart,
    Briefcase,
    Bag,
    Phone,
    Chat,
    Sun,
    Moon,
    Sunset,
    Check,
    Cross,
    Arrow,
    Star,
    Lock,
    Book,
    Bank,
    Home,
    Warning,
    Calendar,
    Heart,
    Gift,
    Store,
    Clock,
    Bulb,
}

pub fn draw(ui: &mut Ui, icon: Icon, c: Vec2, size: f32, color: Color) {
    let s = size / 24.0;
    let p = |x: f32, y: f32| c + Vec2::new(x, y) * s;
    let w = 2.0 * s;
    let dark = rgba(0x000000, (color[3] as f32 * 0.35) as u8);
    match icon {
        Icon::Coin => {
            ui.circle(c, 10.0 * s, color);
            ui.circle(c, 7.5 * s, dark);
            ui.circle(c, 6.0 * s, color);
            ui.line(p(0.0, -4.0), p(0.0, 4.0), 1.6 * s, dark);
            ui.line(p(-2.5, -1.5), p(2.5, 1.5), 1.6 * s, dark);
        }
        Icon::Piggy => {
            ui.rect(Rect::new(c.x - 9.0 * s, c.y - 6.0 * s, 17.0 * s, 12.0 * s), color, 6.0 * s);
            ui.circle(p(8.5, 0.0), 2.8 * s, color);
            ui.circle(p(-3.0, -6.5), 2.4 * s, color);
            ui.rect(Rect::new(c.x - 6.0 * s, c.y + 4.0 * s, 3.0 * s, 5.0 * s), color, 1.0 * s);
            ui.rect(Rect::new(c.x + 2.0 * s, c.y + 4.0 * s, 3.0 * s, 5.0 * s), color, 1.0 * s);
            ui.rect(Rect::new(c.x - 2.5 * s, c.y - 6.5 * s, 5.0 * s, 1.6 * s), dark, 0.8 * s);
            ui.circle(p(3.5, -1.5), 1.0 * s, dark);
        }
        Icon::Target => {
            ui.circle(c, 10.0 * s, color);
            ui.circle(c, 7.5 * s, dark);
            ui.circle(c, 5.0 * s, color);
            ui.circle(c, 2.5 * s, dark);
        }
        Icon::Card => {
            let r = Rect::new(c.x - 10.0 * s, c.y - 7.0 * s, 20.0 * s, 14.0 * s);
            ui.rect(r, color, 2.5 * s);
            ui.rect(Rect::new(r.x, r.y + 3.0 * s, r.w, 3.0 * s), dark, 0.0);
            ui.rect(Rect::new(r.x + 3.0 * s, r.y + 9.0 * s, 6.0 * s, 2.0 * s), dark, 1.0 * s);
        }
        Icon::Chart => {
            for (i, h) in [5.0, 9.0, 7.0, 13.0].iter().enumerate() {
                let x = c.x - 9.0 * s + i as f32 * 5.0 * s;
                ui.rect(Rect::new(x, c.y + 8.0 * s - h * s, 3.5 * s, h * s), color, 1.0 * s);
            }
            ui.line(p(-9.0, 1.0), p(-3.0, -3.0), w * 0.8, color);
            ui.line(p(-3.0, -3.0), p(2.0, -1.0), w * 0.8, color);
            ui.line(p(2.0, -1.0), p(9.0, -9.0), w * 0.8, color);
        }
        Icon::Briefcase => {
            ui.rect(Rect::new(c.x - 10.0 * s, c.y - 5.0 * s, 20.0 * s, 13.0 * s), color, 2.5 * s);
            ui.border(Rect::new(c.x - 4.0 * s, c.y - 9.0 * s, 8.0 * s, 6.0 * s), color, 2.0 * s, 1.8 * s);
            ui.rect(Rect::new(c.x - 10.0 * s, c.y, 20.0 * s, 1.5 * s), dark, 0.0);
        }
        Icon::Bag | Icon::Store => {
            ui.rect(Rect::new(c.x - 8.0 * s, c.y - 4.0 * s, 16.0 * s, 13.0 * s), color, 2.5 * s);
            ui.border(Rect::new(c.x - 4.5 * s, c.y - 10.0 * s, 9.0 * s, 10.0 * s), color, 4.5 * s, 1.8 * s);
        }
        Icon::Phone => {
            let r = Rect::new(c.x - 6.0 * s, c.y - 10.0 * s, 12.0 * s, 20.0 * s);
            ui.rect(r, color, 2.8 * s);
            ui.rect(r.shrink(1.6 * s), dark, 1.8 * s);
            ui.rect(Rect::new(c.x - 2.0 * s, c.y + 7.3 * s, 4.0 * s, 1.0 * s), color, 0.5 * s);
        }
        Icon::Chat => {
            ui.rect(Rect::new(c.x - 10.0 * s, c.y - 8.0 * s, 20.0 * s, 14.0 * s), color, 5.0 * s);
            ui.line(p(-4.0, 5.0), p(-7.0, 10.0), 3.0 * s, color);
            for i in 0..3 {
                ui.circle(p(-5.0 + i as f32 * 5.0, -1.0), 1.4 * s, dark);
            }
        }
        Icon::Sun => {
            ui.circle(c, 5.0 * s, color);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let d = Vec2::new(a.cos(), a.sin());
                ui.line(c + d * 7.5 * s, c + d * 10.0 * s, 1.8 * s, color);
            }
        }
        Icon::Moon => {
            ui.circle(c, 8.0 * s, color);
            ui.circle(p(4.0, -3.0), 7.0 * s, rgba(0x14161c, 255));
        }
        Icon::Sunset => {
            ui.circle(p(0.0, 3.0), 6.0 * s, color);
            ui.rect(Rect::new(c.x - 10.0 * s, c.y + 4.0 * s, 20.0 * s, 6.0 * s), rgba(0x14161c, 255), 0.0);
            ui.line(p(-10.0, 5.0), p(10.0, 5.0), 1.8 * s, color);
            for i in 0..5 {
                let a = std::f32::consts::PI * (1.0 + i as f32 / 4.0);
                let d = Vec2::new(a.cos(), a.sin());
                ui.line(p(0.0, 3.0) + d * 8.0 * s, p(0.0, 3.0) + d * 10.5 * s, 1.6 * s, color);
            }
        }
        Icon::Check => {
            ui.line(p(-7.0, 0.0), p(-2.0, 5.0), 2.6 * s, color);
            ui.line(p(-2.0, 5.0), p(8.0, -6.0), 2.6 * s, color);
        }
        Icon::Cross => {
            ui.line(p(-6.0, -6.0), p(6.0, 6.0), 2.4 * s, color);
            ui.line(p(-6.0, 6.0), p(6.0, -6.0), 2.4 * s, color);
        }
        Icon::Arrow => {
            ui.line(p(-8.0, 0.0), p(7.0, 0.0), 2.4 * s, color);
            ui.line(p(2.0, -5.0), p(8.0, 0.0), 2.4 * s, color);
            ui.line(p(2.0, 5.0), p(8.0, 0.0), 2.4 * s, color);
        }
        Icon::Star => {
            let mut pts = Vec::new();
            for i in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 10.0 * std::f32::consts::TAU;
                let r = if i % 2 == 0 { 10.0 } else { 4.2 };
                pts.push(c + Vec2::new(a.cos(), a.sin()) * r * s);
            }
            for i in 0..10 {
                ui.line(pts[i], pts[(i + 1) % 10], 2.0 * s, color);
            }
            ui.circle(c, 4.5 * s, color);
        }
        Icon::Lock => {
            ui.rect(Rect::new(c.x - 7.0 * s, c.y - 2.0 * s, 14.0 * s, 11.0 * s), color, 2.0 * s);
            ui.border(Rect::new(c.x - 4.5 * s, c.y - 9.0 * s, 9.0 * s, 10.0 * s), color, 4.5 * s, 2.0 * s);
        }
        Icon::Book => {
            ui.rect(Rect::new(c.x - 9.0 * s, c.y - 9.0 * s, 8.0 * s, 17.0 * s), color, 1.5 * s);
            ui.rect(Rect::new(c.x + 1.0 * s, c.y - 9.0 * s, 8.0 * s, 17.0 * s), color, 1.5 * s);
            ui.line(p(0.0, -9.0), p(0.0, 9.0), 1.0 * s, dark);
        }
        Icon::Bank => {
            ui.line(p(-10.0, -4.0), p(0.0, -10.0), 2.0 * s, color);
            ui.line(p(10.0, -4.0), p(0.0, -10.0), 2.0 * s, color);
            ui.rect(Rect::new(c.x - 10.0 * s, c.y - 4.5 * s, 20.0 * s, 2.0 * s), color, 0.5 * s);
            for i in 0..4 {
                let x = c.x - 8.0 * s + i as f32 * 5.0 * s;
                ui.rect(Rect::new(x, c.y - 1.5 * s, 2.5 * s, 8.0 * s), color, 0.5 * s);
            }
            ui.rect(Rect::new(c.x - 10.0 * s, c.y + 7.5 * s, 20.0 * s, 2.0 * s), color, 0.5 * s);
        }
        Icon::Home => {
            ui.line(p(-10.0, 0.0), p(0.0, -9.0), 2.2 * s, color);
            ui.line(p(10.0, 0.0), p(0.0, -9.0), 2.2 * s, color);
            ui.rect(Rect::new(c.x - 7.0 * s, c.y - 1.0 * s, 14.0 * s, 10.0 * s), color, 1.5 * s);
            ui.rect(Rect::new(c.x - 2.0 * s, c.y + 3.0 * s, 4.0 * s, 6.0 * s), dark, 1.0 * s);
        }
        Icon::Warning => {
            ui.line(p(0.0, -9.0), p(-9.5, 8.0), 2.4 * s, color);
            ui.line(p(0.0, -9.0), p(9.5, 8.0), 2.4 * s, color);
            ui.line(p(-9.5, 8.0), p(9.5, 8.0), 2.4 * s, color);
            ui.line(p(0.0, -3.0), p(0.0, 2.5), 2.2 * s, color);
            ui.circle(p(0.0, 5.3), 1.3 * s, color);
        }
        Icon::Calendar => {
            ui.rect(Rect::new(c.x - 9.0 * s, c.y - 7.0 * s, 18.0 * s, 16.0 * s), color, 2.5 * s);
            ui.rect(Rect::new(c.x - 9.0 * s, c.y - 2.0 * s, 18.0 * s, 11.0 * s), dark, 0.0);
            ui.line(p(-4.0, -10.0), p(-4.0, -6.0), 2.0 * s, color);
            ui.line(p(4.0, -10.0), p(4.0, -6.0), 2.0 * s, color);
        }
        Icon::Heart => {
            ui.circle(p(-4.2, -2.5), 5.2 * s, color);
            ui.circle(p(4.2, -2.5), 5.2 * s, color);
            ui.line(p(-8.0, 0.5), p(0.0, 8.5), 4.6 * s, color);
            ui.line(p(8.0, 0.5), p(0.0, 8.5), 4.6 * s, color);
            ui.circle(p(0.0, 1.0), 4.5 * s, color);
        }
        Icon::Gift => {
            ui.rect(Rect::new(c.x - 9.0 * s, c.y - 3.0 * s, 18.0 * s, 12.0 * s), color, 1.5 * s);
            ui.rect(Rect::new(c.x - 10.0 * s, c.y - 6.0 * s, 20.0 * s, 4.0 * s), color, 1.5 * s);
            ui.rect(Rect::new(c.x - 1.3 * s, c.y - 6.0 * s, 2.6 * s, 15.0 * s), dark, 0.0);
            ui.circle(p(-3.0, -8.0), 2.6 * s, color);
            ui.circle(p(3.0, -8.0), 2.6 * s, color);
        }
        Icon::Clock => {
            ui.circle(c, 10.0 * s, color);
            ui.circle(c, 8.0 * s, dark);
            ui.line(c, p(0.0, -6.0), 1.8 * s, color);
            ui.line(c, p(4.0, 2.0), 1.8 * s, color);
        }
        Icon::Bulb => {
            ui.circle(p(0.0, -2.0), 7.0 * s, color);
            ui.rect(Rect::new(c.x - 3.5 * s, c.y + 3.0 * s, 7.0 * s, 6.0 * s), color, 1.5 * s);
            ui.line(p(-3.0, 6.5), p(3.0, 6.5), 1.0 * s, dark);
        }
    }
}
