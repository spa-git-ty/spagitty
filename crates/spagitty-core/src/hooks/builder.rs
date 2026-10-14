// SPDX-License-Identifier: GPL-3.0-or-later

//! Definitions, installed hooks and their records (FEAT-137). A bad definition
//! is an error, never an empty one that Save could silently write over.

use super::blocks::{identifier, validate, Definition, Stage};
use super::generate::{self, MARK};
use crate::shell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, String>;
const MAX_DEFINITION: u64 = 1024 * 1024;
const MAX_RECORD: u64 = 2 * 1024 * 1024;
const MAX_LOG: u64 = 512 * 1024;

pub fn folder(repo: &Path) -> Result<PathBuf> {
    let git = gix::open(repo).map_err(|e| e.to_string())?;
    Ok(crate::compare::common_dir(&git).join("spagitty/hooks"))
}

fn read(file: &Path, limit: u64) -> Result<String> {
    let input = std::fs::File::open(file).map_err(|e| e.to_string())?;
    if input.metadata().map_err(|e| e.to_string())?.len() > limit {
        return Err(format!("{} is too large to read here.", file.display()));
    }
    let mut text = String::new();
    input
        .take(limit)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Ok(text)
}

/// Standard TOML, so a definition edited by the team is read just like one
/// saved by the builder. Unknown fields remain errors rather than being lost.
pub fn encode(definition: &Definition) -> Result<String> {
    validate(definition)?;
    toml::to_string_pretty(definition).map_err(|e| e.to_string())
}

pub fn decode(text: &str) -> Result<Definition> {
    let definition: Definition = toml::from_str(text).map_err(|e| format!("hooks.toml: {e}"))?;
    validate(&definition)?;
    Ok(definition)
}

pub fn load(repo: &Path, personal: &Path) -> Result<Definition> {
    let shared = repo.join(".spagitty/hooks.toml");
    // Personal choices win, but switching back to shared must follow the
    // team's current file rather than a stale personal copy of it.
    if personal.exists() {
        let definition = decode(&read(personal, MAX_DEFINITION)?)?;
        if !definition.shared {
            return Ok(definition);
        }
    }
    if !shared.exists() {
        return Ok(Definition::default());
    }
    let mut definition = decode(&read(&shared, MAX_DEFINITION)?)?;
    definition.shared = true;
    Ok(definition)
}

fn write(file: &Path, text: &str) -> Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = file.with_extension("partial");
    std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, file).map_err(|e| e.to_string())
}

pub fn save(repo: &Path, personal: &Path, definition: &Definition) -> Result<()> {
    let text = encode(definition)?;
    let shared = repo.join(".spagitty/hooks.toml");
    write(if definition.shared { &shared } else { personal }, &text)?;
    // Remember the choice without deleting a person's shared definition.
    if definition.shared && personal.exists() {
        write(personal, &text)?;
    }
    Ok(())
}

fn executable(file: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = file;
    Ok(())
}

pub fn managed(file: &Path) -> bool {
    use std::io::BufRead;
    if std::fs::symlink_metadata(file).is_ok_and(|m| m.file_type().is_symlink()) {
        return false;
    }
    std::fs::File::open(file).is_ok_and(|input| {
        std::io::BufReader::new(input.take(256))
            .lines()
            .nth(1)
            .is_some_and(|line| line.is_ok_and(|line| line == MARK))
    })
}

pub fn preflight(
    repo: &Path,
    definition: &Definition,
    install: bool,
    imported: bool,
) -> Result<PathBuf> {
    validate(definition)?;
    let info = super::list(repo).map_err(|e| e.to_string())?;
    if install && info.manager != super::Manager::Git && !imported {
        return Err(
            "A hook manager owns these hooks. Import explicitly before installing blocks.".into(),
        );
    }
    let directory = if install && info.manager == super::Manager::Git {
        shell::hooks_dir(repo).map_err(|e| e.to_string())?
    } else {
        folder(repo)?
            .join(if install { "installed" } else { "preview" })
            .join(format!(
                "{:016x}",
                generate::hash(encode(definition)?.as_bytes())
            ))
    };
    for stage in Stage::ALL {
        let file = directory.join(stage.name());
        if file.exists() && !managed(&file) {
            return Err(format!(
                "{} is not a Spagitty hook and was left alone.",
                file.display()
            ));
        }
    }
    Ok(directory)
}

pub fn scripts(
    repo: &Path,
    definition: &Definition,
    install: bool,
    imported: bool,
) -> Result<PathBuf> {
    static WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = WRITES.lock().map_err(|e| e.to_string())?;
    let directory = preflight(repo, definition, install, imported)?;
    let switching =
        install && super::list(repo).map_err(|e| e.to_string())?.manager != super::Manager::Git;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    for stage in Stage::ALL {
        let file = directory.join(stage.name());
        write(&file, &generate::script(definition, stage)?)?;
        executable(&file)?;
    }
    if switching {
        shell::set_config(
            repo,
            "--local",
            "core.hooksPath",
            &shell::hook_path(&directory),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(directory)
}

/// The compact Settings hook preview truncates scripts. Installation checks
/// compare their complete bytes, including definitions with many blocks.
pub fn installed(repo: &Path, definition: &Definition) -> Result<bool> {
    let directory = shell::hooks_dir(repo).map_err(|e| e.to_string())?;
    for stage in Stage::ALL {
        let expected = generate::script(definition, stage)?;
        let file = directory.join(stage.name());
        if !file.exists() {
            return Ok(false);
        }
        if std::fs::metadata(&file).map_err(|e| e.to_string())?.len() != expected.len() as u64 {
            return Ok(false);
        }
        if !managed(&file) || read(&file, expected.len() as u64)? != expected {
            return Ok(false);
        }
    }
    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub stage: String,
    pub subject: String,
    pub at: String,
    pub exit: Option<i64>,
    pub events: Vec<Value>,
}

pub fn run(repo: &Path, id: &str) -> Result<Run> {
    read_run(repo, id)?.ok_or_else(|| "The hook record has not started yet.".into())
}

fn read_run(repo: &Path, id: &str) -> Result<Option<Run>> {
    if !identifier(id) {
        return Err("Invalid hook run id.".into());
    }
    let file = folder(repo)?.join("runs").join(format!("{id}.jsonl"));
    let text = read(&file, MAX_RECORD)?;
    // The writer may be halfway through the final line. Read completed lines;
    // a malformed completed event is reported rather than silently hidden.
    let mut events = Vec::new();
    for line in text
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
    {
        events.push(
            serde_json::from_str::<Value>(line)
                .map_err(|e| format!("The hook record could not be read: {e}"))?,
        );
    }
    let Some(first) = events.first() else {
        return Ok(None);
    };
    let subject = events
        .iter()
        .rev()
        .find_map(|e| e.get("subject").and_then(Value::as_str))
        .unwrap_or("")
        .into();
    Ok(Some(Run {
        id: id.into(),
        stage: first["stage"].as_str().unwrap_or("").into(),
        subject,
        at: first["at"].as_str().unwrap_or("").into(),
        exit: events
            .iter()
            .rev()
            .find(|e| e["t"] == "done")
            .and_then(|e| e["exit"].as_i64()),
        events,
    }))
}

pub fn runs(repo: &Path) -> Result<Vec<Run>> {
    let directory = folder(repo)?.join("runs");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut ids = BTreeMap::new();
    for file in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
        let file = file.map_err(|e| e.to_string())?;
        if file.path().extension().is_some_and(|e| e == "jsonl") {
            let name = file
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            ids.insert(
                (
                    file.metadata().and_then(|m| m.modified()).ok(),
                    name.clone(),
                ),
                name,
            );
        }
    }
    ids.values()
        .rev()
        .take(50)
        .filter_map(|id| match read_run(repo, id) {
            Ok(Some(mut run)) => {
                run.events.clear();
                Some(Ok(run))
            }
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

pub fn log(repo: &Path, id: &str, block: &str) -> Result<String> {
    if !identifier(id) || !identifier(block) {
        return Err("Invalid hook log id.".into());
    }
    let file = folder(repo)?
        .join("runs")
        .join(id)
        .join(format!("{block}.log"));
    if !file.exists() {
        return Ok(String::new());
    }
    let mut input = std::fs::File::open(file).map_err(|e| e.to_string())?;
    use std::io::{Seek, SeekFrom};
    let length = input.metadata().map_err(|e| e.to_string())?.len();
    if length > MAX_LOG {
        input
            .seek(SeekFrom::Start(length - MAX_LOG))
            .map_err(|e| e.to_string())?;
    }
    let mut bytes = Vec::new();
    input
        .take(MAX_LOG)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "{}{}",
        if length > MAX_LOG {
            "Earlier output omitted.\n"
        } else {
            ""
        },
        String::from_utf8_lossy(&bytes)
    ))
}

pub fn stop(repo: &Path, id: &str) -> Result<()> {
    if !identifier(id) {
        return Err("Invalid hook run id.".into());
    }
    if run(repo, id)?.exit.is_some() {
        return Err("This hook run has already ended.".into());
    }
    std::fs::write(folder(repo)?.join("runs").join(id).join("stop"), "").map_err(|e| e.to_string())
}

/// Import remains a read. Installation is a separate confirmed action.
pub fn import(repo: &Path) -> Result<Definition> {
    super::import::definition(repo)
}

#[cfg(test)]
mod tests {
    use super::super::blocks::tests::script;
    use super::super::blocks::Action;
    use super::*;
    use crate::fixture::Fixture;

    fn definition(command: &str) -> Definition {
        Definition {
            blocks: vec![script(command)],
            ..Definition::default()
        }
    }

    #[test]
    fn toml_round_trips_shell_quotes_newlines_arrays_and_unicode() {
        let mut definition = definition("printf '%s\\n' 'a\"b'\nprintf 'مرحبا'");
        definition.blocks[0].action = Action::Tests {
            paths: vec!["services/**".into(), "a \"b\"/**".into()],
            command: "printf 'x'\ntrue".into(),
        };
        assert_eq!(
            decode(&encode(&definition).expect("encode")).expect("decode"),
            definition
        );
        assert!(decode("version = 1\nversion = 2").is_err());
        assert!(decode("unknown").is_err());
    }

    #[test]
    fn installing_never_overwrites_a_persons_hook() {
        let fixture = Fixture::linear(1);
        let file = fixture.path().join(".git/hooks/pre-commit");
        std::fs::write(&file, "#!/bin/sh\nprintf 'mine'\n").expect("write");
        assert!(scripts(fixture.path(), &definition("true"), true, false).is_err());
        assert!(std::fs::read_to_string(file)
            .expect("read")
            .contains("mine"));
    }

    #[test]
    fn terminal_commits_write_events_logs_and_the_subject() {
        let fixture = Fixture::linear(1);
        scripts(
            fixture.path(),
            &definition("printf 'hello\\n'"),
            true,
            false,
        )
        .expect("install");
        fixture.git(&["commit", "--allow-empty", "-m", "feat: hello"]);
        let runs = runs(fixture.path()).expect("runs");
        let pre = runs
            .iter()
            .find(|r| r.stage == "pre-commit")
            .expect("pre-commit");
        assert_eq!(pre.subject, "feat: hello");
        assert_eq!(pre.exit, Some(0));
        assert!(log(fixture.path(), &pre.id, "check")
            .expect("log")
            .contains("hello"));
        assert!(run(fixture.path(), &pre.id)
            .expect("record")
            .events
            .iter()
            .any(|e| e["t"] == "end"));
        assert!(log(fixture.path(), "../outside", "check").is_err());
    }

    #[test]
    fn a_failure_blocks_unless_the_block_is_a_warning() {
        let fixture = Fixture::linear(1);
        let mut definition = definition("printf 'refused\\n'; exit 2");
        let directory = scripts(fixture.path(), &definition, false, false).expect("scripts");
        assert!(shell::hook_run(fixture.path(), &directory, "pre-commit", None).is_err());
        definition.blocks[0].warn = true;
        let directory = scripts(fixture.path(), &definition, false, false).expect("scripts");
        shell::hook_run(fixture.path(), &directory, "pre-commit", None)
            .expect("a warning continues");
    }

    #[test]
    fn formatting_never_touches_or_restages_a_partially_staged_file() {
        let fixture = Fixture::linear(1);
        let file = fixture.path().join("a file.txt");
        std::fs::write(&file, "staged\n").expect("file");
        fixture.git(&["add", "a file.txt"]);
        std::fs::write(&file, "staged\nprivate\n").expect("file");
        let before = fixture.git(&["show", ":a file.txt"]);
        let mut definition = definition("true");
        definition.blocks[0].action = Action::Format {
            preset: "custom".into(),
            paths: vec!["**/*.txt".into()],
            command: "printf 'rewritten' >".into(),
            restage: true,
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        assert!(shell::hook_run(fixture.path(), &dir, "pre-commit", None).is_err());
        assert_eq!(fixture.git(&["show", ":a file.txt"]), before);
        assert_eq!(
            std::fs::read_to_string(file).expect("read"),
            "staged\nprivate\n"
        );
    }

    #[test]
    fn spotless_receives_one_property_with_quoted_absolute_file_patterns() {
        let fixture = Fixture::linear(1);
        std::fs::write(fixture.path().join("a (draft).java"), "class A {}\n").expect("file");
        fixture.git(&["add", "a (draft).java"]);
        let mut definition = definition("true");
        definition.blocks[0].action = Action::Format {
            preset: "spotless".into(),
            paths: vec!["**/*.java".into()],
            command: "printf '%s\\n'".into(),
            restage: false,
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        shell::hook_run(fixture.path(), &dir, "pre-commit", None).expect("formatted");
        let record = runs(fixture.path()).expect("runs").remove(0);
        let output = log(fixture.path(), &record.id, "check").expect("log");
        assert!(output.starts_with("-DspotlessFiles=\\Q"), "{output}");
        assert!(output.contains("a (draft).java\\E"), "{output}");
        assert_eq!(output.lines().count(), 1);
    }

    #[test]
    fn path_conditions_skip_unmatched_modules_and_match_double_star() {
        let fixture = Fixture::linear(1);
        std::fs::create_dir_all(fixture.path().join("services/payments/src")).expect("dir");
        std::fs::write(fixture.path().join("services/payments/src/a.txt"), "x").expect("file");
        fixture.git(&["add", "services"]);
        let mut definition = definition("true");
        definition.blocks[0].action = Action::Tests {
            paths: vec!["web/**".into()],
            command: "exit 9".into(),
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        shell::hook_run(fixture.path(), &dir, "pre-commit", None)
            .expect("unmatched modules are skipped");
        definition.blocks[0].action = Action::Tests {
            paths: vec!["services/**".into()],
            command: "exit 9".into(),
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        assert!(shell::hook_run(fixture.path(), &dir, "pre-commit", None).is_err());
    }

    #[test]
    fn a_timeout_ends_the_step_and_is_recorded_as_a_failure() {
        let fixture = Fixture::linear(1);
        let mut definition = definition("sleep 30");
        definition.blocks[0].timeout_seconds = 1;
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        let began = std::time::Instant::now();
        assert!(shell::hook_run(fixture.path(), &dir, "pre-commit", None).is_err());
        assert!(began.elapsed() < std::time::Duration::from_secs(10));
        let record = runs(fixture.path()).expect("runs").remove(0);
        assert_eq!(record.exit, Some(124));
        assert!(log(fixture.path(), &record.id, "check")
            .expect("log")
            .contains("Timed out"));
    }

    #[test]
    fn formatting_restages_only_its_files_and_undo_refuses_a_later_stage_decision() {
        let fixture = Fixture::linear(1);
        let file = fixture.path().join("a file.txt");
        std::fs::write(&file, "original\n").expect("file");
        fixture.git(&["add", "a file.txt"]);
        let mut definition = definition("true");
        definition.blocks[0].action = Action::Format {
            preset: "custom".into(),
            paths: vec!["**/*.txt".into()],
            command: "sh -c 'for file do printf formatted > \"$file\"; done' _".into(),
            restage: true,
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        shell::hook_run(fixture.path(), &dir, "pre-commit", None).expect("formatted");
        assert_eq!(fixture.git(&["show", ":a file.txt"]), "formatted");
        let record = runs(fixture.path()).expect("runs").remove(0);
        let record = run(fixture.path(), &record.id).expect("record");
        let event = record
            .events
            .iter()
            .find(|event| event["t"] == "restaged")
            .expect("restaged");
        let before = event["before"].as_str().expect("before");
        let after = event["after"].as_str().expect("after");
        shell::hook_restore_index(fixture.path(), "a file.txt", before, after, "100644")
            .expect("undo");
        assert_eq!(fixture.git(&["show", ":a file.txt"]), "original\n");
        assert_eq!(std::fs::read_to_string(&file).expect("file"), "formatted");
        std::fs::write(&file, "later").expect("file");
        fixture.git(&["add", "a file.txt"]);
        assert!(
            shell::hook_restore_index(fixture.path(), "a file.txt", before, after, "100644")
                .is_err()
        );
        assert_eq!(fixture.git(&["show", ":a file.txt"]), "later");
    }

    #[test]
    fn secrets_and_large_files_are_checked_in_the_index_and_secrets_are_not_printed() {
        let fixture = Fixture::linear(1);
        std::fs::write(
            fixture.path().join("secret.txt"),
            "-----BEGIN PRIVATE KEY-----\n",
        )
        .expect("file");
        fixture.git(&["add", "secret.txt"]);
        let mut definition = definition("true");
        definition.blocks[0].action = Action::Secrets {
            patterns: Vec::new(),
        };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        let error = shell::hook_run(fixture.path(), &dir, "pre-commit", None)
            .expect_err("secret")
            .to_string();
        assert!(!error.contains("BEGIN PRIVATE"));
        assert!(error.contains("secret pattern"));
        definition.blocks[0].action = Action::LargeFiles { bytes: 2 };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        assert!(shell::hook_run(fixture.path(), &dir, "pre-commit", None).is_err());
        definition.blocks[0].action = Action::LargeFiles { bytes: 1000 };
        let dir = scripts(fixture.path(), &definition, false, false).expect("scripts");
        shell::hook_run(fixture.path(), &dir, "pre-commit", None).expect("small enough");
    }

    #[test]
    fn worktrees_share_records_and_incomplete_appends_wait_for_the_next_write() {
        let fixture = Fixture::linear(1);
        let linked = fixture.path().join("linked");
        fixture.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            linked.to_str().expect("path"),
        ]);
        assert_eq!(
            folder(fixture.path()).expect("common"),
            folder(&linked).expect("linked common")
        );
        let records = folder(fixture.path()).expect("folder").join("runs");
        std::fs::create_dir_all(&records).expect("records");
        std::fs::write(records.join("starting.jsonl"), "{\"t\":").expect("starting record");
        assert!(runs(fixture.path())
            .expect("incomplete first event waits")
            .is_empty());
        std::fs::write(
            records.join("partial.jsonl"),
            "{\"t\":\"run\",\"stage\":\"pre-commit\",\"at\":\"2026-10-14\"}\n{\"t\":\"done\"",
        )
        .expect("record");
        assert_eq!(
            run(&linked, "partial")
                .expect("complete events only")
                .events
                .len(),
            1
        );
    }

    #[test]
    fn switching_back_to_shared_follows_team_edits_and_personal_choices_stay_personal() {
        let fixture = Fixture::linear(1);
        let personal = tempfile::tempdir().expect("personal");
        let file = personal.path().join("hooks.toml");
        let mut shared = definition("printf shared");
        shared.shared = true;
        save(fixture.path(), &file, &shared).expect("shared");
        let private = definition("printf personal");
        save(fixture.path(), &file, &private).expect("personal");
        assert_eq!(load(fixture.path(), &file).expect("read"), private);
        save(fixture.path(), &file, &shared).expect("back to shared");
        shared.blocks[0].label = "Team edit".into();
        std::fs::write(
            fixture.path().join(".spagitty/hooks.toml"),
            encode(&shared).expect("toml"),
        )
        .expect("team edit");
        assert_eq!(load(fixture.path(), &file).expect("latest"), shared);
    }

    #[test]
    fn stopping_an_active_run_records_cancellation() {
        let fixture = Fixture::linear(1);
        let definition = definition("sleep 30");
        let directory = scripts(fixture.path(), &definition, false, false).expect("scripts");
        let repo = fixture.path().to_path_buf();
        let worker =
            std::thread::spawn(move || shell::hook_run(&repo, &directory, "pre-commit", None));
        let began = std::time::Instant::now();
        let id = loop {
            if let Some(run) = runs(fixture.path()).expect("runs").first() {
                break run.id.clone();
            }
            assert!(began.elapsed() < std::time::Duration::from_secs(10));
            std::thread::sleep(std::time::Duration::from_millis(25));
        };
        stop(fixture.path(), &id).expect("stop");
        assert!(worker.join().expect("worker").is_err());
        assert_eq!(run(fixture.path(), &id).expect("record").exit, Some(130));
    }

    #[test]
    fn pre_push_protection_checks_the_remote_destination_even_on_a_feature_branch() {
        let fixture = Fixture::linear(1);
        fixture.git(&["switch", "-c", "feature"]);
        let remote = tempfile::tempdir().expect("remote");
        fixture.git(&["init", "--bare", remote.path().to_str().expect("path")]);
        fixture.git(&[
            "remote",
            "add",
            "origin",
            remote.path().to_str().expect("path"),
        ]);
        let mut definition = definition("true");
        definition.blocks[0].stage = Stage::PrePush;
        definition.blocks[0].action = Action::ProtectBranches {
            branches: vec!["main".into()],
        };
        scripts(fixture.path(), &definition, true, false).expect("install");
        assert!(shell::push(fixture.path(), "origin", "feature:main", false).is_err());
        shell::push(fixture.path(), "origin", "feature:allowed", false)
            .expect("unprotected destination");
    }
}
