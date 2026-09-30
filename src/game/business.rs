//! Bracelet stall at the Saturday fair: costs, pricing, demand, inventory and profit.

use super::finance::Skill;
use super::time::Phase;
use super::{Modal, State};
use crate::math::{money, Rng};

pub const STALL_RENT: i64 = 2_000;
pub const KIT_SMALL: (u32, i64) = (10, 8_000);
pub const KIT_LARGE: (u32, i64) = (30, 19_500);

pub struct Business {
    pub active: bool,
    pub partner: bool,
    pub inventory: u32,
    pub unit_cost: f32,
    pub price: i64,
    pub sessions: u32,
    pub revenue: i64,
    pub costs: i64,
    pub reputation: f32,
    pub history: Vec<(u32, i64)>,
}

impl Business {
    pub fn new() -> Business {
        Business {
            active: false,
            partner: true,
            inventory: 0,
            unit_cost: 0.0,
            price: 2_000,
            sessions: 0,
            revenue: 0,
            costs: 0,
            reputation: 0.3,
            history: Vec::new(),
        }
    }
    pub fn profit(&self) -> i64 {
        self.revenue - self.costs
    }
}

impl Default for Business {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BizPhase {
    Setup,
    Selling,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Sunny,
    Cloudy,
    Rain,
}

impl Weather {
    pub fn label(self) -> &'static str {
        match self {
            Weather::Sunny => "Soleado: mucha gente en la feria",
            Weather::Cloudy => "Nublado: algo menos de público",
            Weather::Rain => "Lluvia: casi nadie se detiene",
        }
    }
    fn factor(self) -> f32 {
        match self {
            Weather::Sunny => 1.0,
            Weather::Cloudy => 0.8,
            Weather::Rain => 0.45,
        }
    }
}

pub struct FeedItem {
    pub text: String,
    pub good: bool,
    pub age: f32,
}

pub struct BizUi {
    pub phase: BizPhase,
    pub kit: u32,
    pub price: i64,
    pub t: f32,
    pub duration: f32,
    pub next: f32,
    pub passersby: u32,
    pub sold: u32,
    pub revenue: i64,
    pub lost: u32,
    pub weather: Weather,
    pub feed: Vec<FeedItem>,
    pub spent: i64,
    pub message: Option<String>,
    pub max_customers: u32,
}

impl BizUi {
    pub fn new(b: &Business) -> BizUi {
        BizUi {
            phase: BizPhase::Setup,
            kit: if b.inventory == 0 { 1 } else { 0 },
            price: b.price,
            t: 0.0,
            duration: 18.0,
            next: 0.6,
            passersby: 0,
            sold: 0,
            revenue: 0,
            lost: 0,
            weather: Weather::Sunny,
            feed: Vec::new(),
            spent: 0,
            message: None,
            max_customers: 24,
        }
    }

    pub fn kit_info(&self) -> (u32, i64) {
        match self.kit {
            1 => KIT_SMALL,
            2 => KIT_LARGE,
            _ => (0, 0),
        }
    }

    /// Probability that a passer-by buys at the given price.
    pub fn buy_prob(price: i64, reputation: f32) -> f32 {
        let p = 0.78 - (price as f32 - 1_500.0) / 3_600.0;
        (p * (0.85 + reputation * 0.3)).clamp(0.04, 0.92)
    }

    pub fn update(&mut self, dt: f32, b: &mut Business, rng: &mut Rng) {
        for f in &mut self.feed {
            f.age += dt;
        }
        if self.phase != BizPhase::Selling {
            return;
        }
        self.t += dt;
        self.next -= dt;
        if self.next <= 0.0 && self.passersby < self.max_customers {
            self.next = rng.range(0.45, 0.95);
            self.passersby += 1;
            let names = ["Una señora", "Un niño con su papá", "Dos amigas", "Un turista", "Una profesora", "Un chico en skate", "Una abuelita", "Un vecino"];
            let who = *rng.pick(&names);
            let p = Self::buy_prob(self.price, b.reputation) * self.weather.factor();
            if rng.chance(p) {
                if b.inventory > 0 {
                    b.inventory -= 1;
                    self.sold += 1;
                    self.revenue += self.price;
                    self.feed.push(FeedItem {
                        text: format!("{who} compró una pulsera (+{})", money(self.price)),
                        good: true,
                        age: 0.0,
                    });
                } else {
                    self.lost += 1;
                    self.feed.push(FeedItem {
                        text: format!("{who} quería comprar... ¡pero no quedan pulseras!"),
                        good: false,
                        age: 0.0,
                    });
                }
            } else {
                let why = if self.price >= 3_000 { "encontró caro el precio" } else { "solo miró" };
                self.feed.push(FeedItem {
                    text: format!("{who} {why}"),
                    good: false,
                    age: 0.0,
                });
            }
            if self.feed.len() > 7 {
                self.feed.remove(0);
            }
        }
        if self.t >= self.duration || (self.passersby >= self.max_customers && self.next <= 0.0) {
            self.phase = BizPhase::Result;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BizAct {
    Kit(u32),
    Price(i64),
    Start,
    Finish,
    Close,
}

pub fn apply(s: &mut State, a: BizAct) {
    let Some(Modal::Business(ui)) = &mut s.modal else {
        return;
    };
    match a {
        BizAct::Kit(k) => {
            ui.kit = k;
            ui.message = None;
        }
        BizAct::Price(p) => ui.price = p.clamp(500, 5_000),
        BizAct::Close => {
            s.modal = None;
        }
        BizAct::Start => {
            let (units, kit_cost) = ui.kit_info();
            let total = STALL_RENT + kit_cost;
            let share = if s.biz.partner { total / 2 } else { total };
            if s.biz.inventory + units == 0 {
                ui.message = Some("No tienes pulseras para vender. Compra materiales.".into());
                return;
            }
            if s.fin.wallet < share {
                ui.message = Some(format!("Necesitas {} en tu billetera para tu parte de los costos.", money(share)));
                return;
            }
            s.fin.spend(share, "Negocio: materiales y puesto");
            // weighted average unit cost
            if units > 0 {
                let old = s.biz.unit_cost * s.biz.inventory as f32;
                s.biz.inventory += units;
                s.biz.unit_cost = (old + kit_cost as f32) / s.biz.inventory as f32;
            }
            s.biz.costs += total;
            s.biz.price = ui.price;
            ui.spent = total;
            ui.phase = BizPhase::Selling;
            let r = s.rng.f();
            ui.weather = if r < 0.6 {
                super::business::Weather::Sunny
            } else if r < 0.88 {
                super::business::Weather::Cloudy
            } else {
                super::business::Weather::Rain
            };
            ui.t = 0.0;
            ui.max_customers = 20 + (s.biz.reputation * 10.0) as u32;
        }
        BizAct::Finish => {
            let revenue = ui.revenue;
            let spent = ui.spent;
            let sold = ui.sold;
            let lost = ui.lost;
            let share_rev = if s.biz.partner { revenue / 2 } else { revenue };
            let share_cost = if s.biz.partner { spent / 2 } else { spent };
            s.biz.revenue += revenue;
            s.biz.sessions += 1;
            s.biz.reputation = (s.biz.reputation + 0.08 + sold as f32 * 0.005).min(1.0);
            let profit = share_rev - share_cost;
            s.biz.history.push((s.week, profit));
            if share_rev > 0 {
                s.fin.earn(share_rev, "Ventas de pulseras");
            }
            s.modal = None;
            s.set_flag("sold_today");
            s.set_phase(Phase::Evening);
            s.loc_mut(crate::world::Loc::Plaza).set_prop_visible("stall_goods", false);
            let total_profit = s.biz.profit();
            let score = if profit > 0 { 2 } else if profit == 0 { 0 } else { -1 };
            let mut txt = format!(
                "Vendiste {} pulseras a {}. Tu parte: ingresos {} - costos {} = {}.",
                sold,
                money(s.biz.price),
                money(share_rev),
                money(share_cost),
                crate::math::money_signed(profit)
            );
            if lost > 0 {
                txt.push_str(&format!(" Te quedaste sin stock: perdiste {lost} ventas."));
            }
            if s.biz.inventory > 0 {
                txt.push_str(&format!(" Quedan {} pulseras para la próxima feria.", s.biz.inventory));
            }
            if s.biz.sessions == 1 {
                s.fin.journal("Primer día de negocio", &txt, Skill::Enterprise, score);
            } else if s.biz.sessions == 3 {
                s.fin.journal(
                    "Un negocio que crece",
                    &format!("Tras tres ferias, el negocio acumula {} de resultado total.", crate::math::money_signed(total_profit)),
                    Skill::Enterprise,
                    if total_profit > 0 { 1 } else { 0 },
                );
            }
            s.toast("chart", "Día de feria terminado", &format!("{} pulseras vendidas", sold), share_rev);
        }
    }
}
