// SPDX-License-Identifier: GPL-3.0-or-later

//! A stand-in for the CodeRabbit CLI, for the extension's end-to-end tests.
//!
//! Never bundled. It reads `scenario.json` beside its own executable, records
//! each argv it was run with in `args.log` there, and answers the way the
//! scenario says — replaying a fixture stream for `review --agent`.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

fn main() {
    let exe = std::env::current_exe().expect("own path");
    let dir: PathBuf = exe.parent().expect("own directory").to_path_buf();
    let scenario: Value = std::fs::read_to_string(dir.join("scenario.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Ok(mut log) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("args.log"))
    {
        let _ = writeln!(log, "{}", serde_json::to_string(&args).unwrap_or_default());
    }
    let get = |key: &str| scenario.get(key).cloned().unwrap_or(Value::Null);
    let read = |key: &str| {
        get(key)
            .as_str()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default()
    };
    let mut out = std::io::stdout();

    let code = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--version"] => {
            println!("{}", get("version").as_str().unwrap_or("0.8.1"));
            0
        }
        ["auth", "status", "--agent"] => {
            print!("{}", read("auth"));
            0
        }
        ["auth", "login", "--agent", ..] => {
            println!(r#"{{"type":"login","status":"browser_opened"}}"#);
            get("loginExit").as_i64().unwrap_or(0) as i32
        }
        ["doctor"] => {
            println!("CLI runtime ... ok");
            println!("Service reachability ... ok");
            0
        }
        ["review", "--agent", ..] => {
            let delay = get("delayMs").as_u64().unwrap_or(0);
            for line in read("review").lines() {
                let _ = writeln!(out, "{line}");
                let _ = out.flush();
                std::thread::sleep(Duration::from_millis(delay));
            }
            let linger = get("sleepMs").as_u64().unwrap_or(0);
            if linger > 0 {
                std::thread::sleep(Duration::from_millis(linger));
                let _ = std::fs::write(dir.join("review-survived"), "the review was not stopped");
            }
            get("exit").as_i64().unwrap_or(0) as i32
        }
        other => {
            eprintln!("fake-coderabbit: unexpected arguments {other:?}");
            2
        }
    };
    std::process::exit(code);
}
