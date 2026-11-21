//! Run any program: `program_run`, the program the AI names,
//! runs only while the person has turned on "Let Wysebee run any program on this PC" — off by
//! default, forgotten on unpairing — and then as an allowed program runs: found the same way,
//! an argument list, never a shell, bounded.

mod common;

use std::time::Duration;

use agent_core::agent::{ANY_OFF, PAUSED};
use agent_core::{Agent, Step, Timing};
use common::{call, fake, gone, pair, until, Bench};
use serde_json::json;

#[tokio::test]
async fn nothing_the_ai_names_runs_until_the_person_turns_it_on() {
    let (fake, bench) = (fake().await, Bench::new());
    let mark = bench.dir.path().join("ran");
    bench.script("tool", &format!("append {}|ran\nargs [ ]\npwd", mark.display()));
    let agent = bench.agent(&fake);
    pair(&agent, &fake).await;
    assert!(!agent.status().any_program);
    assert_eq!(fake.last_status()["any_program"], false);
    let refused = call(&agent, &fake, "c1", "program_run", json!({"program": "tool"})).await;
    assert_eq!(refused, json!({"ok": false, "error": ANY_OFF}));
    assert!(!mark.exists(), "nothing ran");

    // Turned on: the cloud learns at once, and the program runs with each argument as one.
    agent.set_any_program(true);
    assert_eq!(agent.step().await, Step::Paired { calls: 0 });
    assert_eq!(fake.last_status()["any_program"], true);
    assert!(agent.status().any_program);
    let nasty = "a b; rm -rf / $(x) | z > w && \"q\"";
    let ran = call(&agent, &fake, "c2", "program_run", json!({"program": " tool ", "args": [nasty, 2, null]})).await;
    let home = dunce::canonicalize(bench.home()).unwrap();
    assert_eq!(ran, json!({"ok": true, "result": {"exit_code": 0, "stdout": format!("[{nasty}][2][null]\n{}\n", home.display()), "stderr": ""}}));
    assert!(mark.exists());
    // By its full path too.
    let full = bench.bin().join(format!("tool{}", common::EXE)).display().to_string();
    let ran = call(&agent, &fake, "c3", "program_run", json!({"program": full})).await;
    assert_eq!(ran["result"]["exit_code"], 0);

    // What is not a program, or not one here, or not said right, runs nothing.
    let none = call(&agent, &fake, "c4", "program_run", json!({"program": "nope"})).await;
    assert_eq!(none, json!({"ok": false, "error": "There is no program 'nope' on this PC."}));
    for (args, said) in [
        (json!({}), "Name the program to run."),
        (json!({"program": "  "}), "Name the program to run."),
        (json!({"program": "x".repeat(1001)}), "Name the program to run."),
        (json!({"program": "tool", "args": "a b"}), "'args' is a list of at most 30 arguments."),
        (json!({"program": "tool", "args": vec!["x"; 31]}), "'args' is a list of at most 30 arguments."),
    ] {
        let refused = call(&agent, &fake, "c5", "program_run", args.clone()).await;
        assert_eq!(refused, json!({"ok": false, "error": said}), "{args}");
        fake.set(|c| c.results.clear());
    }
    // A dangerous program, even one that is there, is refused by name (guard.rs).
    bench.script("diskpart", "out partitioned");
    let refused = call(&agent, &fake, "c9", "program_run", json!({"program": "diskpart"})).await;
    assert!(refused["error"].as_str().unwrap().starts_with("Wysebee Agent will not run 'diskpart' when the AI chooses it"), "{refused}");
    let plain = bench.bin().join(format!("plain{}", common::EXE));
    std::fs::write(&plain, "not a program").unwrap();
    let broken = call(&agent, &fake, "c6", "program_run", json!({"program": plain.display().to_string()})).await;
    assert!(broken["error"].as_str().unwrap().starts_with(&format!("'{}' could not be started: ", plain.display())), "{broken}");

    // Paused: refused, as every call is.
    agent.set_paused(true).await;
    let paused = call(&agent, &fake, "c7", "program_run", json!({"program": "tool"})).await;
    assert_eq!(paused, json!({"ok": false, "error": PAUSED}));
    agent.set_paused(false).await;

    // Turned off again: refused again.
    agent.set_any_program(false);
    let off = call(&agent, &fake, "c8", "program_run", json!({"program": "tool"})).await;
    assert_eq!(off, json!({"ok": false, "error": ANY_OFF}));

    // On, then unpaired: forgotten, as a new pairing is a new agent.
    agent.set_any_program(true);
    agent.unpair().await;
    assert!(!agent.state().any_program && !agent.status().any_program);
}

/// On Windows a batch file runs through cmd.exe, a shell: never started, and said so.
#[cfg(windows)]
#[tokio::test]
async fn a_batch_file_is_never_run() {
    let (fake, bench) = (fake().await, Bench::new());
    std::fs::write(bench.bin().join("npm.cmd"), "@echo off\r\necho ran\r\n").unwrap();
    let agent = bench.agent(&fake);
    pair(&agent, &fake).await;
    agent.set_any_program(true);
    let refused = call(&agent, &fake, "c1", "program_run", json!({"program": "npm"})).await;
    assert!(refused["error"].as_str().unwrap().starts_with("'npm' is a batch file (npm.cmd), which runs through cmd.exe"), "{refused}");
}

#[tokio::test]
async fn a_program_that_does_not_finish_in_time_is_killed() {
    let (fake, bench) = (fake().await, Bench::new());
    let pid_file = bench.dir.path().join("pid");
    bench.script("slow", &format!("pid {}\nsleep 30", pid_file.display()));
    let agent = Agent::open(bench.options(Some(&fake.url), Timing { time_limit: Duration::from_millis(400), ..Bench::timing() }));
    pair(&agent, &fake).await;
    agent.set_any_program(true);
    let late = call(&agent, &fake, "c1", "program_run", json!({"program": "slow"})).await;
    assert_eq!(late, json!({"ok": false, "error": "'slow' did not finish within 0 seconds and was stopped."}));
    let pid = std::fs::read_to_string(&pid_file).unwrap().trim().to_string();
    until("the program is killed", || gone(&pid)).await;
}
