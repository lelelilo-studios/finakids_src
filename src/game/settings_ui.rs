//! Settings shown in the pause menu: sound (music / effects volume, mute) and graphics.

use crate::audio;
use crate::ui::{pal, rgba, Align, ButtonStyle, Rect, Ui, FONT_BOLD, FONT_REGULAR};

/// Draws the volume controls inside `r` (about 96 units tall).
pub fn audio_panel(ui: &mut Ui, r: Rect) {
    let mut s = audio::settings();
    let before = s;
    let row = 40.0;
    let label_w = 86.0;
    let value_w = 44.0;
    let rows: [(&str, &str, f32, bool); 2] = [("Música", "vol_music", s.music, false), ("Efectos", "vol_sfx", s.sfx, true)];
    for (i, (label, id, value, is_sfx)) in rows.into_iter().enumerate() {
        let y = r.y + i as f32 * row;
        let dim = if s.muted { pal::TEXT_MUTED } else { pal::TEXT };
        ui.text_in(Rect::new(r.x, y, label_w, row), label, 15.0, FONT_BOLD, dim, Align::Left);
        let sr = Rect::new(r.x + label_w, y + 4.0, r.w - label_w - value_w - 8.0, row - 8.0);
        let col = if s.muted { rgba(0x8a8f98, 255) } else { pal::ACCENT };
        if let Some(v) = ui.slider(id, sr, value, col) {
            if is_sfx {
                s.sfx = v;
            } else {
                s.music = v;
            }
            s.muted = false;
        }
        let pct = format!("{:.0}%", if is_sfx { s.sfx } else { s.music } * 100.0);
        ui.text_in(Rect::new(r.right() - value_w, y, value_w, row), &pct, 13.0, FONT_REGULAR, pal::TEXT_DIM, Align::Right);
    }
    let by = r.y + 2.0 * row + 6.0;
    let label = if s.muted { "Activar sonido" } else { "Silenciar" };
    if ui.button("vol_mute", Rect::new(r.x, by, r.w, 38.0), label, ButtonStyle::ghost().size(14.0)) {
        s.muted = !s.muted;
    }
    if s != before {
        audio::set_settings(s);
        // let the player hear the new effects level once they stop dragging
        if (s.sfx - before.sfx).abs() > 1e-4 {
            ui.set_value(crate::ui::hash_id("vol_sfx_touched"), 1.0);
        }
    }
    if !ui.input.down {
        if ui.value(crate::ui::hash_id("vol_sfx_touched")) > 0.5 {
            ui.set_value(crate::ui::hash_id("vol_sfx_touched"), 0.0);
            audio::request_preview();
        }
        audio::save_settings_if_dirty();
    }
}

/// Graphics preference: automatic (dynamic resolution), performance or quality.
pub fn gfx_panel(ui: &mut Ui, r: Rect) {
    ui.text_in(Rect::new(r.x, r.y, r.w, 20.0), "Gráficos", 15.0, FONT_BOLD, pal::TEXT, Align::Left);
    let labels = ["Auto", "Rendimiento", "Calidad"];
    let cur = audio::gfx_mode() as usize;
    let gap = 8.0;
    let w = (r.w - gap * 2.0) / 3.0;
    for (i, label) in labels.iter().enumerate() {
        let br = Rect::new(r.x + i as f32 * (w + gap), r.y + 26.0, w, 38.0);
        let style = if i == cur { ButtonStyle::primary().size(13.5) } else { ButtonStyle::subtle().size(13.5) };
        if ui.button(&format!("gfx_mode{i}"), br, label, style) && i != cur {
            audio::set_gfx_mode(i as u8);
        }
    }
}
