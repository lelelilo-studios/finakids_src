//! Automated play-tester: follows objectives, walks between places, talks,
//! handles every modal and sleeps to advance weeks. Used with `--bot`.

use super::business::{BizAct, BizPhase};
use super::minigame::MiniAct;
use super::script::Who;
use super::*;

#[derive(Default)]
pub struct Bot {
    pub idle: f32,
    pub stuck: f32,
    pub last_week: u32,
    pub done_laptop: bool,
    pub talked: Vec<(u32, &'static str)>,
    pub bought: bool,
    pub log: Vec<String>,
    pub target_desc: String,
    pub finished: bool,
    pub attempts: Vec<(u32, &'static str)>,
    pub cool: f32,
    pub policy: String,
    pub rng: crate::math::Rng,
    pub deposited: Vec<u32>,
}

impl Bot {
    pub fn new(policy: &str) -> Bot {
        let seed = policy.split(':').nth(1).and_then(|x| x.parse().ok()).unwrap_or(1);
        Bot {
            policy: policy.to_string(),
            rng: crate::math::Rng::new(seed),
            ..Default::default()
        }
    }

    fn pick(&mut self, opts: &[script::Opt]) -> usize {
        let enabled: Vec<usize> = (0..opts.len()).filter(|&i| opts[i].enabled).collect();
        if enabled.is_empty() {
            return 0;
        }
        // always go to sleep when offered, so the week advances
        if let Some(&i) = enabled.iter().find(|&&i| opts[i].label.starts_with("Dormir")) {
            return i;
        }
        if self.policy.starts_with("last") {
            return *enabled.last().unwrap();
        }
        if self.policy.starts_with("random") {
            return enabled[self.rng.int(enabled.len())];
        }
        if self.policy.starts_with("wise") {
            let good = ["crear una meta", "usarlo para una meta", "20%", "pagar la reparación", "ignorarlo", "preguntarle", "acepto", "trabajar en el café", "socios", "transmisión", "pagar el viaje", "aceptar el plan", "dormir"];
            let bad = ["cuotas", "crédito", "$4.990", "gastarlo todo", "ir al cine"];
            let score = |o: &script::Opt| {
                let l = o.label.to_lowercase();
                let mut sc = 0;
                for g in good {
                    if l.contains(g) {
                        sc += 2;
                    }
                }
                for b in bad {
                    if l.contains(b) {
                        sc -= 3;
                    }
                }
                sc
            };
            let mut best = enabled[0];
            for &i in &enabled {
                if score(&opts[i]) > score(&opts[best]) {
                    best = i;
                }
            }
            return best;
        }
        enabled[0]
    }
}

fn door_to(from: Loc, to: Loc) -> Option<&'static str> {
    match (from, to) {
        (Loc::Bedroom, _) => Some("door"),
        (Loc::Home, Loc::Bedroom) => Some("hall_door"),
        (Loc::Home, Loc::Plaza) => Some("front_door"),
        (Loc::Plaza, _) => Some("home"),
        _ => None,
    }
}

impl Bot {
    fn tries(&self, week: u32, id: &'static str) -> usize {
        self.attempts.iter().filter(|a| a.0 == week && a.1 == id).count()
    }

    fn note(&mut self, s: &State, msg: &str) {
        let line = format!("[bot w{} {:?} ${}] {}", s.week, s.loc_id(), s.fin.wallet, msg);
        log::info!("{line}");
        self.log.push(line);
    }

    /// Returns UI actions to apply this frame.
    pub fn step(&mut self, s: &mut State, dt: f32) -> Vec<UiAct> {
        let mut acts = Vec::new();
        if s.week != self.last_week {
            self.last_week = s.week;
            let msg = format!(
                "== semana {} | billetera {} ahorro {} metas {} deuda {} | diario {} entradas",
                s.week,
                s.fin.wallet,
                s.fin.savings,
                s.fin.goals_saved(),
                s.fin.total_debt(),
                s.fin.journal.len()
            );
            self.note(s, &msg);
        }
        // modals
        if let Some(m) = &s.modal {
            self.idle = 0.0;
            match m {
                Modal::Shop(st) => {
                    if !self.bought && st.shop == items::Shop::TecnoMundo {
                        self.bought = true;
                        acts.push(UiAct::ShopCompare(true));
                        acts.push(UiAct::Buy(2, None));
                        self.note(s, "compra en TecnoMundo (parlante)");
                    }
                    acts.push(UiAct::CloseModal);
                }
                Modal::GoalPicker(_) => acts.push(UiAct::PickGoal("trip")),
                Modal::Amount(_) => acts.push(UiAct::AmountOk),
                Modal::Business(b) => match b.phase {
                    BizPhase::Setup => {
                        if b.message.is_some() {
                            self.note(s, "no alcanza para abrir el puesto");
                            acts.push(UiAct::Biz(BizAct::Close));
                        } else {
                            acts.push(UiAct::Biz(BizAct::Price(2_000)));
                            acts.push(UiAct::Biz(BizAct::Start));
                        }
                    }
                    BizPhase::Selling => {}
                    BizPhase::Result => {
                        let sold = b.sold;
                        self.note(s, &format!("feria terminada: {sold} vendidas"));
                        acts.push(UiAct::Biz(BizAct::Finish));
                    }
                },
                Modal::Minigame(g) => {
                    self.cool -= dt;
                    if self.cool > 0.0 {
                        return acts;
                    }
                    self.cool = 0.35;
                    if g.done {
                        let (served, tips) = (g.served, g.tips);
                        self.note(s, &format!("turno café: {served} atendidos, propinas {tips}"));
                        acts.push(UiAct::Mini(MiniAct::Finish));
                    } else if let Some(o) = g.orders.first() {
                        if g.tray.len() < o.items.len() {
                            acts.push(UiAct::Mini(MiniAct::Add(o.items[g.tray.len()])));
                        } else {
                            acts.push(UiAct::Mini(MiniAct::Serve));
                        }
                    }
                }
                Modal::Reflection => {
                    self.note(s, "reflexión final mostrada");
                    acts.push(UiAct::FinishReflection);
                }
                Modal::Pause => acts.push(UiAct::Resume),
                _ => acts.push(UiAct::CloseModal),
            }
            return acts;
        }
        if s.dialog.is_some() {
            acts.push(UiAct::Advance);
            acts.push(UiAct::Advance);
            return acts;
        }
        if let Some(c) = &s.choice {
            if c.age > 0.4 {
                let i = self.pick(&c.opts);
                let label = c.opts.get(i).map(|o| o.label.clone()).unwrap_or_default();
                self.note(s, &format!("elige: {label}"));
                acts.push(UiAct::Choose(i));
            }
            return acts;
        }
        if s.dir.busy() {
            self.stuck += dt;
            if self.stuck > 60.0 {
                self.note(s, &format!("¡director atascado! esperando {:?}", s.dir.waiting));
                self.stuck = 0.0;
            }
            return acts;
        }
        self.stuck = 0.0;
        if s.chars[0].anim.speed > 0.05 || !s.chars[0].path.is_empty() || s.pending.is_some() {
            self.idle += dt;
            if self.idle > 25.0 {
                self.note(s, "caminando demasiado tiempo; reintento");
                s.chars[0].path.clear();
                s.pending = None;
                self.idle = 0.0;
            }
            return acts;
        }
        self.idle += dt;
        if self.idle < 0.6 {
            return acts;
        }
        self.idle = 0.0;
        if s.week > story::LAST_WEEK + 1 {
            if !self.finished {
                self.finished = true;
                self.note(s, "FIN de la prueba: se completaron todas las semanas");
            }
            return acts;
        }
        // the wise player saves for the trip every week, works and sells
        if self.policy.starts_with("wise") && !self.deposited.contains(&s.week) {
            if let Some(i) = s.fin.goals.iter().filter(|g| !g.done).position(|g| g.id == "trip") {
                self.deposited.push(s.week);
                let spare = (s.fin.wallet - 2_000).max(0) + s.fin.savings;
                if s.fin.savings > 0 {
                    let sv = s.fin.savings;
                    s.fin.from_savings(sv);
                }
                if spare > 0 {
                    let id = s.fin.goals.iter().filter(|g| !g.done).nth(i).map(|g| g.id).unwrap_or("trip");
                    let moved = s.fin.to_goal(id, spare);
                    self.note(s, &format!("aparta {moved} para el viaje"));
                }
            } else if s.week >= 2 && !s.fin.goals.iter().any(|g| g.id == "trip") {
                story::create_goal(s, "trip");
            }
        }
        if self.policy.starts_with("wise") {
            if s.flag("job") && !s.flag("worked_today") && !s.flag("sold_today") && self.tries(s.week, "work") < 1 {
                self.attempts.push((s.week, "work"));
                s.objective = Some("Trabajar en Café Aroma (Vale)".into());
                self.talked.retain(|t| !(t.0 == s.week && t.1 == "vale"));
            } else if s.biz.active && !s.flag("sold_today") && self.tries(s.week, "sell") < 1 && s.flag("worked_today") {
                self.attempts.push((s.week, "sell"));
                s.objective = Some("Ve al puesto de la feria".into());
            }
        }
        // decide the next target from the objective
        let obj = s.objective.clone().unwrap_or_default().to_lowercase();
        let week = s.week;
        let already = |b: &Bot, w: &'static str| b.talked.contains(&(week, w));
        let (loc, target): (Loc, Result<&'static str, Who>) = if obj.contains("mamá") && !already(self, "mama") {
            (Loc::Home, Err(Who::Mama))
        } else if obj.contains("tomás") && !already(self, "tomas") {
            (Loc::Plaza, Err(Who::Tomas))
        } else if (obj.contains("café aroma") || obj.contains("vale")) && !already(self, "vale") {
            (Loc::Plaza, Err(Who::Vale))
        } else if obj.contains("puesto de la feria") && !s.flag("sold_today") && self.tries(week, "stall") < 3 {
            (Loc::Plaza, Ok("stall"))
        } else if obj.contains("plaza") && s.loc_id() != Loc::Plaza && self.tries(week, "plaza_visit") < 1 {
            self.attempts.push((week, "plaza_visit"));
            (Loc::Plaza, Ok("fountain"))
        } else if obj.contains("tecnomundo") && !self.bought && self.tries(week, "store") < 3 {
            (Loc::Plaza, Ok("store"))
        } else if obj.contains("computador") && !self.done_laptop {
            self.done_laptop = true;
            (Loc::Bedroom, Ok("laptop"))
        } else {
            (Loc::Bedroom, Ok("bed"))
        };
        self.target_desc = format!("{loc:?} {target:?}");
        if s.loc_id() != loc {
            if let Some(d) = door_to(s.loc_id(), loc) {
                self.go_interact(s, d);
            }
            return acts;
        }
        match target {
            Ok(id) => {
                if id == "fountain" {
                    // just arriving at the plaza satisfies the objective
                    return acts;
                }
                self.go_interact(s, id);
            }
            Err(who) => {
                if let Some(i) = s.char_idx(who) {
                    if s.char_loc[i] != s.loc_id() {
                        self.note(s, &format!("{} no está aquí", who.name()));
                        self.talked.push((week, who.id()));
                        return acts;
                    }
                    let p = s.chars[0].anim.pos;
                    let cp = s.chars[i].anim.pos;
                    if (cp - p).length() < 1.4 {
                        self.note(s, &format!("habla con {}", who.name()));
                        self.talked.push((week, who.id()));
                        interact_char(s, i);
                    } else {
                        let dir = (p - cp).normalize_or(Vec3::Z);
                        let t = s.loc().collide(cp + dir * 1.0, 0.24);
                        let path = s.loc().path(p, t, 0.25);
                        s.chars[0].walk_path(path, None);
                    }
                }
            }
        }
        acts
    }

    fn go_interact(&mut self, s: &mut State, id: &'static str) {
        let Some(i) = s.loc().interact.iter().position(|it| it.id == id) else {
            self.note(s, &format!("no existe el interactuable {id}"));
            return;
        };
        let stand = s.loc().interact[i].stand;
        let p = s.chars[0].anim.pos;
        if (stand - p).length() < 0.5 {
            self.note(s, &format!("interactúa: {id}"));
            self.attempts.push((s.week, id));
            interact_object(s, i);
        } else {
            let path = s.loc().path(p, stand, 0.25);
            s.chars[0].anim.stand();
            s.chars[0].walk_path(path, None);
            s.pending = Some(PendingInteract::Object(i));
            self.attempts.push((s.week, id));
        }
    }
}
