//! The notification area icon (the system tray): where Wysebee Agent is seen running, opened,
//! paused and quit.
//!
//! The app's own icon in color (`icons/tray.png`, 32 px: it reads on a light or a dark
//! taskbar alike, and Windows scales it). **A left click opens the window**, as Windows'
//! own tray apps do; a right click opens the menu: **Open Wysebee Agent**, **Pause** /
//! **Resume**, **Quit**. English, like the window.

use std::time::Duration;

use agent_core::Agent;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

const ICON: &[u8] = include_bytes!("../icons/tray.png");

pub const OPEN: &str = "open";
pub const PAUSE: &str = "pause";
pub const QUIT: &str = "quit";
const TRAY_ID: &str = "wysebee-agent";

/// The pause item's text, and the icon's tooltip, for what the agent is doing.
pub fn pause_text(paused: bool) -> &'static str {
    if paused {
        "Resume running programs"
    } else {
        "Pause running programs"
    }
}

pub fn tooltip(state: &str, paused: bool, waiting: usize) -> String {
    match (state, paused, waiting) {
        ("paired", true, _) => "Wysebee Agent: paused".into(),
        ("paired", false, 0) => "Wysebee Agent".into(),
        ("paired", false, n) => format!("Wysebee Agent: {n} waiting for you to allow"),
        _ => "Wysebee Agent: not paired".into(),
    }
}

pub fn install(app: &AppHandle, agent: Agent) -> tauri::Result<()> {
    let first = agent.status();
    let open = MenuItem::with_id(app, OPEN, "Open Wysebee Agent", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, PAUSE, pause_text(first.paused), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit Wysebee Agent", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pause, &PredefinedMenuItem::separator(app)?, &quit])?;
    let clicked = agent.clone();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON)?)
        .tooltip(tooltip(first.state, first.paused, first.waiting))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_window(tray.app_handle());
            }
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            OPEN => crate::show_window(app),
            PAUSE => {
                let agent = clicked.clone();
                tauri::async_runtime::spawn(async move {
                    let paused = agent.status().paused;
                    agent.set_paused(!paused).await;
                });
            }
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;

    // The switch is in the window too: keep the menu's wording and the tooltip in step.
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut shown = (first.paused, tooltip(first.state, first.paused, first.waiting));
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let s = agent.status();
            let now = (s.paused, tooltip(s.state, s.paused, s.waiting));
            if now != shown {
                let _ = pause.set_text(pause_text(now.0));
                if let Some(tray) = handle.tray_by_id(TRAY_ID) {
                    let _ = tray.set_tooltip(Some(&now.1));
                }
                shown = now;
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_and_tooltip_say_what_the_agent_is_doing() {
        assert_eq!(pause_text(false), "Pause running programs");
        assert_eq!(pause_text(true), "Resume running programs");
        assert_eq!(tooltip("paired", false, 0), "Wysebee Agent");
        assert_eq!(tooltip("paired", false, 2), "Wysebee Agent: 2 waiting for you to allow");
        assert_eq!(tooltip("paired", true, 2), "Wysebee Agent: paused");
        assert_eq!(tooltip("waiting", false, 0), "Wysebee Agent: not paired");
    }

    #[test]
    fn the_icon_is_a_png_the_notification_area_can_draw() {
        assert!(ICON.starts_with(b"\x89PNG\r\n\x1a\n"));
        // 32 px square: the notification area's size at 200%, scaled down below it.
        assert_eq!(u32::from_be_bytes(ICON[16..20].try_into().unwrap()), 32);
        assert_eq!(u32::from_be_bytes(ICON[20..24].try_into().unwrap()), 32);
    }
}
