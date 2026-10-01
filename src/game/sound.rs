//! Turns game state into sound: watches the state for changes and triggers
//! effects, picks the music mood and the ambience, and positions footsteps.

use super::time::Phase;
use super::{Modal, Screen, State};
use crate::audio::{Amb, Audio, Mood, Place, Sfx};
use crate::gfx::Camera;
use crate::world::Loc;
use glam::Vec3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModalKind {
    None,
    Phone,
    Shop,
    Amount,
    GoalPicker,
    Summary,
    Business,
    Minigame,
    Reflection,
    Pause,
    Info,
}

fn modal_kind(m: &Option<Modal>) -> ModalKind {
    match m {
        None => ModalKind::None,
        Some(Modal::Phone(_)) => ModalKind::Phone,
        Some(Modal::Shop(_)) => ModalKind::Shop,
        Some(Modal::Amount(_)) => ModalKind::Amount,
        Some(Modal::GoalPicker(_)) => ModalKind::GoalPicker,
        Some(Modal::Summary(_)) => ModalKind::Summary,
        Some(Modal::Business(_)) => ModalKind::Business,
        Some(Modal::Minigame(_)) => ModalKind::Minigame,
        Some(Modal::Reflection) => ModalKind::Reflection,
        Some(Modal::Pause) => ModalKind::Pause,
        Some(Modal::Info(..)) => ModalKind::Info,
    }
}

pub struct Sound {
    pub audio: Audio,
    started: bool,
    modal: ModalKind,
    choice: bool,
    loc: Loc,
    screen: Screen,
    unread: u32,
    cafe: (u32, u32, bool),
    sold: u32,
    money: (i64, i64, i64),
    mood_want: Mood,
    mood_t: f32,
    steps: Vec<Vec3>,
}

impl Sound {
    pub fn new(enabled: bool, trace: bool, save_dir: Option<&str>) -> Sound {
        let mut audio = Audio::new(enabled, save_dir);
        audio.trace = trace;
        Sound {
            audio,
            started: false,
            modal: ModalKind::None,
            choice: false,
            loc: Loc::Bedroom,
            screen: Screen::Title,
            unread: 0,
            cafe: (0, 0, false),
            sold: 0,
            money: (0, 0, 0),
            mood_want: Mood::Silent,
            mood_t: 0.0,
            steps: Vec::new(),
        }
    }

    /// A foot touched the ground at `pos` (world space).
    pub fn step(&mut self, pos: Vec3) {
        if self.steps.len() < 8 {
            self.steps.push(pos);
        }
    }

    fn mood(s: &State) -> Mood {
        if s.screen == Screen::Title {
            return Mood::Title;
        }
        match &s.modal {
            Some(Modal::Reflection) => return Mood::Reflect,
            Some(Modal::Minigame(_)) | Some(Modal::Business(_)) => return Mood::Plaza,
            _ => {}
        }
        // overdue installments weigh on everything
        if s.fin.debts.iter().any(|d| d.overdue > 0) {
            return Mood::Tense;
        }
        match (s.loc_id(), s.phase) {
            (_, Phase::Night) => Mood::Night,
            (_, Phase::Evening) => Mood::Evening,
            (Loc::Plaza, _) => Mood::Plaza,
            (_, Phase::Morning) => Mood::Morning,
            (_, Phase::Afternoon) => Mood::Afternoon,
        }
    }

    /// Call once per frame after the simulation step. `clicks` is the number of
    /// UI elements activated since the last call.
    pub fn update(&mut self, s: &State, cam: &Camera, clicks: u32, dt: f32) {
        let modal = modal_kind(&s.modal);
        let choice = s.choice.is_some();
        let loc = s.loc_id();
        let mut ui_sound = false;
        let money = (s.fin.wallet, s.fin.savings, s.fin.goals_saved());

        if self.started {
            // ---- screens and panels
            if self.screen == Screen::Title && s.screen == Screen::Playing {
                self.audio.play(Sfx::Start);
                ui_sound = true;
            }
            if modal != self.modal {
                ui_sound = true;
                match (self.modal, modal) {
                    (_, ModalKind::Summary) => self.audio.play(Sfx::Sleep),
                    (_, ModalKind::None) => self.audio.play(Sfx::Close),
                    _ => self.audio.play(Sfx::Open),
                }
            }
            if choice != self.choice {
                ui_sound = true;
                self.audio.play(if choice { Sfx::Pop } else { Sfx::Select });
            }
            if loc != self.loc && self.screen == Screen::Playing && s.screen == Screen::Playing {
                self.audio.play(Sfx::Door);
            }

            // ---- money and messages
            if s.unread > self.unread {
                self.audio.play(Sfx::Notify);
            }
            // wallet / savings movements (not when a game starts or the week is summarized)
            let quiet = self.screen != s.screen || (modal == ModalKind::Summary && modal != self.modal);
            if money != self.money && s.screen == Screen::Playing && !quiet {
                let dw = money.0 - self.money.0;
                let kept = (money.1 - self.money.1) + (money.2 - self.money.2);
                if dw > 0 && kept < 0 {
                    self.audio.play(Sfx::Pop);
                } else if dw > 0 || (dw < 0 && kept > 0) {
                    // earning and saving both get the bright sound
                    self.audio.play(Sfx::Coin);
                } else if dw < 0 {
                    self.audio.play(Sfx::Spend);
                }
            }
            for t in &s.toasts {
                // toasts created during this update have only aged one step
                if t.age > dt * 1.5 + 1e-4 {
                    continue;
                }
                let sfx = if t.amount > 0 {
                    Sfx::Coin
                } else if t.amount < 0 {
                    Sfx::Spend
                } else {
                    match t.icon {
                        "chat" => continue,
                        // reaching a goal or fixing a problem is worth a flourish
                        "star" | "check" => Sfx::Success,
                        "warning" | "lock" => Sfx::Warn,
                        _ => Sfx::Pop,
                    }
                };
                self.audio.play(sfx);
            }

            // ---- café shift
            if let Some(Modal::Minigame(g)) = &s.modal {
                let fresh = g.feedback.as_ref().map(|f| f.2 <= dt * 1.5 + 1e-4).unwrap_or(false);
                let cur = (g.served, g.failed, fresh);
                if modal == self.modal {
                    if g.served > self.cafe.0 {
                        self.audio.play(Sfx::Bell);
                        ui_sound = true;
                    } else if g.failed > self.cafe.1 {
                        self.audio.play(Sfx::Warn);
                    } else if fresh && !self.cafe.2 && g.feedback.as_ref().map(|f| !f.1).unwrap_or(false) {
                        self.audio.play(Sfx::Wrong);
                        ui_sound = true;
                    }
                }
                self.cafe = cur;
            }
            // ---- market stall
            if let Some(Modal::Business(b)) = &s.modal {
                if modal == self.modal && b.sold > self.sold {
                    self.audio.play(Sfx::Sale);
                }
                self.sold = b.sold;
            }

            // ---- buttons
            if clicks > 0 && !ui_sound {
                self.audio.play(Sfx::Click);
            }

            // ---- footsteps, positioned relative to the camera
            if s.fade < 0.7 {
                let fwd = (cam.target - cam.pos).normalize_or_zero();
                let right = fwd.cross(Vec3::Y).normalize_or_zero();
                let sfx = if loc == Loc::Plaza { Sfx::StepStone } else { Sfx::StepWood };
                for p in &self.steps {
                    let d = *p - cam.pos;
                    let dist = d.length().max(0.5);
                    let gain = (4.0 / dist).min(1.0).powf(1.2) * 0.9;
                    let pan = (d / dist).dot(right) * 0.7;
                    self.audio.play_at(sfx, gain, pan);
                }
            }
        }
        self.steps.clear();
        self.started = true;
        self.modal = modal;
        self.choice = choice;
        self.loc = loc;
        self.screen = s.screen;
        self.unread = s.unread;
        self.money = money;

        // ---- music: wait until the wanted mood is stable so quick scene
        // changes don't restart the score
        let want = Self::mood(s);
        if want != self.mood_want {
            self.mood_want = want;
            self.mood_t = 0.0;
        } else {
            self.mood_t += dt;
        }
        let wait = if matches!(want, Mood::Title | Mood::Reflect) { 0.0 } else { 0.8 };
        if self.mood_t >= wait {
            self.audio.set_music(want);
        }
        // quieter music while reading, deciding or paused
        let duck = if modal == ModalKind::Pause {
            0.45
        } else if choice {
            0.6
        } else if s.dialog.is_some() {
            0.8
        } else {
            1.0
        };
        self.audio.set_duck(duck * (1.0 - 0.5 * s.fade.clamp(0.0, 1.0)));

        // ---- ambience
        let place = if loc == Loc::Plaza { Place::Plaza } else { Place::Indoor };
        self.audio.set_ambience(Amb { place, hour: s.hour });

        self.audio.update(dt);
    }
}
