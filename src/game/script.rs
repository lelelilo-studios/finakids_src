//! Cutscene / story scripting: a queue of steps executed by the director.

use super::State;
use crate::character::face::Expr;
use crate::ui::{rgba, Color};
use crate::world::Loc;
use glam::Vec3;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Sofia,
    Mama,
    Tomas,
    Abuela,
    Julio,
    Vale,
}

impl Who {
    pub fn id(self) -> &'static str {
        match self {
            Who::Sofia => "sofia",
            Who::Mama => "mama",
            Who::Tomas => "tomas",
            Who::Abuela => "abuela",
            Who::Julio => "julio",
            Who::Vale => "vale",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Who::Sofia => "Sofía",
            Who::Mama => "Mamá",
            Who::Tomas => "Tomás",
            Who::Abuela => "Abuela Rosa",
            Who::Julio => "Don Julio",
            Who::Vale => "Vale",
        }
    }
    pub fn color(self) -> Color {
        match self {
            Who::Sofia => rgba(0xb8b2ff, 255),
            Who::Mama => rgba(0x6fd6cf, 255),
            Who::Tomas => rgba(0x7fe0a8, 255),
            Who::Abuela => rgba(0xf2a3b3, 255),
            Who::Julio => rgba(0x8fb8ff, 255),
            Who::Vale => rgba(0xffc98f, 255),
        }
    }
}

pub type Cb = Box<dyn FnOnce(&mut State)>;
pub type DynFn = Box<dyn FnOnce(&State) -> Vec<Step>>;

pub struct Opt {
    pub label: String,
    pub detail: Option<String>,
    pub enabled: bool,
    pub then: Vec<Step>,
}

pub fn opt(label: &str, then: Vec<Step>) -> Opt {
    Opt {
        label: label.to_string(),
        detail: None,
        enabled: true,
        then,
    }
}

pub fn opt_d(label: &str, detail: &str, then: Vec<Step>) -> Opt {
    Opt {
        label: label.to_string(),
        detail: Some(detail.to_string()),
        enabled: true,
        then,
    }
}

pub fn opt_locked(label: &str, detail: &str) -> Opt {
    Opt {
        label: label.to_string(),
        detail: Some(detail.to_string()),
        enabled: false,
        then: vec![],
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Target {
    Pos(Vec3),
    Char(Who),
    Interact(&'static str),
    Spawn(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Wave,
    Stretch,
    Think,
    ArmsCrossed,
    HandsHips,
    Shrug,
    Celebrate,
    Sad,
    Facepalm,
    Nod,
    Shake,
    Give,
    Receive,
    PhonePocket,
    PhoneAway,
    PickPhone,
    DropPhone,
    Yawn,
    Surprised,
    Scratch,
    CountMoney,
    TypeLaptop,
    Release,
    PointAt(Who),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotKind {
    /// Medium close-up of a character.
    Close(Who),
    /// Over the shoulder of `a` looking at `b`.
    Over(Who, Who),
    /// Two-shot framing both characters.
    Two(Who, Who),
    /// Wide establishing shot of the current location.
    Wide,
    /// Slow push-in on a character.
    PushIn(Who),
    /// Look at an interactable object.
    Object(&'static str),
    /// Low-angle hero shot.
    Hero(Who),
    /// From just beyond the held phone, looking at the face.
    Phone(Who),
}

pub enum Step {
    Say(Who, String, Option<Expr>),
    Think(String),
    Text(String),
    Choice(Option<String>, Vec<Opt>),
    Shot(ShotKind),
    Release,
    Wait(f32),
    Act(Who, Act, bool),
    Walk(Who, Target, bool),
    Face(Who, Target),
    Look(Who, Option<Target>),
    Sit(Who, &'static str),
    Stand(Who),
    Expr(Who, Expr),
    Toast(&'static str, String, String, i64),
    Message(&'static str, String),
    Objective(Option<String>),
    Do(Cb),
    Dyn(DynFn),
    Title(String, String),
    Fade(bool, f32),
    Goto(Loc, &'static str),
    Place(Who, Loc, &'static str),
    Hour(f32, f32),
    Letterbox(bool),
    Modal(super::Modal),
    Flag(&'static str),
    Particles(u32, Target),
    Held(Who, usize, Option<crate::character::HeldProp>),
}

pub fn say(w: Who, t: &str) -> Step {
    Step::Say(w, t.to_string(), None)
}
pub fn say_e(w: Who, e: Expr, t: &str) -> Step {
    Step::Say(w, t.to_string(), Some(e))
}
pub fn think(t: &str) -> Step {
    Step::Think(t.to_string())
}
pub fn text(t: &str) -> Step {
    Step::Text(t.to_string())
}
pub fn choice(prompt: &str, opts: Vec<Opt>) -> Step {
    Step::Choice(if prompt.is_empty() { None } else { Some(prompt.to_string()) }, opts)
}
pub fn act(w: Who, a: Act) -> Step {
    Step::Act(w, a, true)
}
pub fn act_nb(w: Who, a: Act) -> Step {
    Step::Act(w, a, false)
}
pub fn walk(w: Who, t: Target) -> Step {
    Step::Walk(w, t, true)
}
pub fn face(w: Who, t: Target) -> Step {
    Step::Face(w, t)
}
pub fn toast(icon: &'static str, title: &str, body: &str, amount: i64) -> Step {
    Step::Toast(icon, title.to_string(), body.to_string(), amount)
}
pub fn msg(from: &'static str, t: &str) -> Step {
    Step::Message(from, t.to_string())
}
pub fn objective(t: &str) -> Step {
    Step::Objective(Some(t.to_string()))
}
pub fn doit(f: impl FnOnce(&mut State) + 'static) -> Step {
    Step::Do(Box::new(f))
}
pub fn dynamic(f: impl FnOnce(&State) -> Vec<Step> + 'static) -> Step {
    Step::Dyn(Box::new(f))
}
pub fn wait(t: f32) -> Step {
    Step::Wait(t)
}
pub fn shot(k: ShotKind) -> Step {
    Step::Shot(k)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Waiting {
    None,
    Timer(f32),
    Dialogue,
    Choice,
    Walk(Who, f32),
    Action(Who, f32),
    Modal,
    Title(f32),
    Fade(f32),
}

pub struct Director {
    pub queue: VecDeque<Step>,
    pub waiting: Waiting,
    pub cutscene: bool,
}

impl Director {
    pub fn new() -> Director {
        Director {
            queue: VecDeque::new(),
            waiting: Waiting::None,
            cutscene: false,
        }
    }

    pub fn busy(&self) -> bool {
        !self.queue.is_empty() || self.waiting != Waiting::None
    }

    pub fn run(&mut self, steps: Vec<Step>) {
        for s in steps.into_iter().rev() {
            self.queue.push_front(s);
        }
    }

    pub fn append(&mut self, steps: Vec<Step>) {
        self.queue.extend(steps);
    }
}

impl Default for Director {
    fn default() -> Self {
        Self::new()
    }
}
