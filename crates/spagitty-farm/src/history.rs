// SPDX-License-Identifier: GPL-3.0-or-later

//! The repository's history, offered to the farm's agents as tools over MCP
//! (FEAT-145).
//!
//! An agent that wants to know why a line is the way it is walks the history
//! itself: `git log -p`, `git blame`, page after page into its context. Here it
//! asks instead — *blame these lines*, *the history of this file*, *search the
//! commits for this* — and gets a short answer. The answers are cached per
//! commit, on disk, so the second agent to ask the same question of the same
//! commit is answered without git.
//!
//! The same tools let an agent that has no folder of its own — one Spagitty
//! calls by API — read the repository at all: `show` is a file as a commit has
//! it.
//!
//! # The server
//!
//! MCP over stdio: one JSON-RPC message per line in, one per line out. It is
//! started as `spagitty --history-mcp <repository>` by the agent CLI, which the
//! farm tells about it ([`attach`]). It answers `initialize`, `tools/list`,
//! `tools/call` and `ping`, and reads nothing but the repository it was given.
//! Every tool reads; none writes.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use spagitty_core::shell;

use crate::agent::AgentCommand;
use crate::model::AgentProvider;

/// The server's name, as the agents see it: tools are `mcp__spagitty__blame`.
pub const SERVER: &str = "spagitty";

/// The argument that starts the server instead of the application.
pub const FLAG: &str = "--history-mcp";

/// The protocol version answered when the client does not ask for one.
const PROTOCOL: &str = "2025-06-18";

/// Most lines a tool answers with, so an answer never floods a context.
const MAX_LINES: usize = 400;

/// How to start the server: the program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl Server {
    /// The application itself, run with [`FLAG`] and the repository.
    pub fn of(program: PathBuf, repo: &Path) -> Self {
        Server {
            program,
            args: vec![FLAG.to_string(), repo.to_string_lossy().into_owned()],
        }
    }
}

/// Tell an agent's command about the server, for the CLIs that take one.
///
/// Claude Code takes `--mcp-config` with the server and `--allowedTools` so a
/// headless run may call it without asking; Codex takes the server as two
/// `-c` settings. The others are left as they are: their prompt does not
/// mention the tools, so they are not missed.
pub fn attach(provider: AgentProvider, command: &mut AgentCommand, server: &Server) -> bool {
    let program = server.program.to_string_lossy().into_owned();
    let added: Vec<String> = match provider {
        AgentProvider::ClaudeCode => {
            let config = json!({
                "mcpServers": { SERVER: { "command": program, "args": server.args } }
            });
            vec![
                "--mcp-config".into(),
                config.to_string(),
                "--allowedTools".into(),
                format!("mcp__{SERVER}"),
            ]
        }
        AgentProvider::Codex => {
            // TOML literal strings: a Windows path's backslashes stay as they are.
            let literal = |text: &str| format!("'{}'", text.replace('\'', ""));
            let args: Vec<String> = server.args.iter().map(|arg| literal(arg)).collect();
            vec![
                "-c".into(),
                format!("mcp_servers.{SERVER}.command={}", literal(&program)),
                "-c".into(),
                format!("mcp_servers.{SERVER}.args=[{}]", args.join(",")),
            ]
        }
        _ => return false,
    };
    // After the subcommand or the print flag, before everything else, so a
    // prompt passed as the last argument stays last.
    let at = command.args.len().min(1);
    command.args.splice(at..at, added);
    true
}

/// Serve MCP on `input` and `output` until the input ends.
pub fn serve(repo: &Path, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let tools = Tools::new(repo);
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = answer(&tools, &line) else {
            continue;
        };
        writeln!(output, "{reply}")?;
        output.flush()?;
    }
    Ok(())
}

/// The reply to one message, or none for a notification.
fn answer(tools: &Tools, line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(error) => return Some(failure(Value::Null, -32700, &format!("Not JSON: {error}"))),
    };
    // A message without an id is a notification, and is never answered.
    let id = message.get("id").cloned()?;
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER, "version": env!("CARGO_PKG_VERSION") },
            "instructions": "The repository's history. Ask here rather than running git log or git blame yourself."
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": catalogue() }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match tools.call(name, &arguments) {
                Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
                // A tool that could not answer says so as a result, so the
                // agent reads why rather than the client failing the call.
                Err(why) => {
                    json!({ "content": [{ "type": "text", "text": why }], "isError": true })
                }
            }
        }
        _ => return Some(failure(id, -32601, &format!("No method {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The tools, as `tools/list` describes them.
pub fn catalogue() -> Value {
    let rev =
        json!({ "type": "string", "description": "A commit, branch or tag. HEAD when left out." });
    json!([
        {
            "name": "blame",
            "description": "Who last changed each line of a file, in which commit and why. Give a line range for a long file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file, from the repository root." },
                    "rev": rev,
                    "start": { "type": "integer", "description": "First line, 1-based." },
                    "end": { "type": "integer", "description": "Last line, inclusive." }
                },
                "required": ["path"]
            }
        },
        {
            "name": "history",
            "description": "The commits that changed a file, newest first, following renames.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "limit": { "type": "integer", "description": "How many commits. 20 when left out." }
                },
                "required": ["path"]
            }
        },
        {
            "name": "search",
            "description": "Search commit messages (in: commits), the changes that added or removed a string (in: changes), or the files of a commit (in: code).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "in": { "type": "string", "enum": ["commits", "changes", "code"] },
                    "rev": rev,
                    "limit": { "type": "integer" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "show",
            "description": "A file as a commit has it.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string" }, "rev": rev },
                "required": ["path"]
            }
        }
    ])
}

/// The tools over one repository, with their cache.
pub struct Tools {
    repo: PathBuf,
    cache: PathBuf,
}

impl Tools {
    pub fn new(repo: &Path) -> Self {
        Tools {
            repo: repo.to_path_buf(),
            cache: repo
                .join(crate::workspace::FARM_DIR)
                .join("farm")
                .join("history"),
        }
    }

    /// Answer one call, from the cache when this commit was asked it before.
    pub fn call(&self, name: &str, arguments: &Value) -> Result<String, String> {
        let text = |key: &str| {
            arguments
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        };
        let number = |key: &str| {
            arguments
                .get(key)
                .and_then(Value::as_u64)
                .map(|n| n as usize)
        };
        let rev = text("rev").unwrap_or("HEAD");
        let commit =
            shell::commit_id(&self.repo, rev).map_err(|_| format!("There is no commit {rev}."))?;
        let path = text("path");
        if let Some(path) = path {
            if path.contains("..") || Path::new(path).is_absolute() {
                return Err(format!("{path} is not a path in the repository."));
            }
        }

        // `history` follows a path across every commit, so it is keyed by
        // the commit it was asked at like the rest: a new commit is a new key.
        let key = format!("{name}\u{0}{}", canonical(arguments));
        if let Some(hit) = self.cached(&commit, &key) {
            return Ok(hit);
        }
        let answer = match name {
            "blame" => self.blame(
                path.ok_or("blame needs a path")?,
                &commit,
                number("start"),
                number("end"),
            ),
            "history" => self.history(
                path.ok_or("history needs a path")?,
                &commit,
                number("limit").unwrap_or(20),
            ),
            "search" => self.search(
                text("text").ok_or("search needs some text")?,
                text("in").unwrap_or("commits"),
                &commit,
                number("limit").unwrap_or(30),
            ),
            "show" => self.show(path.ok_or("show needs a path")?, &commit),
            other => return Err(format!("There is no tool {other}.")),
        }?;
        self.keep(&commit, &key, &answer);
        Ok(answer)
    }

    fn blame(
        &self,
        path: &str,
        commit: &str,
        start: Option<usize>,
        end: Option<usize>,
    ) -> Result<String, String> {
        let repo = spagitty_core::repo::open(&self.repo).map_err(|e| e.to_string())?;
        let blame = spagitty_core::blame::file(&repo, path, commit).map_err(|e| e.to_string())?;
        if let Some(why) = blame.refused {
            return Err(format!("{path} cannot be blamed: {why:?}."));
        }
        let first = start.unwrap_or(1).max(1);
        let last = end.unwrap_or(usize::MAX);
        let lines: Vec<String> = blame
            .lines
            .iter()
            .filter(|line| (first..=last).contains(&(line.line as usize)))
            .map(|line| {
                format!(
                    "{:>5} {} {} {} | {}",
                    line.line,
                    line.short,
                    day(line.time),
                    line.author_name,
                    line.text
                )
            })
            .collect();
        let mut commits: Vec<String> = Vec::new();
        for line in blame
            .lines
            .iter()
            .filter(|line| (first..=last).contains(&(line.line as usize)))
        {
            let entry = format!("{} {}", line.short, line.summary);
            if !commits.contains(&entry) {
                commits.push(entry);
            }
        }
        Ok(format!(
            "{path} at {}\n{}\n\nCommits:\n{}",
            short(commit),
            clip(lines),
            commits.join("\n")
        ))
    }

    fn history(&self, path: &str, commit: &str, limit: usize) -> Result<String, String> {
        let limit = limit.clamp(1, 200);
        let raw = shell::ran_ok(
            &self.repo,
            &[
                "log",
                "--follow",
                &format!("-{limit}"),
                "--format=%h %ad %an | %s",
                "--date=short",
                commit,
                "--",
                path,
            ],
        )
        .map_err(|e| e.to_string())?;
        if raw.trim().is_empty() {
            return Ok(format!("No commit at {} changed {path}.", short(commit)));
        }
        Ok(clip(raw.lines().map(str::to_string).collect()))
    }

    fn search(
        &self,
        text: &str,
        within: &str,
        commit: &str,
        limit: usize,
    ) -> Result<String, String> {
        let limit = limit.clamp(1, 200).to_string();
        let raw = match within {
            "commits" => shell::ran_ok(
                &self.repo,
                &[
                    "log",
                    "-i",
                    "--fixed-strings",
                    &format!("--grep={text}"),
                    "-n",
                    &limit,
                    "--format=%h %ad %an | %s",
                    "--date=short",
                    commit,
                ],
            ),
            "changes" => shell::ran_ok(
                &self.repo,
                &[
                    "log",
                    &format!("-S{text}"),
                    "-n",
                    &limit,
                    "--format=%h %ad %an | %s",
                    "--date=short",
                    commit,
                ],
            ),
            "code" => shell::ran(
                &self.repo,
                &["grep", "-n", "-I", "--fixed-strings", "-e", text, commit],
            )
            .map(|ran| ran.stdout)
            .map(|out| {
                let prefix = format!("{commit}:");
                out.lines()
                    .take(limit.parse().unwrap_or(30))
                    .map(|line| line.strip_prefix(&prefix).unwrap_or(line).to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            other => return Err(format!("search in {other}? Use commits, changes or code.")),
        }
        .map_err(|e| e.to_string())?;
        if raw.trim().is_empty() {
            return Ok(format!(
                "Nothing found for \"{text}\" in {within} at {}.",
                short(commit)
            ));
        }
        Ok(clip(raw.lines().map(str::to_string).collect()))
    }

    fn show(&self, path: &str, commit: &str) -> Result<String, String> {
        match shell::show_text(&self.repo, commit, path).map_err(|e| e.to_string())? {
            Some(text) => Ok(text),
            None => Err(format!("{path} is not a text file at {}.", short(commit))),
        }
    }

    fn entry(&self, commit: &str, key: &str) -> PathBuf {
        self.cache
            .join(commit)
            .join(format!("{:016x}.txt", fnv(key)))
    }

    fn cached(&self, commit: &str, key: &str) -> Option<String> {
        let text = std::fs::read_to_string(self.entry(commit, key)).ok()?;
        // The key is stored first, so a hash that collides is a miss.
        let (stored, answer) = text.split_once("\n\n")?;
        (stored == key.replace('\u{0}', " ")).then(|| answer.to_string())
    }

    fn keep(&self, commit: &str, key: &str, answer: &str) {
        let file = self.entry(commit, key);
        if let Some(dir) = file.parent() {
            if std::fs::create_dir_all(dir).is_ok() {
                let _ = std::fs::write(&file, format!("{}\n\n{answer}", key.replace('\u{0}', " ")));
            }
        }
    }
}

/// Arguments in a stable order, so the same question is the same key.
fn canonical(arguments: &Value) -> String {
    match arguments {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().filter(|key| *key != "rev").collect();
            keys.sort();
            keys.iter()
                .map(|key| format!("{key}={}", map[*key]))
                .collect::<Vec<_>>()
                .join("&")
        }
        other => other.to_string(),
    }
}

/// FNV-1a: stable across builds, which the standard hasher does not promise.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(7)]
}

fn day(seconds: i64) -> String {
    // Days since the epoch to a civil date (Howard Hinnant's algorithm), so
    // this needs no date crate.
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn clip(lines: Vec<String>) -> String {
    let total = lines.len();
    if total <= MAX_LINES {
        return lines.join("\n");
    }
    let mut kept = lines[..MAX_LINES].join("\n");
    kept.push_str(&format!(
        "\n… {} more lines. Narrow the question.",
        total - MAX_LINES
    ));
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;
    use std::io::Cursor;

    fn session(repo: &Path, messages: &[Value]) -> Vec<Value> {
        let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
        let mut output = Vec::new();
        serve(repo, Cursor::new(input), &mut output).unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn call(id: u64, name: &str, arguments: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": name, "arguments": arguments } })
    }

    fn text(reply: &Value) -> String {
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    fn tracked(repo: &Fixture) -> String {
        repo.git(&["ls-files"]).lines().next().unwrap().to_string()
    }

    #[test]
    fn it_introduces_itself_lists_four_tools_and_ignores_notifications() {
        let repo = Fixture::woven();
        let replies = session(
            repo.path(),
            &[
                json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } }),
                json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
                json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
                json!({ "jsonrpc": "2.0", "id": 3, "method": "nothing/here" }),
            ],
        );

        assert_eq!(replies.len(), 3, "a notification was answered");
        assert_eq!(replies[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(replies[0]["result"]["serverInfo"]["name"], SERVER);
        let names: Vec<&str> = replies[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["blame", "history", "search", "show"]);
        assert_eq!(replies[2]["error"]["code"], -32601);
    }

    #[test]
    fn blame_history_search_and_show_answer_from_the_repository() {
        let repo = Fixture::woven();
        let path = tracked(&repo);
        let summary = repo
            .git(&["log", "-1", "--format=%s", "--", &path])
            .trim()
            .to_string();
        let replies = session(
            repo.path(),
            &[
                call(1, "blame", json!({ "path": path, "start": 1, "end": 1 })),
                call(2, "history", json!({ "path": path })),
                call(3, "search", json!({ "text": summary, "in": "commits" })),
                call(4, "show", json!({ "path": path })),
            ],
        );

        assert!(
            text(&replies[0]).contains("    1 "),
            "{}",
            text(&replies[0])
        );
        assert!(text(&replies[0]).contains("Commits:"));
        assert!(
            text(&replies[1]).contains(&summary),
            "{}",
            text(&replies[1])
        );
        assert!(
            text(&replies[2]).contains(&summary),
            "{}",
            text(&replies[2])
        );
        assert_eq!(
            text(&replies[3]),
            repo.git(&["show", &format!("HEAD:{path}")])
        );
    }

    #[test]
    fn an_answer_is_cached_per_commit_and_a_new_commit_asks_again() {
        let repo = Fixture::woven();
        let tools = Tools::new(repo.path());
        let first = tools
            .call(
                "search",
                &json!({ "text": "zebra-unicorn", "in": "commits" }),
            )
            .unwrap();
        assert!(first.starts_with("Nothing found"));
        let head = repo.rev("HEAD");
        assert!(repo
            .path()
            .join(".spagitty/farm/history")
            .join(&head)
            .is_dir());

        repo.write("z.txt", "z\n");
        repo.git(&["add", "z.txt"]);
        repo.commit("zebra-unicorn arrives");
        let again = tools
            .call(
                "search",
                &json!({ "text": "zebra-unicorn", "in": "commits" }),
            )
            .unwrap();
        assert!(again.contains("zebra-unicorn arrives"), "{again}");
        // The old commit's answer is still its answer.
        let old = tools
            .call(
                "search",
                &json!({ "text": "zebra-unicorn", "in": "commits", "rev": head }),
            )
            .unwrap();
        assert!(old.starts_with("Nothing found"));
    }

    #[test]
    fn a_question_it_cannot_answer_is_a_result_that_says_why() {
        let repo = Fixture::woven();
        let replies = session(
            repo.path(),
            &[
                call(1, "show", json!({ "path": "../outside" })),
                call(2, "blame", json!({ "path": "nope.rs" })),
                call(3, "history", json!({ "path": "x", "rev": "no-such-rev" })),
            ],
        );
        for reply in &replies {
            assert_eq!(reply["result"]["isError"], true, "{reply}");
        }
        assert!(text(&replies[0]).contains("not a path in the repository"));
        assert!(text(&replies[2]).contains("no commit no-such-rev"));
    }

    #[test]
    fn claude_and_codex_are_told_about_the_server_and_the_prompt_stays_last() {
        let server = Server::of(PathBuf::from("/opt/spagitty"), Path::new("/repo"));
        let mut claude = AgentCommand {
            program: PathBuf::from("claude"),
            args: vec!["-p".into(), "--verbose".into(), "Do it".into()],
            stdin: None,
        };
        assert!(attach(AgentProvider::ClaudeCode, &mut claude, &server));
        assert_eq!(claude.args[0], "-p");
        assert_eq!(claude.args[1], "--mcp-config");
        let config: Value = serde_json::from_str(&claude.args[2]).unwrap();
        assert_eq!(
            config["mcpServers"]["spagitty"]["args"],
            json!(["--history-mcp", "/repo"])
        );
        assert_eq!(claude.args[4], "mcp__spagitty");
        assert_eq!(claude.args.last().unwrap(), "Do it");

        let mut codex = AgentCommand {
            program: PathBuf::from("codex"),
            args: vec!["exec".into(), "Do it".into()],
            stdin: None,
        };
        assert!(attach(AgentProvider::Codex, &mut codex, &server));
        assert_eq!(
            codex.args[2],
            "mcp_servers.spagitty.command='/opt/spagitty'"
        );
        assert_eq!(
            codex.args[4],
            "mcp_servers.spagitty.args=['--history-mcp','/repo']"
        );

        let mut other = AgentCommand {
            program: PathBuf::from("cursor-agent"),
            args: vec!["Do it".into()],
            stdin: None,
        };
        assert!(!attach(AgentProvider::Cursor, &mut other, &server));
        assert_eq!(other.args, ["Do it"]);
    }

    #[test]
    fn days_are_civil_dates() {
        assert_eq!(day(0), "1970-01-01");
        assert_eq!(day(1_792_368_000), "2026-10-19");
    }
}
