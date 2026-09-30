//! Save games as a small line-based text format (works with localStorage and files).

use super::finance::{Account, Debt, Goal, InvestKind, Investment, JournalEntry, Skill, Tx};
use super::{ChatMsg, State};
use crate::platform;

const KEY: &str = "finakids_save_v1";

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('|', "\\p").replace('\n', "\\n")
}

fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('p') => out.push('|'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn skill_id(s: Skill) -> u8 {
    match s {
        Skill::Planning => 0,
        Skill::Comparing => 1,
        Skill::Risk => 2,
        Skill::Credit => 3,
        Skill::Enterprise => 4,
        Skill::Resilience => 5,
    }
}

fn skill_from(i: u8) -> Skill {
    match i {
        0 => Skill::Planning,
        1 => Skill::Comparing,
        2 => Skill::Risk,
        3 => Skill::Credit,
        4 => Skill::Enterprise,
        _ => Skill::Resilience,
    }
}

fn kind_id(k: InvestKind) -> u8 {
    match k {
        InvestKind::Deposit => 0,
        InvestKind::Fund => 1,
        InvestKind::Crypto => 2,
    }
}

fn kind_from(i: u8) -> InvestKind {
    match i {
        0 => InvestKind::Deposit,
        1 => InvestKind::Fund,
        _ => InvestKind::Crypto,
    }
}

/// Maps a string back to one of the static ids used by the game.
fn static_id(s: &str) -> &'static str {
    const IDS: &[&str] = &[
        "trip",
        "headphones_pro",
        "bike",
        "guitar",
        "emergency",
        "sneakers",
        "speaker",
        "videogame",
        "headphones_cheap",
        "skate",
        "latte",
        "cookie",
        "tecno",
        "mama",
        "banco",
        "abuela",
        "tomas",
        "vale",
        "colegio",
        "desconocido",
    ];
    IDS.iter().find(|i| **i == s).copied().unwrap_or("otro")
}

pub fn save(s: &State) {
    let f = &s.fin;
    let mut out = String::new();
    let mut line = |k: &str, v: String| {
        out.push_str(k);
        out.push('=');
        out.push_str(&v);
        out.push('\n');
    };
    line("week", s.week.to_string());
    line("wallet", f.wallet.to_string());
    line("savings", f.savings.to_string());
    line("score", f.credit_score.to_string());
    line("mood", f.mood.to_string());
    line("autosave", f.auto_save_pct.to_string());
    line("interest_earned", f.interest_earned.to_string());
    line("late_fees", f.late_fees.to_string());
    line("crypto", f.crypto_price.to_string());
    line("fund", f.fund_index.to_string());
    line("inventory", f.inventory.join(","));
    let flags: Vec<String> = s.flags.iter().cloned().collect();
    line("flags", flags.join(","));
    for g in &f.goals {
        line(
            "goal",
            format!("{}|{}|{}|{}|{}|{}", g.id, esc(&g.name), g.target, g.saved, g.deadline.unwrap_or(0), g.done as u8),
        );
    }
    for d in &f.debts {
        line(
            "debt",
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}",
                esc(&d.name),
                d.installment,
                d.payments_left,
                d.total_payments,
                d.principal,
                d.paid,
                d.late_weeks,
                d.overdue,
                d.lender
            ),
        );
    }
    for i in &f.investments {
        line("inv", format!("{}|{}|{}|{}|{}", kind_id(i.kind), i.invested, i.value, i.start_week, i.locked_until));
    }
    for j in &f.journal {
        line("journal", format!("{}|{}|{}|{}|{}", j.week, esc(&j.title), esc(&j.text), skill_id(j.skill), j.score));
    }
    for t in f.history.iter().rev().take(80).rev() {
        line("tx", format!("{}|{}|{}", t.week, esc(&t.desc), t.amount));
    }
    for m in s.messages.iter().rev().take(40).rev() {
        line("msg", format!("{}|{}|{}", m.from, m.week, esc(&m.text)));
    }
    let b = &s.biz;
    line(
        "biz",
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            b.active as u8, b.partner as u8, b.inventory, b.unit_cost, b.price, b.sessions, b.revenue, b.costs, b.reputation
        ),
    );
    platform::save_text(s.cfg.save_dir.as_deref(), KEY, &out);
}

pub fn exists(s: &State) -> bool {
    if s.cfg.fresh {
        return false;
    }
    platform::load_text(s.cfg.save_dir.as_deref(), KEY).map(|t| t.contains("week=")).unwrap_or(false)
}

pub fn load(s: &mut State) -> bool {
    let Some(text) = platform::load_text(s.cfg.save_dir.as_deref(), KEY) else {
        return false;
    };
    let mut f = super::finance::Finance::new();
    s.flags.clear();
    s.messages.clear();
    let mut biz = super::business::Business::new();
    for l in text.lines() {
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let parts: Vec<&str> = v.split('|').collect();
        let num = |i: usize| -> i64 { parts.get(i).and_then(|x| x.parse().ok()).unwrap_or(0) };
        match k {
            "week" => s.week = v.parse().unwrap_or(1),
            "wallet" => f.wallet = v.parse().unwrap_or(0),
            "savings" => f.savings = v.parse().unwrap_or(0),
            "score" => f.credit_score = v.parse().unwrap_or(700),
            "mood" => f.mood = v.parse().unwrap_or(0.6),
            "autosave" => f.auto_save_pct = v.parse().unwrap_or(0),
            "interest_earned" => f.interest_earned = v.parse().unwrap_or(0),
            "late_fees" => f.late_fees = v.parse().unwrap_or(0),
            "crypto" => f.crypto_price = v.parse().unwrap_or(1.0),
            "fund" => f.fund_index = v.parse().unwrap_or(1.0),
            "inventory" => {
                f.inventory = v.split(',').filter(|x| !x.is_empty()).map(static_id).collect();
            }
            "flags" => {
                for fl in v.split(',').filter(|x| !x.is_empty()) {
                    s.flags.insert(fl.to_string());
                }
            }
            "goal" => f.goals.push(Goal {
                id: static_id(parts.first().copied().unwrap_or("")),
                name: unesc(parts.get(1).copied().unwrap_or("")),
                target: num(2),
                saved: num(3),
                deadline: if num(4) > 0 { Some(num(4) as u32) } else { None },
                done: num(5) != 0,
            }),
            "debt" => f.debts.push(Debt {
                name: unesc(parts.first().copied().unwrap_or("")),
                installment: num(1),
                payments_left: num(2) as u32,
                total_payments: num(3) as u32,
                principal: num(4),
                paid: num(5),
                late_weeks: num(6) as u32,
                overdue: num(7),
                lender: static_id(parts.get(8).copied().unwrap_or("")),
            }),
            "inv" => f.investments.push(Investment {
                kind: kind_from(num(0) as u8),
                invested: num(1),
                value: num(2),
                start_week: num(3) as u32,
                locked_until: num(4) as u32,
                history: vec![num(1), num(2)],
            }),
            "journal" => f.journal.push(JournalEntry {
                week: num(0) as u32,
                title: unesc(parts.get(1).copied().unwrap_or("")),
                text: unesc(parts.get(2).copied().unwrap_or("")),
                skill: skill_from(num(3) as u8),
                score: num(4) as i32,
            }),
            "tx" => f.history.push(Tx {
                week: num(0) as u32,
                desc: unesc(parts.get(1).copied().unwrap_or("")),
                amount: num(2),
                account: Account::Wallet,
            }),
            "msg" => s.messages.push(ChatMsg {
                from: static_id(parts.first().copied().unwrap_or("")),
                week: num(1) as u32,
                text: unesc(parts.get(2).copied().unwrap_or("")),
            }),
            "biz" => {
                biz.active = num(0) != 0;
                biz.partner = num(1) != 0;
                biz.inventory = num(2) as u32;
                biz.unit_cost = parts.get(3).and_then(|x| x.parse().ok()).unwrap_or(0.0);
                biz.price = num(4).max(500);
                biz.sessions = num(5) as u32;
                biz.revenue = num(6);
                biz.costs = num(7);
                biz.reputation = parts.get(8).and_then(|x| x.parse().ok()).unwrap_or(0.3);
            }
            _ => {}
        }
    }
    f.week = s.week;
    s.fin = f;
    s.biz = biz;
    true
}
