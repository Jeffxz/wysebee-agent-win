//! The tests' stand-in for a program the person allowed: one small binary that does what the
//! script appended to its own file says (`tests/common`'s `script` writes the copies). It is
//! what lets the same tests run on Windows, where a `#!/bin/sh` script is no program.
//!
//! Not shipped, and nothing in the app knows of it. One instruction per line:
//!
//! | line | does |
//! | --- | --- |
//! | `out <text>` / `err <text>` | the text and a new line, to stdout / stderr |
//! | `args <open> <close>` | each argument between `open` and `close`, then a new line |
//! | `args-line` | the arguments joined with spaces, then a new line |
//! | `pwd` | the working folder (its real path) and a new line |
//! | `stdin` | what stdin holds, copied to stdout |
//! | `append <file>\|<text>` | the text and a new line, added to the file |
//! | `pid <file>` | this process's id, written to the file |
//! | `sleep <seconds>` | waits |
//! | `repeat <out\|err> <char> <count>` | that character, that many times |
//! | `exit <code>` | ends with that code |

use std::io::{Read, Write};

/// What comes before the script in a copy's file. Made at run time so the binary's own
/// bytes never hold it whole: the script is after its last appearance.
fn marker() -> Vec<u8> {
    ["\n#WYSEBEE", "-TEST-", "SCRIPT#\n"].concat().into_bytes()
}

fn main() {
    let me = std::fs::read(std::env::current_exe().expect("own path")).expect("own file");
    let marker = marker();
    let at = me.windows(marker.len()).rposition(|w| w == marker.as_slice()).expect("no script appended");
    let script = String::from_utf8_lossy(&me[at + marker.len()..]).to_string();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
    for line in script.lines().filter(|l| !l.trim().is_empty()) {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "out" => writeln!(out, "{rest}").unwrap(),
            "err" => writeln!(err, "{rest}").unwrap(),
            "args" => {
                let (open, close) = rest.split_once(' ').unwrap_or((rest, ""));
                for a in &args {
                    write!(out, "{open}{a}{close}").unwrap();
                }
                writeln!(out).unwrap();
            }
            "args-line" => writeln!(out, "{}", args.join(" ")).unwrap(),
            "pwd" => {
                let here = std::env::current_dir().unwrap();
                writeln!(out, "{}", dunce::canonicalize(&here).unwrap_or(here).display()).unwrap();
            }
            "stdin" => {
                let mut all = vec![];
                std::io::stdin().read_to_end(&mut all).unwrap();
                out.write_all(&all).unwrap();
            }
            "append" => {
                let (file, text) = rest.rsplit_once('|').expect("append <file>|<text>");
                let mut f = std::fs::OpenOptions::new().create(true).append(true).open(file).unwrap();
                writeln!(f, "{text}").unwrap();
            }
            "pid" => std::fs::write(rest, std::process::id().to_string()).unwrap(),
            "sleep" => std::thread::sleep(std::time::Duration::from_secs(rest.parse().unwrap())),
            "repeat" => {
                let parts: Vec<&str> = rest.split(' ').collect();
                let text = parts[1].repeat(parts[2].parse().unwrap());
                if parts[0] == "err" {
                    err.write_all(text.as_bytes()).unwrap();
                } else {
                    out.write_all(text.as_bytes()).unwrap();
                }
            }
            "exit" => {
                out.flush().unwrap();
                std::process::exit(rest.parse().unwrap());
            }
            other => panic!("unknown instruction {other}"),
        }
    }
}
