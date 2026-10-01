//! Platform-neutral input state collected from winit events.

use glam::Vec2;
use std::collections::HashSet;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameKey {
    Up,
    Down,
    Left,
    Right,
    Run,
    Interact,
    Confirm,
    Phone,
    Back,
    CamLeft,
    CamRight,
    Num(u8),
    Tab,
    Debug,
    Fullscreen,
}

#[derive(Default)]
pub struct Input {
    pub held: HashSet<GameKey>,
    pub pressed: Vec<GameKey>,
    pub pointer: Option<Vec2>,
    pub pointer_down: bool,
    pub pointer_pressed: bool,
    pub pointer_released: bool,
    pub press_pos: Option<Vec2>,
    pub drag_dist: f32,
    pub drag_delta: Vec2,
    pub right_down: bool,
    pub wheel: f32,
    pub is_touch: bool,
    touch_id: Option<u64>,
    second_touch: Option<(u64, Vec2)>,
    pub pinch: f32,
    last_pinch_dist: Option<f32>,
    /// Surface pixels per window pixel (the web canvas may render below device resolution).
    pub coord_scale: f32,
}

fn map_key(code: KeyCode) -> Option<GameKey> {
    Some(match code {
        KeyCode::KeyW | KeyCode::ArrowUp => GameKey::Up,
        KeyCode::KeyS | KeyCode::ArrowDown => GameKey::Down,
        KeyCode::KeyA | KeyCode::ArrowLeft => GameKey::Left,
        KeyCode::KeyD | KeyCode::ArrowRight => GameKey::Right,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => GameKey::Run,
        KeyCode::KeyE => GameKey::Interact,
        KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter => GameKey::Confirm,
        KeyCode::Tab | KeyCode::KeyP => GameKey::Phone,
        KeyCode::Escape | KeyCode::Backspace => GameKey::Back,
        KeyCode::KeyQ | KeyCode::KeyZ => GameKey::CamLeft,
        KeyCode::KeyR | KeyCode::KeyX => GameKey::CamRight,
        KeyCode::Digit1 | KeyCode::Numpad1 => GameKey::Num(1),
        KeyCode::Digit2 | KeyCode::Numpad2 => GameKey::Num(2),
        KeyCode::Digit3 | KeyCode::Numpad3 => GameKey::Num(3),
        KeyCode::Digit4 | KeyCode::Numpad4 => GameKey::Num(4),
        KeyCode::Digit5 | KeyCode::Numpad5 => GameKey::Num(5),
        KeyCode::F3 => GameKey::Debug,
        KeyCode::F11 | KeyCode::KeyF => GameKey::Fullscreen,
        _ => return None,
    })
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.pointer_pressed = false;
        self.pointer_released = false;
        self.drag_delta = Vec2::ZERO;
        self.wheel = 0.0;
        self.pinch = 0.0;
    }

    pub fn is_held(&self, k: GameKey) -> bool {
        self.held.contains(&k)
    }

    pub fn was_pressed(&self, k: GameKey) -> bool {
        self.pressed.contains(&k)
    }

    fn to_surface(&self, x: f64, y: f64) -> Vec2 {
        let k = if self.coord_scale > 0.0 { self.coord_scale } else { 1.0 };
        Vec2::new(x as f32 * k, y as f32 * k)
    }

    fn pointer_move(&mut self, p: Vec2) {
        if let Some(old) = self.pointer {
            if self.pointer_down || self.right_down {
                let d = p - old;
                self.drag_delta += d;
                self.drag_dist += d.length();
            }
        }
        self.pointer = Some(p);
    }

    fn pointer_button(&mut self, down: bool) {
        if down {
            self.pointer_down = true;
            self.pointer_pressed = true;
            self.press_pos = self.pointer;
            self.drag_dist = 0.0;
        } else if self.pointer_down {
            self.pointer_down = false;
            self.pointer_released = true;
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if let Some(k) = map_key(code) {
                        match event.state {
                            ElementState::Pressed => {
                                if !event.repeat {
                                    self.pressed.push(k);
                                }
                                self.held.insert(k);
                            }
                            ElementState::Released => {
                                self.held.remove(&k);
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.is_touch = false;
                let p = self.to_surface(position.x, position.y);
                self.pointer_move(p);
            }
            WindowEvent::CursorLeft { .. } => {
                if !self.pointer_down {
                    self.pointer = None;
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.is_touch = false;
                let down = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.pointer_button(down),
                    MouseButton::Right | MouseButton::Middle => {
                        self.right_down = down;
                        if down {
                            self.drag_dist = 0.0;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.wheel += match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
                };
            }
            WindowEvent::Touch(t) => self.touch(t),
            WindowEvent::Focused(false) => {
                self.held.clear();
                self.pointer_down = false;
                self.right_down = false;
            }
            _ => {}
        }
    }

    fn touch(&mut self, t: &Touch) {
        self.is_touch = true;
        let p = self.to_surface(t.location.x, t.location.y);
        match t.phase {
            TouchPhase::Started => {
                if self.touch_id.is_none() {
                    self.touch_id = Some(t.id);
                    self.pointer = Some(p);
                    self.pointer_button(true);
                } else if self.second_touch.is_none() {
                    self.second_touch = Some((t.id, p));
                    self.last_pinch_dist = None;
                }
            }
            TouchPhase::Moved => {
                if self.touch_id == Some(t.id) {
                    self.pointer_move(p);
                } else if let Some((id, _)) = self.second_touch {
                    if id == t.id {
                        self.second_touch = Some((id, p));
                    }
                }
                if let (Some(a), Some((_, b))) = (self.pointer, self.second_touch) {
                    let d = a.distance(b);
                    if let Some(last) = self.last_pinch_dist {
                        self.pinch += (d - last) / 200.0;
                    }
                    self.last_pinch_dist = Some(d);
                    self.drag_dist += 100.0; // a pinch is never a tap
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.touch_id == Some(t.id) {
                    self.touch_id = None;
                    self.pointer = Some(p);
                    if t.phase == TouchPhase::Cancelled {
                        self.drag_dist += 100.0;
                    }
                    self.pointer_button(false);
                } else if let Some((id, _)) = self.second_touch {
                    if id == t.id {
                        self.second_touch = None;
                        self.last_pinch_dist = None;
                    }
                }
            }
        }
    }
}
