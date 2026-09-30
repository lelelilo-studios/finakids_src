//! Café Aroma shift: serve orders quickly and correctly to earn tips.

use super::finance::Skill;
use super::State;
use crate::math::{money, Rng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prod {
    Coffee,
    Latte,
    Juice,
    Cookie,
    Sandwich,
}

impl Prod {
    pub fn all() -> [Prod; 5] {
        [Prod::Coffee, Prod::Latte, Prod::Juice, Prod::Cookie, Prod::Sandwich]
    }
    pub fn name(self) -> &'static str {
        match self {
            Prod::Coffee => "Café",
            Prod::Latte => "Latte",
            Prod::Juice => "Jugo",
            Prod::Cookie => "Galleta",
            Prod::Sandwich => "Sándwich",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Prod::Coffee => 0x7a4a2a,
            Prod::Latte => 0xd9b48a,
            Prod::Juice => 0xf29a2e,
            Prod::Cookie => 0xb8834a,
            Prod::Sandwich => 0xe8d27a,
        }
    }
}

pub struct Order {
    pub name: &'static str,
    pub items: Vec<Prod>,
    pub patience: f32,
    pub max_patience: f32,
}

pub struct CafeGame {
    pub t: f32,
    pub dur: f32,
    pub orders: Vec<Order>,
    pub tray: Vec<Prod>,
    pub served: u32,
    pub failed: u32,
    pub tips: i64,
    pub done: bool,
    pub feedback: Option<(String, bool, f32)>,
    rng: Rng,
    spawn: f32,
    pub busy: bool,
    pub wage: i64,
}

impl CafeGame {
    pub fn new(busy: bool, seed: u64) -> CafeGame {
        let mut g = CafeGame {
            t: 0.0,
            dur: 45.0,
            orders: Vec::new(),
            tray: Vec::new(),
            served: 0,
            failed: 0,
            tips: 0,
            done: false,
            feedback: None,
            rng: Rng::new(seed * 31 + 5),
            spawn: 0.0,
            busy,
            wage: 6_000,
        };
        g.new_order();
        g
    }

    fn new_order(&mut self) {
        let names = ["Señora Marta", "Profe Andrés", "Camila", "Don Pedro", "Una turista", "Joaquín", "Fernanda", "Un ciclista"];
        let n = 1 + self.rng.int(if self.t > 20.0 { 3 } else { 2 });
        let mut items = Vec::new();
        for _ in 0..n {
            items.push(*self.rng.pick(&Prod::all()));
        }
        let patience = 14.0 - n as f32 + self.rng.range(0.0, 3.0);
        self.orders.push(Order {
            name: *self.rng.pick(&names),
            items,
            patience,
            max_patience: patience,
        });
    }

    pub fn update(&mut self, dt: f32) {
        if let Some((_, _, t)) = &mut self.feedback {
            *t += dt;
        }
        if self.done {
            return;
        }
        self.t += dt;
        self.spawn += dt;
        let interval = if self.busy { 4.5 } else { 6.0 };
        if self.spawn > interval && self.orders.len() < 3 {
            self.spawn = 0.0;
            self.new_order();
        }
        let mut expired = 0;
        for o in &mut self.orders {
            o.patience -= dt;
            if o.patience <= 0.0 {
                expired += 1;
            }
        }
        if expired > 0 {
            self.orders.retain(|o| o.patience > 0.0);
            self.failed += expired;
            self.feedback = Some(("Un cliente se cansó de esperar y se fue".into(), false, 0.0));
        }
        if self.orders.is_empty() && self.t < self.dur - 3.0 {
            self.new_order();
        }
        if self.t >= self.dur {
            self.done = true;
        }
    }

    fn serve(&mut self) {
        if self.orders.is_empty() {
            return;
        }
        let mut want = self.orders[0].items.clone();
        let mut have = self.tray.clone();
        want.sort_by_key(|p| *p as u8);
        have.sort_by_key(|p| *p as u8);
        if want == have {
            let o = self.orders.remove(0);
            let frac = (o.patience / o.max_patience).clamp(0.0, 1.0);
            let base = if self.busy { 350.0 } else { 250.0 };
            let tip = ((base + frac * 700.0) / 50.0).round() as i64 * 50;
            self.tips += tip;
            self.served += 1;
            self.feedback = Some((format!("¡Perfecto! {} dejó {} de propina", o.name, money(tip)), true, 0.0));
        } else {
            self.feedback = Some(("Ese no es el pedido. Revisa la bandeja.".into(), false, 0.0));
        }
        self.tray.clear();
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MiniAct {
    Add(Prod),
    Clear,
    Serve,
    Finish,
}

pub fn apply(s: &mut State, a: MiniAct) {
    let Some(super::Modal::Minigame(g)) = &mut s.modal else {
        return;
    };
    match a {
        MiniAct::Add(p) => {
            if !g.done && g.tray.len() < 4 {
                g.tray.push(p);
            }
        }
        MiniAct::Clear => g.tray.clear(),
        MiniAct::Serve => g.serve(),
        MiniAct::Finish => {
            if !g.done {
                return;
            }
            let wage = g.wage;
            let tips = g.tips;
            let served = g.served;
            let failed = g.failed;
            s.modal = None;
            s.fin.earn(wage, "Sueldo turno Café Aroma");
            if tips > 0 {
                s.fin.earn(tips, "Propinas");
            }
            s.toast("briefcase", "Turno terminado", &format!("{} clientes atendidos", served), wage + tips);
            if !s.flag("first_shift") {
                s.set_flag("first_shift");
                s.fin.journal(
                    "Primer sueldo",
                    &format!(
                        "Trabajaste 3 horas y ganaste {} de sueldo más {} en propinas. Ahora sabes cuánto tiempo cuesta lo que compras: unos audífonos de $39.990 son casi 6 turnos.",
                        money(wage),
                        money(tips)
                    ),
                    Skill::Enterprise,
                    if failed <= served { 1 } else { 0 },
                );
            }
        }
    }
}
