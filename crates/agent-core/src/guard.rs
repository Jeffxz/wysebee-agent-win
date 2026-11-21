//! What a program the AI names itself (`program_run`, Run any program) may not be — a
//! **safeguard, not a sandbox**.
//!
//! The program is the file found (`programs::resolve`), checked by its name without the
//! extension, any case: a tool that formats, partitions or encrypts disks, changes how Windows
//! starts, deletes backups or event logs, changes accounts, permissions, services, scheduled
//! tasks, the registry or the firewall, shuts the PC down, wipes files, or installs software —
//! and every shell or script host (`cmd`, PowerShell, WSL, Windows Script Host, `mshta`,
//! `rundll32`, …), which would run any command and so get round this list. An interpreter is
//! refused when given code inline (`python -c`, `node -e`, …): a script file it runs is the
//! person's. A few everyday tools are refused only with the arguments that destroy data
//! (`git clean`, `git reset --hard`, a forced `git push`, `robocopy /MIR`).
//!
//! A program the **person** set up and allowed (`local_run`) is never checked here: they chose
//! it. Nothing here limits what a program does once it runs, or what is typed on the screen
//! (`screen_action`) or run through Developer access's shell (`dev_shell`, the person's other
//! switch): a list cannot be complete, which is why Run any program stays off until the person
//! turns it on.

use std::path::Path;

/// Disks, booting and recovery.
const DISK: &[&str] = &[
    "format", "diskpart", "fsutil", "chkdsk", "chkntfs", "defrag", "mountvol", "bcdedit", "bcdboot", "bootrec", "bootsect", "manage-bde", "cipher", "sdelete", "sdelete64", "vssadmin", "wbadmin", "reagentc", "systemreset", "recoverydrive", "dism", "diskshadow", "mkfs", "wipefs", "dd",
];
/// Shells and script hosts: each would run any command, and so get round this list.
const SHELLS: &[&str] = &[
    "cmd", "command", "powershell", "powershell_ise", "pwsh", "wsl", "wslhost", "bash", "sh", "zsh", "fish", "wscript", "cscript", "mshta", "rundll32", "regsvr32", "msbuild", "installutil", "regasm", "regsvcs", "forfiles", "conhost", "hh", "pcalua", "scriptrunner", "wmic",
];
/// The system's settings: accounts, permissions, services, tasks, the registry, the
/// firewall, policies, logs, protection.
const SYSTEM: &[&str] = &[
    "net", "net1", "netsh", "sc", "schtasks", "at", "reg", "regedit", "regini", "takeown", "icacls", "cacls", "secedit", "auditpol", "wevtutil", "mpcmdrun", "bitsadmin", "certutil", "setx", "msiexec", "winget", "choco", "scoop", "pnputil",
];
/// Ending the session.
const POWER: &[&str] = &["shutdown", "logoff", "tsdiscon"];
/// Deleting files outright.
const WIPE: &[&str] = &["rm", "rmdir", "del", "erase", "rd", "shred", "srm"];
/// Interpreters, and their flags that take code inline.
const INLINE: &[(&[&str], &[&str])] = &[
    (&["python", "python3", "pythonw", "py"], &["-c"]),
    (&["node", "nodejs", "bun", "deno"], &["-e", "-p", "--eval", "--print", "eval"]),
    (&["ruby"], &["-e"]),
    (&["perl"], &["-e", "-E"]),
    (&["php"], &["-r"]),
    (&["rscript"], &["-e"]),
];

/// Why the AI may not run `exe` with `argv` on its own, or None when it may.
pub fn refusal(exe: &Path, argv: &[String]) -> Option<String> {
    // Its file name after the last `\` or `/` (on every platform), without the extension.
    let file = exe.to_string_lossy().rsplit(['\\', '/']).next().unwrap_or_default().to_lowercase();
    let name = file.rsplit_once('.').map_or(file.as_str(), |(stem, _)| stem).to_string();
    // Without a version at its end too: `python3.12` is `python`, `pwsh7` is `pwsh`.
    let base = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    let is = |list: &[&str]| list.contains(&name.as_str()) || list.contains(&base);
    let why = if is(DISK) {
        Some("it works on disks, backups or how Windows starts")
    } else if is(SHELLS) {
        Some("it is a shell or script host, which would run any command")
    } else if is(SYSTEM) {
        Some("it changes Windows' settings, accounts, permissions, services, software or logs")
    } else if is(POWER) {
        Some("it would shut down or sign out of this PC")
    } else if is(WIPE) {
        Some("it deletes files outright")
    } else if INLINE.iter().any(|(names, flags)| is(names) && argv.iter().any(|a| flags.contains(&a.as_str()))) {
        Some("code given inline to an interpreter is not run; run a script file instead")
    } else {
        destroys_data(&name, argv)
    };
    why.map(|why| format!("Wysebee Agent will not run '{name}' when the AI chooses it: {why}. If the person wants it, they can set it up as a program tool in Wysebee and allow it in Wysebee Agent."))
}

/// An everyday tool given the arguments that destroy data.
fn destroys_data(name: &str, argv: &[String]) -> Option<&'static str> {
    let has = |words: &[&str]| argv.iter().any(|a| words.iter().any(|w| a.eq_ignore_ascii_case(w)));
    let first = argv.first().map(|a| a.to_lowercase()).unwrap_or_default();
    match name {
        "git" if first == "clean" || (first == "reset" && has(&["--hard"])) || (first == "push" && (has(&["-f", "--force", "--force-with-lease", "--mirror", "--delete", "-d"]) || argv.iter().any(|a| a.starts_with('+')))) => Some("those git arguments throw work away"),
        "robocopy" if has(&["/mir", "/purge", "/move", "/mov"]) => Some("those robocopy arguments delete or move files"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn says(exe: &str, argv: &[&str]) -> Option<String> {
        refusal(Path::new(exe), &argv.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn dangerous_programs_are_refused_by_name_in_any_case_and_folder() {
        for exe in [r"C:\Windows\System32\format.com", "DISKPART.EXE", "bcdedit.exe", "vssadmin.exe", "cipher.exe", "manage-bde.exe"] {
            assert!(says(exe, &[]).unwrap().contains("disks, backups or how Windows starts"), "{exe}");
        }
        for exe in ["cmd.exe", "PowerShell.exe", "pwsh.exe", "wsl.exe", "wscript.exe", "mshta.exe", "rundll32.exe", r"C:\Program Files\Git\bin\bash.exe"] {
            assert!(says(exe, &[]).unwrap().contains("shell or script host"), "{exe}");
        }
        for exe in ["reg.exe", "schtasks.exe", "net.exe", "icacls.exe", "msiexec.exe", "winget.exe", "wevtutil.exe"] {
            assert!(says(exe, &[]).unwrap().contains("settings, accounts"), "{exe}");
        }
        assert!(says("shutdown.exe", &["/s"]).unwrap().contains("shut down"));
        assert!(says("pwsh7.exe", &[]).unwrap().contains("shell or script host"), "a version at its end");
        assert!(says(r"C:\Program Files\Git\usr\bin\rm.exe", &["-rf", "x"]).unwrap().contains("deletes files outright"));
        let said = says("format.com", &["C:"]).unwrap();
        assert!(said.starts_with("Wysebee Agent will not run 'format' when the AI chooses it: ") && said.ends_with("allow it in Wysebee Agent."), "{said}");
    }

    #[test]
    fn interpreters_run_script_files_but_not_inline_code() {
        assert_eq!(says("python.exe", &["tests/run.py", "-v"]), None);
        assert_eq!(says("node.exe", &["server.js"]), None);
        for (exe, argv) in [("python.exe", vec!["-c", "import os"]), ("py.exe", vec!["-3", "-c", "x"]), ("node.exe", vec!["-e", "x"]), ("node.exe", vec!["--eval", "x"]), ("perl.exe", vec!["-E", "x"]), ("php.exe", vec!["-r", "x"]), ("ruby.exe", vec!["-e", "x"]), ("Rscript.exe", vec!["-e", "x"]), ("deno.exe", vec!["eval", "x"]), ("python3.12.exe", vec!["-c", "x"])] {
            assert!(says(exe, &argv).unwrap().contains("inline"), "{exe} {argv:?}");
        }
    }

    #[test]
    fn everyday_tools_are_refused_only_with_the_arguments_that_destroy_data() {
        for argv in [vec!["status"], vec!["log", "-1"], vec!["reset", "HEAD~1"], vec!["push", "origin", "main"], vec![]] {
            assert_eq!(says("git.exe", &argv), None, "{argv:?}");
        }
        for argv in [vec!["clean", "-fdx"], vec!["reset", "--hard"], vec!["push", "--force"], vec!["push", "-f", "origin"], vec!["push", "origin", "+main"], vec!["push", "--delete", "origin", "x"]] {
            assert!(says("git.exe", &argv).unwrap().contains("throw work away"), "{argv:?}");
        }
        assert_eq!(says("robocopy.exe", &["a", "b", "/E"]), None);
        assert!(says("robocopy.exe", &["a", "b", "/MIR"]).unwrap().contains("delete or move"));
        for exe in ["systeminfo.exe", "ipconfig.exe", "tasklist.exe", "whoami.exe", "notepad.exe", "", "explorer.exe"] {
            assert_eq!(says(exe, &[]), None, "{exe}");
        }
    }
}
