//! Glyph rasterisation (fontdue) into a shelf-packed R8 atlas.

use std::collections::HashMap;

pub const FONT_REGULAR: usize = 0;
pub const FONT_BOLD: usize = 1;

#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub uv: [f32; 4], // u0 v0 u1 v1
    pub w: f32,
    pub h: f32,
    pub xmin: f32,
    pub ymin: f32,
    pub advance: f32,
}

pub struct Fonts {
    fonts: Vec<fontdue::Font>,
    cache: HashMap<(usize, char, u32), Glyph>,
    size: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
    pub uploads: Vec<(u32, u32, u32, u32, Vec<u8>)>,
    pub generation: u32,
}

impl Fonts {
    pub fn new(atlas_size: u32) -> Fonts {
        let settings = fontdue::FontSettings {
            scale: 40.0,
            ..Default::default()
        };
        let regular = fontdue::Font::from_bytes(
            include_bytes!("../../assets/fonts/Inter-Medium-sub.ttf") as &[u8],
            settings,
        )
        .expect("font");
        let bold = fontdue::Font::from_bytes(
            include_bytes!("../../assets/fonts/Inter-Bold-sub.ttf") as &[u8],
            settings,
        )
        .expect("font");
        Fonts {
            fonts: vec![regular, bold],
            cache: HashMap::new(),
            size: atlas_size,
            cursor_x: 1,
            cursor_y: 1,
            row_h: 0,
            uploads: Vec::new(),
            generation: 0,
        }
    }

    fn reset(&mut self) {
        self.cache.clear();
        self.cursor_x = 1;
        self.cursor_y = 1;
        self.row_h = 0;
        self.generation += 1;
    }

    pub fn glyph(&mut self, font: usize, c: char, px: u32) -> Glyph {
        if let Some(g) = self.cache.get(&(font, c, px)) {
            return *g;
        }
        let f = &self.fonts[font.min(self.fonts.len() - 1)];
        let (m, bitmap) = f.rasterize(c, px as f32);
        let (w, h) = (m.width as u32, m.height as u32);
        if self.cursor_x + w + 1 >= self.size {
            self.cursor_x = 1;
            self.cursor_y += self.row_h + 1;
            self.row_h = 0;
        }
        if self.cursor_y + h + 1 >= self.size {
            self.reset();
        }
        let (x, y) = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + 1;
        self.row_h = self.row_h.max(h);
        if w > 0 && h > 0 {
            self.uploads.push((x, y, w, h, bitmap));
        }
        let s = self.size as f32;
        let g = Glyph {
            uv: [x as f32 / s, y as f32 / s, (x + w) as f32 / s, (y + h) as f32 / s],
            w: w as f32,
            h: h as f32,
            xmin: m.xmin as f32,
            ymin: m.ymin as f32,
            advance: m.advance_width,
        };
        self.cache.insert((font, c, px), g);
        g
    }

    pub fn kern(&self, font: usize, a: char, b: char, px: u32) -> f32 {
        self.fonts[font]
            .horizontal_kern(a, b, px as f32)
            .unwrap_or(0.0)
    }

    pub fn line_metrics(&self, font: usize, px: u32) -> (f32, f32) {
        match self.fonts[font].horizontal_line_metrics(px as f32) {
            Some(m) => (m.ascent, m.descent),
            None => (px as f32 * 0.9, -(px as f32) * 0.25),
        }
    }

    /// Width of a single line in physical pixels.
    pub fn measure(&mut self, font: usize, text: &str, px: u32) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<char> = None;
        for c in text.chars() {
            if let Some(p) = prev {
                w += self.kern(font, p, c, px);
            }
            w += self.glyph(font, c, px).advance;
            prev = Some(c);
        }
        w
    }
}
