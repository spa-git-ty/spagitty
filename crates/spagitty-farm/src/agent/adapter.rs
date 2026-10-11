// SPDX-License-Identifier: GPL-3.0-or-later

//! The one interface every provider implements.
//!
//! # Why this trait is synchronous
//!
//! The plan writes it `#[async_trait]`. Spagitty is not an async program:
//! `spagitty-core` is blocking throughout, the workers in `src-tauri` are OS
//! threads, and there is no runtime anywhere in the workspace. Making one trait
//! async would pull tokio into a desktop application whose concurrency is
//! "a handful of long-lived child processes", and every caller would then be
//! async too. So the trait blocks, the runner puts it on a thread, and
//! cancellation is a kill signal rather than a dropped future — which is what
//! it has to be anyway, because these are processes and not tasks.
//!
//! # What an adapter is allowed to know
//!
//! An adapter knows one provider's command line and nothing else. It does not
//! know about worktrees, dependencies, verification or review: it is handed a
//! working directory and a prompt, and it produces a command to run. That is
//! the whole boundary, and it is what keeps provider-specific strings out of
//! the engine.

use std::path::{Path, PathBuf};

use crate::execution::narrate::{Narrator, Verbatim};
use crate::model::{AgentAvailability, AgentDefinition, AgentProvider};

/// What the runner needs to start one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunRequest {
    /// The worktree the agent runs in. Always the task's own; never the
    /// repository the user is looking at.
    pub workdir: PathBuf,
    /// The whole prompt, already assembled by the context builder.
    pub prompt: String,
    /// Whether the farm's permissions allow the agent to act without asking.
    /// Adapters map this onto whatever their provider calls it.
    pub unattended: bool,
}

/// Whether the prompt has to travel on standard input rather than as an
/// argument.
///
/// On Windows a CLI installed with npm is a `.cmd` shim, and Windows cannot
/// pass a line break through one: Rust refuses to start it, and `cmd.exe` would
/// cut the argument at the first line if it did. A farm prompt is many lines,
/// so for a shim the prompt goes in on stdin and the adapter drops the
/// argument. Elsewhere, and for a real executable, nothing changes.
pub fn prompt_needs_stdin(executable: &Path, prompt: &str) -> bool {
    is_batch_shim(executable) && prompt.contains(['\n', '\r'])
}

fn is_batch_shim(executable: &Path) -> bool {
    cfg!(windows)
        && executable
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
}

/// The command line an adapter wants run, and how the prompt reaches it.
///
/// A plain description rather than a spawned process: the adapter decides
/// *what* to run and [`crate::execution`] decides *how*, so one place owns
/// stream capture, cancellation and logging for every provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// `Some` when the prompt goes to standard input rather than an argument.
    pub stdin: Option<String>,
}

impl AgentCommand {
    /// The command as one line, for the activity log.
    ///
    /// Arguments containing spaces are quoted so the line can be pasted into a
    /// shell and reproduce the run. That is the point of showing it at all —
    /// the same reasoning as `spagitty_core::record` for git commands.
    pub fn display(&self) -> String {
        let mut out = quoted(&self.program.to_string_lossy());
        for arg in &self.args {
            out.push(' ');
            out.push_str(&quoted(arg));
        }
        out
    }
}

fn quoted(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|c| c.is_alphanumeric() || "._-/=:@+,".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// One provider.
pub trait AgentAdapter: Send + Sync {
    fn provider(&self) -> AgentProvider;

    /// The command names to look for on `PATH`, most specific first.
    fn executables(&self) -> &'static [&'static str];

    /// A definition with this provider's defaults, for a freshly detected
    /// agent. The user edits it afterwards; nothing here is permanent.
    fn default_definition(&self, executable: PathBuf) -> AgentDefinition;

    /// Is it installed, and does it run?
    ///
    /// The default walks `PATH` and asks for a version. A provider that cannot
    /// answer `--version` overrides it rather than being reported broken.
    fn detect(&self) -> AgentAvailability {
        match super::detector::find(self.executables()) {
            Some(path) => super::detector::probe(&path, self.version_args()),
            None => AgentAvailability::Missing,
        }
    }

    /// The arguments that make this provider print its version and exit.
    fn version_args(&self) -> &'static [&'static str] {
        &["--version"]
    }

    /// Is the executable at `path` this provider, working?
    ///
    /// Asking for a version is the right question for a provider Spagitty
    /// knows, because it knows the flag exists. It is the wrong question for
    /// one it does not — see [`crate::agent::adapters::custom`].
    fn probe(&self, path: &Path) -> AgentAvailability {
        super::detector::probe(path, self.version_args())
    }

    /// Build the command line for one run.
    fn command(&self, definition: &AgentDefinition, request: &AgentRunRequest) -> AgentCommand;

    /// How this provider's output should be read.
    ///
    /// The default is verbatim, which is right for a provider that narrates its
    /// own work in plain text and for a custom agent, which is whatever the
    /// user pointed at. A provider whose command line above asks for a machine
    /// format overrides this with the narrator that reads it — the two belong
    /// to the same decision and are kept next to each other for that reason.
    fn narrator(&self) -> Box<dyn Narrator> {
        Box::new(Verbatim)
    }

    /// Does this provider need a terminal?
    ///
    /// Asked before a run rather than discovered during one: an agent that
    /// opens an interactive session with no tty attached hangs, and a hung
    /// child looks exactly like a slow one until somebody notices an hour
    /// later.
    fn is_headless(&self, definition: &AgentDefinition) -> bool {
        definition.traits.headless
    }
}

/// Where the adapter should run, given a worktree.
///
/// Trivial today, and here so that a provider needing a subdirectory — a
/// monorepo tool, say — has somewhere to say so rather than every call site
/// growing a special case.
pub fn workdir_for(worktree: &Path) -> PathBuf {
    worktree.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::prompt_needs_stdin;
    use std::path::Path;

    #[test]
    fn only_a_windows_shim_needs_a_long_prompt_on_stdin() {
        let many = "line one\nline two";
        assert!(!prompt_needs_stdin(Path::new("codex.exe"), many));
        assert!(!prompt_needs_stdin(Path::new("/usr/bin/codex"), many));
        assert!(!prompt_needs_stdin(Path::new("codex.cmd"), "one line"));
        assert_eq!(
            prompt_needs_stdin(Path::new("C:/npm/codex.CMD"), many),
            cfg!(windows)
        );
        assert_eq!(
            prompt_needs_stdin(Path::new("omp.bat"), many),
            cfg!(windows)
        );
    }

    use super::*;

    #[test]
    fn a_plain_command_displays_unquoted() {
        let command = AgentCommand {
            program: PathBuf::from("/usr/bin/claude"),
            args: vec!["-p".into(), "--permission-mode=plan".into()],
            stdin: None,
        };
        assert_eq!(
            command.display(),
            "/usr/bin/claude -p --permission-mode=plan"
        );
    }

    #[test]
    fn an_argument_with_spaces_is_quoted_so_the_line_can_be_pasted() {
        let command = AgentCommand {
            program: PathBuf::from("codex"),
            args: vec!["exec".into(), "Implement refresh rotation".into()],
            stdin: None,
        };
        assert_eq!(command.display(), "codex exec 'Implement refresh rotation'");
    }

    #[test]
    fn a_quote_inside_an_argument_survives_the_quoting() {
        let command = AgentCommand {
            program: PathBuf::from("codex"),
            args: vec!["it's fine".into()],
            stdin: None,
        };
        assert_eq!(command.display(), r"codex 'it'\''s fine'");
    }

    #[test]
    fn an_empty_argument_is_visible_rather_than_lost() {
        let command = AgentCommand {
            program: PathBuf::from("x"),
            args: vec![String::new()],
            stdin: None,
        };
        assert_eq!(command.display(), "x ''");
    }
}
