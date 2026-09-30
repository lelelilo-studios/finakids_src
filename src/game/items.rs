//! Shop catalogue.

use crate::ui::icons::Icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shop {
    TecnoMundo,
    Feria,
    Cafe,
}

impl Shop {
    pub fn name(self) -> &'static str {
        match self {
            Shop::TecnoMundo => "TecnoMundo",
            Shop::Feria => "Feria del barrio",
            Shop::Cafe => "Café Aroma",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub price: i64,
    pub regular: i64,
    pub shop: Shop,
    pub icon: Icon,
    /// Mood boost when bought (fades over the following weeks).
    pub joy: f32,
    /// Quality 1..=3 (cheap items may break)
    pub quality: u8,
    /// Prop shown in the bedroom when owned.
    pub prop: Option<&'static str>,
    pub credit: bool,
    pub consumable: bool,
}

pub fn catalog() -> Vec<Item> {
    vec![
        Item {
            id: "headphones_pro",
            name: "Audífonos inalámbricos Pro",
            desc: "Cancelación de ruido, 30 h de batería. Garantía de 1 año.",
            price: 39_990,
            regular: 49_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Star,
            joy: 0.35,
            quality: 3,
            prop: Some("item_headphones"),
            credit: true,
            consumable: false,
        },
        Item {
            id: "sneakers",
            name: "Zapatillas urbanas",
            desc: "Las que usan todos en el colegio. Edición de temporada.",
            price: 34_990,
            regular: 34_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Bag,
            joy: 0.3,
            quality: 3,
            prop: Some("item_sneakers"),
            credit: true,
            consumable: false,
        },
        Item {
            id: "speaker",
            name: "Parlante bluetooth",
            desc: "Pequeño pero potente. Resistente al agua.",
            price: 19_990,
            regular: 24_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Bulb,
            joy: 0.2,
            quality: 2,
            prop: Some("item_speaker"),
            credit: true,
            consumable: false,
        },
        Item {
            id: "videogame",
            name: "Videojuego de estreno",
            desc: "El juego que todos comentan. Digital, sin reventa.",
            price: 24_990,
            regular: 24_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Star,
            joy: 0.3,
            quality: 3,
            prop: None,
            credit: true,
            consumable: false,
        },
        Item {
            id: "guitar",
            name: "Guitarra acústica",
            desc: "Para empezar a aprender. Incluye funda.",
            price: 89_990,
            regular: 89_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Heart,
            joy: 0.4,
            quality: 3,
            prop: Some("item_guitar"),
            credit: true,
            consumable: false,
        },
        Item {
            id: "bike",
            name: "Bicicleta urbana",
            desc: "Aro 26, 21 cambios. Te ahorra la micro para siempre.",
            price: 149_990,
            regular: 149_990,
            shop: Shop::TecnoMundo,
            icon: Icon::Target,
            joy: 0.45,
            quality: 3,
            prop: Some("item_bike"),
            credit: true,
            consumable: false,
        },
        Item {
            id: "headphones_cheap",
            name: "Audífonos genéricos",
            desc: "Se ven parecidos a los Pro... sin garantía.",
            price: 12_990,
            regular: 12_990,
            shop: Shop::Feria,
            icon: Icon::Star,
            joy: 0.2,
            quality: 1,
            prop: Some("item_headphones"),
            credit: false,
            consumable: false,
        },
        Item {
            id: "skate",
            name: "Skate usado",
            desc: "Buen estado. El vendedor dice que \"casi no se usó\".",
            price: 15_000,
            regular: 29_990,
            shop: Shop::Feria,
            icon: Icon::Target,
            joy: 0.25,
            quality: 2,
            prop: Some("item_skate"),
            credit: false,
            consumable: false,
        },
        Item {
            id: "latte",
            name: "Latte de vainilla",
            desc: "Rico... y se acaba en 10 minutos.",
            price: 2_900,
            regular: 2_900,
            shop: Shop::Cafe,
            icon: Icon::Heart,
            joy: 0.06,
            quality: 3,
            prop: None,
            credit: false,
            consumable: true,
        },
        Item {
            id: "cookie",
            name: "Galleta gigante",
            desc: "Chocolate belga.",
            price: 1_500,
            regular: 1_500,
            shop: Shop::Cafe,
            icon: Icon::Heart,
            joy: 0.03,
            quality: 3,
            prop: None,
            credit: false,
            consumable: true,
        },
    ]
}

pub fn item(id: &str) -> Option<Item> {
    catalog().into_iter().find(|i| i.id == id)
}
