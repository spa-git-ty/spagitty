// SPDX-License-Identifier: GPL-3.0-or-later

//! The Terminal screen's terminals (FEAT-130).
//!
//! A terminal is a PTY from `spagitty-process`, running either the person's
//! shell or a multiplexer's client attached to the open repository's session.
//! Its output goes to the webview over a Tauri [`Channel`] as raw bytes — not
//! JSON, which would turn every byte into a number and a comma — and its
//! keystrokes and resizes come back as commands.
//!
//! Application state rather than repository state: a terminal belongs to the
//! window, and outlives the repository tab it was opened under.
//!
//! **Closing Spagitty detaches; it never kills.** With a multiplexer, what is
//! in the PTY is the *client*. Dropping it — here, or when the process ends —
//! ends the client, and the session carries on in the multiplexer's server.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use spagitty_process::mux::{self, Action, Mux};
use spagitty_process::pty::{Pty, PtySpec};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// The folder in the application's resources the bundled multiplexer is in.
const BUNDLED_DIR: &str = "multiplexer";

/// The event a terminal's exit is announced with.
const EXIT_EVENT: &str = "terminal-exit";

#[derive(Default)]
pub struct TerminalState {
    next: AtomicU32,
    open: Mutex<HashMap<u32, Arc<Pty>>>,
    /// Looked for once. A multiplexer installed while Spagitty runs is found
    /// the next time it starts.
    mux: OnceLock<Option<Mux>>,
}

impl TerminalState {
    fn mux<R: Runtime>(&self, app: &AppHandle<R>) -> Option<&Mux> {
        self.mux
            .get_or_init(|| {
                let bundled = app
                    .path()
                    .resource_dir()
                    .ok()
                    .map(|dir| dir.join(BUNDLED_DIR))
                    .filter(|dir| dir.is_dir());
                mux::detect(bundled.as_deref())
            })
            .as_ref()
    }

    fn pty(&self, id: u32) -> Option<Arc<Pty>> {
        self.open
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&id)
            .cloned()
    }
}

/// The multiplexer in use, for the header's pill and its hover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MuxInfo {
    /// `tmux` or `psmux`.
    pub name: &'static str,
    pub version: String,
    pub path: String,
    /// True when it is the one Spagitty ships.
    pub bundled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    /// `None` when no multiplexer can run, or the person turned it off: every
    /// tab is then a plain terminal.
    pub mux: Option<MuxInfo>,
    /// The program a new terminal runs.
    pub shell: String,
}

/// The shell a terminal runs when the person has not chosen one.
///
/// `$SHELL` on macOS and Linux. On Windows, PowerShell 7 if it is installed,
/// then Windows PowerShell, then `cmd`.
pub fn default_shell(
    windows: bool,
    env_shell: Option<String>,
    on_path: impl Fn(&str) -> bool,
) -> String {
    if windows {
        return ["pwsh.exe", "powershell.exe"]
            .into_iter()
            .find(|program| on_path(program))
            .unwrap_or("cmd.exe")
            .to_string();
    }
    env_shell
        .map(|shell| shell.trim().to_string())
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// The shell to run: the one chosen in Settings, or this platform's default.
fn shell_for(chosen: &str) -> String {
    let chosen = chosen.trim();
    if chosen.is_empty() {
        default_shell(cfg!(windows), std::env::var("SHELL").ok(), on_path)
    } else {
        chosen.to_string()
    }
}

/// What to run for a new terminal.
///
/// With a multiplexer and a session to attach to, its client on Spagitty's
/// socket; otherwise the shell, plainly.
pub fn spec_for(
    mux: Option<&Mux>,
    session: Option<&str>,
    cwd: &Path,
    shell: &str,
    chosen_shell: &str,
    cols: u16,
    rows: u16,
) -> PtySpec {
    let cwd = cwd.is_dir().then(|| cwd.to_path_buf());
    match (mux, session) {
        (Some(mux), Some(session)) => PtySpec {
            program: mux.path.to_string_lossy().into_owned(),
            args: mux::attach_args(
                session,
                cwd.as_deref().unwrap_or(Path::new(".")),
                Some(chosen_shell),
            ),
            cwd,
            env: Vec::new(),
            cols,
            rows,
        },
        _ => PtySpec {
            program: shell.to_string(),
            args: Vec::new(),
            cwd,
            env: Vec::new(),
            cols,
            rows,
        },
    }
}

fn info_of(mux: &Mux) -> MuxInfo {
    MuxInfo {
        name: mux.kind.name(),
        version: mux.version.clone(),
        path: mux.path.to_string_lossy().into_owned(),
        bundled: mux.bundled,
    }
}

/// The multiplexer to use, or none when the person turned it off.
fn mux_in_use<'a, R: Runtime>(app: &AppHandle<R>, state: &'a TerminalState) -> Option<&'a Mux> {
    crate::settings::load(app)
        .terminal_multiplexer
        .then(|| state.mux(app))
        .flatten()
}

/// Which multiplexer is in use, and which shell a terminal runs.
#[tauri::command(async)]
pub fn terminal_info<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
) -> TerminalInfo {
    TerminalInfo {
        mux: mux_in_use(&app, &state).map(info_of),
        shell: shell_for(&crate::settings::load(&app).terminal_shell),
    }
}

/// How a terminal is asked for.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Open {
    /// The multiplexer session to attach to. `None` is a plain terminal.
    pub session: Option<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Exited {
    id: u32,
    /// `None` when the program was killed and has no code.
    code: Option<u32>,
}

/// Open a terminal. Its output arrives on `output` as raw bytes; its exit is
/// announced as a `terminal-exit` event carrying its id.
#[tauri::command(async)]
pub fn terminal_open<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
    ask: Open,
    output: Channel<InvokeResponseBody>,
) -> Result<u32, String> {
    let settings = crate::settings::load(&app);
    let spec = spec_for(
        mux_in_use(&app, &state),
        ask.session.as_deref(),
        &PathBuf::from(&ask.cwd),
        &shell_for(&settings.terminal_shell),
        &settings.terminal_shell,
        ask.cols,
        ask.rows,
    );

    let id = state.next.fetch_add(1, Ordering::SeqCst) + 1;
    let gone = app.clone();
    let pty = Pty::spawn(
        &spec,
        move |bytes| {
            let _ = output.send(InvokeResponseBody::Raw(bytes.to_vec()));
        },
        move |code| {
            if let Some(state) = gone.try_state::<TerminalState>() {
                state
                    .open
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .remove(&id);
            }
            let _ = gone.emit(EXIT_EVENT, Exited { id, code });
        },
    )
    .map_err(|error| format!("{} could not be started: {error}", spec.program))?;

    state
        .open
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(id, Arc::new(pty));
    Ok(id)
}

/// Keystrokes, or a paste, for a terminal. A terminal that has gone takes
/// them and does nothing: the screen may be a frame behind its exit.
#[tauri::command]
pub fn terminal_write(state: State<'_, TerminalState>, id: u32, data: String) {
    if let Some(pty) = state.pty(id) {
        let _ = pty.write(data.as_bytes());
    }
}

#[tauri::command]
pub fn terminal_resize(state: State<'_, TerminalState>, id: u32, cols: u16, rows: u16) {
    if let Some(pty) = state.pty(id) {
        let _ = pty.resize(cols, rows);
    }
}

/// Close a terminal. With a multiplexer this ends the client, and the session
/// carries on.
#[tauri::command]
pub fn terminal_close(state: State<'_, TerminalState>, id: u32) {
    let pty = state
        .open
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&id);
    if let Some(pty) = pty {
        pty.kill();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub name: String,
    pub windows: u32,
    pub attached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub index: u32,
    pub name: String,
    pub active: bool,
}

/// The session a repository's terminal attaches to: its folder's name.
#[tauri::command]
pub fn terminal_session_name(path: String) -> String {
    mux::session_name(Path::new(&path))
}

/// The sessions on Spagitty's socket. Empty with no multiplexer.
#[tauri::command(async)]
pub fn terminal_sessions<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
) -> Vec<SessionInfo> {
    mux_in_use(&app, &state)
        .map(Mux::sessions)
        .unwrap_or_default()
        .into_iter()
        .map(|session| SessionInfo {
            name: session.name,
            windows: session.windows,
            attached: session.attached,
        })
        .collect()
}

#[tauri::command(async)]
pub fn terminal_windows<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
    session: String,
) -> Vec<WindowInfo> {
    mux_in_use(&app, &state)
        .map(|mux| mux.windows(&session))
        .unwrap_or_default()
        .into_iter()
        .map(|window| WindowInfo {
            index: window.index,
            name: window.name,
            active: window.active,
        })
        .collect()
}

/// Something asked of a session from the screen's header.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Ask {
    NewWindow { cwd: String },
    SelectWindow { index: u32 },
    SplitRight,
    SplitDown,
    Detach,
}

impl From<Ask> for Action {
    fn from(ask: Ask) -> Action {
        match ask {
            Ask::NewWindow { cwd } => Action::NewWindow {
                cwd: PathBuf::from(cwd),
            },
            Ask::SelectWindow { index } => Action::SelectWindow { index },
            Ask::SplitRight => Action::SplitRight,
            Ask::SplitDown => Action::SplitDown,
            Ask::Detach => Action::Detach,
        }
    }
}

#[tauri::command(async)]
pub fn terminal_act<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
    session: String,
    action: Ask,
) -> Result<(), String> {
    let mux = mux_in_use(&app, &state).ok_or_else(|| "no multiplexer is in use".to_string())?;
    mux.act(&session, &action.into())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_process::mux::Kind;

    #[test]
    fn the_shell_is_the_persons_own_on_macos_and_linux() {
        assert_eq!(
            default_shell(false, Some("/usr/bin/fish".into()), |_| false),
            "/usr/bin/fish"
        );
        // Unset, or set to nothing: the one shell every system has.
        assert_eq!(default_shell(false, None, |_| true), "/bin/sh");
        assert_eq!(default_shell(false, Some("  ".into()), |_| true), "/bin/sh");
    }

    /// The item: `pwsh`, then Windows PowerShell, then `cmd`.
    #[test]
    fn on_windows_it_is_powershell_seven_then_windows_powershell_then_cmd() {
        assert_eq!(default_shell(true, None, |_| true), "pwsh.exe");
        assert_eq!(
            default_shell(true, None, |program| program == "powershell.exe"),
            "powershell.exe"
        );
        assert_eq!(default_shell(true, None, |_| false), "cmd.exe");
        // `$SHELL` is not Windows' to honour: Git Bash sets it to a path
        // ConPTY cannot run.
        assert_eq!(
            default_shell(true, Some("/usr/bin/bash".into()), |_| false),
            "cmd.exe"
        );
    }

    fn a_mux() -> Mux {
        Mux {
            kind: Kind::Tmux,
            path: PathBuf::from("/usr/bin/tmux"),
            version: "3.5a".into(),
            bundled: false,
        }
    }

    #[test]
    fn with_a_multiplexer_and_a_session_the_terminal_is_its_client() {
        let here = std::env::temp_dir();
        let spec = spec_for(
            Some(&a_mux()),
            Some("spagitty"),
            &here,
            "/bin/zsh",
            "",
            120,
            40,
        );

        assert_eq!(spec.program, "/usr/bin/tmux");
        assert_eq!(
            &spec.args[..6],
            ["-L", "spagitty", "new-session", "-A", "-s", "spagitty"]
        );
        assert_eq!(spec.args[6], "-c");
        assert_eq!(spec.cwd.as_deref(), Some(here.as_path()));
        assert_eq!((spec.cols, spec.rows), (120, 40));
        // No shell chosen: the multiplexer's own default.
        assert_eq!(spec.args.len(), 8);
    }

    #[test]
    fn a_chosen_shell_is_what_a_new_session_runs() {
        let spec = spec_for(
            Some(&a_mux()),
            Some("spagitty"),
            &std::env::temp_dir(),
            "fish",
            "fish",
            80,
            24,
        );

        assert_eq!(spec.args.last().map(String::as_str), Some("fish"));
    }

    /// With no multiplexer each tab is a plain PTY.
    #[test]
    fn with_no_multiplexer_or_no_session_it_is_the_shell_plainly() {
        let here = std::env::temp_dir();

        let plain = spec_for(None, Some("spagitty"), &here, "/bin/zsh", "", 80, 24);
        assert_eq!(plain.program, "/bin/zsh");
        assert!(plain.args.is_empty());
        assert_eq!(plain.cwd.as_deref(), Some(here.as_path()));

        let tab = spec_for(Some(&a_mux()), None, &here, "/bin/zsh", "", 80, 24);
        assert_eq!(tab.program, "/bin/zsh");
    }

    #[test]
    fn a_folder_that_has_gone_is_not_started_in() {
        let gone = Path::new("/spagitty/no/such/folder");

        let spec = spec_for(None, None, gone, "/bin/sh", "", 80, 24);

        assert_eq!(spec.cwd, None);
    }

    #[test]
    fn the_pill_says_which_multiplexer_and_whose() {
        let info = info_of(&Mux {
            kind: Kind::Psmux,
            path: PathBuf::from("C:/app/multiplexer/psmux.exe"),
            version: "3.3.8".into(),
            bundled: true,
        });

        assert_eq!(info.name, "psmux");
        assert_eq!(info.version, "3.3.8");
        assert!(info.bundled);
        assert_eq!(info.path, "C:/app/multiplexer/psmux.exe");
    }

    #[test]
    fn what_the_screen_asks_of_a_session_is_what_the_multiplexer_is_told() {
        let asked: Ask = serde_json::from_str(r#"{"kind":"selectWindow","index":3}"#).expect("ask");
        assert_eq!(Action::from(asked), Action::SelectWindow { index: 3 });

        let asked: Ask = serde_json::from_str(r#"{"kind":"newWindow","cwd":"/r"}"#).expect("ask");
        assert_eq!(
            Action::from(asked),
            Action::NewWindow {
                cwd: PathBuf::from("/r")
            }
        );

        for (text, action) in [
            (r#"{"kind":"splitRight"}"#, Action::SplitRight),
            (r#"{"kind":"splitDown"}"#, Action::SplitDown),
            (r#"{"kind":"detach"}"#, Action::Detach),
        ] {
            let asked: Ask = serde_json::from_str(text).expect("ask");
            assert_eq!(Action::from(asked), action);
        }
        // Nothing the screen can ask kills a session.
        assert!(serde_json::from_str::<Ask>(r#"{"kind":"killSession"}"#).is_err());
    }
}
