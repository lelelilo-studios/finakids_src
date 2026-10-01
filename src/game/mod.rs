//! Game state, simulation loop, scripted scenes and rendering glue.

pub mod bot;
pub mod business;
pub mod camera;
pub mod config;
pub mod finance;
pub mod hud;
pub mod items;
pub mod minigame;
pub mod save;
pub mod script;
pub mod settings_ui;
pub mod sound;
pub mod story;
pub mod time;

pub use config::GameConfig;

use crate::character::actions as acts;
use crate::character::appearance::Appearance;
use crate::character::build::Quality;
use crate::character::factory::{self, CharFactory};
use crate::character::{Character, HeldProp, SharedCharMeshes};
use crate::gfx::renderer::{FrameScene, Light};
use crate::gfx::{Draw, DrawPass, Gpu, Renderer};
use crate::input::{GameKey, Input};
use crate::math::*;
use crate::ui::{Ui, UiInput, UiKey};
use crate::world::{self, Loc, Location};
use camera::{CamCtl, Shot};
use finance::{Finance, InvestKind, WeekReport};
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use script::*;
use std::collections::HashSet;
use time::Phase;

// ------------------------------------------------------------------ UI-facing state

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhoneApp {
    Home,
    Bank,
    Goals,
    Credit,
    Invest,
    Business,
    Chat,
    Journal,
}

pub struct AmountState {
    pub title: String,
    pub body: String,
    pub min: i64,
    pub max: i64,
    pub step: i64,
    pub value: i64,
    pub confirm: String,
    pub on_ok: Option<Box<dyn FnOnce(&mut State, i64)>>,
    pub cancel: bool,
}

pub struct ShopState {
    pub shop: items::Shop,
    pub selected: Option<usize>,
    pub message: Option<(String, bool)>,
    pub compare: bool,
}

pub struct GoalPick {
    pub on_pick: Option<Box<dyn FnOnce(&mut State, &'static str)>>,
    pub fund_with: i64,
}

pub enum Modal {
    Phone(PhoneApp),
    Shop(ShopState),
    Amount(AmountState),
    GoalPicker(GoalPick),
    Summary(WeekReport),
    Business(business::BizUi),
    Minigame(minigame::CafeGame),
    Reflection,
    Pause,
    Info(String, String),
}

pub struct DialogLine {
    pub who: Option<Who>,
    pub text: String,
    pub think: bool,
    pub reveal: f32,
    pub age: f32,
}

pub struct ChoiceState {
    pub prompt: Option<String>,
    pub opts: Vec<Opt>,
    pub age: f32,
}

pub struct Toast {
    pub icon: &'static str,
    pub title: String,
    pub body: String,
    pub amount: i64,
    pub age: f32,
}

pub struct ChatMsg {
    pub from: &'static str,
    pub text: String,
    pub week: u32,
}

pub struct FloatText {
    pub text: String,
    pub color: [u8; 4],
    pub age: f32,
    pub slot: u32,
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: Vec3,
    pub size: f32,
    pub life: f32,
    pub age: f32,
    pub spin: f32,
}

pub enum UiAct {
    Advance,
    Choose(usize),
    CloseModal,
    OpenPhone(PhoneApp),
    Buy(usize, Option<(u32, f32)>),
    ShopSelect(Option<usize>),
    ShopCompare(bool),
    AmountSet(i64),
    AmountOk,
    TransferToSavings,
    TransferFromSavings,
    GoalDeposit(usize),
    GoalWithdraw(usize),
    GoalNew,
    PickGoal(&'static str),
    Invest(InvestKind),
    Withdraw(usize),
    PayDebtExtra(usize),
    SummaryContinue,
    Biz(business::BizAct),
    Mini(minigame::MiniAct),
    AutoSave(u32),
    StartGame(bool),
    Pause,
    Resume,
    Quit,
    Interact,
    FinishReflection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Playing,
}

pub struct Npc {
    pub char_idx: usize,
    pub waypoints: Vec<Vec3>,
    pub next: usize,
    pub pause: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum PendingInteract {
    Object(usize),
    Char(usize),
}

pub struct State {
    pub cfg: GameConfig,
    pub time: f32,
    pub hour: f32,
    pub hour_target: f32,
    pub week: u32,
    pub phase: Phase,
    pub locs: Vec<Location>,
    pub cur: usize,
    pub chars: Vec<Character>,
    pub char_loc: Vec<Loc>,
    pub cam: CamCtl,
    pub fin: Finance,
    pub dir: Director,
    pub modal: Option<Modal>,
    pub dialog: Option<DialogLine>,
    pub choice: Option<ChoiceState>,
    pub toasts: Vec<Toast>,
    pub objective: Option<String>,
    pub flags: HashSet<String>,
    pub title_card: Option<(String, String, f32)>,
    pub fade: f32,
    pub fade_target: f32,
    pub letterbox: f32,
    pub letterbox_target: f32,
    pub messages: Vec<ChatMsg>,
    pub unread: u32,
    pub near: Option<usize>,
    pub near_char: Option<usize>,
    pub pending: Option<PendingInteract>,
    pub screen: Screen,
    pub biz: business::Business,
    pub particles: Vec<Particle>,
    pub floats: Vec<FloatText>,
    pub shown_wallet: f32,
    pub shown_savings: f32,
    pub last_wallet: i64,
    pub last_savings: i64,
    pub quit: bool,
    pub fps: f32,
    pub debug: bool,
    pub rng: Rng,
    pub npcs: Vec<Npc>,
    /// Characters still being built in the background.
    pub build_queue: Vec<PendingChar>,
    pub build_total: usize,
    pub quality: Quality,
    pub flash: f32,
    pub has_save: bool,
    pub ending_done: bool,
    pub hint: Option<(String, f32)>,
    pub tap_marker: Option<(Vec3, f32)>,
    pub loading: f32,
}

impl State {
    pub fn loc(&self) -> &Location {
        &self.locs[self.cur]
    }
    pub fn loc_id(&self) -> Loc {
        self.locs[self.cur].id
    }
    pub fn loc_mut(&mut self, id: Loc) -> &mut Location {
        let i = self.locs.iter().position(|l| l.id == id).unwrap_or(0);
        &mut self.locs[i]
    }
    pub fn char_idx(&self, w: Who) -> Option<usize> {
        self.chars.iter().position(|c| c.id == w.id())
    }
    pub fn ch(&mut self, w: Who) -> Option<&mut Character> {
        let i = self.char_idx(w)?;
        Some(&mut self.chars[i])
    }
    pub fn player(&self) -> &Character {
        &self.chars[0]
    }
    pub fn flag(&self, f: &str) -> bool {
        self.flags.contains(f)
    }
    pub fn set_flag(&mut self, f: &str) {
        self.flags.insert(f.to_string());
    }
    pub fn present(&self, w: Who) -> bool {
        match self.char_idx(w) {
            Some(i) => self.char_loc[i] == self.loc_id() && self.chars[i].visible,
            None => false,
        }
    }

    pub fn toast(&mut self, icon: &'static str, title: &str, body: &str, amount: i64) {
        self.toasts.push(Toast {
            icon,
            title: title.to_string(),
            body: body.to_string(),
            amount,
            age: 0.0,
        });
    }

    pub fn message(&mut self, from: &'static str, text: &str) {
        self.messages.push(ChatMsg {
            from,
            text: text.to_string(),
            week: self.week,
        });
        self.unread += 1;
        let short: String = text.chars().take(96).collect();
        self.toast("chat", contact_name(from), &short, 0);
    }

    pub fn hint(&mut self, t: &str) {
        self.hint = Some((t.to_string(), 0.0));
    }

    pub fn burst(&mut self, kind: u32, at: Vec3) {
        let cols = match kind {
            0 => vec![Vec3::new(1.0, 0.8, 0.2), Vec3::new(1.0, 0.95, 0.5)],
            1 => vec![Vec3::new(0.4, 0.9, 0.7), Vec3::new(1.0, 0.6, 0.3), Vec3::new(0.7, 0.6, 1.0), Vec3::new(1.0, 0.4, 0.5)],
            _ => vec![Vec3::new(1.0, 1.0, 1.0)],
        };
        let n = if kind == 1 { 70 } else { 24 };
        for i in 0..n {
            let a = self.rng.range(0.0, TAU);
            let up = self.rng.range(1.5, 3.8);
            let sp = self.rng.range(0.4, 1.6);
            self.particles.push(Particle {
                pos: at,
                vel: Vec3::new(a.cos() * sp, up, a.sin() * sp),
                color: cols[i % cols.len()],
                size: if kind == 1 { self.rng.range(0.6, 1.0) } else { self.rng.range(0.5, 0.9) },
                life: self.rng.range(1.2, 2.2),
                age: 0.0,
                spin: self.rng.range(-8.0, 8.0),
            });
        }
    }

    /// Advances time of day to the given phase (animated).
    pub fn set_phase(&mut self, p: Phase) {
        self.phase = p;
        self.hour_target = p.hour();
    }

    pub fn goto(&mut self, loc: Loc, spawn: &str) {
        let Some(li) = self.locs.iter().position(|l| l.id == loc) else {
            return;
        };
        self.cur = li;
        let (p, yaw) = self.locs[li].spawn(spawn);
        self.chars[0].path.clear();
        self.chars[0].anim.teleport(p, yaw);
        self.char_loc[0] = loc;
        let rig = self.locs[li].cam;
        let lim = if self.locs[li].interior {
            let (lo, hi) = self.locs[li].bounds;
            Some((lo.x + 0.35, hi.x - 0.35, -0.85, 0.85))
        } else {
            None
        };
        self.cam.set_limits(lim);
        self.cam.set_area(self.locs[li].cam_area);
        self.cam.reset_to(&rig, p + Vec3::Y * rig.look_height);
        self.pending = None;
    }

    pub fn place(&mut self, w: Who, loc: Loc, spawn: &str) {
        let Some(i) = self.char_idx(w) else {
            return;
        };
        let Some(li) = self.locs.iter().position(|l| l.id == loc) else {
            return;
        };
        let (p, yaw) = self.locs[li].spawn(spawn);
        self.chars[i].anim.teleport(p, yaw);
        self.chars[i].path.clear();
        self.char_loc[i] = loc;
        self.chars[i].visible = true;
    }

    fn target_pos(&self, t: Target) -> Vec3 {
        match t {
            Target::Pos(p) => p,
            Target::Char(w) => self.char_idx(w).map(|i| self.chars[i].anim.pos).unwrap_or(Vec3::ZERO),
            Target::Interact(id) => self.loc().interactable(id).map(|i| i.stand).unwrap_or(Vec3::ZERO),
            Target::Spawn(s) => self.loc().spawn(s).0,
        }
    }

    fn look_pos(&self, t: Target) -> Vec3 {
        match t {
            Target::Char(w) => self.char_idx(w).map(|i| self.chars[i].anim.head_world()).unwrap_or(Vec3::ZERO),
            Target::Interact(id) => self.loc().interactable(id).map(|i| i.pos).unwrap_or(Vec3::ZERO),
            _ => self.target_pos(t) + Vec3::Y * 1.4,
        }
    }

    /// Computes a cinematic shot.
    fn make_shot(&self, k: ShotKind) -> Option<Shot> {
        let head = |w: Who| -> Option<(Vec3, f32)> {
            let i = self.char_idx(w)?;
            Some((self.chars[i].anim.head_world(), self.chars[i].anim.yaw))
        };
        let loc = self.loc();
        let clamp = |p: Vec3| -> Vec3 {
            if loc.interior {
                let (lo, hi) = loc.bounds;
                Vec3::new(p.x.clamp(lo.x + 0.25, hi.x - 0.25), p.y.clamp(0.4, 2.45), p.z.clamp(lo.y + 0.25, hi.y - 0.25))
            } else {
                p
            }
        };
        match k {
            ShotKind::Close(w) => {
                let (h, yaw) = head(w)?;
                let f = Quat::from_rotation_y(yaw) * Vec3::Z;
                let side = Quat::from_rotation_y(yaw) * Vec3::X;
                let pos = clamp(h + f * 1.05 + side * 0.28 + Vec3::Y * 0.04);
                Some(Shot::dolly(pos, pos + (h - pos) * 0.06, h - Vec3::Y * 0.05, 30.0, 5.0).blend(0.6))
            }
            ShotKind::PushIn(w) => {
                let (h, yaw) = head(w)?;
                let f = Quat::from_rotation_y(yaw) * Vec3::Z;
                let side = Quat::from_rotation_y(yaw) * Vec3::X;
                let a = clamp(h + f * 1.8 + side * 0.4 + Vec3::Y * 0.1);
                let b = clamp(h + f * 0.95 + side * 0.22 + Vec3::Y * 0.03);
                Some(Shot::dolly(a, b, h - Vec3::Y * 0.06, 32.0, 6.0).blend(0.8))
            }
            ShotKind::Over(a, b) => {
                let (ha, _) = head(a)?;
                let (hb, _) = head(b)?;
                let dir = (hb - ha).normalize_or(Vec3::Z);
                let side = dir.cross(Vec3::Y).normalize_or(Vec3::X);
                let pos = clamp(ha - dir * 0.75 + side * 0.33 + Vec3::Y * 0.12);
                Some(Shot::dolly(pos, pos + dir * 0.08, hb - Vec3::Y * 0.06, 34.0, 5.0).blend(0.5))
            }
            ShotKind::Two(a, b) => {
                let (ha, _) = head(a)?;
                let (hb, _) = head(b)?;
                let mid = (ha + hb) * 0.5;
                let d = (hb - ha).length().max(0.6);
                let dir = (hb - ha).normalize_or(Vec3::X);
                let mut perp = Vec3::new(-dir.z, 0.0, dir.x);
                let cam = self.cam.cam.pos;
                if perp.dot(cam - mid) < 0.0 {
                    perp = -perp;
                }
                let pos = clamp(mid + perp * (d * 1.35 + 0.9) + Vec3::Y * 0.05);
                Some(Shot::dolly(pos, pos - perp * 0.15, mid - Vec3::Y * 0.15, 38.0, 6.0).blend(0.7))
            }
            ShotKind::Wide => {
                let c = if loc.interior { Vec3::new(0.0, 1.0, 0.0) } else { self.player().anim.pos + Vec3::Y };
                let (a, b) = if loc.interior {
                    (Vec3::new(3.4, 2.6, 4.2), Vec3::new(2.6, 2.3, 3.6))
                } else {
                    (c + Vec3::new(7.0, 4.0, 9.0), c + Vec3::new(5.5, 3.2, 7.5))
                };
                Some(Shot::dolly(a, b, c, 48.0, 8.0).blend(1.0))
            }
            ShotKind::Object(id) => {
                let it = loc.interactable(id)?;
                let p = it.pos;
                let from = self.player().anim.head_world();
                let dir = (p - from).normalize_or(Vec3::Z);
                let pos = clamp(p - dir * 0.9 + Vec3::Y * 0.35);
                Some(Shot::dolly(pos, pos + dir * 0.12, p, 34.0, 4.0).blend(0.7))
            }
            ShotKind::Phone(w) => {
                let (h, yaw) = head(w)?;
                let f = Quat::from_rotation_y(yaw) * Vec3::Z;
                let side = Quat::from_rotation_y(yaw) * Vec3::X;
                let pos = clamp(h + f * 0.92 - side * 0.24 - Vec3::Y * 0.2);
                let target = h - Vec3::Y * 0.05 + f * 0.1;
                Some(Shot::dolly(pos, pos + f * 0.06, target, 36.0, 5.0).blend(0.7))
            }
            ShotKind::Hero(w) => {
                let (h, yaw) = head(w)?;
                let f = Quat::from_rotation_y(yaw) * Vec3::Z;
                let side = Quat::from_rotation_y(yaw) * Vec3::X;
                let pos = clamp(h + f * 1.6 - side * 0.5 - Vec3::Y * 0.75);
                Some(Shot::dolly(pos, pos + f * 0.2, h - Vec3::Y * 0.1, 40.0, 6.0).blend(0.8))
            }
        }
    }

    fn walk_char(&mut self, i: usize, target: Vec3, arrive_yaw: Option<f32>) {
        let from = self.chars[i].anim.pos;
        let path = if self.char_loc[i] == self.loc_id() { self.loc().path(from, target, 0.25) } else { vec![target] };
        self.chars[i].walk_path(path, arrive_yaw);
    }
}

pub fn contact_name(from: &str) -> &'static str {
    match from {
        "abuela" => "Abuela Rosa",
        "mama" => "Mamá",
        "tomas" => "Tomás",
        "vale" => "Vale · Café Aroma",
        "banco" => "Banco Futuro",
        "tecno" => "TecnoMundo",
        "colegio" => "Colegio",
        "desconocido" => "+56 9 **** 1234",
        _ => "Mensaje",
    }
}

// ------------------------------------------------------------------ Game

pub struct PendingChar {
    pub id: &'static str,
    pub app: Appearance,
    pub loc: Loc,
    pub spawn: &'static str,
    pub ticket: u32,
}

/// Build key used by the character factory for a cast member.
fn app_key(id: &str) -> String {
    match id {
        "npc_a" => "random:11".into(),
        "npc_b" => "random:4".into(),
        other => other.to_string(),
    }
}

pub struct Game {
    pub ui: Ui,
    pub s: State,
    shared: SharedCharMeshes,
    factory: CharFactory,
    /// Debug / autostart work deferred until every character is built.
    deferred_start: bool,
    ui_acts: Vec<UiAct>,
    pending_input: UiInput,
    world_tap: Option<Vec2>,
    ui_hover: bool,
    bot: Option<bot::Bot>,
    sound: sound::Sound,
}


impl Game {
    pub fn new(gpu: &Gpu, r: &mut Renderer, cfg: GameConfig) -> Game {
        let t0 = web_time::Instant::now();
        let locs = vec![world::build_bedroom(gpu, r), world::build_home(gpu, r), world::build_plaza(gpu, r)];
        log::info!("world built in {:.2}s", t0.elapsed().as_secs_f32());
        let quality = match cfg.quality.as_deref() {
            Some("high") => Quality::High,
            Some("low") => Quality::Low,
            Some("mobile") => Quality::Mobile,
            _ if gpu.info.is_mobile => Quality::Mobile,
            _ if gpu.info.is_webgl || cfg!(target_arch = "wasm32") => Quality::Low,
            _ => Quality::High,
        };
        // every character is built in the background, the player first
        let mut factory = CharFactory::new();
        let roster: [(&'static str, Appearance, Loc, &'static str); 7] = [
            ("sofia", Appearance::sofia(), Loc::Bedroom, "start"),
            ("mama", Appearance::mama(), Loc::Home, "mom"),
            ("tomas", Appearance::tomas(), Loc::Plaza, "tomas"),
            ("vale", Appearance::vale(), Loc::Plaza, "vale"),
            ("julio", Appearance::don_julio(), Loc::Plaza, "julio"),
            ("npc_a", Appearance::random(11), Loc::Plaza, "fountain"),
            ("npc_b", Appearance::random(4), Loc::Plaza, "fountain"),
        ];
        let build_queue: Vec<PendingChar> = roster
            .into_iter()
            .map(|(id, app, loc, spawn)| {
                let q = if id.starts_with("npc") && quality == Quality::High { Quality::Low } else { quality };
                let ticket = factory.request(&format!("{}:{}", app_key(id), factory::quality_key(q)));
                PendingChar { id, app, loc, spawn, ticket }
            })
            .collect();
        let chars = Vec::new();
        let shared = SharedCharMeshes::new(gpu, r);
        let mut s = State {
            cfg: cfg.clone(),
            time: 0.0,
            hour: 19.0,
            hour_target: 19.0,
            week: 1,
            phase: Phase::Morning,
            locs,
            cur: 0,
            chars,
            char_loc: Vec::new(),
            cam: CamCtl::new(),
            fin: Finance::new(),
            dir: Director::new(),
            modal: None,
            dialog: None,
            choice: None,
            toasts: Vec::new(),
            objective: None,
            flags: HashSet::new(),
            title_card: None,
            fade: 1.0,
            fade_target: 0.0,
            letterbox: 0.0,
            letterbox_target: 0.0,
            messages: Vec::new(),
            unread: 0,
            near: None,
            near_char: None,
            pending: None,
            screen: Screen::Title,
            biz: business::Business::new(),
            particles: Vec::new(),
            floats: Vec::new(),
            shown_wallet: 0.0,
            shown_savings: 0.0,
            last_wallet: 0,
            last_savings: 0,
            quit: false,
            fps: 60.0,
            debug: false,
            rng: Rng::new(7),
            npcs: Vec::new(),
            build_total: build_queue.len(),
            build_queue,
            quality,
            flash: 0.0,
            has_save: false,
            ending_done: false,
            hint: None,
            tap_marker: None,
            loading: 0.0,
        };
        s.has_save = save::exists(&s);
        let ui = Ui::new(r.atlas_size);
        let deferred_start = cfg.autostart || cfg.scene.is_some() || cfg.beat.is_some() || cfg.cam.is_some();
        let mut g = Game {
            ui,
            s,
            shared,
            factory,
            deferred_start,
            ui_acts: Vec::new(),
            pending_input: UiInput::default(),
            world_tap: None,
            ui_hover: false,
            bot: if cfg.bot { Some(bot::Bot::new(&cfg.bot_policy)) } else { None },
            sound: sound::Sound::new(cfg.audio, cfg.audio_trace, cfg.save_dir.as_deref()),
        };
        // native debug starts and screenshots build everything up front
        #[cfg(not(target_arch = "wasm32"))]
        if g.deferred_start {
            while let Some((ticket, m)) = g.factory.wait() {
                g.add_built(gpu, r, ticket, m);
            }
        }
        g.pump_builds(gpu, r);
        g
    }

    /// Surface pixels per logical pixel; keeps the UI legible on dense screens.
    pub fn set_density(&mut self, d: f32) {
        self.ui.density = self.s.cfg.density.unwrap_or(d).max(0.5);
    }

    /// True once the player exists and the title can be shown.
    pub fn ready(&self) -> bool {
        !self.s.chars.is_empty()
    }

    /// True while characters are still being built.
    pub fn is_loading(&self) -> bool {
        !self.s.build_queue.is_empty()
    }

    /// Loading progress in [0, 1].
    pub fn load_progress(&self) -> f32 {
        1.0 - self.s.build_queue.len() as f32 / self.s.build_total.max(1) as f32
    }

    /// Receives finished character builds and uploads them.
    fn pump_builds(&mut self, gpu: &Gpu, r: &mut Renderer) {
        // the synchronous fallback builds at most one per frame
        let mut budget = 4;
        while budget > 0 && !self.s.build_queue.is_empty() {
            budget -= 1;
            let Some((ticket, m)) = self.factory.poll() else { break };
            self.add_built(gpu, r, ticket, m);
        }
        if self.deferred_start && self.s.build_queue.is_empty() && !self.s.chars.is_empty() {
            self.deferred_start = false;
            self.debug_start();
        }
    }

    fn add_built(&mut self, gpu: &Gpu, r: &mut Renderer, ticket: u32, m: crate::character::build::CharacterMeshes) {
        let Some(qi) = self.s.build_queue.iter().position(|p| p.ticket == ticket) else {
            return;
        };
        let PendingChar { id, app, loc, spawn, .. } = self.s.build_queue.remove(qi);
        let li = self.s.locs.iter().position(|l| l.id == loc).unwrap_or(0);
        let (p, yaw) = self.s.locs[li].spawn(spawn);
        let c = Character::from_meshes(gpu, r, id, app, m, p, yaw);
        // the player is always chars[0]
        if id == "sofia" {
            self.s.chars.insert(0, c);
            self.s.char_loc.insert(0, loc);
            for n in &mut self.s.npcs {
                n.char_idx += 1;
            }
            story::setup_title(&mut self.s);
        } else {
            self.s.chars.push(c);
            self.s.char_loc.push(loc);
        }
        if id.starts_with("npc") {
            let idx = self.s.chars.len() - 1;
            let pts = if id == "npc_a" {
                vec![Vec3::new(-9.0, 0.0, 5.0), Vec3::new(9.0, 0.0, 5.0), Vec3::new(10.0, 0.0, -4.5), Vec3::new(-9.0, 0.0, -4.5)]
            } else {
                vec![Vec3::new(12.0, 0.0, 13.8), Vec3::new(-12.0, 0.0, 13.8), Vec3::new(-11.0, 0.0, 6.2), Vec3::new(11.5, 0.0, 6.0)]
            };
            self.s.chars[idx].anim.teleport(pts[0], 0.0);
            self.s.chars[idx].walk_speed = 1.05 + self.s.rng.range(0.0, 0.25);
            self.s.npcs.push(Npc {
                char_idx: idx,
                waypoints: pts,
                next: 1,
                pause: 0.0,
            });
        }
        story::on_char_built(&mut self.s, id);
    }

    fn debug_start(&mut self) {
        let g = self;
        if g.s.cfg.autostart && g.s.cfg.scene.is_none() && g.s.cfg.beat.is_none() && g.s.cfg.cam.is_none() {
            g.start(false);
            return;
        }
        g.start(false);
        g.s.dir = Director::new();
        g.s.dialog = None;
        g.s.choice = None;
        g.s.title_card = None;
        g.s.letterbox_target = 0.0;
        if let Some(b) = g.s.cfg.beat.clone() {
            story::debug_beat(&mut g.s, &b);
        }
        if let Some(sc) = g.s.cfg.scene.clone() {
            if let Some(l) = Loc::from_str(&sc) {
                let spawn = if l == Loc::Plaza { "fountain" } else if l == Loc::Home { "hall" } else { "start" };
                g.s.goto(l, spawn);
            }
        }
        if let Some(h) = g.s.cfg.hour {
            g.s.hour = h;
            g.s.hour_target = h;
        }
        g.s.fade = 0.0;
        g.s.fade_target = 0.0;
    }

    pub fn start(&mut self, continue_save: bool) {
        let s = &mut self.s;
        s.screen = Screen::Playing;
        s.dir = Director::new();
        s.dialog = None;
        s.choice = None;
        s.modal = None;
        s.cam.release(0.0);
        for c in &mut s.chars {
            c.anim.clear_action();
            c.anim.stand();
            c.held = [None, None];
            c.anim.set_expr(crate::character::face::Expr::Neutral);
        }
        if continue_save && save::load(s) {
            s.fade = 1.0;
            s.fade_target = 0.0;
            story::resume(s);
        } else {
            s.fin = Finance::new();
            s.flags.clear();
            s.week = 1;
            s.messages.clear();
            s.unread = 0;
            s.biz = business::Business::new();
            story::new_game(s);
        }
        s.shown_wallet = s.fin.wallet as f32;
        s.shown_savings = s.fin.savings as f32;
        s.last_wallet = s.fin.wallet;
        s.last_savings = s.fin.savings;
    }

    pub fn quit_requested(&self) -> bool {
        self.s.quit
    }

    pub fn save(&mut self) {
        if self.s.screen == Screen::Playing && !self.s.dir.busy() {
            save::save(&self.s);
        }
    }

    /// Silences / restores sound when the app goes to the background.
    pub fn set_audio_paused(&mut self, paused: bool) {
        self.sound.audio.set_paused(paused);
    }

    // ------------------------------------------------------------ update

    pub fn update(&mut self, dt: f32, input: &mut Input, w: u32, h: u32) {
        let dt = dt.min(0.1);
        if !self.ready() {
            self.s.time += dt;
            return;
        }
        self.feed_input(input);
        self.s.time += dt;
        self.s.fps = self.s.fps * 0.95 + (1.0 / dt.max(1e-4)) * 0.05;
        if input.was_pressed(GameKey::Debug) {
            self.s.debug = !self.s.debug;
        }
        let acts = std::mem::take(&mut self.ui_acts);
        for a in acts {
            self.apply(a);
        }
        {
            let s = &mut self.s;
            let dh = wrap24(s.hour_target - s.hour);
            if dh.abs() > 0.01 {
                let step = dh.signum() * (4.0 * dt).min(dh.abs());
                s.hour = (s.hour + step).rem_euclid(24.0);
            }
            s.fade += (s.fade_target - s.fade) * damp_factor(3.5, dt);
            if (s.fade - s.fade_target).abs() < 0.003 {
                s.fade = s.fade_target;
            }
            s.letterbox += (s.letterbox_target - s.letterbox) * damp_factor(4.0, dt);
            s.flash = (s.flash - dt * 2.0).max(0.0);
        }
        if self.s.screen == Screen::Playing {
            self.run_director(dt);
            if let Some(b) = &mut self.bot {
                let acts = b.step(&mut self.s, dt);
                self.ui_acts.extend(acts);
            }
            self.player_control(dt, input, w, h);
        } else {
            story::update_title(&mut self.s, dt);
        }
        let s = &mut self.s;
        npc_update(s, dt);
        let loc_id = s.loc_id();
        for i in 0..s.chars.len() {
            if s.char_loc[i] != loc_id {
                continue;
            }
            s.chars[i].update(dt);
            if i == 0 || s.chars[i].id.starts_with("npc") {
                let p = s.chars[i].anim.pos;
                let np = s.locs[s.cur].collide(p, 0.24);
                s.chars[i].anim.pos = np;
            }
            let evs = s.chars[i].anim.events.clone();
            for e in evs {
                match e {
                    acts::EV_GRAB => {
                        s.chars[i].held[1] = Some(HeldProp::Phone);
                        if i == 0 && s.loc_id() == Loc::Bedroom {
                            s.locs[s.cur].set_prop_visible("phone_table", false);
                        }
                    }
                    acts::EV_RELEASE => {
                        s.chars[i].held[1] = None;
                    }
                    acts::EV_DROP => {
                        s.chars[i].held[1] = None;
                        s.flash = 0.3;
                    }
                    crate::character::anim::EV_STEP => self.sound.step(s.chars[i].anim.pos),
                    _ => {}
                }
            }
        }
        let rig = s.locs[s.cur].cam;
        let focus = s.chars[0].anim.pos + Vec3::Y * rig.look_height;
        s.cam.aspect = w as f32 / h.max(1) as f32;
        s.cam.update(dt, focus, &rig);
        if let Some(c) = s.cfg.cam.clone() {
            debug_cam(s, &c);
        }
        let sky = time::sky_at(s.hour, 0.45);
        let evening = s.hour >= 18.3 || s.hour < 6.5;
        let cur = s.cur;
        s.locs[cur].update_lamps(if evening { 1.0 } else { sky.night }, sky.daylight, dt);
        for t in &mut s.toasts {
            t.age += dt;
        }
        s.toasts.retain(|t| t.age < 5.5);
        while s.toasts.len() > 3 {
            s.toasts.remove(0);
        }
        for f in &mut s.floats {
            f.age += dt;
        }
        s.floats.retain(|f| f.age < 2.2);
        for p in &mut s.particles {
            p.age += dt;
            p.vel.y -= 5.5 * dt;
            p.vel *= 1.0 - 0.8 * dt;
            p.pos += p.vel * dt;
            if p.pos.y < 0.01 {
                p.pos.y = 0.01;
                p.vel = Vec3::ZERO;
            }
        }
        s.particles.retain(|p| p.age < p.life);
        if let Some((_, t)) = &mut s.hint {
            *t += dt;
        }
        if s.hint.as_ref().map(|h| h.1 > 8.0).unwrap_or(false) {
            s.hint = None;
        }
        if let Some((_, t)) = &mut s.tap_marker {
            *t += dt;
        }
        if s.tap_marker.map(|m| m.1 > 0.8).unwrap_or(false) {
            s.tap_marker = None;
        }
        let fw = s.fin.wallet;
        let fs = s.fin.savings;
        if fw != s.last_wallet {
            let d = fw - s.last_wallet;
            s.floats.push(FloatText {
                text: money_signed(d),
                color: if d > 0 { [111, 220, 140, 255] } else { [255, 125, 125, 255] },
                age: 0.0,
                slot: 0,
            });
            s.last_wallet = fw;
        }
        if fs != s.last_savings {
            let d = fs - s.last_savings;
            s.floats.push(FloatText {
                text: money_signed(d),
                color: if d > 0 { [111, 220, 140, 255] } else { [255, 125, 125, 255] },
                age: 0.0,
                slot: 1,
            });
            s.last_savings = fs;
        }
        s.shown_wallet += (fw as f32 - s.shown_wallet) * damp_factor(6.0, dt);
        s.shown_savings += (fs as f32 - s.shown_savings) * damp_factor(6.0, dt);
        if let Some(d) = &mut s.dialog {
            d.age += dt;
            d.reveal += dt * 48.0;
        }
        if let Some(c) = &mut s.choice {
            c.age += dt;
        }
        if let Some((_, _, t)) = &mut s.title_card {
            *t += dt;
        }
        if let Some(Modal::Minigame(m)) = &mut s.modal {
            m.update(dt);
        }
        if let Some(Modal::Business(b)) = &mut s.modal {
            b.update(dt, &mut s.biz, &mut s.rng);
        }
        if input.was_pressed(GameKey::Back) && s.screen == Screen::Playing {
            if s.modal.is_none() && s.dialog.is_none() && s.choice.is_none() {
                s.modal = Some(Modal::Pause);
            } else if matches!(s.modal, Some(Modal::Pause)) {
                s.modal = None;
            }
        }
        if s.cfg.autoplay {
            if let Some(d) = &s.dialog {
                if d.age > 1.4 {
                    self.ui_acts.push(UiAct::Advance);
                    self.ui_acts.push(UiAct::Advance);
                }
            }
            if let Some(c) = &s.choice {
                if c.age > 1.2 {
                    let i = c.opts.iter().position(|o| o.enabled).unwrap_or(0);
                    self.ui_acts.push(UiAct::Choose(i));
                }
            }
            match &s.modal {
                Some(Modal::Summary(_)) | Some(Modal::Phone(_)) | Some(Modal::Info(..)) => self.ui_acts.push(UiAct::CloseModal),
                Some(Modal::Amount(_)) => self.ui_acts.push(UiAct::AmountOk),
                _ => {}
            }
        }
        if s.cfg.cam.is_some() && s.dialog.is_some() {
            // screenshot runs: auto advance dialogue
            if let Some(d) = &mut s.dialog {
                d.reveal = 1000.0;
            }
        }
        // sound follows the state: effects, music mood, ambience
        let clicks = std::mem::take(&mut self.ui.clicks);
        let cam = self.s.cam.cam;
        self.sound.update(&self.s, &cam, clicks, dt);
    }

    fn feed_input(&mut self, input: &Input) {
        let mut keys = Vec::new();
        for k in &input.pressed {
            match k {
                GameKey::Confirm => keys.push(UiKey::Confirm),
                GameKey::Interact => keys.push(UiKey::Interact),
                GameKey::Back => keys.push(UiKey::Back),
                GameKey::Up => keys.push(UiKey::Up),
                GameKey::Down => keys.push(UiKey::Down),
                GameKey::Left => keys.push(UiKey::Left),
                GameKey::Right => keys.push(UiKey::Right),
                GameKey::Phone => keys.push(UiKey::Phone),
                GameKey::Num(n) => keys.push(UiKey::Num(*n)),
                _ => {}
            }
        }
        self.pending_input = UiInput {
            pointer: input.pointer,
            down: input.pointer_down,
            pressed: input.pointer_pressed,
            released: input.pointer_released,
            drag_dist: input.drag_dist,
            drag_delta: input.drag_delta,
            keys,
            scroll: input.wheel,
            touch: input.is_touch,
        };
    }

    fn apply(&mut self, a: UiAct) {
        if let UiAct::StartGame(cont) = a {
            self.start(cont);
            return;
        }
        let s = &mut self.s;
        match a {
            UiAct::Advance => {
                if let Some(d) = &mut s.dialog {
                    let n = d.text.chars().count() as f32;
                    if d.reveal < n {
                        d.reveal = n + 1.0;
                    } else {
                        s.dialog = None;
                        for c in &mut s.chars {
                            c.anim.talking = false;
                        }
                    }
                }
            }
            UiAct::Choose(i) => {
                if let Some(mut c) = s.choice.take() {
                    if i < c.opts.len() && c.opts[i].enabled {
                        let o = c.opts.remove(i);
                        s.dir.run(o.then);
                    } else {
                        s.choice = Some(c);
                    }
                }
            }
            UiAct::CloseModal => {
                if matches!(s.modal, Some(Modal::Phone(_))) && s.chars[0].held[1] == Some(HeldProp::Phone) && !s.dir.busy() {
                    let k = s.chars[0].k();
                    s.chars[0].anim.play(acts::put_away_phone(k));
                }
                s.modal = None;
            }
            UiAct::OpenPhone(app) => {
                if app == PhoneApp::Chat {
                    s.unread = 0;
                }
                let opening = !matches!(s.modal, Some(Modal::Phone(_)));
                s.modal = Some(Modal::Phone(app));
                if opening && !s.dir.busy() && s.chars[0].held[1].is_none() && !s.chars[0].anim.is_seated() {
                    let k = s.chars[0].k();
                    s.chars[0].anim.play(acts::phone_from_pocket(k));
                }
            }
            UiAct::Buy(idx, credit) => story::buy(s, idx, credit),
            UiAct::ShopSelect(sel) => {
                if let Some(Modal::Shop(st)) = &mut s.modal {
                    st.selected = sel;
                    st.message = None;
                }
            }
            UiAct::ShopCompare(v) => {
                if let Some(Modal::Shop(st)) = &mut s.modal {
                    st.compare = v;
                }
                if v {
                    s.set_flag("compared_prices");
                }
            }
            UiAct::AmountSet(v) => {
                if let Some(Modal::Amount(a)) = &mut s.modal {
                    a.value = v.clamp(a.min, a.max);
                }
            }
            UiAct::AmountOk => {
                if let Some(Modal::Amount(mut a)) = s.modal.take() {
                    if let Some(f) = a.on_ok.take() {
                        f(s, a.value);
                    }
                }
            }
            UiAct::TransferToSavings => {
                let max = s.fin.wallet.max(0);
                s.modal = Some(Modal::Amount(AmountState {
                    title: "Mover a ahorro".into(),
                    body: "Tu cuenta de ahorro paga 0,5% de interés por semana. El dinero sigue siendo tuyo y puedes sacarlo cuando lo necesites.".into(),
                    min: 0,
                    max,
                    step: 500,
                    value: (max / 2 / 500) * 500,
                    confirm: "Ahorrar".into(),
                    on_ok: Some(Box::new(|s: &mut State, v| {
                        if v > 0 {
                            s.fin.to_savings(v);
                            s.set_flag("saved_manually");
                        }
                        s.modal = Some(Modal::Phone(PhoneApp::Bank));
                    })),
                    cancel: true,
                }));
            }
            UiAct::TransferFromSavings => {
                let max = s.fin.savings;
                s.modal = Some(Modal::Amount(AmountState {
                    title: "Sacar del ahorro".into(),
                    body: "Pasar dinero del ahorro a tu billetera para poder gastarlo.".into(),
                    min: 0,
                    max,
                    step: 500,
                    value: 0,
                    confirm: "Retirar".into(),
                    on_ok: Some(Box::new(|s: &mut State, v| {
                        if v > 0 {
                            s.fin.from_savings(v);
                        }
                        s.modal = Some(Modal::Phone(PhoneApp::Bank));
                    })),
                    cancel: true,
                }));
            }
            UiAct::GoalDeposit(i) => {
                if let Some(g) = s.fin.goals.iter().filter(|g| !g.done).nth(i) {
                    let id = g.id;
                    let name = g.name.clone();
                    let rest = g.target - g.saved;
                    let max = s.fin.wallet.min(rest).max(0);
                    s.modal = Some(Modal::Amount(AmountState {
                        title: format!("Apartar para: {name}"),
                        body: format!("Te faltan {}. El dinero apartado no se gasta por accidente.", money(rest)),
                        min: 0,
                        max,
                        step: 500,
                        value: (max / 2 / 500) * 500,
                        confirm: "Apartar".into(),
                        on_ok: Some(Box::new(move |s: &mut State, v| {
                            if v > 0 {
                                s.fin.to_goal(id, v);
                                story::check_goals(s);
                            }
                            if s.modal.is_none() {
                                s.modal = Some(Modal::Phone(PhoneApp::Goals));
                            }
                        })),
                        cancel: true,
                    }));
                }
            }
            UiAct::GoalWithdraw(i) => {
                if let Some(g) = s.fin.goals.iter().filter(|g| !g.done).nth(i) {
                    let id = g.id;
                    let max = g.saved;
                    let title = format!("Sacar de: {}", g.name);
                    s.modal = Some(Modal::Amount(AmountState {
                        title,
                        body: "Sacar dinero de una meta la retrasa.".into(),
                        min: 0,
                        max,
                        step: 500,
                        value: 0,
                        confirm: "Sacar".into(),
                        on_ok: Some(Box::new(move |s: &mut State, v| {
                            if v > 0 {
                                s.fin.from_goal(id, v);
                                s.set_flag("raided_goal");
                            }
                            s.modal = Some(Modal::Phone(PhoneApp::Goals));
                        })),
                        cancel: true,
                    }));
                }
            }
            UiAct::GoalNew => {
                s.modal = Some(Modal::GoalPicker(GoalPick {
                    on_pick: Some(Box::new(|s: &mut State, _id| {
                        s.modal = Some(Modal::Phone(PhoneApp::Goals));
                    })),
                    fund_with: 0,
                }));
            }
            UiAct::PickGoal(id) => {
                if let Some(Modal::GoalPicker(mut gp)) = s.modal.take() {
                    story::create_goal(s, id);
                    if gp.fund_with > 0 {
                        let moved = s.fin.to_goal(id, gp.fund_with);
                        if moved > 0 {
                            s.toast("goal", "Meta creada", &format!("Apartaste {} para tu meta", money(moved)), 0);
                        }
                    }
                    if let Some(f) = gp.on_pick.take() {
                        f(s, id);
                    }
                    story::check_goals(s);
                }
            }
            UiAct::Invest(kind) => story::open_invest(s, kind),
            UiAct::Withdraw(i) => {
                if let Some(g) = s.fin.withdraw_investment(i) {
                    let t = if g >= 0 { "Ganancia" } else { "Pérdida" };
                    s.toast("chart", "Inversión rescatada", &format!("{t}: {}", money_signed(g)), 0);
                    story::on_withdraw(s, g);
                } else {
                    s.toast("lock", "Aún no disponible", "El depósito a plazo está bloqueado hasta su vencimiento.", 0);
                }
            }
            UiAct::PayDebtExtra(i) => story::prepay_debt(s, i),
            UiAct::SummaryContinue => s.modal = None,
            UiAct::Biz(b) => business::apply(s, b),
            UiAct::Mini(m) => minigame::apply(s, m),
            UiAct::AutoSave(p) => {
                s.fin.auto_save_pct = p;
                if p > 0 {
                    s.toast("piggy", "Regla de ahorro", &format!("Cada semana se ahorrará el {p}% de tu mesada."), 0);
                }
            }
            UiAct::Pause => {
                if s.modal.is_none() && s.screen == Screen::Playing && !s.dir.busy() {
                    s.modal = Some(Modal::Pause);
                }
            }
            UiAct::Resume => s.modal = None,
            UiAct::Quit => {
                save::save(s);
                s.screen = Screen::Title;
                s.modal = None;
                s.has_save = true;
                story::setup_title(s);
            }
            UiAct::Interact => {
                if let Some(i) = s.near_char {
                    interact_char(s, i);
                } else if let Some(i) = s.near {
                    interact_object(s, i);
                }
            }
            UiAct::FinishReflection => {
                s.modal = None;
                s.ending_done = true;
            }
            UiAct::StartGame(_) => {}
        }
    }

    // ------------------------------------------------------------ director

    fn run_director(&mut self, dt: f32) {
        let s = &mut self.s;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 300 {
                break;
            }
            match s.dir.waiting {
                Waiting::None => {}
                Waiting::Timer(t) => {
                    let t = t - dt;
                    if t > 0.0 {
                        s.dir.waiting = Waiting::Timer(t);
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Dialogue => {
                    if s.dialog.is_some() {
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Choice => {
                    if s.choice.is_some() {
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Walk(w, t) => {
                    let t = t - dt;
                    let done = s.char_idx(w).map(|i| s.chars[i].path.is_empty() && s.chars[i].anim.speed < 0.08).unwrap_or(true);
                    if !done && t > 0.0 {
                        s.dir.waiting = Waiting::Walk(w, t);
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Action(w, t) => {
                    let t = t - dt;
                    let busy = s.char_idx(w).map(|i| s.chars[i].anim.busy()).unwrap_or(false);
                    if busy && t > 0.0 {
                        s.dir.waiting = Waiting::Action(w, t);
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Modal => {
                    if s.modal.is_some() {
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Title(t) => {
                    let t = t - dt;
                    if t > 0.0 {
                        s.dir.waiting = Waiting::Title(t);
                        break;
                    }
                    s.title_card = None;
                    s.dir.waiting = Waiting::None;
                }
                Waiting::Fade(t) => {
                    let t = t - dt;
                    if t > 0.0 {
                        s.dir.waiting = Waiting::Fade(t);
                        break;
                    }
                    s.dir.waiting = Waiting::None;
                }
            }
            let Some(step) = s.dir.queue.pop_front() else {
                break;
            };
            s.dir.cutscene = true;
            exec_step(s, step);
        }
        if !s.dir.busy() && s.dir.cutscene {
            s.dir.cutscene = false;
            s.letterbox_target = 0.0;
            s.cam.release(1.0);
            for c in s.chars.iter_mut() {
                c.anim.talking = false;
            }
            story::after_scene(s);
        }
    }

    // ------------------------------------------------------------ player control

    fn player_control(&mut self, _dt: f32, input: &mut Input, w: u32, h: u32) {
        let world_tap = self.world_tap.take();
        let ui_hover = self.ui_hover;
        let s = &mut self.s;
        let blocked = s.dir.busy() || s.modal.is_some() || s.dialog.is_some() || s.choice.is_some();
        let rig = s.locs[s.cur].cam;
        if s.modal.is_none() {
            if input.was_pressed(GameKey::CamLeft) {
                s.cam.rotate(0.5);
            }
            if input.was_pressed(GameKey::CamRight) {
                s.cam.rotate(-0.5);
            }
            if input.wheel.abs() > 0.0 {
                s.cam.zoom(input.wheel, &rig);
            }
            if input.pinch.abs() > 0.0 {
                s.cam.zoom(input.pinch * 3.0, &rig);
            }
        }
        if blocked {
            s.chars[0].anim.desired_vel = Vec3::ZERO;
            s.near = None;
            s.near_char = None;
            for p in &mut s.locs[s.cur].props {
                p.highlight = 0.0;
            }
            return;
        }
        if (input.pointer_down || input.right_down) && input.drag_dist > self.ui.tap_slop() && !ui_hover {
            s.cam.rotate(-input.drag_delta.x * 0.006);
        }
        let mut mv = Vec2::ZERO;
        if input.is_held(GameKey::Up) {
            mv.y -= 1.0;
        }
        if input.is_held(GameKey::Down) {
            mv.y += 1.0;
        }
        if input.is_held(GameKey::Left) {
            mv.x -= 1.0;
        }
        if input.is_held(GameKey::Right) {
            mv.x += 1.0;
        }
        let yaw = s.cam.yaw;
        let fwd = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let run = input.is_held(GameKey::Run);
        s.chars[0].walk_speed = if run { 2.3 } else { 1.35 };
        if mv.length() > 0.1 {
            let d = (fwd * -mv.y + right * mv.x).normalize();
            s.chars[0].path.clear();
            s.pending = None;
            s.chars[0].anim.stand();
            s.chars[0].anim.desired_vel = d * s.chars[0].walk_speed;
        } else if s.chars[0].path.is_empty() {
            s.chars[0].anim.desired_vel = Vec3::ZERO;
        }

        // nearest interactable
        let p = s.chars[0].anim.pos;
        let mut best: Option<(usize, f32)> = None;
        for (i, it) in s.locs[s.cur].interact.iter().enumerate() {
            if !it.enabled {
                continue;
            }
            let d = Vec2::new(it.stand.x - p.x, it.stand.z - p.z).length().min(Vec2::new(it.pos.x - p.x, it.pos.z - p.z).length());
            if d < it.radius + 0.55 && best.map(|b| d < b.1).unwrap_or(true) {
                best = Some((i, d));
            }
        }
        let loc_id = s.loc_id();
        let mut best_char: Option<(usize, f32)> = None;
        for i in 1..s.chars.len() {
            if s.char_loc[i] != loc_id || !s.chars[i].visible || s.chars[i].id.starts_with("npc") {
                continue;
            }
            let d = (s.chars[i].anim.pos - p).length();
            if d < 1.7 && best_char.map(|b| d < b.1).unwrap_or(true) {
                best_char = Some((i, d));
            }
        }
        s.near = best.map(|b| b.0);
        s.near_char = best_char.map(|b| b.0);
        if let (Some(o), Some(c)) = (best, best_char) {
            if o.1 < c.1 - 0.3 {
                s.near_char = None;
            } else {
                s.near = None;
            }
        }
        let cur = s.cur;
        for pr in s.locs[cur].props.iter_mut() {
            pr.highlight = 0.0;
        }
        if let Some(i) = s.near {
            if let Some(pi) = s.locs[cur].interact[i].prop {
                s.locs[cur].props[pi].highlight = 0.35;
            }
        }
        for c in s.chars.iter_mut() {
            c.highlight = 0.0;
        }
        if let Some(ci) = s.near_char {
            s.chars[ci].highlight = 0.25;
        }
        if input.was_pressed(GameKey::Interact) || input.was_pressed(GameKey::Confirm) {
            if let Some(ci) = s.near_char {
                interact_char(s, ci);
                return;
            } else if let Some(i) = s.near {
                interact_object(s, i);
                return;
            }
        }
        if input.was_pressed(GameKey::Phone) {
            s.modal = Some(Modal::Phone(PhoneApp::Home));
            if s.chars[0].held[1].is_none() && !s.chars[0].anim.is_seated() {
                let k = s.chars[0].k();
                s.chars[0].anim.play(acts::phone_from_pocket(k));
            }
            return;
        }

        // tap / click to move or interact
        if let Some(pp) = world_tap {
            let aspect = w as f32 / h.max(1) as f32;
            let ndc = Vec2::new(pp.x / w as f32 * 2.0 - 1.0, 1.0 - pp.y / h as f32 * 2.0);
            let cam = s.cam.cam;
            let mut pick: Option<(PendingInteract, f32)> = None;
            for i in 1..s.chars.len() {
                if s.char_loc[i] != loc_id || !s.chars[i].visible || s.chars[i].id.starts_with("npc") {
                    continue;
                }
                let c = s.chars[i].anim.pos + Vec3::Y * 1.1;
                if let Some(sp) = cam.project(c, w as f32, h as f32) {
                    let d = (sp - pp).length() / h as f32;
                    if d < 0.09 && pick.map(|p| d < p.1).unwrap_or(true) {
                        pick = Some((PendingInteract::Char(i), d));
                    }
                }
            }
            for (i, it) in s.locs[cur].interact.iter().enumerate() {
                if !it.enabled {
                    continue;
                }
                if let Some(sp) = cam.project(it.pos, w as f32, h as f32) {
                    let d = (sp - pp).length() / h as f32;
                    if d < 0.055 && pick.map(|p| d < p.1).unwrap_or(true) {
                        pick = Some((PendingInteract::Object(i), d));
                    }
                }
            }
            if let Some((pi, _)) = pick {
                let target = match pi {
                    PendingInteract::Char(i) => {
                        let cp = s.chars[i].anim.pos;
                        let dir = (p - cp).normalize_or(Vec3::Z);
                        cp + dir * 1.0
                    }
                    PendingInteract::Object(i) => s.locs[cur].interact[i].stand,
                };
                if (target - p).length() < 0.45 {
                    match pi {
                        PendingInteract::Char(i) => interact_char(s, i),
                        PendingInteract::Object(i) => interact_object(s, i),
                    }
                } else {
                    let path = s.locs[cur].path(p, target, 0.25);
                    s.chars[0].anim.stand();
                    s.chars[0].walk_path(path, None);
                    s.pending = Some(pi);
                    s.tap_marker = Some((target, 0.0));
                }
            } else {
                let (o, d) = cam.ray(ndc, aspect);
                if d.y < -0.01 {
                    let t = -o.y / d.y;
                    let hit = o + d * t;
                    let target = s.locs[cur].collide(hit, 0.24);
                    let path = s.locs[cur].path(p, target, 0.25);
                    s.chars[0].anim.stand();
                    s.chars[0].walk_path(path, None);
                    s.pending = None;
                    s.tap_marker = Some((target, 0.0));
                }
            }
        }
        if let Some(pi) = s.pending {
            if s.chars[0].path.is_empty() && s.chars[0].anim.speed < 0.2 {
                s.pending = None;
                match pi {
                    PendingInteract::Char(i) => interact_char(s, i),
                    PendingInteract::Object(i) => interact_object(s, i),
                }
            }
        }
    }

    // ------------------------------------------------------------ rendering

    pub fn render(&mut self, gpu: &Gpu, r: &mut Renderer, target: &wgpu::TextureView, w: u32, h: u32) {
        // characters arriving from the background builders
        self.pump_builds(gpu, r);
        if !self.ready() {
            self.render_boot(gpu, r, target, w, h);
            return;
        }
        let mut scene = FrameScene::new();
        {
            let s = &mut self.s;
            let sky = time::sky_at(s.hour, if s.loc_id() == Loc::Plaza { 0.5 } else { 0.4 });
            let li = s.cur;
            let interior = s.locs[li].interior;
            {
                let p = &mut scene.params;
                let cam = &s.cam.cam;
                p.cam_pos = cam.pos;
                p.view = cam.view();
                p.proj = cam.proj(w as f32 / h.max(1) as f32);
                p.time = s.time;
                p.sun_dir = sky.sun_dir;
                p.sun_color = sky.sun_color;
                p.sun_disc = sky.sun_disc;
                p.shadow_strength = 1.0;
                p.sky_up = sky.amb_up * s.locs[li].ambient_tint;
                p.sky_down = sky.amb_down;
                p.ambient = if interior { 0.72 } else { 1.0 };
                p.env_spec = if interior { 0.5 } else { 1.0 };
                p.zenith = sky.zenith;
                p.horizon = sky.horizon;
                p.clouds = sky.clouds;
                p.stars = sky.stars;
                p.fog_color = sky.fog;
                p.fog_density = if interior { 0.0 } else { 0.005 };
                p.rim_color = Vec3::new(1.0, 0.88, 0.75) * (0.05 + 0.12 * sky.daylight);
                p.rim_strength = 1.0;
                p.shadow_center = if interior { s.locs[li].shadow_center } else { s.chars[0].anim.pos + Vec3::Y };
                p.shadow_radius = if interior { s.locs[li].shadow_radius } else { 14.0 };
                p.draw_sky = true;
                p.room = if interior {
                    let (lo, hi) = s.locs[li].bounds;
                    Some([lo.x - 0.2, lo.y - 0.2, hi.x + 0.2, hi.y + 0.2])
                } else {
                    None
                };
            }
            let post = &mut scene.post;
            post.exposure = sky.exposure * if interior { 1.12 } else { 1.0 };
            post.tint = sky.grade;
            post.fade = s.fade;
            post.letterbox = s.letterbox;
            post.flash = s.flash;
            if s.modal.is_some() && !matches!(s.modal, Some(Modal::Business(_))) {
                post.vignette = 0.5;
            }
            let mut lights: Vec<Light> = Vec::new();
            s.locs[li].lights(&mut lights);
            for c in &s.chars {
                if let Some((p, g)) = c.phone_light() {
                    lights.push(Light {
                        pos: p,
                        radius: 1.3,
                        color: Vec3::new(0.6, 0.75, 1.0) * g * 0.9,
                    });
                }
            }
            {
                let cam = &s.cam.cam;
                let fwd = cam.forward();
                let right = fwd.cross(Vec3::Y).normalize_or(Vec3::X);
                let fill_pos = cam.pos + right * 0.8 + Vec3::Y * 0.6 - fwd * 0.3;
                let strength = if interior { 0.9 } else { 0.45 } * (0.6 + 0.4 * sky.night) * if s.cam.in_shot() { 1.4 } else { 1.0 };
                lights.push(Light {
                    pos: fill_pos,
                    radius: 9.0,
                    color: Vec3::new(1.0, 0.93, 0.85) * strength,
                });
            }
            scene.params.lights = lights;
            s.locs[li].draw(&mut scene, s.cam.cam.pos, s.time);
            for wd in s.locs[li].windows.clone() {
                world::beam_draw(&s.locs[li], &wd, sky.sun_dir, sky.daylight * (1.0 - sky.night), &mut scene);
            }
            let loc_id = s.loc_id();
            if let Some(lab) = s.cfg.cam.clone().filter(|c| c.starts_with("lab")) {
                render_lab(s, &self.shared, gpu, r, &mut scene, &lab, w, h);
            } else {
                for i in 0..s.chars.len() {
                    if s.char_loc[i] != loc_id {
                        continue;
                    }
                    s.chars[i].draw(gpu, r, &self.shared, &mut scene, true);
                }
            }
            for p in &s.particles {
                let t = p.age / p.life;
                let a = (1.0 - t).min(1.0) * smoothstep(0.0, 0.05, p.age);
                let rot = Quat::from_rotation_y(p.age * p.spin) * Quat::from_rotation_x(p.age * p.spin * 0.7);
                let xf = Mat4::from_scale_rotation_translation(Vec3::splat(p.size), rot, p.pos);
                scene.draws.push(Draw::new(self.shared.bills, xf).tint(p.color.extend(a)).no_shadow());
            }
            if let Some((pos, t)) = s.tap_marker {
                let k = 1.0 - t / 0.8;
                let xf = Mat4::from_scale_rotation_translation(Vec3::new(0.35 + t * 0.4, 1.0, 0.35 + t * 0.4), Quat::IDENTITY, pos + Vec3::Y * 0.01);
                scene.draws.push(Draw::new(self.shared.blob, xf).tint(Vec4::new(1.0, 0.8, 0.5, 0.4 * k)).pass(DrawPass::Additive));
            }
        }
        // UI
        let ui_in = std::mem::take(&mut self.pending_input);
        let released = ui_in.released;
        let drag = ui_in.drag_dist;
        let pointer = ui_in.pointer;
        self.ui.begin(w, h, ui_in, 1.0 / 60.0, self.s.time, self.s.cfg.ui_scale);
        if self.s.cfg.icon {
            hud::app_icon(&mut self.ui);
        } else if !self.s.cfg.no_ui {
            let cam = self.s.cam.cam;
            hud::draw(&mut self.ui, &self.s, &cam, &mut self.ui_acts);
        }
        self.ui_hover = self.ui.pointer_over_ui;
        if released && drag < self.ui.tap_slop() && !self.ui.pointer_over_ui && !self.ui.click_consumed {
            self.world_tap = pointer;
        }
        let batch = self.ui.end();
        r.render(gpu, target, w, h, &scene, batch);
    }
}

impl Game {
    /// Black frame with a progress bar while the player is still being built.
    fn render_boot(&mut self, gpu: &Gpu, r: &mut Renderer, target: &wgpu::TextureView, w: u32, h: u32) {
        let mut scene = FrameScene::new();
        scene.post.fade = 1.0;
        let _ = std::mem::take(&mut self.pending_input);
        self.ui.begin(w, h, UiInput::default(), 1.0 / 60.0, self.s.time, self.s.cfg.ui_scale);
        if !self.s.cfg.no_ui {
            let progress = self.load_progress();
            hud::boot_screen(&mut self.ui, progress, self.s.time);
        }
        let batch = self.ui.end();
        r.render(gpu, target, w, h, &scene, batch);
    }
}

fn wrap24(d: f32) -> f32 {
    let mut d = d % 24.0;
    if d > 12.0 {
        d -= 24.0;
    }
    if d < -12.0 {
        d += 24.0;
    }
    d
}

fn npc_update(s: &mut State, dt: f32) {
    for n in &mut s.npcs {
        let c = &mut s.chars[n.char_idx];
        if c.path.is_empty() {
            n.pause -= dt;
            if n.pause <= 0.0 {
                let t = n.waypoints[n.next];
                c.walk_to(t, None);
                n.next = (n.next + 1) % n.waypoints.len();
                n.pause = 1.5 + (n.next as f32 * 1.7) % 3.0;
            }
        }
    }
}

pub fn interact_char(s: &mut State, i: usize) {
    let who = match s.chars[i].id {
        "mama" => Who::Mama,
        "tomas" => Who::Tomas,
        "vale" => Who::Vale,
        "julio" => Who::Julio,
        _ => return,
    };
    s.chars[0].path.clear();
    s.chars[0].anim.desired_vel = Vec3::ZERO;
    if let Some(steps) = story::talk(s, who) {
        s.dir.run(steps);
    }
}

pub fn interact_object(s: &mut State, i: usize) {
    let id = s.locs[s.cur].interact[i].id;
    s.chars[0].path.clear();
    s.chars[0].anim.desired_vel = Vec3::ZERO;
    let face = s.locs[s.cur].interact[i].pos;
    let d = face - s.chars[0].anim.pos;
    if d.length() > 0.1 {
        s.chars[0].anim.target_yaw = Some(yaw_of(d));
    }
    if let Some(steps) = story::interact(s, id) {
        s.dir.run(steps);
    }
}

fn exec_step(s: &mut State, step: Step) {
    match step {
        Step::Say(w, t, e) => {
            if let Some(i) = s.char_idx(w) {
                if let Some(e) = e {
                    s.chars[i].anim.set_expr(e);
                }
                s.chars[i].anim.talking = true;
                let head = s.chars[i].anim.head_world();
                let loc = s.char_loc[i];
                let mut best: Option<(f32, Vec3)> = None;
                for j in 0..s.chars.len() {
                    if j != i && s.char_loc[j] == loc && !s.chars[j].id.starts_with("npc") {
                        s.chars[j].anim.look_at = Some(head);
                        s.chars[j].anim.talking = false;
                        let d = (s.chars[j].anim.pos - s.chars[i].anim.pos).length();
                        if d < 4.0 && best.map(|b| d < b.0).unwrap_or(true) {
                            best = Some((d, s.chars[j].anim.head_world()));
                        }
                    }
                }
                s.chars[i].anim.look_at = best.map(|b| b.1);
                if !s.chars[i].anim.busy() && s.rng.chance(0.35) && !s.chars[i].anim.is_seated() && s.chars[i].held[1].is_none() {
                    let k = s.chars[i].k();
                    let seed = s.rng.int(100) as u32;
                    s.chars[i].anim.play(acts::talk_gesture(k, seed));
                }
            }
            s.dialog = Some(DialogLine {
                who: Some(w),
                text: t,
                think: false,
                reveal: 0.0,
                age: 0.0,
            });
            s.dir.waiting = Waiting::Dialogue;
        }
        Step::Think(t) => {
            s.dialog = Some(DialogLine {
                who: Some(Who::Sofia),
                text: t,
                think: true,
                reveal: 0.0,
                age: 0.0,
            });
            s.dir.waiting = Waiting::Dialogue;
        }
        Step::Text(t) => {
            s.dialog = Some(DialogLine {
                who: None,
                text: t,
                think: false,
                reveal: 0.0,
                age: 0.0,
            });
            s.dir.waiting = Waiting::Dialogue;
        }
        Step::Choice(prompt, opts) => {
            s.choice = Some(ChoiceState { prompt, opts, age: 0.0 });
            s.dir.waiting = Waiting::Choice;
        }
        Step::Shot(k) => {
            if let Some(sh) = s.make_shot(k) {
                s.cam.play(sh);
            }
        }
        Step::Release => s.cam.release(1.0),
        Step::Wait(t) => s.dir.waiting = Waiting::Timer(t),
        Step::Act(w, a, wait) => {
            if let Some(i) = s.char_idx(w) {
                let k = s.chars[i].k();
                let action = match a {
                    Act::Wave => Some(acts::wave(k)),
                    Act::Stretch => Some(acts::stretch(k)),
                    Act::Think => Some(acts::think(k)),
                    Act::ArmsCrossed => Some(acts::arms_crossed(k)),
                    Act::HandsHips => Some(acts::hands_on_hips(k)),
                    Act::Shrug => Some(acts::shrug(k)),
                    Act::Celebrate => Some(acts::celebrate(k)),
                    Act::Sad => Some(acts::sad(k)),
                    Act::Facepalm => Some(acts::facepalm(k)),
                    Act::Nod => Some(acts::nod(k)),
                    Act::Shake => Some(acts::shake_head(k)),
                    Act::Give => Some(acts::give(k)),
                    Act::Receive => Some(acts::receive(k)),
                    Act::PhonePocket => Some(acts::phone_from_pocket(k)),
                    Act::PhoneAway => Some(acts::put_away_phone(k)),
                    Act::PickPhone => {
                        let target = s.loc().interactable("phone").map(|i| i.pos).unwrap_or(Vec3::ZERO);
                        let local = s.chars[i].anim.to_local(target);
                        Some(acts::pick_up_phone(k, local))
                    }
                    Act::DropPhone => Some(acts::drop_phone(k)),
                    Act::Yawn => Some(acts::yawn(k)),
                    Act::Surprised => Some(acts::surprised(k)),
                    Act::Scratch => Some(acts::scratch_head(k)),
                    Act::CountMoney => Some(acts::count_money(k)),
                    Act::TypeLaptop => Some(acts::type_laptop(k, 0.77)),
                    Act::Release => {
                        s.chars[i].anim.release();
                        None
                    }
                    Act::PointAt(who) => {
                        let t = s.char_idx(who).map(|j| s.chars[j].anim.head_world()).unwrap_or(Vec3::ZERO);
                        let local = s.chars[i].anim.to_local(t);
                        Some(acts::point_at(k, local))
                    }
                };
                if let Some(ac) = action {
                    let dur: f32 = ac.phases.iter().map(|p| p.dur).sum();
                    s.chars[i].anim.play(ac);
                    if wait {
                        s.dir.waiting = Waiting::Action(w, dur + 0.2);
                    }
                }
            }
        }
        Step::Walk(w, t, wait) => {
            if let Some(i) = s.char_idx(w) {
                let p = s.target_pos(t);
                s.walk_char(i, p, None);
                if wait {
                    s.dir.waiting = Waiting::Walk(w, 10.0);
                }
            }
        }
        Step::Face(w, t) => {
            let p = s.target_pos(t);
            if let Some(c) = s.ch(w) {
                c.face_towards(p);
            }
        }
        Step::Look(w, t) => {
            let p = t.map(|t| s.look_pos(t));
            if let Some(c) = s.ch(w) {
                c.anim.look_at = p;
            }
        }
        Step::Sit(w, seat) => {
            if let Some(st) = s.loc().seat(seat).cloned() {
                if let Some(c) = s.ch(w) {
                    c.anim.sit(st.pos, st.yaw, st.feet);
                }
            }
        }
        Step::Stand(w) => {
            if let Some(c) = s.ch(w) {
                c.anim.stand();
            }
        }
        Step::Expr(w, e) => {
            if let Some(c) = s.ch(w) {
                c.anim.set_expr(e);
            }
        }
        Step::Toast(icon, title, body, amount) => s.toast(icon, &title, &body, amount),
        Step::Message(from, t) => s.message(from, &t),
        Step::Objective(o) => s.objective = o,
        Step::Do(f) => f(s),
        Step::Dyn(f) => {
            let steps = f(s);
            s.dir.run(steps);
        }
        Step::Title(a, b) => {
            s.title_card = Some((a, b, 0.0));
            s.dir.waiting = Waiting::Title(3.6);
        }
        Step::Fade(out, t) => {
            s.fade_target = if out { 1.0 } else { 0.0 };
            s.dir.waiting = Waiting::Fade(t);
        }
        Step::Goto(l, sp) => s.goto(l, sp),
        Step::Place(w, l, sp) => s.place(w, l, sp),
        Step::Hour(h, _t) => s.hour_target = h,
        Step::Letterbox(on) => s.letterbox_target = if on { 1.0 } else { 0.0 },
        Step::Modal(m) => {
            s.modal = Some(m);
            s.dir.waiting = Waiting::Modal;
        }
        Step::Flag(f) => s.set_flag(f),
        Step::Particles(k, t) => {
            let p = s.look_pos(t);
            s.burst(k, p);
        }
        Step::Held(w, side, p) => {
            if let Some(c) = s.ch(w) {
                c.held[side] = p;
            }
        }
    }
}

fn debug_cam(s: &mut State, name: &str) {
    let c = &s.chars[0];
    let p = c.anim.pos;
    let head = c.anim.head_world();
    let yaw = c.anim.yaw;
    let (pos, target, fov) = match name {
        "face" => {
            let f = Quat::from_rotation_y(yaw) * Vec3::Z;
            (head + f * 0.55 + Vec3::new(0.08, 0.02, 0.0), head - Vec3::Y * 0.02, 30.0)
        }
        "face3q" => {
            let f = Quat::from_rotation_y(yaw + 0.6) * Vec3::Z;
            (head + f * 0.6 + Vec3::new(0.0, 0.03, 0.0), head - Vec3::Y * 0.03, 30.0)
        }
        "body" => {
            let f = Quat::from_rotation_y(yaw) * Vec3::Z;
            (p + f * 2.6 + Vec3::Y * 1.1, p + Vec3::Y * 0.9, 38.0)
        }
        "side" => {
            let f = Quat::from_rotation_y(yaw + 1.2) * Vec3::Z;
            (p + f * 2.4 + Vec3::Y * 1.2, p + Vec3::Y * 0.9, 38.0)
        }
        "back" => {
            let f = Quat::from_rotation_y(yaw + 2.6) * Vec3::Z;
            (p + f * 1.6 + Vec3::Y * 1.5, p + Vec3::Y * 1.3, 38.0)
        }
        "wide" => (Vec3::new(3.5, 3.2, 5.5), Vec3::new(0.0, 0.8, -0.5), 50.0),
        _ => return,
    };
    s.cam.cam.pos = pos;
    s.cam.cam.target = target;
    s.cam.cam.fov = fov;
}

#[allow(clippy::too_many_arguments)]
fn render_lab(s: &mut State, shared: &SharedCharMeshes, gpu: &Gpu, r: &mut Renderer, scene: &mut FrameScene, lab: &str, w: u32, h: u32) {
    let id = if lab.ends_with("mom") {
        "mama"
    } else if lab.ends_with("tomas") {
        "tomas"
    } else if lab.ends_with("vale") {
        "vale"
    } else if lab.ends_with("julio") {
        "julio"
    } else {
        "sofia"
    };
    let idx = s.chars.iter().position(|c| c.id == id).unwrap_or(0);
    let head_mode = lab.starts_with("lab_head");
    scene.draws.clear();
    scene.params.draw_sky = false;
    scene.params.clear_color = Vec3::new(0.18, 0.19, 0.21);
    scene.params.lights.clear();
    scene.params.sun_dir = Vec3::new(0.5, 0.7, 0.6).normalize();
    scene.params.sun_color = Vec3::new(1.0, 0.95, 0.9) * 2.6;
    scene.params.sky_up = Vec3::new(0.5, 0.55, 0.62);
    scene.params.sky_down = Vec3::new(0.25, 0.22, 0.2);
    scene.params.ambient = 1.0;
    scene.params.fog_density = 0.0;
    scene.params.shadow_center = Vec3::new(0.0, 1.0, 0.0);
    scene.params.shadow_radius = 4.0;
    scene.post.exposure = 1.0;
    scene.post.tint = Vec3::ONE;
    scene.post.fade = 0.0;
    scene.post.letterbox = 0.0;
    scene.params.lights.push(Light { pos: Vec3::new(-3.0, 2.5, 2.0), radius: 12.0, color: Vec3::new(0.6, 0.7, 0.9) * 3.0 });
    scene.params.lights.push(Light { pos: Vec3::new(0.0, 2.0, -3.0), radius: 10.0, color: Vec3::new(1.0, 0.9, 0.8) * 3.0 });
    let angles = [0.0f32, 0.7, 1.57, 3.14];
    let spacing = if head_mode { 0.32 } else { 0.9 };
    let base_pos = s.chars[idx].anim.pos;
    let base_model = s.chars[idx].anim.model;
    for (i, a) in angles.iter().enumerate() {
        let x = (i as f32 - 1.5) * spacing;
        let q = Quat::from_rotation_y(*a);
        s.chars[idx].anim.model = Mat4::from_rotation_translation(q, Vec3::new(x, 0.0, 0.0));
        s.chars[idx].anim.hips_world = Vec3::new(x, 0.9, 0.0);
        s.chars[idx].anim.pos = Vec3::new(x, 0.0, 0.0);
        s.chars[idx].draw(gpu, r, shared, scene, true);
    }
    s.chars[idx].anim.pos = base_pos;
    s.chars[idx].anim.model = base_model;
    let k = s.chars[idx].k();
    let (pos, target, fov) = if head_mode {
        (Vec3::new(0.0, 1.57 * k, 1.35), Vec3::new(0.0, 1.55 * k, 0.0), 30.0)
    } else {
        (Vec3::new(0.0, 1.0, 4.6), Vec3::new(0.0, 0.85, 0.0), 32.0)
    };
    scene.params.cam_pos = pos;
    let cam = crate::gfx::Camera { pos, target, fov, ..Default::default() };
    scene.params.view = cam.view();
    scene.params.proj = cam.proj(w as f32 / h.max(1) as f32);
}
