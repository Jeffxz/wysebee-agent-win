//! This PC's screen for screen control (agent-core's `screen.rs`): the primary display
//! captured (`xcap`), the mouse and keyboard worked (`enigo`, `SendInput`), and **a ring shown
//! where the pointer acts**, so a person watching sees what Wysebee does.
//!
//! The ring is a small window of its own (`pointer.html`): transparent, always on top, never
//! focused (`focusable(false)`, so typing still goes to the app under the pointer), clicks
//! passing through it, out of the taskbar. It is hidden before each screenshot, which draws
//! the pointer in itself.
//!
//! Coordinates are the primary display's physical pixels (the app is per-monitor DPI aware);
//! another display is not captured or reached.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use agent_core::screen::{Action, Button, Frame, Screen};
use enigo::{Axis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

const POINTER: &str = "pointer";
/// The ring's window, in physical pixels.
const RING: u32 = 64;
/// How long the ring stays after an action, unless a screenshot comes first.
const SHOWN: Duration = Duration::from_millis(1200);
/// Between a drag's press, its moves and its release, so apps see a drag.
const DRAG_STEP: Duration = Duration::from_millis(40);

/// The primary display, the mouse and the keyboard. Its window for the ring is made once the
/// app runs ([`Desktop::attach`]).
#[derive(Default)]
pub struct Desktop {
    app: OnceLock<AppHandle>,
    /// Bumped at each showing, so an older showing's timer does not hide a newer one.
    shown: Mutex<u64>,
}

impl Desktop {
    /// Make the ring's window (hidden), once the app is set up.
    pub fn attach(&self, app: &AppHandle) -> tauri::Result<()> {
        let window = WebviewWindowBuilder::new(app, POINTER, WebviewUrl::App("pointer.html".into()))
            .title("Wysebee pointer")
            .inner_size(RING as f64, RING as f64)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .focusable(false)
            .visible(false)
            .on_navigation(|url| matches!(url.scheme(), "tauri") || url.host_str() == Some("tauri.localhost"))
            .build()?;
        window.set_ignore_cursor_events(true)?;
        let _ = self.app.set(app.clone());
        Ok(())
    }

    fn primary() -> Result<xcap::Monitor, String> {
        let all = xcap::Monitor::all().map_err(|e| format!("The screen could not be found: {e}"))?;
        all.into_iter().find(|m| m.is_primary().unwrap_or(false)).ok_or_else(|| "There is no primary display.".to_string())
    }

    fn origin() -> Result<(i32, i32), String> {
        let m = Self::primary()?;
        Ok((m.x().unwrap_or(0), m.y().unwrap_or(0)))
    }

    /// The ring at `x`, `y` (the display's pixels) for a moment.
    fn ring(&self, x: i32, y: i32) {
        let Some(window) = self.app.get().and_then(|a| a.get_webview_window(POINTER)) else { return };
        let (ox, oy) = Self::origin().unwrap_or((0, 0));
        let half = (RING / 2) as i32;
        let _ = window.set_size(PhysicalSize::new(RING, RING));
        let _ = window.set_position(PhysicalPosition::new(ox + x - half, oy + y - half));
        let _ = window.show();
        let _ = window.eval("window.flash && window.flash()");
        let mine = {
            let mut shown = self.shown.lock().unwrap();
            *shown += 1;
            *shown
        };
        // Hidden after a moment, unless shown again meanwhile (the app manages this Desktop).
        let app = self.app.get().cloned();
        std::thread::spawn(move || {
            std::thread::sleep(SHOWN);
            if let Some(desktop) = app.as_ref().and_then(|a| a.try_state::<std::sync::Arc<Desktop>>()) {
                if *desktop.shown.lock().unwrap() == mine {
                    desktop.hide();
                }
            }
        });
    }

    fn hide(&self) {
        if let Some(window) = self.app.get().and_then(|a| a.get_webview_window(POINTER)) {
            let _ = window.hide();
        }
    }

    fn input() -> Result<Enigo, String> {
        Enigo::new(&Settings::default()).map_err(|e| format!("The mouse and keyboard could not be used: {e}"))
    }
}

fn failed(e: impl std::fmt::Display) -> String {
    format!("The mouse or keyboard could not be worked: {e}")
}

/// A key's name (agent-core's `screen::KEYS`, or one character) as enigo's.
fn key_of(name: &str) -> Key {
    match name {
        "ctrl" => Key::Control,
        "alt" => Key::Alt,
        "shift" => Key::Shift,
        "win" => Key::Meta,
        "enter" => Key::Return,
        "tab" => Key::Tab,
        "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "insert" => Key::Insert,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "capslock" => Key::CapsLock,
        "printscreen" => Key::PrintScr,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        other => Key::Unicode(other.chars().next().unwrap_or(' ')),
    }
}

impl Screen for Desktop {
    fn capture(&self) -> Result<Frame, String> {
        // The ring is not part of the screen: hidden, and the pointer drawn in instead.
        self.hide();
        std::thread::sleep(Duration::from_millis(60));
        let m = Self::primary()?;
        let image = m.capture_image().map_err(|e| format!("The screen could not be captured: {e}"))?;
        let (ox, oy) = (m.x().unwrap_or(0), m.y().unwrap_or(0));
        let cursor = Self::input().ok().and_then(|e| e.location().ok()).map(|(x, y)| (x - ox, y - oy));
        let (width, height) = (image.width(), image.height());
        Ok(Frame { width, height, rgba: image.into_raw(), cursor })
    }

    fn size(&self) -> Result<(u32, u32), String> {
        let m = Self::primary()?;
        Ok((m.width().map_err(failed)?, m.height().map_err(failed)?))
    }

    fn act(&self, action: &Action) -> Result<(), String> {
        let (ox, oy) = Self::origin()?;
        let mut input = Self::input()?;
        let to = |input: &mut Enigo, x: i32, y: i32| input.move_mouse(ox + x, oy + y, Coordinate::Abs).map_err(failed);
        match action {
            Action::Move { x, y } => {
                self.ring(*x, *y);
                to(&mut input, *x, *y)
            }
            Action::Click { x, y, button, count } => {
                self.ring(*x, *y);
                to(&mut input, *x, *y)?;
                let button = match button {
                    Button::Left => enigo::Button::Left,
                    Button::Right => enigo::Button::Right,
                    Button::Middle => enigo::Button::Middle,
                };
                for _ in 0..*count {
                    input.button(button, Direction::Click).map_err(failed)?;
                }
                Ok(())
            }
            Action::Drag { x, y, to_x, to_y } => {
                self.ring(*x, *y);
                to(&mut input, *x, *y)?;
                input.button(enigo::Button::Left, Direction::Press).map_err(failed)?;
                for step in 1..=10 {
                    std::thread::sleep(DRAG_STEP);
                    let (sx, sy) = (x + (to_x - x) * step / 10, y + (to_y - y) * step / 10);
                    to(&mut input, sx, sy)?;
                }
                self.ring(*to_x, *to_y);
                std::thread::sleep(DRAG_STEP);
                input.button(enigo::Button::Left, Direction::Release).map_err(failed)
            }
            Action::Scroll { x, y, dx, dy } => {
                self.ring(*x, *y);
                to(&mut input, *x, *y)?;
                if *dy != 0 {
                    input.scroll(*dy, Axis::Vertical).map_err(failed)?;
                }
                if *dx != 0 {
                    input.scroll(*dx, Axis::Horizontal).map_err(failed)?;
                }
                Ok(())
            }
            Action::Type { text } => input.text(text).map_err(failed),
            Action::Key { keys } => {
                let keys: Vec<Key> = keys.iter().map(|k| key_of(k)).collect();
                for k in &keys {
                    input.key(*k, Direction::Press).map_err(failed)?;
                }
                for k in keys.iter().rev() {
                    input.key(*k, Direction::Release).map_err(failed)?;
                }
                Ok(())
            }
        }
    }
}
