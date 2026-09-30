//! Character appearance descriptions and cast presets.

use crate::math::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hair {
    Ponytail,
    Short,
    Bob,
    Bun,
    Curly,
    Buzz,
    Long,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Top {
    Hoodie,
    Tee,
    Blouse,
    Sweater,
    Apron,
    Jacket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bottom {
    Jeans,
    Pants,
    Skirt,
}

#[derive(Clone, Debug)]
pub struct Appearance {
    pub name: &'static str,
    pub height: f32,
    /// 0 = feminine proportions, 1 = masculine.
    pub masc: f32,
    /// 0 teen, 1 adult, 2 elder
    pub age: f32,
    pub build: f32,
    pub skin: u32,
    pub hair: Hair,
    pub hair_color: u32,
    pub eye_color: u32,
    pub top: Top,
    pub top_color: u32,
    pub top_color2: u32,
    pub bottom: Bottom,
    pub bottom_color: u32,
    pub shoe_color: u32,
    pub sole_color: u32,
    pub mustache: bool,
    pub glasses: bool,
    pub seed: u64,
}

impl Appearance {
    pub fn sofia() -> Appearance {
        Appearance {
            name: "Sofía",
            height: 1.62,
            masc: 0.0,
            age: 0.0,
            build: 0.45,
            skin: 0xd4a488,
            hair: Hair::Ponytail,
            hair_color: 0x4a2f1e,
            eye_color: 0x7a5232,
            top: Top::Hoodie,
            top_color: 0xa6a2d4,
            top_color2: 0x8783bd,
            bottom: Bottom::Jeans,
            bottom_color: 0x557699,
            shoe_color: 0xf2efe9,
            sole_color: 0xe8e2d8,
            mustache: false,
            glasses: false,
            seed: 1,
        }
    }

    pub fn mama() -> Appearance {
        Appearance {
            name: "Carolina",
            height: 1.64,
            masc: 0.05,
            age: 1.0,
            build: 0.5,
            skin: 0xc2906f,
            hair: Hair::Bob,
            hair_color: 0x3a241a,
            eye_color: 0x4a3020,
            top: Top::Blouse,
            top_color: 0x3a7f86,
            top_color2: 0xe9e2d4,
            bottom: Bottom::Pants,
            bottom_color: 0x2e3440,
            shoe_color: 0x5a3c2c,
            sole_color: 0x2a1e18,
            mustache: false,
            glasses: false,
            seed: 2,
        }
    }

    pub fn tomas() -> Appearance {
        Appearance {
            name: "Tomás",
            height: 1.72,
            masc: 1.0,
            age: 0.1,
            build: 0.45,
            skin: 0xa9765a,
            hair: Hair::Curly,
            hair_color: 0x1c1410,
            eye_color: 0x3a2618,
            top: Top::Tee,
            top_color: 0x2f8f7a,
            top_color2: 0x246e5e,
            bottom: Bottom::Pants,
            bottom_color: 0x3d3a35,
            shoe_color: 0x1e1e22,
            sole_color: 0xf0ede6,
            mustache: false,
            glasses: false,
            seed: 3,
        }
    }

    pub fn abuela() -> Appearance {
        Appearance {
            name: "Abuela Rosa",
            height: 1.56,
            masc: 0.0,
            age: 2.0,
            build: 0.6,
            skin: 0xd6ab8e,
            hair: Hair::Bun,
            hair_color: 0xc9c5c0,
            eye_color: 0x4a3526,
            top: Top::Sweater,
            top_color: 0x9c4f5c,
            top_color2: 0x7d3c48,
            bottom: Bottom::Skirt,
            bottom_color: 0x4a4d5c,
            shoe_color: 0x3a2a22,
            sole_color: 0x221a15,
            mustache: false,
            glasses: true,
            seed: 4,
        }
    }

    pub fn don_julio() -> Appearance {
        Appearance {
            name: "Don Julio",
            height: 1.74,
            masc: 1.0,
            age: 1.6,
            build: 0.7,
            skin: 0xb88466,
            hair: Hair::Buzz,
            hair_color: 0x5b5550,
            eye_color: 0x3a2a1c,
            top: Top::Jacket,
            top_color: 0x34495e,
            top_color2: 0xe6e2da,
            bottom: Bottom::Pants,
            bottom_color: 0x2b2b30,
            shoe_color: 0x1f1a17,
            sole_color: 0x151210,
            mustache: true,
            glasses: false,
            seed: 5,
        }
    }

    pub fn vale() -> Appearance {
        Appearance {
            name: "Vale",
            height: 1.66,
            masc: 0.0,
            age: 0.8,
            build: 0.45,
            skin: 0x8d5b43,
            hair: Hair::Long,
            hair_color: 0x1a120e,
            eye_color: 0x2e1d12,
            top: Top::Apron,
            top_color: 0xece6da,
            top_color2: 0x3e6b58,
            bottom: Bottom::Jeans,
            bottom_color: 0x2a3140,
            shoe_color: 0xe9e4dc,
            sole_color: 0xdad3c8,
            mustache: false,
            glasses: false,
            seed: 6,
        }
    }

    /// Random pedestrian.
    pub fn random(seed: u64) -> Appearance {
        let mut r = Rng::new(seed * 7919 + 17);
        let masc = if r.chance(0.5) { 1.0 } else { 0.0 };
        let skins = [0xe0b89a, 0xc99a7e, 0xa9765a, 0x8d5b43, 0x6b4430, 0xd6ab8e];
        let hair_cols = [0x1c1410, 0x3a241a, 0x5a3a22, 0x8a6a4a, 0x2e1d15, 0xb89a6a];
        let tops = [0x2f4f7f, 0x8f3b3b, 0xd9b44a, 0x3b7f5a, 0x6a4a8f, 0xe8e2d8, 0x2b2b30, 0xc9724a];
        let bottoms = [0x2b2f3a, 0x3b5578, 0x4a4238, 0x1f1f24, 0x6b6a5a];
        let hair = if masc > 0.5 {
            *r.pick(&[Hair::Short, Hair::Curly, Hair::Buzz])
        } else {
            *r.pick(&[Hair::Ponytail, Hair::Bob, Hair::Long, Hair::Bun])
        };
        let tc = *r.pick(&tops);
        Appearance {
            name: "Vecino",
            height: if masc > 0.5 { r.range(1.66, 1.82) } else { r.range(1.55, 1.70) },
            masc,
            age: r.range(0.6, 1.6),
            build: r.range(0.35, 0.75),
            skin: *r.pick(&skins),
            hair,
            hair_color: *r.pick(&hair_cols),
            eye_color: 0x3a2618,
            top: *r.pick(&[Top::Tee, Top::Jacket, Top::Sweater, Top::Hoodie]),
            top_color: tc,
            top_color2: crate::math::scale_rgb(crate::math::rgb(tc), 0.8)
                .iter()
                .take(3)
                .fold(0u32, |acc, &c| (acc << 8) | c as u32),
            bottom: *r.pick(&[Bottom::Jeans, Bottom::Pants]),
            bottom_color: *r.pick(&bottoms),
            shoe_color: *r.pick(&[0x1e1e22, 0xf2efe9, 0x5a3c2c, 0x3a4a6a]),
            sole_color: 0xe8e2d8,
            mustache: masc > 0.5 && r.chance(0.2),
            glasses: r.chance(0.2),
            seed,
        }
    }
}
