//! Start at sign-in: one value, `Wysebee Agent`, under
//! `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`, holding this exe's
//! path **in quotes** (the install folder, `...\Programs\Wysebee Agent\`, has a space, and
//! an unquoted path in a Run value is one Windows may start a different program for).
//! The installer's "Start Wysebee Agent when I sign in" writes the same value, so the
//! switch in the window shows it and turns it off.
//!
//! Written here rather than by tauri-plugin-autostart, which writes the path unquoted.

use std::path::Path;

use windows_registry::CURRENT_USER;

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// The value's name: the product's (the installer's [Registry] entry uses it too).
pub const VALUE: &str = "Wysebee Agent";

/// What the Run value holds for `exe`: its path in quotes, no arguments.
pub fn command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Whether the app starts at sign-in: the value is there.
pub fn is_enabled() -> bool {
    CURRENT_USER.open(RUN).and_then(|key| key.get_string(VALUE)).is_ok()
}

/// Start at sign-in (this exe, where it is now), or not.
pub fn set(on: bool) -> Result<(), String> {
    if on {
        let exe = std::env::current_exe().map_err(|e| format!("Start at sign-in could not be turned on: {e}"))?;
        return CURRENT_USER.create(RUN).and_then(|key| key.set_string(VALUE, command(&exe))).map_err(|e| format!("Start at sign-in could not be turned on: {e}"));
    }
    if !is_enabled() {
        return Ok(());
    }
    CURRENT_USER.options().write().open(RUN).and_then(|key| key.remove_value(VALUE)).map_err(|e| format!("Start at sign-in could not be turned off: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_run_value_is_the_exe_path_in_quotes() {
        let exe = Path::new(r"C:\Users\a\AppData\Local\Programs\Wysebee Agent\Wysebee Agent.exe");
        assert_eq!(command(exe), r#""C:\Users\a\AppData\Local\Programs\Wysebee Agent\Wysebee Agent.exe""#);
    }
}
