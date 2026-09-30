//! All in-game interface: HUD, dialogue, choices, phone apps, shop and modal screens.

use super::business::{BizAct, BizPhase, BizUi, KIT_LARGE, KIT_SMALL, STALL_RENT};
use super::finance::{Finance, InvestKind, Skill};
use super::items::Shop;
use super::minigame::{CafeGame, MiniAct, Prod};
use super::script::Who;
use super::story;
use super::time::Phase;
use super::*;
use crate::gfx::Camera;
use crate::math::{money, money_signed};
use crate::ui::icons::{self, Icon};
use crate::ui::{hash_id, mix, pal, rgba, with_alpha, Align, ButtonStyle, Color, Rect, Ui, UiKey, FONT_BOLD, FONT_REGULAR};
use glam::Vec2;

const M: f32 = 22.0;

fn icon_of(name: &str) -> Icon {
    match name {
        "bank" => Icon::Bank,
        "chat" => Icon::Chat,
        "piggy" => Icon::Piggy,
        "goal" | "target" => Icon::Target,
        "phone" => Icon::Phone,
        "bag" => Icon::Bag,
        "check" => Icon::Check,
        "chart" => Icon::Chart,
        "lock" => Icon::Lock,
        "warning" => Icon::Warning,
        "star" => Icon::Star,
        "briefcase" => Icon::Briefcase,
        "card" => Icon::Card,
        "gift" => Icon::Gift,
        _ => Icon::Bulb,
    }
}

fn phase_icon(p: Phase) -> Icon {
    match p {
        Phase::Morning | Phase::Afternoon => Icon::Sun,
        Phase::Evening => Icon::Sunset,
        Phase::Night => Icon::Moon,
    }
}

fn amount_color(v: i64) -> Color {
    if v > 0 {
        pal::GREEN
    } else if v < 0 {
        pal::RED
    } else {
        pal::TEXT_DIM
    }
}

pub fn draw(ui: &mut Ui, s: &State, cam: &Camera, acts: &mut Vec<UiAct>) {
    if s.screen == Screen::Title {
        title_screen(ui, s, acts);
        return;
    }
    let cutscene = s.letterbox > 0.3;
    let hud_alpha = ui.anim(hash_id("hud_alpha"), if cutscene || s.modal.is_some() { 0.0 } else { 1.0 }, 6.0);
    if hud_alpha > 0.01 {
        ui.opacity = hud_alpha;
        world_labels(ui, s, cam);
        top_left(ui, s);
        money_panel(ui, s);
        bottom_right(ui, s, acts);
        interact_prompt(ui, s, acts);
        hint(ui, s);
        ui.opacity = 1.0;
    }
    toasts(ui, s);
    if let Some(d) = &s.dialog {
        dialogue(ui, d, acts);
    }
    if let Some(c) = &s.choice {
        choices(ui, c, acts);
    }
    if let Some((a, b, t)) = &s.title_card {
        title_card(ui, a, b, *t);
    }
    if let Some(m) = &s.modal {
        modal(ui, s, m, acts);
    }
    if s.debug {
        let t = format!("{:.0} fps · semana {} · hora {:.1}", s.fps, s.week, s.hour);
        ui.text(12.0, ui.height - 24.0, &t, 12.0, FONT_REGULAR, pal::TEXT_DIM);
    }
}

// ------------------------------------------------------------------ title

fn title_screen(ui: &mut Ui, s: &State, acts: &mut Vec<UiAct>) {
    let w = ui.width;
    let h = ui.height;
    // left gradient for readability
    ui.rect_grad(Rect::new(0.0, 0.0, w * 0.55, h), rgba(0x0b0c10, 200), rgba(0x0b0c10, 120), 0.0);
    ui.rect(Rect::new(w * 0.55, 0.0, w * 0.2, h), rgba(0x0b0c10, 40), 0.0);
    let appear = ui.anim_from(hash_id("title_in"), 0.0, 1.0, 1.6);
    let x = 72.0;
    let y = h * 0.26;
    ui.opacity = appear;
    let tw = ui.measure("FINAKIDS", 84.0, FONT_BOLD);
    ui.text_shadowed(x, y - 20.0 * (1.0 - appear), "FINAKIDS", 84.0, FONT_BOLD, pal::WHITE);
    ui.rect_grad(Rect::new(x + 2.0, y + 102.0, tw * 0.42, 4.0), pal::ACCENT, rgba(0xffb35c, 0), 2.0);
    ui.text(x, y + 120.0, "Tu vida. Tu dinero. Tus decisiones.", 24.0, FONT_REGULAR, pal::TEXT);
    ui.paragraph(
        x,
        y + 166.0,
        460.0,
        "Una simulación de vida en 3D: recibe dinero, trabaja, ahorra, emprende, invierte... y vive las consecuencias de cada decisión.",
        16.0,
        FONT_REGULAR,
        pal::TEXT_DIM,
        1.5,
        None,
    );
    let by = y + 260.0;
    let loading = !s.build_queue.is_empty();
    if loading {
        let r = Rect::new(x, by, 300.0, 56.0);
        ui.glass(r, 16.0, rgba(0x1a1d24, 170));
        let t = s.time;
        for i in 0..3 {
            let a = ((t * 3.0 - i as f32 * 0.4).sin() * 0.5 + 0.5) * 0.8 + 0.2;
            ui.circle(Vec2::new(r.x + 28.0 + i as f32 * 14.0, r.center().y), 4.0, with_alpha(pal::ACCENT, a));
        }
        ui.text(r.x + 76.0, r.y + 17.0, "Preparando el mundo...", 17.0, FONT_REGULAR, pal::TEXT);
    } else {
        if ui.button("start", Rect::new(x, by, 260.0, 58.0), "Comenzar historia", ButtonStyle::primary().size(19.0)) || ui.consume_key(UiKey::Confirm) {
            acts.push(UiAct::StartGame(false));
        }
        if s.has_save && ui.button("continue", Rect::new(x + 276.0, by, 200.0, 58.0), "Continuar", ButtonStyle::ghost().size(18.0)) {
            acts.push(UiAct::StartGame(true));
        }
    }
    let help = "Mover: WASD / flechas o toca el suelo  ·  Interactuar: E  ·  Teléfono: TAB  ·  Cámara: arrastrar, Q / R, rueda";
    ui.text(x, h - 54.0, help, 13.0, FONT_REGULAR, pal::TEXT_MUTED);
    ui.text(x, h - 32.0, "Hecho con Rust + wgpu · Todo el mundo y los personajes son generados proceduralmente.", 12.0, FONT_REGULAR, with_alpha(pal::TEXT_MUTED, 0.7));
    ui.opacity = 1.0;
}

// ------------------------------------------------------------------ HUD

fn top_left(ui: &mut Ui, s: &State) {
    let r = Rect::new(M, M, 262.0, 62.0);
    ui.panel(r, 18.0);
    let ic = Vec2::new(r.x + 31.0, r.center().y);
    ui.circle(ic, 20.0, rgba(0xffffff, 18));
    let col = match s.phase {
        Phase::Night => pal::PURPLE,
        Phase::Evening => pal::ACCENT,
        _ => pal::YELLOW,
    };
    icons::draw(ui, phase_icon(s.phase), ic, 22.0, col);
    ui.text(r.x + 60.0, r.y + 11.0, &format!("Semana {} · Sábado", s.week), 16.0, FONT_BOLD, pal::TEXT);
    ui.text(r.x + 60.0, r.y + 33.0, &format!("{} · {}", s.phase.label(), clock(s.hour)), 13.0, FONT_REGULAR, pal::TEXT_DIM);
    if let Some(o) = &s.objective {
        let lines = ui.wrap_lines(o, 14.0, FONT_REGULAR, 300.0);
        let hh = 22.0 + lines.len() as f32 * 19.0;
        let orr = Rect::new(M, r.bottom() + 10.0, 350.0, hh);
        ui.panel(orr, 14.0);
        icons::draw(ui, Icon::Target, Vec2::new(orr.x + 20.0, orr.y + 20.0), 16.0, pal::ACCENT);
        for (i, l) in lines.iter().enumerate() {
            ui.text(orr.x + 38.0, orr.y + 10.0 + i as f32 * 19.0, l, 14.0, FONT_REGULAR, pal::TEXT);
        }
    }
}

fn clock(hour: f32) -> String {
    let h = hour.rem_euclid(24.0);
    let hh = h.floor() as i32;
    let mm = ((h - h.floor()) * 60.0) as i32 / 5 * 5;
    format!("{hh:02}:{mm:02}")
}

fn money_panel(ui: &mut Ui, s: &State) {
    let w = 470.0;
    let r = Rect::new(ui.width - M - w, M, w, 62.0);
    ui.panel(r, 18.0);
    let cols = [
        (Icon::Coin, "Billetera", s.shown_wallet.round() as i64, pal::YELLOW),
        (Icon::Piggy, "Ahorro", s.shown_savings.round() as i64, pal::ACCENT2),
    ];
    for (i, (ic, label, v, c)) in cols.iter().enumerate() {
        let x = r.x + 16.0 + i as f32 * 150.0;
        ui.circle(Vec2::new(x + 16.0, r.center().y), 16.0, with_alpha(*c, 0.18));
        icons::draw(ui, *ic, Vec2::new(x + 16.0, r.center().y), 18.0, *c);
        ui.text(x + 40.0, r.y + 11.0, label, 12.0, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(x + 40.0, r.y + 27.0, &money(*v), 19.0, FONT_BOLD, pal::TEXT);
    }
    // main goal ring
    let gx = r.x + 316.0;
    if let Some(g) = s.fin.main_goal() {
        let c = Vec2::new(gx + 18.0, r.center().y);
        ui.ring(c, 18.0, 4.0, g.progress(), pal::ACCENT);
        ui.text_in(Rect::new(c.x - 18.0, c.y - 9.0, 36.0, 18.0), &format!("{:.0}", g.progress() * 100.0), 10.0, FONT_BOLD, pal::TEXT, Align::Center);
        let name: String = g.name.chars().take(16).collect();
        ui.text(gx + 44.0, r.y + 11.0, "Meta", 12.0, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(gx + 44.0, r.y + 28.0, &name, 13.0, FONT_BOLD, pal::TEXT);
    } else {
        icons::draw(ui, Icon::Target, Vec2::new(gx + 18.0, r.center().y), 20.0, pal::TEXT_MUTED);
        ui.text(gx + 44.0, r.y + 11.0, "Meta", 12.0, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(gx + 44.0, r.y + 28.0, "Sin meta", 13.0, FONT_BOLD, pal::TEXT_MUTED);
    }
    // floating deltas
    for f in &s.floats {
        let x = r.x + 56.0 + f.slot as f32 * 150.0;
        let a = 1.0 - (f.age / 2.2);
        let y = r.bottom() + 6.0 + f.age * 14.0;
        ui.text_shadowed(x, y, &f.text, 16.0, FONT_BOLD, with_alpha(f.color, a));
    }
    // debt chip
    let debt = s.fin.total_debt();
    if debt > 0 {
        let t = format!("Deuda: {}", money(debt));
        let tw = ui.measure(&t, 13.0, FONT_BOLD);
        let dr = Rect::new(r.right() - tw - 40.0, r.bottom() + 8.0, tw + 40.0, 28.0);
        ui.glass(dr, 14.0, rgba(0x3a1216, 190));
        icons::draw(ui, Icon::Card, Vec2::new(dr.x + 16.0, dr.center().y), 14.0, pal::RED);
        ui.text(dr.x + 30.0, dr.y + 6.0, &t, 13.0, FONT_BOLD, pal::RED);
    }
}

fn bottom_right(ui: &mut Ui, s: &State, acts: &mut Vec<UiAct>) {
    let c = Vec2::new(ui.width - M - 38.0, ui.height - M - 38.0);
    let r = Rect::new(c.x - 38.0, c.y - 38.0, 76.0, 76.0);
    let (hover, clicked) = ui.hit(r);
    let hv = ui.anim(hash_id("phone_btn"), if hover { 1.0 } else { 0.0 }, 12.0);
    ui.shadow(r.offset(0.0, 4.0), 38.0, 16.0, rgba(0x000000, 90));
    ui.glass(r.scale_center(1.0 + hv * 0.05), 38.0, rgba(0x14161c, 180));
    ui.border(r.scale_center(1.0 + hv * 0.05), with_alpha(pal::ACCENT, 0.3 + 0.5 * hv), 38.0, 1.5);
    icons::draw(ui, Icon::Phone, c, 30.0, pal::TEXT);
    if s.unread > 0 {
        let b = c + Vec2::new(24.0, -24.0);
        let pulse = 1.0 + 0.08 * (s.time * 5.0).sin();
        ui.circle(b, 12.0 * pulse, pal::RED);
        ui.text_in(Rect::new(b.x - 12.0, b.y - 9.0, 24.0, 18.0), &s.unread.min(9).to_string(), 12.0, FONT_BOLD, pal::WHITE, Align::Center);
    }
    ui.text_in(Rect::new(r.x - 20.0, r.bottom() + 2.0, r.w + 40.0, 16.0), "Teléfono", 11.0, FONT_REGULAR, pal::TEXT_DIM, Align::Center);
    if clicked || ui.consume_key(UiKey::Phone) {
        acts.push(UiAct::OpenPhone(PhoneApp::Home));
    }
}

fn interact_prompt(ui: &mut Ui, s: &State, acts: &mut Vec<UiAct>) {
    let label = if let Some(ci) = s.near_char {
        let n = match s.chars[ci].id {
            "mama" => "Mamá",
            "tomas" => "Tomás",
            "vale" => "Vale",
            "julio" => "Don Julio",
            _ => "",
        };
        Some(format!("Hablar con {n}"))
    } else {
        s.near.map(|i| s.loc().interact[i].label.clone())
    };
    let a = ui.anim(hash_id("prompt"), if label.is_some() { 1.0 } else { 0.0 }, 10.0);
    if a < 0.01 {
        return;
    }
    let text = label.unwrap_or_default();
    let tw = ui.measure(&text, 17.0, FONT_BOLD);
    let w = tw + 88.0;
    let r = Rect::new((ui.width - w) * 0.5, ui.height - M - 64.0 + (1.0 - a) * 20.0, w, 54.0);
    let (hover, clicked) = ui.hit(r);
    let old = ui.opacity;
    ui.opacity *= a;
    ui.shadow(r.offset(0.0, 4.0), 27.0, 16.0, rgba(0x000000, 90));
    ui.glass(r, 27.0, rgba(0x14161c, if hover { 210 } else { 180 }));
    ui.border(r, with_alpha(pal::ACCENT, 0.5 + 0.4 * hover as i32 as f32), 27.0, 1.5);
    let key = Rect::new(r.x + 10.0, r.y + 10.0, 34.0, 34.0);
    ui.rect(key, pal::ACCENT, 10.0);
    ui.text_in(key, if ui.input.touch { "•" } else { "E" }, 17.0, FONT_BOLD, pal::INK, Align::Center);
    ui.text(r.x + 56.0, r.y + 15.0, &text, 17.0, FONT_BOLD, pal::TEXT);
    ui.opacity = old;
    if clicked {
        acts.push(UiAct::Interact);
    }
}

fn world_labels(ui: &mut Ui, s: &State, cam: &Camera) {
    let (pw, ph) = (ui.phys_w, ui.phys_h);
    let scale = ui.scale;
    let draw_label = |ui: &mut Ui, p: glam::Vec3, t: &str, strong: bool| {
        if let Some(sp) = cam.project(p, pw, ph) {
            let sp = sp / scale;
            let tw = ui.measure(t, 13.0, FONT_BOLD);
            let r = Rect::new(sp.x - tw * 0.5 - 12.0, sp.y - 16.0, tw + 24.0, 28.0);
            ui.glass(r, 14.0, rgba(0x14161c, if strong { 190 } else { 140 }));
            if strong {
                ui.border(r, with_alpha(pal::ACCENT, 0.6), 14.0, 1.2);
            }
            ui.text(r.x + 12.0, r.y + 6.0, t, 13.0, FONT_BOLD, if strong { pal::TEXT } else { pal::TEXT_DIM });
        }
    };
    let loc = s.loc();
    let p = s.player().anim.pos;
    for (i, it) in loc.interact.iter().enumerate() {
        if !it.enabled {
            continue;
        }
        let d = (it.pos - p).length();
        let near = s.near == Some(i);
        if near {
            draw_label(ui, it.pos + glam::Vec3::Y * 0.35, &it.label, true);
        } else if s.near.is_none() && (d < 3.2 && !loc.interior || d < 1.9) {
            let a = 1.0 - ((d - 1.0) / 1.5).clamp(0.0, 1.0);
            if a > 0.05 {
                let old = ui.opacity;
                ui.opacity *= a * 0.8;
                draw_label(ui, it.pos + glam::Vec3::Y * 0.3, &it.label, false);
                ui.opacity = old;
            }
        }
    }
    for (i, c) in s.chars.iter().enumerate().skip(1) {
        if s.char_loc[i] != s.loc_id() || c.id.starts_with("npc") || !c.visible {
            continue;
        }
        let d = (c.anim.pos - p).length();
        if d < 6.0 {
            let name = match c.id {
                "mama" => "Mamá",
                "tomas" => "Tomás",
                "vale" => "Vale",
                "julio" => "Don Julio",
                _ => "",
            };
            let a = 1.0 - ((d - 2.0) / 4.0).clamp(0.0, 1.0);
            let old = ui.opacity;
            ui.opacity *= a;
            draw_label(ui, c.anim.head_world() + glam::Vec3::Y * 0.28, name, s.near_char == Some(i));
            ui.opacity = old;
        }
    }
}

fn hint(ui: &mut Ui, s: &State) {
    let Some((t, age)) = &s.hint else {
        return;
    };
    let a = (age * 3.0).min(1.0) * (1.0 - ((age - 7.0) / 1.0).clamp(0.0, 1.0));
    let lines = ui.wrap_lines(t, 14.0, FONT_REGULAR, 420.0);
    let h = 24.0 + lines.len() as f32 * 20.0;
    let r = Rect::new(M, ui.height - M - h, 470.0, h);
    let old = ui.opacity;
    ui.opacity *= a;
    ui.panel(r, 16.0);
    icons::draw(ui, Icon::Bulb, Vec2::new(r.x + 22.0, r.y + 22.0), 18.0, pal::YELLOW);
    for (i, l) in lines.iter().enumerate() {
        ui.text(r.x + 42.0, r.y + 12.0 + i as f32 * 20.0, l, 14.0, FONT_REGULAR, pal::TEXT);
    }
    ui.opacity = old;
}

fn toasts(ui: &mut Ui, s: &State) {
    let w = 440.0;
    let mut y = M;
    for t in s.toasts.iter().rev() {
        let a_in = (t.age * 4.0).min(1.0);
        let a_out = 1.0 - ((t.age - 4.7) / 0.8).clamp(0.0, 1.0);
        let a = a_in * a_out;
        let body_lines = ui.wrap_lines(&t.body, 13.0, FONT_REGULAR, w - 150.0);
        let h = 40.0 + body_lines.len().min(3) as f32 * 17.0;
        let r = Rect::new((ui.width - w) * 0.5, y - (1.0 - a_in) * 30.0, w, h);
        let old = ui.opacity;
        ui.opacity *= a;
        ui.panel(r, 18.0);
        let ic = Vec2::new(r.x + 30.0, r.y + 30.0);
        let col = match t.icon {
            "bank" | "piggy" => pal::ACCENT2,
            "chat" => pal::BLUE,
            "warning" | "lock" => pal::RED,
            "star" | "goal" | "target" => pal::YELLOW,
            _ => pal::ACCENT,
        };
        ui.circle(ic, 19.0, with_alpha(col, 0.2));
        icons::draw(ui, icon_of(t.icon), ic, 20.0, col);
        ui.text(r.x + 58.0, r.y + 11.0, &t.title, 15.0, FONT_BOLD, pal::TEXT);
        for (i, l) in body_lines.iter().take(3).enumerate() {
            ui.text(r.x + 58.0, r.y + 31.0 + i as f32 * 17.0, l, 13.0, FONT_REGULAR, pal::TEXT_DIM);
        }
        if t.amount != 0 {
            let at = money_signed(t.amount);
            ui.text_in(Rect::new(r.right() - 130.0, r.y + 10.0, 116.0, 22.0), &at, 17.0, FONT_BOLD, amount_color(t.amount), Align::Right);
        }
        ui.opacity = old;
        y += h + 10.0;
    }
}

fn title_card(ui: &mut Ui, a: &str, b: &str, t: f32) {
    let alpha = (t * 1.6).min(1.0) * (1.0 - ((t - 2.9) / 0.7).clamp(0.0, 1.0));
    let old = ui.opacity;
    ui.opacity = alpha;
    let cy = ui.height * 0.42;
    let size = if a.len() < 12 { 64.0 } else { 44.0 };
    let tw = ui.measure(a, size, FONT_BOLD);
    let x = (ui.width - tw) * 0.5;
    let rise = (1.0 - (t * 1.2).min(1.0)) * 16.0;
    ui.text_shadowed(x, cy - size * 0.7 + rise, a, size, FONT_BOLD, pal::WHITE);
    if !b.is_empty() {
        let lw = 60.0 + 80.0 * (t * 1.5).min(1.0);
        ui.rect(Rect::new(ui.width * 0.5 - lw * 0.5, cy + size * 0.55, lw, 3.0), pal::ACCENT, 1.5);
        ui.text_in(Rect::new(0.0, cy + size * 0.55 + 16.0, ui.width, 30.0), b, 22.0, FONT_REGULAR, pal::TEXT, Align::Center);
    }
    ui.opacity = old;
}

// ------------------------------------------------------------------ dialogue & choices

fn dialogue(ui: &mut Ui, d: &DialogLine, acts: &mut Vec<UiAct>) {
    let w = (ui.width - 80.0).min(900.0);
    let size = 19.0;
    let lines = ui.wrap_lines(&d.text, size, FONT_REGULAR, w - 64.0);
    let h = (lines.len() as f32 * size * 1.5 + 50.0).max(100.0);
    let a = (d.age * 6.0).min(1.0);
    let r = Rect::new((ui.width - w) * 0.5, ui.height - h - 26.0 + (1.0 - a) * 16.0, w, h);
    ui.opacity = a;
    let tint = if d.think { rgba(0x1c1830, 190) } else { rgba(0x111318, 200) };
    ui.shadow(r.offset(0.0, 6.0), 22.0, 24.0, rgba(0x000000, 110));
    ui.glass(r, 22.0, tint);
    ui.border(r, rgba(0xffffff, 24), 22.0, 1.0);
    if let Some(w) = d.who {
        let name = if d.think { format!("{} (piensa)", w.name()) } else { w.name().to_string() };
        let tw = ui.measure(&name, 15.0, FONT_BOLD);
        let chip = Rect::new(r.x + 24.0, r.y - 18.0, tw + 40.0, 34.0);
        ui.rect(chip, rgba(0x15171d, 240), 17.0);
        ui.border(chip, with_alpha(w.color(), 0.6), 17.0, 1.5);
        ui.circle(Vec2::new(chip.x + 17.0, chip.center().y), 5.0, w.color());
        ui.text(chip.x + 28.0, chip.y + 8.0, &name, 15.0, FONT_BOLD, w.color());
    }
    let col = if d.think {
        rgba(0xd9d4ff, 255)
    } else if d.who.is_none() {
        pal::TEXT_DIM
    } else {
        pal::TEXT
    };
    let reveal = d.reveal as usize;
    ui.paragraph(r.x + 32.0, r.y + 26.0, w - 64.0, &d.text, size, FONT_REGULAR, col, 1.5, Some(reveal));
    let done = reveal >= d.text.chars().count();
    if done {
        let blink = ((d.age * 3.0).sin() * 0.5 + 0.5) * 0.6 + 0.4;
        let p = Vec2::new(r.right() - 30.0, r.bottom() - 22.0);
        ui.line(p + Vec2::new(-6.0, -5.0), p, 2.5, with_alpha(pal::ACCENT, blink));
        ui.line(p + Vec2::new(6.0, -5.0), p, 2.5, with_alpha(pal::ACCENT, blink));
    }
    ui.opacity = 1.0;
    // advance: click anywhere, space, enter, E
    let full = Rect::new(0.0, 0.0, ui.width, ui.height);
    let (_, clicked) = ui.hit(full);
    if (clicked || ui.consume_key(UiKey::Confirm) || ui.consume_key(UiKey::Interact)) && d.age > 0.15 {
        acts.push(UiAct::Advance);
    }
}

fn choices(ui: &mut Ui, c: &ChoiceState, acts: &mut Vec<UiAct>) {
    let w = 620.0f32.min(ui.width - 60.0);
    let x = (ui.width - w) * 0.5;
    let mut heights = Vec::new();
    for o in &c.opts {
        heights.push(if o.detail.is_some() { 70.0 } else { 54.0 });
    }
    let total: f32 = heights.iter().sum::<f32>() + (c.opts.len() as f32 - 1.0) * 10.0;
    let prompt_h = if c.prompt.is_some() { 56.0 } else { 0.0 };
    let mut y = ((ui.height - total - prompt_h) * 0.5).max(90.0) + prompt_h;
    let a = (c.age * 5.0).min(1.0);
    ui.rect(Rect::new(0.0, 0.0, ui.width, ui.height), rgba(0x000000, (70.0 * a) as u8), 0.0);
    ui.block(Rect::new(0.0, 0.0, ui.width, ui.height));
    if let Some(p) = &c.prompt {
        let lines = ui.wrap_lines(p, 20.0, FONT_BOLD, w);
        let py = y - prompt_h - (lines.len() as f32 - 1.0) * 26.0;
        for (i, l) in lines.iter().enumerate() {
            let tw = ui.measure(l, 20.0, FONT_BOLD);
            ui.text_shadowed((ui.width - tw) * 0.5, py + i as f32 * 26.0, l, 20.0, FONT_BOLD, pal::WHITE);
        }
    }
    let mut enabled_idx = 0;
    for (i, o) in c.opts.iter().enumerate() {
        let h = heights[i];
        let delay = i as f32 * 0.06;
        let ai = ((c.age - delay) * 5.0).clamp(0.0, 1.0);
        let r = Rect::new(x + (1.0 - ai) * 30.0, y, w, h);
        ui.opacity = ai;
        let id = format!("choice_{i}");
        let (hover, clicked) = ui.hit(r);
        let hv = ui.anim(hash_id(&id), if hover && o.enabled { 1.0 } else { 0.0 }, 14.0);
        let rr = r.offset(hv * 6.0, 0.0);
        ui.shadow(rr.offset(0.0, 4.0), 16.0, 14.0, rgba(0x000000, 80));
        ui.glass(rr, 16.0, if o.enabled { rgba(0x15171d, (190.0 + 40.0 * hv) as u8) } else { rgba(0x15171d, 140) });
        ui.border(rr, if o.enabled { with_alpha(pal::ACCENT, 0.25 + 0.6 * hv) } else { rgba(0xffffff, 16) }, 16.0, 1.5);
        let badge = Rect::new(rr.x + 14.0, rr.y + (h - 30.0) * 0.5, 30.0, 30.0);
        if o.enabled {
            enabled_idx += 1;
            ui.rect(badge, mix(rgba(0xffffff, 30), pal::ACCENT, hv), 9.0);
            ui.text_in(badge, &(i + 1).to_string(), 15.0, FONT_BOLD, if hv > 0.5 { pal::INK } else { pal::TEXT }, Align::Center);
        } else {
            ui.rect(badge, rgba(0xffffff, 14), 9.0);
            icons::draw(ui, Icon::Lock, badge.center(), 14.0, pal::TEXT_MUTED);
        }
        let tc = if o.enabled { pal::TEXT } else { pal::TEXT_MUTED };
        if let Some(dt) = &o.detail {
            ui.text(rr.x + 58.0, rr.y + 13.0, &o.label, 17.0, FONT_BOLD, tc);
            ui.text(rr.x + 58.0, rr.y + 39.0, dt, 13.5, FONT_REGULAR, if o.enabled { pal::TEXT_DIM } else { pal::TEXT_MUTED });
        } else {
            ui.text(rr.x + 58.0, rr.y + (h - 22.0) * 0.5, &o.label, 17.0, FONT_BOLD, tc);
        }
        if o.enabled && (clicked || ui.consume_key(UiKey::Num((i + 1) as u8))) && c.age > 0.3 {
            acts.push(UiAct::Choose(i));
        }
        y += h + 10.0;
    }
    let _ = enabled_idx;
    ui.opacity = 1.0;
}

// ------------------------------------------------------------------ modals

fn modal(ui: &mut Ui, s: &State, m: &Modal, acts: &mut Vec<UiAct>) {
    let a = ui.anim(hash_id("modal_dim"), 1.0, 8.0);
    let dim = match m {
        Modal::Phone(_) => 110.0,
        Modal::Business(_) | Modal::Minigame(_) => 90.0,
        _ => 140.0,
    };
    ui.rect(Rect::new(0.0, 0.0, ui.width, ui.height), rgba(0x05060a, (dim * a) as u8), 0.0);
    ui.block(Rect::new(0.0, 0.0, ui.width, ui.height));
    match m {
        Modal::Phone(app) => phone(ui, s, *app, acts),
        Modal::Shop(st) => shop(ui, s, st, acts),
        Modal::Amount(am) => amount(ui, am, acts),
        Modal::GoalPicker(gp) => goal_picker(ui, s, gp, acts),
        Modal::Summary(rep) => summary(ui, s, rep, acts),
        Modal::Business(b) => business(ui, s, b, acts),
        Modal::Minigame(g) => minigame(ui, g, acts),
        Modal::Reflection => reflection(ui, s, acts),
        Modal::Pause => pause(ui, acts),
        Modal::Info(t, b) => info(ui, t, b, acts),
    }
}

fn centered(ui: &Ui, w: f32, h: f32) -> Rect {
    let w = w.min(ui.width - 40.0);
    let h = h.min(ui.height - 40.0);
    Rect::new((ui.width - w) * 0.5, (ui.height - h) * 0.5, w, h)
}

fn modal_panel(ui: &mut Ui, r: Rect, title: &str, icon: Option<(Icon, Color)>) {
    ui.shadow(r.offset(0.0, 10.0), 26.0, 34.0, rgba(0x000000, 140));
    ui.glass(r, 26.0, rgba(0x111318, 215));
    ui.border(r, rgba(0xffffff, 26), 26.0, 1.0);
    let mut x = r.x + 30.0;
    if let Some((ic, c)) = icon {
        ui.circle(Vec2::new(r.x + 46.0, r.y + 44.0), 20.0, with_alpha(c, 0.18));
        icons::draw(ui, ic, Vec2::new(r.x + 46.0, r.y + 44.0), 22.0, c);
        x = r.x + 78.0;
    }
    ui.text(x, r.y + 30.0, title, 24.0, FONT_BOLD, pal::TEXT);
}

fn close_button(ui: &mut Ui, r: Rect, id: &str) -> bool {
    let c = Rect::new(r.right() - 58.0, r.y + 22.0, 40.0, 40.0);
    let (hover, clicked) = ui.hit(c);
    ui.rect(c, rgba(0xffffff, if hover { 36 } else { 16 }), 20.0);
    icons::draw(ui, Icon::Cross, c.center(), 16.0, pal::TEXT);
    let _ = id;
    clicked || ui.consume_key(UiKey::Back)
}

fn pause(ui: &mut Ui, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 420.0, 300.0);
    modal_panel(ui, r, "Pausa", None);
    if ui.button("resume", Rect::new(r.x + 40.0, r.y + 100.0, r.w - 80.0, 56.0), "Continuar", ButtonStyle::primary()) {
        acts.push(UiAct::Resume);
    }
    if ui.button("quit", Rect::new(r.x + 40.0, r.y + 170.0, r.w - 80.0, 56.0), "Guardar y volver al menú", ButtonStyle::ghost()) {
        acts.push(UiAct::Quit);
    }
    ui.text_in(Rect::new(r.x, r.bottom() - 44.0, r.w, 20.0), "El juego se guarda al terminar cada semana.", 12.0, FONT_REGULAR, pal::TEXT_MUTED, Align::Center);
}

fn info(ui: &mut Ui, t: &str, b: &str, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 520.0, 300.0);
    modal_panel(ui, r, t, Some((Icon::Bulb, pal::YELLOW)));
    ui.paragraph(r.x + 32.0, r.y + 90.0, r.w - 64.0, b, 16.0, FONT_REGULAR, pal::TEXT_DIM, 1.5, None);
    if ui.button("info_ok", Rect::new(r.right() - 170.0, r.bottom() - 76.0, 140.0, 50.0), "Entendido", ButtonStyle::primary()) || ui.consume_key(UiKey::Confirm) {
        acts.push(UiAct::CloseModal);
    }
}

// ------------------------------------------------------------------ amount picker

fn amount(ui: &mut Ui, a: &AmountState, acts: &mut Vec<UiAct>) {
    let body_lines = ui.wrap_lines(&a.body, 15.0, FONT_REGULAR, 500.0);
    let h = 330.0 + body_lines.len() as f32 * 22.0;
    let r = centered(ui, 580.0, h);
    modal_panel(ui, r, &a.title, Some((Icon::Coin, pal::YELLOW)));
    let mut y = r.y + 84.0;
    for l in &body_lines {
        ui.text(r.x + 32.0, y, l, 15.0, FONT_REGULAR, pal::TEXT_DIM);
        y += 22.0;
    }
    y += 12.0;
    ui.text_in(Rect::new(r.x, y, r.w, 56.0), &money(a.value), 46.0, FONT_BOLD, pal::WHITE, Align::Center);
    y += 72.0;
    let range = (a.max - a.min).max(1) as f32;
    let v01 = (a.value - a.min) as f32 / range;
    let sr = Rect::new(r.x + 90.0, y, r.w - 180.0, 30.0);
    if let Some(nv) = ui.slider("amount_slider", sr, v01, pal::ACCENT) {
        let raw = a.min as f32 + nv * range;
        let v = ((raw / a.step as f32).round() as i64 * a.step).clamp(a.min, a.max);
        acts.push(UiAct::AmountSet(v));
    }
    if ui.button("amt_minus", Rect::new(r.x + 30.0, y - 5.0, 44.0, 40.0), "–", ButtonStyle::ghost().size(22.0)) || ui.consume_key(UiKey::Left) {
        acts.push(UiAct::AmountSet((a.value - a.step).max(a.min)));
    }
    if ui.button("amt_plus", Rect::new(r.right() - 74.0, y - 5.0, 44.0, 40.0), "+", ButtonStyle::ghost().size(22.0)) || ui.consume_key(UiKey::Right) {
        acts.push(UiAct::AmountSet((a.value + a.step).min(a.max)));
    }
    y += 50.0;
    let chips = [("0", 0.0), ("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("Todo", 1.0)];
    let cw = (r.w - 64.0 - 4.0 * 8.0) / 5.0;
    for (i, (label, f)) in chips.iter().enumerate() {
        let cr = Rect::new(r.x + 32.0 + i as f32 * (cw + 8.0), y, cw, 36.0);
        if ui.button(&format!("chip{i}"), cr, label, ButtonStyle::subtle().size(14.0)) {
            let raw = a.min as f32 + range * f;
            let v = ((raw / a.step as f32).floor() as i64 * a.step).clamp(a.min, a.max);
            let v = if *f >= 1.0 { a.max } else { v };
            acts.push(UiAct::AmountSet(v));
        }
    }
    let by = r.bottom() - 78.0;
    if a.cancel && ui.button("amt_cancel", Rect::new(r.x + 32.0, by, 160.0, 52.0), "Cancelar", ButtonStyle::ghost().color(pal::TEXT_DIM)) {
        acts.push(UiAct::CloseModal);
    }
    if ui.button("amt_ok", Rect::new(r.right() - 232.0, by, 200.0, 52.0), &a.confirm, ButtonStyle::primary()) || ui.consume_key(UiKey::Confirm) {
        acts.push(UiAct::AmountOk);
    }
}

// ------------------------------------------------------------------ goal picker

fn goal_picker(ui: &mut Ui, s: &State, gp: &GoalPick, acts: &mut Vec<UiAct>) {
    let opts = story::goal_options();
    let r = centered(ui, 760.0, 170.0 + ((opts.len() + 1) / 2) as f32 * 108.0);
    modal_panel(ui, r, "Elige una meta", Some((Icon::Target, pal::ACCENT)));
    if gp.fund_with > 0 {
        ui.text(r.x + 78.0, r.y + 62.0, &format!("Se apartarán {} de tu billetera para esta meta.", money(gp.fund_with)), 14.0, FONT_REGULAR, pal::TEXT_DIM);
    }
    let cw = (r.w - 64.0 - 16.0) / 2.0;
    for (i, (id, name, target, deadline)) in opts.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let cr = Rect::new(r.x + 32.0 + col * (cw + 16.0), r.y + 100.0 + row * 108.0, cw, 94.0);
        let exists = s.fin.goals.iter().any(|g| g.id == *id && !g.done);
        let (hover, clicked) = ui.hit(cr);
        let hv = ui.anim(hash_id(&format!("gp{i}")), if hover && !exists { 1.0 } else { 0.0 }, 12.0);
        ui.rect(cr, rgba(0xffffff, (10.0 + 18.0 * hv) as u8), 18.0);
        ui.border(cr, with_alpha(pal::ACCENT, 0.15 + 0.6 * hv), 18.0, 1.3);
        let ic = match *id {
            "trip" => Icon::Calendar,
            "bike" => Icon::Target,
            "guitar" => Icon::Heart,
            "emergency" => Icon::Warning,
            _ => Icon::Star,
        };
        ui.circle(Vec2::new(cr.x + 38.0, cr.center().y), 24.0, with_alpha(pal::ACCENT, 0.16));
        icons::draw(ui, ic, Vec2::new(cr.x + 38.0, cr.center().y), 24.0, pal::ACCENT);
        ui.text(cr.x + 76.0, cr.y + 18.0, name, 16.0, FONT_BOLD, if exists { pal::TEXT_MUTED } else { pal::TEXT });
        let sub = match deadline {
            Some(w) => format!("{} · antes de la semana {}", money(*target), w),
            None => money(*target),
        };
        ui.text(cr.x + 76.0, cr.y + 44.0, &sub, 14.0, FONT_REGULAR, pal::TEXT_DIM);
        if exists {
            ui.text(cr.x + 76.0, cr.y + 64.0, "Ya es una de tus metas", 12.0, FONT_REGULAR, pal::ACCENT2);
        }
        if clicked && !exists {
            acts.push(UiAct::PickGoal(id));
        }
    }
}

// ------------------------------------------------------------------ weekly summary

fn summary(ui: &mut Ui, s: &State, rep: &super::finance::WeekReport, acts: &mut Vec<UiAct>) {
    let note_lines: Vec<Vec<String>> = rep.notes.iter().map(|n| ui.wrap_lines(n, 14.0, FONT_REGULAR, 560.0)).collect();
    let notes_h: f32 = note_lines.iter().map(|l| l.len() as f32 * 20.0 + 8.0).sum();
    let h = 250.0 + rep.lines.len() as f32 * 32.0 + notes_h + 90.0;
    let r = centered(ui, 680.0, h);
    modal_panel(ui, r, &format!("Resumen de la semana {}", rep.week), Some((Icon::Calendar, pal::BLUE)));
    ui.text(r.x + 78.0, r.y + 62.0, "Así se movió tu dinero mientras ibas al colegio.", 14.0, FONT_REGULAR, pal::TEXT_DIM);
    let mut y = r.y + 100.0;
    let t = ui.anim_from(hash_id("summary_in"), 0.0, 1.0, 2.0);
    let mut net = 0;
    for (i, (label, v)) in rep.lines.iter().enumerate() {
        let li = (t * (rep.lines.len() as f32 + 1.0) - i as f32).clamp(0.0, 1.0);
        ui.opacity = li;
        ui.rect(Rect::new(r.x + 30.0, y + 28.0, r.w - 60.0, 1.0), rgba(0xffffff, 14), 0.0);
        ui.text(r.x + 36.0, y + 4.0, label, 16.0, FONT_REGULAR, pal::TEXT);
        ui.text_in(Rect::new(r.right() - 230.0, y + 2.0, 196.0, 24.0), &money_signed(*v), 17.0, FONT_BOLD, amount_color(*v), Align::Right);
        net += v;
        y += 32.0;
    }
    ui.opacity = 1.0;
    ui.text(r.x + 36.0, y + 8.0, "Resultado de la semana", 16.0, FONT_BOLD, pal::TEXT_DIM);
    ui.text_in(Rect::new(r.right() - 230.0, y + 6.0, 196.0, 24.0), &money_signed(net), 19.0, FONT_BOLD, amount_color(net), Align::Right);
    y += 48.0;
    for lines in &note_lines {
        icons::draw(ui, Icon::Bulb, Vec2::new(r.x + 44.0, y + 10.0), 14.0, pal::YELLOW);
        for (i, l) in lines.iter().enumerate() {
            ui.text(r.x + 62.0, y + i as f32 * 20.0, l, 14.0, FONT_REGULAR, pal::TEXT_DIM);
        }
        y += lines.len() as f32 * 20.0 + 8.0;
    }
    y += 6.0;
    let f = &s.fin;
    let stats = [
        ("Billetera", f.wallet, pal::YELLOW),
        ("Ahorro", f.savings, pal::ACCENT2),
        ("Metas", f.goals_saved(), pal::ACCENT),
        ("Deudas", -f.total_debt(), pal::RED),
    ];
    let cw = (r.w - 60.0) / 4.0;
    for (i, (label, v, c)) in stats.iter().enumerate() {
        let x = r.x + 30.0 + i as f32 * cw;
        ui.text(x + 6.0, y, label, 12.0, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(x + 6.0, y + 16.0, &money(*v), 17.0, FONT_BOLD, *c);
    }
    if ui.button("sum_ok", Rect::new(r.right() - 220.0, r.bottom() - 74.0, 190.0, 50.0), "Continuar", ButtonStyle::primary()) || ui.consume_key(UiKey::Confirm) {
        acts.push(UiAct::SummaryContinue);
    }
}

// ------------------------------------------------------------------ shop

fn shop(ui: &mut Ui, s: &State, st: &ShopState, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 1100.0, 640.0);
    let (ic, col) = match st.shop {
        Shop::TecnoMundo => (Icon::Store, pal::BLUE),
        Shop::Feria => (Icon::Bag, pal::ACCENT),
        Shop::Cafe => (Icon::Heart, pal::ACCENT2),
    };
    modal_panel(ui, r, st.shop.name(), Some((ic, col)));
    if close_button(ui, r, "shop_close") {
        acts.push(UiAct::CloseModal);
    }
    // money available
    let info = format!("Billetera: {}   ·   Ahorro: {}", money(s.fin.wallet), money(s.fin.savings));
    ui.text(r.x + 78.0, r.y + 62.0, &info, 14.0, FONT_REGULAR, pal::TEXT_DIM);
    let items = story::shop_items(s, st.shop);
    let list = Rect::new(r.x + 24.0, r.y + 100.0, r.w * 0.45, r.h - 124.0);
    let off = ui.scroll_area(hash_id("shop_scroll"), list, items.len() as f32 * 90.0);
    ui.set_clip(Some(list));
    for (i, it) in items.iter().enumerate() {
        let cr = Rect::new(list.x, list.y + i as f32 * 90.0 - off, list.w - 8.0, 80.0);
        let sel = st.selected == Some(i);
        let owned = s.fin.inventory.contains(&it.id) && !it.consumable;
        let (hover, clicked) = ui.hit(cr);
        let hv = ui.anim(hash_id(&format!("shop_item{i}")), if hover || sel { 1.0 } else { 0.0 }, 12.0);
        ui.rect(cr, if sel { rgba(0x2a2f3a, 230) } else { rgba(0xffffff, (8.0 + 12.0 * hv) as u8) }, 16.0);
        if sel {
            ui.border(cr, with_alpha(pal::ACCENT, 0.8), 16.0, 1.5);
        }
        ui.circle(Vec2::new(cr.x + 38.0, cr.center().y), 24.0, with_alpha(col, 0.15));
        icons::draw(ui, it.icon, Vec2::new(cr.x + 38.0, cr.center().y), 22.0, col);
        ui.text(cr.x + 76.0, cr.y + 14.0, it.name, 16.0, FONT_BOLD, if owned { pal::TEXT_MUTED } else { pal::TEXT });
        let price = display_price(s, it);
        ui.text(cr.x + 76.0, cr.y + 42.0, &money(price), 17.0, FONT_BOLD, pal::ACCENT);
        if it.regular > price {
            let pw = ui.measure(&money(price), 17.0, FONT_BOLD);
            let rt = money(it.regular);
            let rw = ui.measure(&rt, 13.0, FONT_REGULAR);
            let rx = cr.x + 86.0 + pw;
            ui.text(rx, cr.y + 45.0, &rt, 13.0, FONT_REGULAR, pal::TEXT_MUTED);
            ui.rect(Rect::new(rx, cr.y + 53.0, rw, 1.2), pal::TEXT_MUTED, 0.0);
            let tag = Rect::new(cr.right() - 84.0, cr.y + 14.0, 70.0, 22.0);
            ui.rect(tag, pal::RED, 11.0);
            ui.text_in(tag, "OFERTA", 11.0, FONT_BOLD, pal::WHITE, Align::Center);
        }
        if owned {
            ui.text_in(Rect::new(cr.right() - 100.0, cr.y + 44.0, 86.0, 20.0), "Lo tienes", 12.0, FONT_REGULAR, pal::ACCENT2, Align::Right);
        }
        if clicked {
            acts.push(UiAct::ShopSelect(Some(i)));
        }
    }
    ui.set_clip(None);
    // detail
    let d = Rect::new(list.right() + 20.0, list.y, r.right() - list.right() - 44.0, list.h);
    ui.rect(d, rgba(0xffffff, 8), 20.0);
    let Some(sel) = st.selected.and_then(|i| items.get(i).map(|it| (i, it))) else {
        ui.text_in(Rect::new(d.x, d.center().y - 40.0, d.w, 24.0), "Elige un producto para ver detalles", 16.0, FONT_REGULAR, pal::TEXT_MUTED, Align::Center);
        let tip = match st.shop {
            Shop::TecnoMundo => "Consejo: antes de comprar, compara el costo total si pagas en cuotas.",
            Shop::Feria => "En la feria no hay garantía ni cuotas: solo efectivo.",
            Shop::Cafe => "Los gustos pequeños también suman al final de la semana.",
        };
        ui.text_in(Rect::new(d.x + 20.0, d.center().y, d.w - 40.0, 20.0), tip, 13.0, FONT_REGULAR, pal::TEXT_DIM, Align::Center);
        return;
    };
    let (idx, it) = sel;
    let price = display_price(s, it);
    let mut y = d.y + 24.0;
    ui.text(d.x + 24.0, y, it.name, 22.0, FONT_BOLD, pal::TEXT);
    y += 36.0;
    y += ui.paragraph(d.x + 24.0, y, d.w - 48.0, it.desc, 15.0, FONT_REGULAR, pal::TEXT_DIM, 1.45, None) + 8.0;
    // quality
    ui.text(d.x + 24.0, y, "Calidad", 13.0, FONT_REGULAR, pal::TEXT_MUTED);
    for q in 0..3 {
        let c = if (q as u8) < it.quality { pal::YELLOW } else { rgba(0xffffff, 30) };
        icons::draw(ui, Icon::Star, Vec2::new(d.x + 96.0 + q as f32 * 22.0, y + 8.0), 14.0, c);
    }
    y += 34.0;
    let goal_money = s.fin.goals.iter().find(|g| g.id == it.id && !g.done).map(|g| g.saved).unwrap_or(0);
    if goal_money > 0 {
        ui.text(d.x + 24.0, y, &format!("Tienes {} apartados en tu meta para esto.", money(goal_money)), 13.0, FONT_REGULAR, pal::ACCENT2);
        y += 24.0;
    }
    // payment options
    let can_cash = s.fin.wallet + goal_money >= price;
    let owned = s.fin.inventory.contains(&it.id) && !it.consumable;
    let bw = d.w - 48.0;
    let opt_h = 58.0;
    let mut opts: Vec<(String, String, Option<(u32, f32)>, bool)> = vec![(
        format!("Pagar al contado: {}", money(price)),
        if can_cash { "Con el dinero que tienes".to_string() } else { format!("Te faltan {}", money(price - s.fin.wallet - goal_money)) },
        None,
        can_cash,
    )];
    if it.credit {
        let q3 = Finance::quote(price, 3, 0.0);
        let q6 = Finance::quote(price, 6, 0.04);
        let free = s.fin.weekly_free();
        let warn = |q: i64| if q > free { format!(" · ¡más que tus {} libres/sem.!", money(free.max(0))) } else { String::new() };
        opts.push((format!("3 cuotas sin interés de {}", money(q3)), format!("Total {}{}", money(q3 * 3), warn(q3)), Some((3, 0.0)), true));
        opts.push((format!("6 cuotas de {}", money(q6)), format!("Total {} · 4% semanal{}", money(q6 * 6), warn(q6)), Some((6, 0.04)), true));
    }
    for (i, (label, sub, credit, enabled)) in opts.iter().enumerate() {
        let br = Rect::new(d.x + 24.0, y, bw, opt_h);
        let (hover, clicked) = ui.hit(br);
        let en = *enabled && !owned;
        let hv = ui.anim(hash_id(&format!("payopt{i}")), if hover && en { 1.0 } else { 0.0 }, 12.0);
        let base = if credit.is_none() { pal::ACCENT } else { pal::BLUE };
        ui.rect(br, if en { mix(rgba(0xffffff, 12), with_alpha(base, 0.35), hv) } else { rgba(0xffffff, 6) }, 14.0);
        ui.border(br, if en { with_alpha(base, 0.35 + 0.5 * hv) } else { rgba(0xffffff, 12) }, 14.0, 1.3);
        icons::draw(ui, if credit.is_none() { Icon::Coin } else { Icon::Card }, Vec2::new(br.x + 26.0, br.center().y), 18.0, if en { base } else { pal::TEXT_MUTED });
        ui.text(br.x + 50.0, br.y + 10.0, label, 15.0, FONT_BOLD, if en { pal::TEXT } else { pal::TEXT_MUTED });
        ui.text(br.x + 50.0, br.y + 32.0, sub, 12.5, FONT_REGULAR, pal::TEXT_DIM);
        if clicked && en {
            acts.push(UiAct::Buy(idx, *credit));
        }
        y += opt_h + 10.0;
    }
    if it.credit {
        let cr = Rect::new(d.x + 24.0, y, bw, 40.0);
        if ui.button("compare", cr, if st.compare { "Ocultar comparación" } else { "Comparar costo total" }, ButtonStyle::subtle()) {
            acts.push(UiAct::ShopCompare(!st.compare));
        }
        y += 50.0;
        if st.compare {
            let q6 = Finance::quote(price, 6, 0.04);
            let extra = q6 * 6 - price;
            let weeks = (price as f32 / 4_000.0).ceil() as i64;
            let rows = [
                format!("Al contado pagas {}.", money(price)),
                format!("En 6 cuotas pagas {} ({} más).", money(q6 * 6), money(extra)),
                format!("Con $4.000 libres por semana, ahorrarlo te tomaría {weeks} semanas."),
                format!("Trabajando en el café, son unos {} turnos.", (price as f32 / 6_500.0).ceil() as i64),
            ];
            for row in rows {
                ui.text(d.x + 30.0, y, &row, 13.5, FONT_REGULAR, pal::TEXT_DIM);
                y += 21.0;
            }
        }
    }
    if let Some((m, ok)) = &st.message {
        let mr = Rect::new(d.x + 24.0, d.bottom() - 70.0, bw, 50.0);
        ui.rect(mr, if *ok { rgba(0x1d3a2c, 220) } else { rgba(0x3a1d1d, 220) }, 12.0);
        icons::draw(ui, if *ok { Icon::Check } else { Icon::Warning }, Vec2::new(mr.x + 24.0, mr.center().y), 16.0, if *ok { pal::GREEN } else { pal::RED });
        let lines = ui.wrap_lines(m, 13.0, FONT_REGULAR, bw - 60.0);
        for (i, l) in lines.iter().take(2).enumerate() {
            ui.text(mr.x + 44.0, mr.y + 8.0 + i as f32 * 17.0 + if lines.len() == 1 { 8.0 } else { 0.0 }, l, 13.0, FONT_REGULAR, pal::TEXT);
        }
    }
}

fn display_price(s: &State, it: &super::items::Item) -> i64 {
    if it.id == "speaker" && s.flag("flash_sale") {
        return 12_990;
    }
    if s.flag("flash_sale") && s.week > story::LAST_WEEK && it.shop == Shop::TecnoMundo {
        return it.price * 7 / 10;
    }
    it.price
}

// ------------------------------------------------------------------ phone

fn phone(ui: &mut Ui, s: &State, app: PhoneApp, acts: &mut Vec<UiAct>) {
    let h = (ui.height - 50.0).min(780.0);
    let w = h * 0.5;
    let appear = ui.anim_from(hash_id("phone_in"), 0.0, 1.0, 9.0);
    let x = (ui.width * 0.5 + 60.0).min(ui.width - w - 30.0).max((ui.width - w) * 0.5);
    let r = Rect::new(x, (ui.height - h) * 0.5 + (1.0 - appear) * 60.0, w, h);
    ui.opacity = appear;
    ui.shadow(r.offset(0.0, 16.0), 44.0, 40.0, rgba(0x000000, 160));
    ui.rect_grad(r, rgba(0x2c2f36, 255), rgba(0x1a1c21, 255), 44.0);
    ui.border(r, rgba(0xffffff, 40), 44.0, 1.5);
    let sc = r.shrink(10.0);
    ui.rect_grad(sc, rgba(0x171a24, 255), rgba(0x0e1016, 255), 36.0);
    // notch
    ui.rect(Rect::new(sc.center().x - 44.0, sc.y + 10.0, 88.0, 24.0), rgba(0x000000, 255), 12.0);
    // status bar
    ui.text(sc.x + 26.0, sc.y + 12.0, &clock(s.hour), 13.0, FONT_BOLD, pal::TEXT);
    ui.rect(Rect::new(sc.right() - 50.0, sc.y + 15.0, 26.0, 12.0), rgba(0xffffff, 200), 3.0);
    ui.rect(Rect::new(sc.right() - 23.0, sc.y + 18.0, 3.0, 6.0), rgba(0xffffff, 200), 1.0);
    let content = Rect::new(sc.x, sc.y + 44.0, sc.w, sc.h - 44.0 - 30.0);
    match app {
        PhoneApp::Home => phone_home(ui, s, content, acts),
        PhoneApp::Bank => phone_bank(ui, s, content, acts),
        PhoneApp::Goals => phone_goals(ui, s, content, acts),
        PhoneApp::Credit => phone_credit(ui, s, content, acts),
        PhoneApp::Invest => phone_invest(ui, s, content, acts),
        PhoneApp::Business => phone_business(ui, s, content),
        PhoneApp::Chat => phone_chat(ui, s, content),
        PhoneApp::Journal => phone_journal(ui, s, content),
    }
    // home indicator / back
    let hb = Rect::new(sc.center().x - 60.0, sc.bottom() - 20.0, 120.0, 6.0);
    ui.rect(hb, rgba(0xffffff, 150), 3.0);
    let hit = Rect::new(sc.x, sc.bottom() - 34.0, sc.w, 34.0);
    let (_, clicked) = ui.hit(hit);
    if clicked {
        if app == PhoneApp::Home {
            acts.push(UiAct::CloseModal);
        } else {
            acts.push(UiAct::OpenPhone(PhoneApp::Home));
        }
    }
    if ui.consume_key(UiKey::Back) || ui.consume_key(UiKey::Phone) {
        if app == PhoneApp::Home {
            acts.push(UiAct::CloseModal);
        } else {
            acts.push(UiAct::OpenPhone(PhoneApp::Home));
        }
    }
    // close button outside the phone
    let cb = Rect::new(r.x - 58.0, r.y + 10.0, 44.0, 44.0);
    let (hov, cl) = ui.hit(cb);
    ui.glass(cb, 22.0, rgba(0x14161c, if hov { 220 } else { 170 }));
    icons::draw(ui, Icon::Cross, cb.center(), 16.0, pal::TEXT);
    if cl {
        acts.push(UiAct::CloseModal);
    }
    // cracked screen overlay
    if s.flag("phone_cracked") {
        let c = Vec2::new(sc.x + sc.w * 0.7, sc.y + sc.h * 0.62);
        let mut rng = crate::math::Rng::new(9);
        for _ in 0..14 {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let len = rng.range(60.0, sc.h * 0.6);
            let mut p = c;
            let mut dir = Vec2::new(a.cos(), a.sin());
            let n = 5;
            for _ in 0..n {
                let q = p + dir * (len / n as f32);
                let q = Vec2::new(q.x.clamp(sc.x, sc.right()), q.y.clamp(sc.y, sc.bottom()));
                ui.line(p, q, 1.3, rgba(0xffffff, 150));
                p = q;
                let t = rng.range(-0.4, 0.4);
                dir = Vec2::new(dir.x * t.cos() - dir.y * t.sin(), dir.x * t.sin() + dir.y * t.cos());
            }
        }
    }
    ui.opacity = 1.0;
}

fn app_header(ui: &mut Ui, c: Rect, title: &str, col: Color, acts: &mut Vec<UiAct>) -> f32 {
    let back = Rect::new(c.x + 14.0, c.y + 6.0, 40.0, 40.0);
    let (hover, clicked) = ui.hit(back);
    ui.rect(back, rgba(0xffffff, if hover { 30 } else { 12 }), 20.0);
    ui.line(back.center() + Vec2::new(4.0, -7.0), back.center() + Vec2::new(-4.0, 0.0), 2.4, pal::TEXT);
    ui.line(back.center() + Vec2::new(4.0, 7.0), back.center() + Vec2::new(-4.0, 0.0), 2.4, pal::TEXT);
    if clicked {
        acts.push(UiAct::OpenPhone(PhoneApp::Home));
    }
    ui.text(c.x + 66.0, c.y + 14.0, title, 19.0, FONT_BOLD, pal::TEXT);
    ui.rect(Rect::new(c.x + 66.0, c.y + 40.0, 28.0, 3.0), col, 1.5);
    c.y + 58.0
}

fn phone_home(ui: &mut Ui, s: &State, c: Rect, acts: &mut Vec<UiAct>) {
    ui.text(c.x + 22.0, c.y + 8.0, "Hola, Sofía", 24.0, FONT_BOLD, pal::TEXT);
    ui.text(c.x + 22.0, c.y + 40.0, &format!("Semana {} · {}", s.week, s.phase.label()), 13.0, FONT_REGULAR, pal::TEXT_DIM);
    // balance card
    let card = Rect::new(c.x + 16.0, c.y + 70.0, c.w - 32.0, 118.0);
    ui.rect_grad(card, rgba(0x3b3f8f, 255), rgba(0x6a4aa8, 255), 20.0);
    ui.text(card.x + 18.0, card.y + 14.0, "Saldo total", 13.0, FONT_REGULAR, rgba(0xffffff, 200));
    let total = s.fin.wallet + s.fin.savings + s.fin.goals_saved() + s.fin.invested_value();
    ui.text(card.x + 18.0, card.y + 34.0, &money(total), 30.0, FONT_BOLD, pal::WHITE);
    ui.text(card.x + 18.0, card.y + 80.0, &format!("Billetera {}  ·  Ahorro {}", money(s.fin.wallet), money(s.fin.savings)), 12.5, FONT_REGULAR, rgba(0xffffff, 220));
    let apps: [(PhoneApp, &str, Icon, Color, bool); 7] = [
        (PhoneApp::Bank, "Banco", Icon::Bank, pal::ACCENT2, true),
        (PhoneApp::Goals, "Metas", Icon::Target, pal::ACCENT, true),
        (PhoneApp::Credit, "Crédito", Icon::Card, pal::RED, true),
        (PhoneApp::Invest, "Inversiones", Icon::Chart, pal::BLUE, s.flag("invest_open")),
        (PhoneApp::Business, "Negocio", Icon::Briefcase, pal::YELLOW, s.biz.active),
        (PhoneApp::Chat, "Mensajes", Icon::Chat, pal::GREEN, true),
        (PhoneApp::Journal, "Diario", Icon::Book, pal::PURPLE, true),
    ];
    let cols = 3;
    let cell = (c.w - 32.0) / cols as f32;
    for (i, (app, name, ic, col, unlocked)) in apps.iter().enumerate() {
        let cx = c.x + 16.0 + (i % cols) as f32 * cell + cell * 0.5;
        let cy = c.y + 236.0 + (i / cols) as f32 * 104.0;
        let r = Rect::new(cx - 32.0, cy - 32.0, 64.0, 64.0);
        let (hover, clicked) = ui.hit(r.expand(8.0));
        let hv = ui.anim(hash_id(&format!("app{i}")), if hover { 1.0 } else { 0.0 }, 14.0);
        let rr = r.scale_center(1.0 + hv * 0.06);
        if *unlocked {
            ui.rect_grad(rr, mix(*col, pal::WHITE, 0.15), mix(*col, pal::INK, 0.35), 18.0);
            icons::draw(ui, *ic, rr.center(), 30.0, pal::WHITE);
        } else {
            ui.rect(rr, rgba(0xffffff, 18), 18.0);
            icons::draw(ui, Icon::Lock, rr.center(), 22.0, pal::TEXT_MUTED);
        }
        if *app == PhoneApp::Chat && s.unread > 0 {
            ui.circle(Vec2::new(rr.right() - 2.0, rr.y + 2.0), 11.0, pal::RED);
            ui.text_in(Rect::new(rr.right() - 13.0, rr.y - 7.0, 22.0, 18.0), &s.unread.min(9).to_string(), 11.0, FONT_BOLD, pal::WHITE, Align::Center);
        }
        ui.text_in(Rect::new(cx - cell * 0.5, cy + 38.0, cell, 18.0), name, 12.5, FONT_REGULAR, if *unlocked { pal::TEXT } else { pal::TEXT_MUTED }, Align::Center);
        if clicked && *unlocked {
            acts.push(UiAct::OpenPhone(*app));
        } else if clicked {
            let _ = 0;
        }
    }
}

fn phone_bank(ui: &mut Ui, s: &State, c: Rect, acts: &mut Vec<UiAct>) {
    let mut y = app_header(ui, c, "Banco Futuro", pal::ACCENT2, acts);
    let f = &s.fin;
    let cards = [
        ("Billetera", "Disponible para gastar", f.wallet, pal::YELLOW, Icon::Coin),
        ("Cuenta de ahorro", "+0,5% de interés semanal", f.savings, pal::ACCENT2, Icon::Piggy),
    ];
    for (title, sub, v, col, ic) in cards {
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 84.0);
        ui.rect(r, rgba(0xffffff, 12), 18.0);
        ui.circle(Vec2::new(r.x + 32.0, r.center().y), 20.0, with_alpha(col, 0.18));
        icons::draw(ui, ic, Vec2::new(r.x + 32.0, r.center().y), 20.0, col);
        ui.text(r.x + 64.0, r.y + 14.0, title, 14.0, FONT_BOLD, pal::TEXT);
        ui.text(r.x + 64.0, r.y + 34.0, sub, 11.5, FONT_REGULAR, pal::TEXT_DIM);
        ui.text_in(Rect::new(r.x, r.y + 50.0, r.w - 16.0, 24.0), &money(v), 20.0, FONT_BOLD, pal::WHITE, Align::Right);
        y += 94.0;
    }
    if f.interest_earned > 0 {
        ui.text(c.x + 20.0, y - 4.0, &format!("Intereses ganados en total: {}", money(f.interest_earned)), 12.0, FONT_REGULAR, pal::ACCENT2);
        y += 18.0;
    }
    let bw = (c.w - 42.0) / 2.0;
    if ui.button("bank_save", Rect::new(c.x + 16.0, y, bw, 46.0), "Ahorrar", ButtonStyle::primary().color(pal::ACCENT2).size(15.0)) {
        acts.push(UiAct::TransferToSavings);
    }
    if ui.button("bank_take", Rect::new(c.x + 26.0 + bw, y, bw, 46.0), "Sacar", ButtonStyle::ghost().color(pal::ACCENT2).size(15.0)) {
        acts.push(UiAct::TransferFromSavings);
    }
    y += 58.0;
    ui.text(c.x + 20.0, y, "Ahorro automático semanal", 13.0, FONT_BOLD, pal::TEXT);
    y += 22.0;
    let pcts = [0u32, 10, 20, 30];
    let pw = (c.w - 32.0 - 18.0) / 4.0;
    for (i, p) in pcts.iter().enumerate() {
        let r = Rect::new(c.x + 16.0 + i as f32 * (pw + 6.0), y, pw, 34.0);
        let sel = f.auto_save_pct == *p;
        let (hover, clicked) = ui.hit(r);
        ui.rect(r, if sel { pal::ACCENT2 } else { rgba(0xffffff, if hover { 26 } else { 12 }) }, 10.0);
        ui.text_in(r, &format!("{p}%"), 13.0, FONT_BOLD, if sel { pal::INK } else { pal::TEXT }, Align::Center);
        if clicked {
            acts.push(UiAct::AutoSave(*p));
        }
    }
    y += 50.0;
    ui.text(c.x + 20.0, y, "Movimientos", 13.0, FONT_BOLD, pal::TEXT);
    y += 24.0;
    let area = Rect::new(c.x, y, c.w, c.bottom() - y);
    let n = f.history.len();
    let off = ui.scroll_area(hash_id("bank_hist"), area, n as f32 * 40.0);
    ui.set_clip(Some(area));
    for (i, t) in f.history.iter().rev().enumerate() {
        let ry = area.y + i as f32 * 40.0 - off;
        if ry > area.bottom() || ry < area.y - 40.0 {
            continue;
        }
        let desc: String = t.desc.chars().take(28).collect();
        ui.text(c.x + 20.0, ry + 4.0, &desc, 13.0, FONT_REGULAR, pal::TEXT);
        ui.text(c.x + 20.0, ry + 21.0, &format!("Semana {}", t.week), 11.0, FONT_REGULAR, pal::TEXT_MUTED);
        if t.amount != 0 {
            ui.text_in(Rect::new(c.x, ry + 6.0, c.w - 20.0, 18.0), &money_signed(t.amount), 14.0, FONT_BOLD, amount_color(t.amount), Align::Right);
        }
    }
    ui.set_clip(None);
}

fn phone_goals(ui: &mut Ui, s: &State, c: Rect, acts: &mut Vec<UiAct>) {
    let mut y = app_header(ui, c, "Mis metas", pal::ACCENT, acts);
    let goals: Vec<&super::finance::Goal> = s.fin.goals.iter().filter(|g| !g.done).collect();
    if goals.is_empty() {
        ui.paragraph(c.x + 24.0, y + 10.0, c.w - 48.0, "Todavía no tienes metas. Una meta le da un propósito a cada peso que ahorras.", 14.0, FONT_REGULAR, pal::TEXT_DIM, 1.5, None);
        y += 80.0;
    }
    for (i, g) in goals.iter().enumerate() {
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 138.0);
        ui.rect(r, rgba(0xffffff, 12), 18.0);
        ui.text(r.x + 18.0, r.y + 14.0, &g.name, 15.0, FONT_BOLD, pal::TEXT);
        let sub = match g.deadline {
            Some(w) => {
                let left = w as i64 - s.week as i64;
                format!("{} de {} · quedan {} semanas", money(g.saved), money(g.target), left.max(0))
            }
            None => format!("{} de {}", money(g.saved), money(g.target)),
        };
        ui.text(r.x + 18.0, r.y + 38.0, &sub, 12.0, FONT_REGULAR, pal::TEXT_DIM);
        let bar = Rect::new(r.x + 18.0, r.y + 62.0, r.w - 36.0, 10.0);
        ui.rect(bar, rgba(0xffffff, 22), 5.0);
        let p = ui.anim(hash_id(&format!("goalbar{i}")), g.progress(), 5.0);
        ui.rect_grad(Rect::new(bar.x, bar.y, bar.w * p, bar.h), pal::ACCENT, mix(pal::ACCENT, pal::YELLOW, 0.5), 5.0);
        if let Some(w) = g.deadline {
            let weeks_left = (w as i64 - s.week as i64).max(1);
            let per_week = ((g.target - g.saved).max(0) as f32 / weeks_left as f32).ceil() as i64;
            ui.text(r.x + 18.0, r.y + 78.0, &format!("Necesitas apartar ~{} por semana", money(per_week)), 11.5, FONT_REGULAR, pal::YELLOW);
        }
        let bw = (r.w - 46.0) / 2.0;
        if ui.button(&format!("gdep{i}"), Rect::new(r.x + 16.0, r.y + 96.0, bw, 34.0), "Apartar", ButtonStyle::primary().size(13.5)) {
            acts.push(UiAct::GoalDeposit(i));
        }
        if ui.button(&format!("gwd{i}"), Rect::new(r.x + 30.0 + bw, r.y + 96.0, bw, 34.0), "Sacar", ButtonStyle::ghost().size(13.5)) {
            acts.push(UiAct::GoalWithdraw(i));
        }
        y += 148.0;
    }
    let done: Vec<&super::finance::Goal> = s.fin.goals.iter().filter(|g| g.done).collect();
    for g in done {
        ui.text(c.x + 24.0, y, &format!("✓ {} (cumplida)", g.name), 13.0, FONT_REGULAR, pal::GREEN);
        y += 22.0;
    }
    if y + 50.0 < c.bottom() && ui.button("goal_new", Rect::new(c.x + 16.0, y + 4.0, c.w - 32.0, 44.0), "+ Nueva meta", ButtonStyle::ghost().size(15.0)) {
        acts.push(UiAct::GoalNew);
    }
}

fn phone_credit(ui: &mut Ui, s: &State, c: Rect, acts: &mut Vec<UiAct>) {
    let mut y = app_header(ui, c, "Crédito", pal::RED, acts);
    let f = &s.fin;
    // credit score gauge
    let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 92.0);
    ui.rect(r, rgba(0xffffff, 12), 18.0);
    ui.text(r.x + 18.0, r.y + 14.0, "Tu historial de pago", 13.0, FONT_BOLD, pal::TEXT);
    let sc = (f.credit_score as f32 / 1000.0).clamp(0.0, 1.0);
    let bar = Rect::new(r.x + 18.0, r.y + 44.0, r.w - 36.0, 10.0);
    ui.rect_grad(bar, pal::RED, pal::GREEN, 5.0);
    ui.circle(Vec2::new(bar.x + bar.w * sc, bar.center().y), 9.0, pal::WHITE);
    let label = if f.credit_score >= 720 { "Excelente" } else if f.credit_score >= 620 { "Bueno" } else { "Con atrasos" };
    ui.text(r.x + 18.0, r.y + 62.0, &format!("{} · {}", f.credit_score, label), 12.0, FONT_REGULAR, pal::TEXT_DIM);
    y += 104.0;
    if f.debts.is_empty() {
        ui.paragraph(c.x + 24.0, y + 6.0, c.w - 48.0, "No tienes deudas. Cuando compras en cuotas, aquí verás cuánto pagas cada semana y el costo total.", 14.0, FONT_REGULAR, pal::TEXT_DIM, 1.5, None);
        return;
    }
    for (i, d) in f.debts.iter().enumerate() {
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 142.0);
        ui.rect(r, if d.overdue > 0 { rgba(0x3a1d1d, 200) } else { rgba(0xffffff, 12) }, 18.0);
        let name: String = d.name.chars().take(26).collect();
        ui.text(r.x + 18.0, r.y + 12.0, &name, 14.0, FONT_BOLD, pal::TEXT);
        ui.text(r.x + 18.0, r.y + 34.0, &format!("Cuota: {} por semana", money(d.installment)), 12.5, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(r.x + 18.0, r.y + 52.0, &format!("Quedan {} de {} cuotas", d.payments_left, d.total_payments), 12.5, FONT_REGULAR, pal::TEXT_DIM);
        let interest = d.interest();
        let it = if interest > 0 { format!("Costo total {} (intereses {})", money(d.total_cost()), money(interest)) } else { format!("Costo total {} (sin interés)", money(d.total_cost())) };
        ui.text(r.x + 18.0, r.y + 70.0, &it, 12.0, FONT_REGULAR, if interest > 0 { pal::RED } else { pal::TEXT_DIM });
        if d.overdue > 0 {
            ui.text(r.x + 18.0, r.y + 88.0, &format!("Atrasada: {}", money(d.overdue)), 12.0, FONT_BOLD, pal::RED);
        }
        if ui.button(&format!("prepay{i}"), Rect::new(r.x + 16.0, r.y + 102.0, r.w - 32.0, 32.0), "Pagar una cuota ahora", ButtonStyle::ghost().color(pal::RED).size(13.0)) {
            acts.push(UiAct::PayDebtExtra(i));
        }
        y += 152.0;
        if y > c.bottom() - 40.0 {
            break;
        }
    }
}

fn sparkline(ui: &mut Ui, r: Rect, data: &[f32], col: Color) {
    if data.len() < 2 {
        ui.line(Vec2::new(r.x, r.center().y), Vec2::new(r.right(), r.center().y), 1.5, with_alpha(col, 0.5));
        return;
    }
    let lo = data.iter().cloned().fold(f32::MAX, f32::min);
    let hi = data.iter().cloned().fold(f32::MIN, f32::max);
    let span = (hi - lo).max(1e-3);
    let n = data.len();
    let pts: Vec<Vec2> = data
        .iter()
        .enumerate()
        .map(|(i, v)| Vec2::new(r.x + r.w * i as f32 / (n - 1) as f32, r.bottom() - (v - lo) / span * r.h))
        .collect();
    for w in pts.windows(2) {
        ui.line(w[0], w[1], 2.0, col);
    }
    ui.circle(*pts.last().unwrap(), 3.5, col);
}

fn phone_invest(ui: &mut Ui, s: &State, c: Rect, acts: &mut Vec<UiAct>) {
    let mut y = app_header(ui, c, "Inversiones", pal::BLUE, acts);
    if !s.flag("invest_open") {
        ui.paragraph(c.x + 24.0, y + 10.0, c.w - 48.0, "Disponible más adelante.", 14.0, FONT_REGULAR, pal::TEXT_DIM, 1.5, None);
        return;
    }
    let f = &s.fin;
    let fund: Vec<f32> = f.fund_history.clone();
    let crypto: Vec<f32> = f.crypto_history.clone();
    let prods: [(InvestKind, &str, u32, &str, Vec<f32>, Color); 3] = [
        (InvestKind::Deposit, "Depósito a plazo", 1, "+3% en 4 semanas · bloqueado", vec![1.0, 1.0075, 1.015, 1.0225, 1.03], pal::GREEN),
        (InvestKind::Fund, "Fondo mutuo moderado", 3, "Variable · promedio +0,6%/sem", fund, pal::BLUE),
        (InvestKind::Crypto, "MoonCoin", 5, "Extremadamente volátil", crypto, pal::PURPLE),
    ];
    for (i, (k, name, risk, sub, data, col)) in prods.iter().enumerate() {
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 102.0);
        ui.rect(r, rgba(0xffffff, 12), 16.0);
        ui.text(r.x + 16.0, r.y + 12.0, name, 14.0, FONT_BOLD, pal::TEXT);
        ui.text(r.x + 16.0, r.y + 32.0, sub, 11.5, FONT_REGULAR, pal::TEXT_DIM);
        ui.text(r.x + 16.0, r.y + 52.0, "Riesgo", 11.0, FONT_REGULAR, pal::TEXT_MUTED);
        for q in 0..5u32 {
            let cc = if q < *risk { mix(pal::GREEN, pal::RED, q as f32 / 4.0) } else { rgba(0xffffff, 26) };
            ui.rect(Rect::new(r.x + 60.0 + q as f32 * 14.0, r.y + 56.0, 10.0, 8.0), cc, 2.0);
        }
        sparkline(ui, Rect::new(r.right() - 110.0, r.y + 14.0, 94.0, 36.0), data, *col);
        if ui.button(&format!("inv{i}"), Rect::new(r.x + 16.0, r.y + 70.0, r.w - 32.0, 26.0), "Invertir", ButtonStyle::subtle().size(13.0)) {
            acts.push(UiAct::Invest(*k));
        }
        y += 110.0;
    }
    ui.text(c.x + 20.0, y + 2.0, "Mis inversiones", 13.0, FONT_BOLD, pal::TEXT);
    y += 24.0;
    if f.investments.is_empty() {
        ui.text(c.x + 20.0, y, "Aún no inviertes.", 12.5, FONT_REGULAR, pal::TEXT_MUTED);
    }
    for (i, inv) in f.investments.iter().enumerate() {
        if y > c.bottom() - 44.0 {
            break;
        }
        let gain = inv.value - inv.invested;
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, 46.0);
        ui.rect(r, rgba(0xffffff, 10), 12.0);
        ui.text(r.x + 12.0, r.y + 6.0, inv.kind.name(), 12.5, FONT_BOLD, pal::TEXT);
        ui.text(r.x + 12.0, r.y + 24.0, &format!("{} ({})", money(inv.value), money_signed(gain)), 12.0, FONT_REGULAR, amount_color(gain));
        let locked = inv.locked_until > s.week;
        let label = if locked { format!("Sem. {}", inv.locked_until) } else { "Rescatar".to_string() };
        if ui.button(&format!("wd{i}"), Rect::new(r.right() - 96.0, r.y + 8.0, 86.0, 30.0), &label, ButtonStyle::ghost().size(12.0)) {
            acts.push(UiAct::Withdraw(i));
        }
        y += 52.0;
    }
}

fn phone_business(ui: &mut Ui, s: &State, c: Rect) {
    let b = &s.biz;
    let mut y = c.y + 14.0;
    ui.text(c.x + 22.0, y, "Pulseras Sofi & Tomás", 19.0, FONT_BOLD, pal::TEXT);
    y += 40.0;
    let stats = [
        ("Días de feria", b.sessions.to_string(), pal::TEXT),
        ("Ventas totales", money(b.revenue), pal::GREEN),
        ("Costos totales", money(b.costs), pal::RED),
        ("Resultado", money_signed(b.profit()), amount_color(b.profit())),
        ("Inventario", format!("{} pulseras", b.inventory), pal::TEXT),
        ("Costo por pulsera", money(b.unit_cost.round() as i64), pal::TEXT),
        ("Precio actual", money(b.price), pal::TEXT),
    ];
    for (l, v, col) in stats {
        ui.text(c.x + 24.0, y, l, 13.0, FONT_REGULAR, pal::TEXT_DIM);
        ui.text_in(Rect::new(c.x, y - 1.0, c.w - 24.0, 18.0), &v, 14.0, FONT_BOLD, col, Align::Right);
        y += 30.0;
    }
    ui.text(c.x + 24.0, y + 6.0, "Reputación", 13.0, FONT_REGULAR, pal::TEXT_DIM);
    let bar = Rect::new(c.x + 120.0, y + 10.0, c.w - 144.0, 8.0);
    ui.rect(bar, rgba(0xffffff, 22), 4.0);
    ui.rect(Rect::new(bar.x, bar.y, bar.w * b.reputation, bar.h), pal::YELLOW, 4.0);
    y += 40.0;
    if b.partner {
        ui.paragraph(c.x + 24.0, y, c.w - 48.0, "Socios 50/50 con Tomás: comparten costos y ganancias.", 12.5, FONT_REGULAR, pal::TEXT_MUTED, 1.4, None);
    }
}

fn phone_chat(ui: &mut Ui, s: &State, c: Rect) {
    ui.text(c.x + 22.0, c.y + 10.0, "Mensajes", 19.0, FONT_BOLD, pal::TEXT);
    let area = Rect::new(c.x, c.y + 48.0, c.w, c.h - 48.0);
    let mut heights = Vec::new();
    for m in &s.messages {
        let lines = ui.wrap_lines(&m.text, 13.0, FONT_REGULAR, c.w - 80.0);
        heights.push(lines.len() as f32 * 18.0 + 40.0);
    }
    let total: f32 = heights.iter().sum::<f32>() + 10.0;
    let key = hash_id("chat_scroll");
    let max = (total - area.h).max(0.0);
    if ui.value(key + 1) != s.messages.len() as f32 {
        ui.set_value(key, max);
        ui.set_value(key + 1, s.messages.len() as f32);
    }
    let off = ui.scroll_area(key, area, total);
    ui.set_clip(Some(area));
    let mut y = area.y - off;
    for (m, h) in s.messages.iter().zip(heights.iter()) {
        let r = Rect::new(c.x + 16.0, y, c.w - 48.0, h - 8.0);
        if r.bottom() > area.y && r.y < area.bottom() {
            let col = match m.from {
                "abuela" => Who::Abuela.color(),
                "mama" => Who::Mama.color(),
                "tomas" => Who::Tomas.color(),
                "vale" => Who::Vale.color(),
                "desconocido" => pal::RED,
                _ => pal::BLUE,
            };
            ui.rect(r, rgba(0xffffff, 14), 16.0);
            ui.text(r.x + 14.0, r.y + 8.0, contact_name(m.from), 12.0, FONT_BOLD, col);
            ui.paragraph(r.x + 14.0, r.y + 26.0, r.w - 28.0, &m.text, 13.0, FONT_REGULAR, pal::TEXT, 1.38, None);
        }
        y += h;
    }
    ui.set_clip(None);
}

fn phone_journal(ui: &mut Ui, s: &State, c: Rect) {
    ui.text(c.x + 22.0, c.y + 10.0, "Diario de decisiones", 19.0, FONT_BOLD, pal::TEXT);
    ui.text(c.x + 22.0, c.y + 36.0, "Lo que decidiste y lo que pasó después.", 12.0, FONT_REGULAR, pal::TEXT_DIM);
    let area = Rect::new(c.x, c.y + 60.0, c.w, c.h - 60.0);
    if s.fin.journal.is_empty() {
        ui.paragraph(c.x + 24.0, area.y + 10.0, c.w - 48.0, "Aún no hay entradas. Tus decisiones importantes aparecerán aquí.", 13.0, FONT_REGULAR, pal::TEXT_MUTED, 1.5, None);
        return;
    }
    let mut heights = Vec::new();
    for j in s.fin.journal.iter().rev() {
        let lines = ui.wrap_lines(&j.text, 12.5, FONT_REGULAR, c.w - 72.0);
        heights.push(lines.len() as f32 * 18.0 + 58.0);
    }
    let total: f32 = heights.iter().sum();
    let off = ui.scroll_area(hash_id("journal_scroll"), area, total);
    ui.set_clip(Some(area));
    let mut y = area.y - off;
    for (j, h) in s.fin.journal.iter().rev().zip(heights.iter()) {
        let r = Rect::new(c.x + 16.0, y, c.w - 32.0, h - 8.0);
        if r.bottom() > area.y && r.y < area.bottom() {
            let col = if j.score > 0 { pal::GREEN } else if j.score < 0 { pal::RED } else { pal::YELLOW };
            ui.rect(r, rgba(0xffffff, 12), 14.0);
            ui.rect(Rect::new(r.x, r.y + 10.0, 4.0, r.h - 20.0), col, 2.0);
            ui.text(r.x + 16.0, r.y + 8.0, &j.title, 13.5, FONT_BOLD, pal::TEXT);
            let tag = format!("Semana {} · {}", j.week, j.skill.name());
            ui.text(r.x + 16.0, r.y + 28.0, &tag, 11.0, FONT_REGULAR, col);
            ui.paragraph(r.x + 16.0, r.y + 46.0, r.w - 32.0, &j.text, 12.5, FONT_REGULAR, pal::TEXT_DIM, 1.42, None);
        }
        y += h;
    }
    ui.set_clip(None);
}

// ------------------------------------------------------------------ business

fn business(ui: &mut Ui, s: &State, b: &BizUi, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 900.0, 600.0);
    modal_panel(ui, r, "Puesto de pulseras", Some((Icon::Briefcase, pal::YELLOW)));
    match b.phase {
        BizPhase::Setup => {
            if close_button(ui, r, "biz_close") {
                acts.push(UiAct::Biz(BizAct::Close));
            }
            let biz = &s.biz;
            let mut y = r.y + 96.0;
            let lx = r.x + 32.0;
            let lw = r.w * 0.52;
            ui.text(lx, y, "1. Materiales", 16.0, FONT_BOLD, pal::TEXT);
            y += 30.0;
            let kits = [
                (0u32, format!("Usar inventario ({} pulseras)", biz.inventory), "Sin costo extra".to_string()),
                (1, format!("Kit pequeño: {} pulseras", KIT_SMALL.0), format!("{} · {} c/u", money(KIT_SMALL.1), money(KIT_SMALL.1 / KIT_SMALL.0 as i64))),
                (2, format!("Kit grande: {} pulseras", KIT_LARGE.0), format!("{} · {} c/u (más barato por unidad)", money(KIT_LARGE.1), money(KIT_LARGE.1 / KIT_LARGE.0 as i64))),
            ];
            for (k, title, sub) in kits.iter() {
                let cr = Rect::new(lx, y, lw, 58.0);
                let sel = b.kit == *k;
                let (hover, clicked) = ui.hit(cr);
                ui.rect(cr, if sel { rgba(0x3a3220, 230) } else { rgba(0xffffff, if hover { 20 } else { 10 }) }, 14.0);
                if sel {
                    ui.border(cr, pal::YELLOW, 14.0, 1.5);
                }
                ui.text(cr.x + 18.0, cr.y + 10.0, title, 14.5, FONT_BOLD, pal::TEXT);
                ui.text(cr.x + 18.0, cr.y + 32.0, sub, 12.5, FONT_REGULAR, pal::TEXT_DIM);
                if clicked {
                    acts.push(UiAct::Biz(BizAct::Kit(*k)));
                }
                y += 66.0;
            }
            y += 8.0;
            ui.text(lx, y, "2. Precio por pulsera", 16.0, FONT_BOLD, pal::TEXT);
            ui.text_in(Rect::new(lx, y - 2.0, lw, 24.0), &money(b.price), 20.0, FONT_BOLD, pal::YELLOW, Align::Right);
            y += 36.0;
            let v01 = (b.price - 500) as f32 / 4_500.0;
            if let Some(nv) = ui.slider("biz_price", Rect::new(lx + 10.0, y, lw - 20.0, 26.0), v01, pal::YELLOW) {
                let p = ((500.0 + nv * 4_500.0) / 100.0).round() as i64 * 100;
                acts.push(UiAct::Biz(BizAct::Price(p)));
            }
            y += 34.0;
            ui.text(lx, y, "$500", 12.0, FONT_REGULAR, pal::TEXT_MUTED);
            ui.text_in(Rect::new(lx, y, lw, 16.0), "$5.000", 12.0, FONT_REGULAR, pal::TEXT_MUTED, Align::Right);
            // estimates
            let ex = lx + lw + 30.0;
            let ew = r.right() - ex - 32.0;
            let er = Rect::new(ex, r.y + 96.0, ew, 360.0);
            ui.rect(er, rgba(0xffffff, 10), 18.0);
            let (units, kit_cost) = b.kit_info();
            let total_units = biz.inventory + units;
            let unit_cost = if total_units > 0 {
                ((biz.unit_cost * biz.inventory as f32 + kit_cost as f32) / total_units as f32).round() as i64
            } else {
                0
            };
            let margin = b.price - unit_cost;
            let fixed = STALL_RENT;
            let breakeven = if margin > 0 { ((fixed + kit_cost) as f32 / margin as f32).ceil() as i64 } else { -1 };
            let p = BizUi::buy_prob(b.price, biz.reputation);
            let exp_sold = ((20.0 + biz.reputation * 10.0) * p * 0.87).min(total_units as f32);
            let mut ey = er.y + 18.0;
            ui.text(er.x + 18.0, ey, "Tus números", 15.0, FONT_BOLD, pal::TEXT);
            ey += 32.0;
            let rows = [
                ("Costo por pulsera", money(unit_cost)),
                ("Ganancia por pulsera", money_signed(margin)),
                ("Arriendo del puesto", money(fixed)),
                ("Materiales hoy", money(kit_cost)),
                (
                    "Punto de equilibrio",
                    if breakeven >= 0 { format!("{} pulseras", breakeven) } else { "¡Pierdes en cada venta!".into() },
                ),
                ("Ventas estimadas", format!("~{:.0} pulseras", exp_sold)),
            ];
            for (l, v) in rows {
                ui.text(er.x + 18.0, ey, l, 13.0, FONT_REGULAR, pal::TEXT_DIM);
                ui.text_in(Rect::new(er.x, ey - 1.0, er.w - 18.0, 18.0), &v, 13.5, FONT_BOLD, pal::TEXT, Align::Right);
                ey += 28.0;
            }
            ey += 6.0;
            let note = if biz.partner {
                "Tomás paga la mitad de los costos y se lleva la mitad de las ventas."
            } else {
                "Todo el costo y toda la ganancia son tuyos."
            };
            ui.paragraph(er.x + 18.0, ey, er.w - 36.0, note, 12.5, FONT_REGULAR, pal::TEXT_MUTED, 1.45, None);
            ey += 44.0;
            let tip = if b.price >= 3_500 {
                "Un precio alto da más margen, pero menos gente compra."
            } else if b.price <= 1_000 {
                "Un precio muy bajo vende mucho... pero ¿cubre tus costos?"
            } else {
                "Busca el equilibrio entre precio y cantidad vendida."
            };
            ui.paragraph(er.x + 18.0, ey, er.w - 36.0, tip, 12.5, FONT_REGULAR, pal::YELLOW, 1.45, None);
            if let Some(m) = &b.message {
                ui.text(r.x + 32.0, r.bottom() - 108.0, m, 13.5, FONT_REGULAR, pal::RED);
            }
            let share = (STALL_RENT + kit_cost) / if biz.partner { 2 } else { 1 };
            let label = format!("Abrir el puesto (pagas {})", money(share));
            if ui.button("biz_start", Rect::new(r.right() - 332.0, r.bottom() - 80.0, 300.0, 54.0), &label, ButtonStyle::primary().color(pal::YELLOW)) {
                acts.push(UiAct::Biz(BizAct::Start));
            }
        }
        BizPhase::Selling | BizPhase::Result => {
            let y0 = r.y + 96.0;
            let stats = [
                ("Vendidas", b.sold.to_string(), pal::GREEN),
                ("Ingresos", money(b.revenue), pal::GREEN),
                ("Quedan", s.biz.inventory.to_string(), pal::TEXT),
                ("Gente que pasó", b.passersby.to_string(), pal::TEXT_DIM),
            ];
            let cw = (r.w - 64.0) / 4.0;
            for (i, (l, v, c)) in stats.iter().enumerate() {
                let cr = Rect::new(r.x + 32.0 + i as f32 * cw, y0, cw - 12.0, 74.0);
                ui.rect(cr, rgba(0xffffff, 10), 14.0);
                ui.text(cr.x + 16.0, cr.y + 12.0, l, 12.5, FONT_REGULAR, pal::TEXT_DIM);
                ui.text(cr.x + 16.0, cr.y + 34.0, v, 24.0, FONT_BOLD, *c);
            }
            ui.text(r.x + 32.0, y0 + 92.0, b.weather.label(), 13.0, FONT_REGULAR, pal::TEXT_MUTED);
            let bar = Rect::new(r.x + 32.0, y0 + 116.0, r.w - 64.0, 8.0);
            ui.rect(bar, rgba(0xffffff, 22), 4.0);
            ui.rect(Rect::new(bar.x, bar.y, bar.w * (b.t / b.duration).min(1.0), bar.h), pal::YELLOW, 4.0);
            let mut fy = y0 + 142.0;
            for f in &b.feed {
                let a = (f.age * 4.0).min(1.0);
                ui.opacity = a;
                let c = if f.good { pal::GREEN } else { pal::TEXT_DIM };
                icons::draw(ui, if f.good { Icon::Coin } else { Icon::Arrow }, Vec2::new(r.x + 46.0, fy + 10.0), 14.0, c);
                ui.text(r.x + 64.0, fy, &f.text, 14.0, FONT_REGULAR, c);
                ui.opacity = 1.0;
                fy += 26.0;
            }
            if b.phase == BizPhase::Result {
                let share_rev = if s.biz.partner { b.revenue / 2 } else { b.revenue };
                let share_cost = if s.biz.partner { b.spent / 2 } else { b.spent };
                let profit = share_rev - share_cost;
                let rr = Rect::new(r.x + 32.0, r.bottom() - 170.0, r.w - 64.0, 80.0);
                ui.rect(rr, if profit >= 0 { rgba(0x1d3a2c, 220) } else { rgba(0x3a1d1d, 220) }, 16.0);
                ui.text(rr.x + 20.0, rr.y + 14.0, "Tu resultado del día", 14.0, FONT_BOLD, pal::TEXT);
                ui.text(rr.x + 20.0, rr.y + 40.0, &format!("Ingresos {} – costos {} =", money(share_rev), money(share_cost)), 14.0, FONT_REGULAR, pal::TEXT_DIM);
                ui.text_in(Rect::new(rr.x, rr.y + 30.0, rr.w - 20.0, 30.0), &money_signed(profit), 26.0, FONT_BOLD, amount_color(profit), Align::Right);
                if b.lost > 0 {
                    ui.text(r.x + 32.0, r.bottom() - 82.0, &format!("Te quedaste sin stock: {} clientes se fueron sin comprar.", b.lost), 13.0, FONT_REGULAR, pal::YELLOW);
                }
                if ui.button("biz_finish", Rect::new(r.right() - 262.0, r.bottom() - 70.0, 230.0, 50.0), "Cerrar el puesto", ButtonStyle::primary().color(pal::YELLOW)) || ui.consume_key(UiKey::Confirm) {
                    acts.push(UiAct::Biz(BizAct::Finish));
                }
            }
        }
    }
}

// ------------------------------------------------------------------ café minigame

fn prod_color(p: Prod) -> Color {
    rgba(p.color(), 255)
}

fn minigame(ui: &mut Ui, g: &CafeGame, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 960.0, 580.0);
    modal_panel(ui, r, "Turno en Café Aroma", Some((Icon::Heart, pal::ACCENT2)));
    let remaining = (g.dur - g.t).max(0.0);
    ui.text_in(Rect::new(r.x, r.y + 30.0, r.w - 40.0, 28.0), &format!("{:.0} s", remaining), 22.0, FONT_BOLD, if remaining < 10.0 { pal::RED } else { pal::TEXT }, Align::Right);
    let bar = Rect::new(r.x + 32.0, r.y + 84.0, r.w - 64.0, 6.0);
    ui.rect(bar, rgba(0xffffff, 22), 3.0);
    ui.rect(Rect::new(bar.x, bar.y, bar.w * (1.0 - g.t / g.dur).clamp(0.0, 1.0), bar.h), pal::ACCENT2, 3.0);
    ui.text(r.x + 32.0, r.y + 100.0, &format!("Atendidos: {}   ·   Propinas: {}   ·   Se fueron: {}", g.served, money(g.tips), g.failed), 14.0, FONT_REGULAR, pal::TEXT_DIM);
    if g.done {
        let rr = Rect::new(r.x + 32.0, r.y + 150.0, r.w - 64.0, 250.0);
        ui.rect(rr, rgba(0xffffff, 10), 18.0);
        ui.text(rr.x + 24.0, rr.y + 22.0, "¡Turno terminado!", 22.0, FONT_BOLD, pal::TEXT);
        let rows = [
            ("Sueldo (3 horas)", g.wage),
            ("Propinas", g.tips),
            ("Total ganado", g.wage + g.tips),
        ];
        let mut y = rr.y + 70.0;
        for (l, v) in rows {
            ui.text(rr.x + 24.0, y, l, 16.0, FONT_REGULAR, pal::TEXT_DIM);
            ui.text_in(Rect::new(rr.x, y - 2.0, rr.w - 24.0, 22.0), &money(v), 18.0, FONT_BOLD, pal::GREEN, Align::Right);
            y += 34.0;
        }
        let per_hour = (g.wage + g.tips) / 3;
        ui.text(rr.x + 24.0, y + 8.0, &format!("Ganaste unos {} por hora. ¿Cuántas horas vale lo que quieres comprar?", money(per_hour)), 14.0, FONT_REGULAR, pal::YELLOW);
        if ui.button("mini_finish", Rect::new(r.right() - 262.0, r.bottom() - 76.0, 230.0, 52.0), "Cobrar el turno", ButtonStyle::primary().color(pal::ACCENT2)) || ui.consume_key(UiKey::Confirm) {
            acts.push(UiAct::Mini(MiniAct::Finish));
        }
        return;
    }
    // orders
    let oy = r.y + 134.0;
    for (i, o) in g.orders.iter().enumerate() {
        let cr = Rect::new(r.x + 32.0 + i as f32 * 208.0, oy, 196.0, 150.0);
        let first = i == 0;
        ui.rect(cr, if first { rgba(0x203a33, 230) } else { rgba(0xffffff, 10) }, 16.0);
        if first {
            ui.border(cr, pal::ACCENT2, 16.0, 1.5);
        }
        ui.text(cr.x + 14.0, cr.y + 12.0, o.name, 14.0, FONT_BOLD, pal::TEXT);
        for (k, p) in o.items.iter().enumerate() {
            let c = Vec2::new(cr.x + 30.0 + k as f32 * 50.0, cr.y + 66.0);
            ui.circle(c, 20.0, prod_color(*p));
            ui.text_in(Rect::new(c.x - 30.0, c.y + 22.0, 60.0, 16.0), p.name(), 11.0, FONT_REGULAR, pal::TEXT_DIM, Align::Center);
        }
        let pb = Rect::new(cr.x + 14.0, cr.bottom() - 20.0, cr.w - 28.0, 6.0);
        let frac = (o.patience / o.max_patience).clamp(0.0, 1.0);
        ui.rect(pb, rgba(0xffffff, 22), 3.0);
        ui.rect(Rect::new(pb.x, pb.y, pb.w * frac, pb.h), mix(pal::RED, pal::GREEN, frac), 3.0);
    }
    // products
    let py = r.y + 312.0;
    let pw = (r.w - 64.0 - 4.0 * 12.0) / 5.0;
    for (i, p) in Prod::all().iter().enumerate() {
        let pr = Rect::new(r.x + 32.0 + i as f32 * (pw + 12.0), py, pw, 92.0);
        let (hover, clicked) = ui.hit(pr);
        let hv = ui.anim(hash_id(&format!("prod{i}")), if hover { 1.0 } else { 0.0 }, 14.0);
        ui.rect(pr, rgba(0xffffff, (12.0 + 18.0 * hv) as u8), 16.0);
        ui.circle(Vec2::new(pr.center().x, pr.y + 36.0), 22.0 + hv * 2.0, prod_color(*p));
        ui.text_in(Rect::new(pr.x, pr.y + 64.0, pr.w, 18.0), &format!("{} · {}", i + 1, p.name()), 13.0, FONT_BOLD, pal::TEXT, Align::Center);
        if clicked || ui.consume_key(UiKey::Num((i + 1) as u8)) {
            acts.push(UiAct::Mini(MiniAct::Add(*p)));
        }
    }
    // tray
    let ty = py + 108.0;
    let tray = Rect::new(r.x + 32.0, ty, r.w * 0.5, 64.0);
    ui.rect(tray, rgba(0xffffff, 8), 16.0);
    ui.text(tray.x + 16.0, tray.y + 8.0, "Bandeja", 12.0, FONT_REGULAR, pal::TEXT_MUTED);
    for (k, p) in g.tray.iter().enumerate() {
        ui.circle(Vec2::new(tray.x + 110.0 + k as f32 * 50.0, tray.center().y), 18.0, prod_color(*p));
    }
    if ui.button("mini_clear", Rect::new(tray.right() + 16.0, ty + 8.0, 130.0, 48.0), "Vaciar", ButtonStyle::ghost().color(pal::TEXT_DIM)) {
        acts.push(UiAct::Mini(MiniAct::Clear));
    }
    if ui.button("mini_serve", Rect::new(r.right() - 232.0, ty + 8.0, 200.0, 48.0), "Entregar pedido", ButtonStyle::primary().color(pal::ACCENT2)) || ui.consume_key(UiKey::Confirm) {
        acts.push(UiAct::Mini(MiniAct::Serve));
    }
    if let Some((t, ok, age)) = &g.feedback {
        if *age < 2.0 {
            let a = 1.0 - (age / 2.0);
            ui.opacity = a;
            ui.text(r.x + 32.0, r.bottom() - 40.0, t, 15.0, FONT_BOLD, if *ok { pal::GREEN } else { pal::RED });
            ui.opacity = 1.0;
        }
    }
}

// ------------------------------------------------------------------ reflection

fn reflection(ui: &mut Ui, s: &State, acts: &mut Vec<UiAct>) {
    let r = centered(ui, 1040.0, 640.0);
    modal_panel(ui, r, "Tu historia financiera", Some((Icon::Star, pal::YELLOW)));
    ui.text(r.x + 78.0, r.y + 62.0, "No se trata de quién tiene más dinero, sino de cómo decides.", 14.0, FONT_REGULAR, pal::TEXT_DIM);
    let t = ui.anim_from(hash_id("refl_in"), 0.0, 1.0, 1.2);
    let mut y = r.y + 110.0;
    let lx = r.x + 36.0;
    let lw = r.w * 0.44;
    for (i, sk) in Skill::all().iter().enumerate() {
        let score: i32 = s.fin.journal.iter().filter(|j| j.skill == *sk).map(|j| j.score).sum();
        let n = s.fin.journal.iter().filter(|j| j.skill == *sk).count();
        let lvl = if n == 0 { 0.2 } else { ((score as f32 + 2.0) / 6.0).clamp(0.08, 1.0) };
        ui.text(lx, y, sk.name(), 15.0, FONT_BOLD, pal::TEXT);
        let bar = Rect::new(lx, y + 26.0, lw, 10.0);
        ui.rect(bar, rgba(0xffffff, 20), 5.0);
        let li = (t * 7.0 - i as f32).clamp(0.0, 1.0);
        let col = if n == 0 { rgba(0x8a8f99, 255) } else { mix(pal::RED, pal::GREEN, lvl) };
        ui.rect(Rect::new(bar.x, bar.y, bar.w * lvl * li, bar.h), col, 5.0);
        let label = if n == 0 {
            "Aún sin experiencias"
        } else if lvl > 0.75 {
            "Muy bien desarrollada"
        } else if lvl > 0.45 {
            "En progreso"
        } else {
            "Para seguir practicando"
        };
        ui.text_in(Rect::new(lx, y, lw, 18.0), label, 12.0, FONT_REGULAR, pal::TEXT_DIM, Align::Right);
        y += 62.0;
    }
    // highlights
    let rx = lx + lw + 40.0;
    let rw = r.right() - rx - 36.0;
    let mut hy = r.y + 110.0;
    ui.text(rx, hy, "Momentos clave", 15.0, FONT_BOLD, pal::TEXT);
    hy += 30.0;
    let mut entries: Vec<&super::finance::JournalEntry> = s.fin.journal.iter().collect();
    entries.sort_by_key(|j| -(j.score.abs()));
    for j in entries.iter().take(4) {
        let lines = ui.wrap_lines(&j.text, 12.5, FONT_REGULAR, rw - 30.0);
        let h = 34.0 + lines.len().min(3) as f32 * 17.0;
        let cr = Rect::new(rx, hy, rw, h);
        let col = if j.score > 0 { pal::GREEN } else if j.score < 0 { pal::RED } else { pal::YELLOW };
        ui.rect(cr, rgba(0xffffff, 10), 12.0);
        ui.rect(Rect::new(cr.x, cr.y + 8.0, 3.0, cr.h - 16.0), col, 1.5);
        ui.text(cr.x + 14.0, cr.y + 8.0, &j.title, 13.0, FONT_BOLD, pal::TEXT);
        for (i, l) in lines.iter().take(3).enumerate() {
            ui.text(cr.x + 14.0, cr.y + 28.0 + i as f32 * 17.0, l, 12.5, FONT_REGULAR, pal::TEXT_DIM);
        }
        hy += h + 8.0;
    }
    let f = &s.fin;
    let stats = format!(
        "Ahorro + metas: {}   ·   Intereses ganados: {}   ·   Deudas: {}",
        money(f.savings + f.goals_saved()),
        money(f.interest_earned),
        money(f.total_debt())
    );
    ui.text(r.x + 36.0, r.bottom() - 70.0, &stats, 13.5, FONT_REGULAR, pal::TEXT_DIM);
    if ui.button("refl_ok", Rect::new(r.right() - 262.0, r.bottom() - 82.0, 230.0, 52.0), "Seguir viviendo", ButtonStyle::primary()) || ui.consume_key(UiKey::Confirm) {
        acts.push(UiAct::FinishReflection);
    }
}

/// App icon drawn with the UI renderer (used by `--icon` for packaging).
pub fn app_icon(ui: &mut Ui) {
    let s = ui.width.min(ui.height);
    let r = Rect::new((ui.width - s) * 0.5, (ui.height - s) * 0.5, s, s);
    ui.rect_grad(r, rgba(0x7b5cc8, 255), rgba(0x241d4a, 255), 0.0);
    ui.circle(Vec2::new(r.x + s * 0.3, r.y + s * 0.25), s * 0.42, rgba(0xffffff, 14));
    let fs = s * 0.66;
    ui.text(r.x + s * 0.17, r.y + s * 0.07, "F", fs, FONT_BOLD, pal::WHITE);
    let c = Vec2::new(r.x + s * 0.68, r.y + s * 0.68);
    ui.circle(c + Vec2::new(0.0, s * 0.025), s * 0.2, rgba(0x000000, 90));
    ui.circle(c, s * 0.2, rgba(0xffc56b, 255));
    ui.circle(c, s * 0.16, rgba(0xeb9d3c, 255));
    ui.circle(c, s * 0.135, rgba(0xffc56b, 255));
    let ts = s * 0.2;
    let tw = ui.measure("$", ts, FONT_BOLD);
    ui.text(c.x - tw * 0.5, c.y - ts * 0.62, "$", ts, FONT_BOLD, rgba(0x9a5616, 255));
}
