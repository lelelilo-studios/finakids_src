//! Personal finance model: wallet, savings, goals, credit, investments and history.

use crate::math::{money, Rng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Account {
    Wallet,
    Savings,
    Goal,
    Credit,
    Invest,
    Business,
}

#[derive(Clone, Debug)]
pub struct Tx {
    pub week: u32,
    pub desc: String,
    pub amount: i64,
    pub account: Account,
}

#[derive(Clone, Debug)]
pub struct Goal {
    pub id: &'static str,
    pub name: String,
    pub target: i64,
    pub saved: i64,
    pub deadline: Option<u32>,
    pub done: bool,
}

impl Goal {
    pub fn progress(&self) -> f32 {
        (self.saved as f32 / self.target.max(1) as f32).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Debt {
    pub name: String,
    pub installment: i64,
    pub payments_left: u32,
    pub total_payments: u32,
    pub principal: i64,
    pub paid: i64,
    pub late_weeks: u32,
    pub overdue: i64,
    pub lender: &'static str,
}

impl Debt {
    pub fn total_cost(&self) -> i64 {
        self.installment * self.total_payments as i64
    }
    pub fn interest(&self) -> i64 {
        self.total_cost() - self.principal
    }
    pub fn remaining(&self) -> i64 {
        self.installment * self.payments_left as i64 + self.overdue
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvestKind {
    Deposit,
    Fund,
    Crypto,
}

impl InvestKind {
    pub fn name(self) -> &'static str {
        match self {
            InvestKind::Deposit => "Depósito a plazo",
            InvestKind::Fund => "Fondo mutuo moderado",
            InvestKind::Crypto => "MoonCoin",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Investment {
    pub kind: InvestKind,
    pub invested: i64,
    pub value: i64,
    pub start_week: u32,
    pub locked_until: u32,
    pub history: Vec<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skill {
    Planning,
    Comparing,
    Risk,
    Credit,
    Enterprise,
    Resilience,
}

impl Skill {
    pub fn name(self) -> &'static str {
        match self {
            Skill::Planning => "Planificar",
            Skill::Comparing => "Comparar alternativas",
            Skill::Risk => "Entender el riesgo",
            Skill::Credit => "Usar bien el crédito",
            Skill::Enterprise => "Emprender",
            Skill::Resilience => "Enfrentar imprevistos",
        }
    }
    pub fn all() -> [Skill; 6] {
        [Skill::Planning, Skill::Comparing, Skill::Risk, Skill::Credit, Skill::Enterprise, Skill::Resilience]
    }
}

#[derive(Clone, Debug)]
pub struct JournalEntry {
    pub week: u32,
    pub title: String,
    pub text: String,
    pub skill: Skill,
    /// -2..=2: how the decision played out
    pub score: i32,
}

#[derive(Clone, Debug, Default)]
pub struct WeekReport {
    pub week: u32,
    pub lines: Vec<(String, i64)>,
    pub notes: Vec<String>,
}

pub struct Finance {
    pub wallet: i64,
    pub savings: i64,
    pub goals: Vec<Goal>,
    pub debts: Vec<Debt>,
    pub investments: Vec<Investment>,
    pub history: Vec<Tx>,
    pub journal: Vec<JournalEntry>,
    pub inventory: Vec<&'static str>,
    pub credit_score: i32,
    pub mood: f32,
    pub allowance: i64,
    pub auto_save_pct: u32,
    pub savings_rate: f32,
    pub week: u32,
    pub interest_earned: i64,
    pub interest_paid: i64,
    pub late_fees: i64,
    pub rng: Rng,
    pub crypto_price: f32,
    pub crypto_history: Vec<f32>,
    pub fund_history: Vec<f32>,
    pub fund_index: f32,
}

impl Finance {
    /// Lunches and bus fare paid every week.
    pub const FIXED_WEEKLY: i64 = 6_000;

    /// Money left each week after fixed expenses and current installments.
    pub fn weekly_free(&self) -> i64 {
        let inst: i64 = self.debts.iter().filter(|d| d.payments_left > 0).map(|d| d.installment).sum();
        self.allowance - Self::FIXED_WEEKLY - inst
    }

    pub fn new() -> Finance {
        Finance {
            wallet: 2_500,
            savings: 0,
            goals: Vec::new(),
            debts: Vec::new(),
            investments: Vec::new(),
            history: Vec::new(),
            journal: Vec::new(),
            inventory: Vec::new(),
            credit_score: 700,
            mood: 0.6,
            allowance: 10_000,
            auto_save_pct: 0,
            savings_rate: 0.005,
            week: 1,
            interest_earned: 0,
            interest_paid: 0,
            late_fees: 0,
            rng: Rng::new(2026),
            crypto_price: 1.0,
            crypto_history: vec![1.0],
            fund_history: vec![1.0],
            fund_index: 1.0,
        }
    }

    pub fn log(&mut self, desc: impl Into<String>, amount: i64, account: Account) {
        self.history.push(Tx {
            week: self.week,
            desc: desc.into(),
            amount,
            account,
        });
    }

    pub fn journal(&mut self, title: impl Into<String>, text: impl Into<String>, skill: Skill, score: i32) {
        self.journal.push(JournalEntry {
            week: self.week,
            title: title.into(),
            text: text.into(),
            skill,
            score,
        });
    }

    pub fn goals_saved(&self) -> i64 {
        self.goals.iter().filter(|g| !g.done).map(|g| g.saved).sum()
    }

    pub fn invested_value(&self) -> i64 {
        self.investments.iter().map(|i| i.value).sum()
    }

    pub fn total_debt(&self) -> i64 {
        self.debts.iter().map(|d| d.remaining()).sum()
    }

    pub fn net_worth(&self) -> i64 {
        self.wallet + self.savings + self.goals_saved() + self.invested_value() - self.total_debt()
    }

    /// Money available to spend right now (wallet).
    pub fn can_pay(&self, amount: i64) -> bool {
        self.wallet >= amount
    }

    pub fn earn(&mut self, amount: i64, desc: &str) {
        self.wallet += amount;
        self.log(desc, amount, Account::Wallet);
    }

    pub fn spend(&mut self, amount: i64, desc: &str) -> bool {
        if self.wallet < amount {
            return false;
        }
        self.wallet -= amount;
        self.log(desc, -amount, Account::Wallet);
        true
    }

    pub fn to_savings(&mut self, amount: i64) -> bool {
        let a = amount.min(self.wallet).max(0);
        if a == 0 {
            return false;
        }
        self.wallet -= a;
        self.savings += a;
        self.log("Transferencia a ahorro", -a, Account::Savings);
        true
    }

    pub fn from_savings(&mut self, amount: i64) -> bool {
        let a = amount.min(self.savings).max(0);
        if a == 0 {
            return false;
        }
        self.savings -= a;
        self.wallet += a;
        self.log("Retiro desde ahorro", a, Account::Wallet);
        true
    }

    pub fn add_goal(&mut self, id: &'static str, name: &str, target: i64, deadline: Option<u32>) {
        if self.goals.iter().any(|g| g.id == id && !g.done) {
            return;
        }
        self.goals.push(Goal {
            id,
            name: name.to_string(),
            target,
            saved: 0,
            deadline,
            done: false,
        });
    }

    pub fn goal_mut(&mut self, id: &str) -> Option<&mut Goal> {
        self.goals.iter_mut().find(|g| g.id == id && !g.done)
    }

    pub fn main_goal(&self) -> Option<&Goal> {
        self.goals.iter().find(|g| !g.done)
    }

    pub fn to_goal(&mut self, id: &str, amount: i64) -> i64 {
        let a = amount.min(self.wallet).max(0);
        if let Some(g) = self.goals.iter_mut().find(|g| g.id == id && !g.done) {
            let a = a.min(g.target - g.saved).max(0);
            g.saved += a;
            let name = g.name.clone();
            self.wallet -= a;
            self.log(format!("Apartado para: {name}"), -a, Account::Goal);
            return a;
        }
        0
    }

    pub fn from_goal(&mut self, id: &str, amount: i64) -> i64 {
        if let Some(g) = self.goals.iter_mut().find(|g| g.id == id && !g.done) {
            let a = amount.min(g.saved).max(0);
            g.saved -= a;
            let name = g.name.clone();
            self.wallet += a;
            self.log(format!("Retiro de meta: {name}"), a, Account::Wallet);
            return a;
        }
        0
    }

    /// Credit purchase: pays in `n` weekly installments with a weekly rate.
    pub fn take_credit(&mut self, name: &str, principal: i64, n: u32, weekly_rate: f32, lender: &'static str) -> i64 {
        let inst = if weekly_rate <= 0.0 {
            (principal as f32 / n as f32).ceil() as i64
        } else {
            let r = weekly_rate as f64;
            let p = principal as f64;
            let inst = p * r / (1.0 - (1.0 + r).powi(-(n as i32)));
            (inst / 10.0).ceil() as i64 * 10
        };
        self.debts.push(Debt {
            name: name.to_string(),
            installment: inst,
            payments_left: n,
            total_payments: n,
            principal,
            paid: 0,
            late_weeks: 0,
            overdue: 0,
            lender,
        });
        self.log(format!("Compra en {n} cuotas: {name}"), 0, Account::Credit);
        inst
    }

    /// Estimated installment for a credit offer.
    pub fn quote(principal: i64, n: u32, weekly_rate: f32) -> i64 {
        if weekly_rate <= 0.0 {
            return (principal as f32 / n as f32).ceil() as i64;
        }
        let r = weekly_rate as f64;
        let inst = principal as f64 * r / (1.0 - (1.0 + r).powi(-(n as i32)));
        (inst / 10.0).ceil() as i64 * 10
    }

    pub fn invest(&mut self, kind: InvestKind, amount: i64) -> bool {
        let a = amount.min(self.wallet).max(0);
        if a <= 0 {
            return false;
        }
        self.wallet -= a;
        let locked = if kind == InvestKind::Deposit { self.week + 4 } else { self.week };
        self.investments.push(Investment {
            kind,
            invested: a,
            value: a,
            start_week: self.week,
            locked_until: locked,
            history: vec![a],
        });
        self.log(format!("Inversión: {}", kind.name()), -a, Account::Invest);
        true
    }

    pub fn withdraw_investment(&mut self, idx: usize) -> Option<i64> {
        if idx >= self.investments.len() {
            return None;
        }
        if self.investments[idx].locked_until > self.week {
            return None;
        }
        let inv = self.investments.remove(idx);
        self.wallet += inv.value;
        let gain = inv.value - inv.invested;
        self.log(format!("Rescate: {} ({})", inv.kind.name(), crate::math::money_signed(gain)), inv.value, Account::Wallet);
        Some(gain)
    }

    /// Advances one week. Returns the report shown to the player.
    pub fn end_week(&mut self, crypto_crash: bool) -> WeekReport {
        let mut rep = WeekReport {
            week: self.week,
            ..Default::default()
        };
        // allowance
        self.wallet += self.allowance;
        self.log("Mesada semanal", self.allowance, Account::Wallet);
        rep.lines.push(("Mesada".into(), self.allowance));
        // fixed weekly expenses
        let lunch = 4_000;
        let bus = 2_000;
        let fixed = Self::FIXED_WEEKLY;
        self.wallet -= fixed;
        self.log("Colaciones y transporte", -fixed, Account::Wallet);
        rep.lines.push(("Colaciones de la semana".into(), -lunch));
        rep.lines.push(("Transporte (micro)".into(), -bus));
        // automatic savings rule
        if self.auto_save_pct > 0 {
            let a = (self.allowance * self.auto_save_pct as i64 / 100).min(self.wallet.max(0));
            if a > 0 {
                self.wallet -= a;
                self.savings += a;
                self.log("Ahorro automático", -a, Account::Savings);
                rep.lines.push((format!("Ahorro automático ({}%)", self.auto_save_pct), -a));
                rep.notes.push(format!("Tu regla de ahorro guardó {} sin que tuvieras que pensarlo.", money(a)));
            }
        }
        // savings interest
        if self.savings > 0 {
            let i = (self.savings as f32 * self.savings_rate).round() as i64;
            if i > 0 {
                self.savings += i;
                self.interest_earned += i;
                self.log("Interés cuenta de ahorro", i, Account::Savings);
                rep.notes.push(format!("Tu cuenta de ahorro generó {} de interés.", money(i)));
            }
        }
        // debts
        let mut paid_total = 0;
        let mut debts = std::mem::take(&mut self.debts);
        for d in &mut debts {
            if d.payments_left == 0 && d.overdue == 0 {
                continue;
            }
            let due = if d.payments_left > 0 { d.installment } else { 0 } + d.overdue;
            // if the wallet is short, the bank takes the rest from savings
            if self.wallet < due && self.wallet + self.savings >= due {
                let from_sav = due - self.wallet.max(0);
                self.savings -= from_sav;
                self.wallet += from_sav;
                rep.notes.push(format!(
                    "Tu billetera no alcanzaba para la cuota de {}: se usaron {} de tu ahorro.",
                    d.name,
                    money(from_sav)
                ));
            }
            if self.wallet >= due {
                self.wallet -= due;
                d.paid += due;
                paid_total += due;
                if d.payments_left > 0 {
                    d.payments_left -= 1;
                }
                d.overdue = 0;
                rep.lines.push((format!("Cuota: {}", d.name), -due));
            } else {
                // cannot pay: late fee + interest on the unpaid installment
                let fee = 1_500;
                let unpaid = due + fee + (due as f32 * 0.03) as i64;
                d.overdue = unpaid;
                if d.payments_left > 0 {
                    d.payments_left -= 1;
                }
                d.late_weeks += 1;
                self.late_fees += fee;
                self.credit_score -= 45;
                rep.lines.push((format!("Cuota impaga: {}", d.name), 0));
                rep.notes.push(format!(
                    "No alcanzaste a pagar la cuota de {}. Se cobró una multa de {} y la deuda creció.",
                    d.name,
                    money(fee)
                ));
            }
        }
        self.debts = debts;
        if paid_total > 0 {
            self.log("Pago de cuotas", -paid_total, Account::Credit);
        }
        let finished: Vec<String> = self
            .debts
            .iter()
            .filter(|d| d.payments_left == 0 && d.overdue == 0)
            .map(|d| d.name.clone())
            .collect();
        for n in finished {
            rep.notes.push(format!("¡Terminaste de pagar {n}!"));
            self.credit_score += 20;
        }
        self.debts.retain(|d| d.payments_left > 0 || d.overdue > 0);
        // markets
        let fund_ret = 0.006 + self.rng.normal() * 0.022;
        self.fund_index *= 1.0 + fund_ret;
        self.fund_history.push(self.fund_index);
        let crypto_ret = if crypto_crash {
            -0.88
        } else if self.week <= 7 {
            0.14 + self.rng.normal() * 0.18
        } else {
            self.rng.normal() * 0.25
        };
        self.crypto_price = (self.crypto_price * (1.0 + crypto_ret)).max(0.01);
        self.crypto_history.push(self.crypto_price);
        for inv in &mut self.investments {
            let before = inv.value;
            match inv.kind {
                InvestKind::Deposit => {
                    if self.week < inv.locked_until {
                        // 3% over 4 weeks, credited at maturity
                        if self.week + 1 == inv.locked_until {
                            inv.value = (inv.invested as f32 * 1.03).round() as i64;
                        }
                    }
                }
                InvestKind::Fund => {
                    inv.value = (inv.value as f32 * (1.0 + fund_ret)).round() as i64;
                }
                InvestKind::Crypto => {
                    inv.value = (inv.value as f32 * (1.0 + crypto_ret)).round().max(0.0) as i64;
                }
            }
            inv.history.push(inv.value);
            let diff = inv.value - before;
            if diff != 0 {
                rep.notes.push(format!("{}: {} esta semana.", inv.kind.name(), crate::math::money_signed(diff)));
            }
        }
        // mood slowly returns to baseline (hedonic adaptation)
        self.mood += (0.55 - self.mood) * 0.35;
        self.week += 1;
        rep
    }
}

impl Default for Finance {
    fn default() -> Self {
        Self::new()
    }
}
