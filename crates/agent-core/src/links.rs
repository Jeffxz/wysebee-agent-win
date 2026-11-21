//! Links for the tests, on every platform: a folder link (a junction on Windows, which needs
//! no privilege) and a file link (a symbolic link; on Windows that needs Developer Mode or an
//! administrator, and a test that needs one fails saying so — `scripts/check.ps1` checks
//! for it first).
//!
//! Shared by the unit tests (`crate::links`) and, through `#[path]`, the integration tests.

use std::path::Path;

/// `at` made a link to the folder `target`.
pub fn link_dir(target: &Path, at: &Path) {
    #[cfg(windows)]
    junction::create(target, at).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, at).unwrap();
}

/// `at` made a link to `target` (a file, or a relative path to one).
pub fn link_file(target: &Path, at: &Path) {
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, at).expect("a file link could not be made: turn on Developer Mode (Settings > System > For developers)");
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, at).unwrap();
}
