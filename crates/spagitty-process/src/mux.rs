// SPDX-License-Identifier: GPL-3.0-or-later

//! The terminal multiplexer: tmux, or psmux on Windows (FEAT-130).
//!
//! A multiplexer is what keeps a terminal session alive when the window that
//! showed it is gone, and splits it into panes. Spagitty runs the shell inside
//! one when one can be run, and as a plain terminal when none can — and says
//! nothing about the difference, because a terminal that works is not missing
//! anything.
//!
//! # Its own socket
//!
//! Everything here goes to `-L spagitty`: a server of Spagitty's own, beside
//! whatever tmux the person already runs and never touching it. Their
//! `~/.tmux.conf` still applies, since tmux reads it whichever socket it is on.
//!
//! # Which binary
//!
//! [`choose`] is the whole rule, and it is a pure function so the rule is
//! tested without a multiplexer installed:
//!
//! - on macOS and Linux, the system tmux when it is at least [`TMUX_MIN`],
//!   and otherwise the one Spagitty ships;
//! - on Windows, the psmux Spagitty ships, and otherwise one on the `PATH`.
//!
//! # Detaching, never killing
//!
//! A session is attached to with `new-session -A`, which makes it when it is
//! not there and joins it when it is. Leaving is killing the *client* — the
//! process in Spagitty's PTY. The server and the session carry on, which is
//! what a multiplexer is for.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The socket every command here is sent to.
pub const SOCKET: &str = "spagitty";

/// The oldest system tmux that is used rather than passed over: 3.2 is where
/// `display-popup` and the extended keys this was written against arrived.
pub const TMUX_MIN: (u32, u32) = (3, 2);

/// Which multiplexer a binary is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tmux,
    /// tmux's commands on Windows' ConPTY. Answers to `tmux`, `psmux` and `pmux`.
    Psmux,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Tmux => "tmux",
            Kind::Psmux => "psmux",
        }
    }
}

/// A multiplexer that can be run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mux {
    pub kind: Kind,
    pub path: PathBuf,
    /// As it says itself: `3.5a`, `3.3.8`.
    pub version: String,
    /// True when it is the one Spagitty ships, not the system's.
    pub bundled: bool,
}

/// A binary that answered `-V`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub kind: Kind,
    pub path: PathBuf,
    pub version: String,
    pub major: u32,
    pub minor: u32,
}

/// What `tmux -V` or `psmux -V` printed, as a kind and a version.
///
/// `tmux 3.3a`, `tmux next-3.5`, `tmux openbsd-7.4` (which has no number of
/// its own and is taken as new), `psmux 3.3.8`, `psmux v3.3.8`.
///
/// psmux answers to `tmux` too, and says so first: its `-V` is `tmux 3.3.8`
/// and then `psmux 3.3.8 (66cf613 2026-08-18)`. A line that says psmux is
/// believed over one that says tmux.
pub fn parse_version(output: &str) -> Option<(Kind, String, u32, u32)> {
    let named = |line: &&str| {
        let first = line.split_whitespace().next().unwrap_or("");
        first.eq_ignore_ascii_case("psmux") || first.eq_ignore_ascii_case("pmux")
    };
    let line = output
        .lines()
        .find(named)
        .or_else(|| output.lines().find(|line| !line.trim().is_empty()))?
        .trim();
    let mut words = line.split_whitespace();
    let name = words.next()?.to_ascii_lowercase();
    let kind = match name.as_str() {
        "tmux" => Kind::Tmux,
        "psmux" | "pmux" => Kind::Psmux,
        _ => return None,
    };
    let version = words.next()?.trim_start_matches('v').to_string();

    // OpenBSD's tmux is numbered by the system it shipped with, and every
    // one of those still supported is past 3.2.
    if version.starts_with("openbsd-") {
        return Some((kind, version, TMUX_MIN.0, TMUX_MIN.1));
    }
    let numbers = version.strip_prefix("next-").unwrap_or(&version);
    let mut parts = numbers.split('.');
    let major = digits(parts.next()?)?;
    let minor = parts.next().and_then(digits).unwrap_or(0);
    Some((kind, version, major, minor))
}

/// The number a version part starts with: `3` from `3`, `3` from `3a`.
fn digits(part: &str) -> Option<u32> {
    let end = part
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(part.len());
    part[..end].parse().ok()
}

/// Which of the two is used, if either.
///
/// `system` is what was found on the `PATH`, and `bundled` what Spagitty
/// shipped. See the module's header for the rule.
pub fn choose(system: Option<Probe>, bundled: Option<Probe>, windows: bool) -> Option<Mux> {
    let take = |probe: Probe, bundled: bool| Mux {
        kind: probe.kind,
        path: probe.path,
        version: probe.version,
        bundled,
    };
    if windows {
        // The one that shipped is the one that was tested with.
        return bundled
            .map(|probe| take(probe, true))
            .or_else(|| system.map(|probe| take(probe, false)));
    }
    let recent = |probe: &Probe| (probe.major, probe.minor) >= TMUX_MIN;
    match system {
        Some(probe) if recent(&probe) => Some(take(probe, false)),
        // Too old, or not there: the bundled one, when there is one. An old
        // system tmux is not used on its own: it is the reason one ships.
        _ => bundled.map(|probe| take(probe, true)),
    }
}

/// A command that never opens a console window of its own on Windows.
fn quiet(program: &Path) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Ask a binary what it is. `None` when it will not run, or is neither.
pub fn probe(path: &Path) -> Option<Probe> {
    let output = quiet(path).arg("-V").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let (kind, version, major, minor) = parse_version(&String::from_utf8_lossy(&output.stdout))?;
    Some(Probe {
        kind,
        path: path.to_path_buf(),
        version,
        major,
        minor,
    })
}

/// The names a multiplexer has on this platform, as files.
fn names(windows: bool) -> &'static [&'static str] {
    if windows {
        &["psmux.exe", "tmux.exe", "pmux.exe"]
    } else {
        &["tmux"]
    }
}

/// The first of `names` that is a file in one of `dirs`.
fn find_in(dirs: impl Iterator<Item = PathBuf>, names: &[&str]) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = dirs.collect();
    names.iter().find_map(|name| {
        dirs.iter()
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    })
}

/// The multiplexer to use on this machine, if there is one.
///
/// `bundled_dir` is where Spagitty's own was put when it was built. Looked
/// for once at start and when the Terminal screen is first opened; a tmux
/// installed while Spagitty runs is found the next time.
pub fn detect(bundled_dir: Option<&Path>) -> Option<Mux> {
    let windows = cfg!(windows);
    let system = std::env::var_os("PATH")
        .and_then(|path| find_in(std::env::split_paths(&path), names(windows)))
        .and_then(|path| probe(&path));
    let bundled = bundled_dir
        .and_then(|dir| find_in(std::iter::once(dir.to_path_buf()), names(windows)))
        .and_then(|path| probe(&path));
    choose(system, bundled, windows)
}

/// FNV-1a over `bytes`: small, and the same on every machine and every run.
fn fnv(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// A session's name for a repository: its folder's name, without the
/// characters tmux keeps for itself, and six letters made from its whole
/// path.
///
/// tmux reads `.` and `:` in a target as separators — `session:window.pane` —
/// so a folder called `my.app` would name a pane. Both become `_`, which is
/// what tmux itself does when asked to make such a session.
///
/// The letters are what keeps `/work/api` and `/personal/api` apart. A
/// session is attached to with `new-session -A`, which joins one that is
/// there and ignores the folder it was given. With the folder's name alone,
/// the second repository's terminal would be a shell in the first, and a
/// command typed there would run on the wrong repository.
///
/// With no repository there is no path, and the name is `spagitty`.
pub fn session_name(repository: &Path) -> String {
    if repository.as_os_str().is_empty() {
        return "spagitty".to_string();
    }
    // One spelling for one folder where the file system does not tell them
    // apart.
    let whole = repository.to_string_lossy();
    let whole = if cfg!(windows) {
        whole.replace('\\', "/").to_lowercase()
    } else {
        whole.into_owned()
    };
    let mark = fnv(whole.trim_end_matches('/').as_bytes()) & 0x00ff_ffff;
    format!("{}-{mark:06x}", readable(repository))
}

/// The part of a session's name a person reads: the folder's.
fn readable(repository: &Path) -> String {
    let folder = repository
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name: String = folder
        .chars()
        .map(|c| match c {
            '.' | ':' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let name = name.trim();
    if name.is_empty() {
        "spagitty".to_string()
    } else {
        name.to_string()
    }
}

/// The arguments that attach to `session`, making it at `cwd` if it is not there.
///
/// `shell` is run in a session that is made, when the person chose one. tmux's
/// own default is `default-shell`, and then `$SHELL`.
pub fn attach_args(session: &str, cwd: &Path, shell: Option<&str>) -> Vec<String> {
    let mut args = vec![
        "-L".to_string(),
        SOCKET.to_string(),
        "new-session".to_string(),
        "-A".to_string(),
        "-s".to_string(),
        session.to_string(),
        "-c".to_string(),
        cwd.to_string_lossy().into_owned(),
    ];
    if let Some(shell) = shell.map(str::trim).filter(|shell| !shell.is_empty()) {
        args.push(shell.to_string());
    }
    args
}

/// A session on Spagitty's socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub windows: u32,
    /// A client is attached to it: Spagitty's, or the person's own terminal.
    pub attached: bool,
}

/// One window of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub index: u32,
    pub name: String,
    /// The window the session is showing.
    pub active: bool,
}

/// What separates fields in the formats below. A tab, which a name cannot hold
/// without being quoted by the shell that typed it.
const FIELD: char = '\t';

const SESSIONS_FORMAT: &str = "#{session_name}\t#{session_windows}\t#{session_attached}";
const WINDOWS_FORMAT: &str = "#{window_index}\t#{window_name}\t#{window_active}";

/// `list-sessions`'s output as sessions. Lines that are not sessions are left out.
pub fn parse_sessions(output: &str) -> Vec<Session> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(FIELD);
            let name = fields.next()?.to_string();
            let windows = fields.next()?.trim().parse().ok()?;
            let attached = fields.next()?.trim().parse::<u32>().ok()? > 0;
            (!name.is_empty()).then_some(Session {
                name,
                windows,
                attached,
            })
        })
        .collect()
}

/// `list-windows`'s output as windows.
pub fn parse_windows(output: &str) -> Vec<Window> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(FIELD);
            let index = fields.next()?.trim().parse().ok()?;
            let name = fields.next()?.to_string();
            let active = fields.next()?.trim() == "1";
            Some(Window {
                index,
                name,
                active,
            })
        })
        .collect()
}

/// Something done to a session from the screen's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    NewWindow {
        cwd: PathBuf,
    },
    SelectWindow {
        index: u32,
    },
    SplitRight,
    SplitDown,
    /// Let go of the session. It carries on.
    Detach,
}

/// The arguments for `action` on `session`, after `-L spagitty`.
pub fn action_args(session: &str, action: &Action) -> Vec<String> {
    let target = format!("{session}:");
    let own = |args: &[&str]| args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    match action {
        Action::NewWindow { cwd } => {
            let mut args = own(&["new-window", "-t", &target, "-c"]);
            args.push(cwd.to_string_lossy().into_owned());
            args
        }
        Action::SelectWindow { index } => {
            own(&["select-window", "-t", &format!("{session}:{index}")])
        }
        // tmux names a split by the line it draws: `-h` puts panes side by side.
        Action::SplitRight => own(&["split-window", "-h", "-t", &target]),
        Action::SplitDown => own(&["split-window", "-v", "-t", &target]),
        Action::Detach => own(&["detach-client", "-s", session]),
    }
}

impl Mux {
    /// Run a command on Spagitty's socket and return what it printed.
    fn run(&self, args: &[String]) -> std::io::Result<String> {
        let output = quiet(&self.path)
            .arg("-L")
            .arg(SOCKET)
            .args(args)
            .stderr(Stdio::piped())
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// The sessions on Spagitty's socket. None at all when no server runs
    /// there, which is the state before the first terminal and not a failure.
    pub fn sessions(&self) -> Vec<Session> {
        self.run(&["list-sessions".into(), "-F".into(), SESSIONS_FORMAT.into()])
            .map(|out| parse_sessions(&out))
            .unwrap_or_default()
    }

    pub fn windows(&self, session: &str) -> Vec<Window> {
        self.run(&[
            "list-windows".into(),
            "-t".into(),
            format!("{session}:"),
            "-F".into(),
            WINDOWS_FORMAT.into(),
        ])
        .map(|out| parse_windows(&out))
        .unwrap_or_default()
    }

    pub fn act(&self, session: &str, action: &Action) -> std::io::Result<()> {
        self.run(&action_args(session, action)).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_of(kind: Kind, version: &str, path: &str) -> Probe {
        let (_, version, major, minor) =
            parse_version(&format!("{} {version}", kind.name())).expect("a version");
        Probe {
            kind,
            path: PathBuf::from(path),
            version,
            major,
            minor,
        }
    }

    #[test]
    fn a_version_is_read_as_each_multiplexer_writes_it() {
        assert_eq!(
            parse_version("tmux 3.3a\n"),
            Some((Kind::Tmux, "3.3a".into(), 3, 3))
        );
        assert_eq!(
            parse_version("tmux 3.5"),
            Some((Kind::Tmux, "3.5".into(), 3, 5))
        );
        assert_eq!(
            parse_version("tmux next-3.6"),
            Some((Kind::Tmux, "next-3.6".into(), 3, 6))
        );
        assert_eq!(
            parse_version("tmux 2.9a"),
            Some((Kind::Tmux, "2.9a".into(), 2, 9))
        );
        assert_eq!(
            parse_version("psmux 3.3.8"),
            Some((Kind::Psmux, "3.3.8".into(), 3, 3))
        );
        assert_eq!(
            parse_version("psmux v3.4.0\r\n"),
            Some((Kind::Psmux, "3.4.0".into(), 3, 4))
        );
        // psmux answers to tmux as well, and says so first.
        assert_eq!(
            parse_version("tmux 3.3.8\npsmux 3.3.8 (66cf613 2026-08-18)\n"),
            Some((Kind::Psmux, "3.3.8".into(), 3, 3))
        );
        // OpenBSD's has no number of its own, and is taken as recent.
        assert_eq!(
            parse_version("tmux openbsd-7.4"),
            Some((Kind::Tmux, "openbsd-7.4".into(), 3, 2))
        );
    }

    #[test]
    fn what_is_not_a_multiplexer_has_no_version() {
        for output in [
            "",
            "\n",
            "screen 4.9",
            "tmux",
            "tmux master",
            "bash: tmux: not found",
        ] {
            assert_eq!(parse_version(output), None, "{output:?}");
        }
    }

    /// The item: the system tmux is used when it is at least 3.2; otherwise
    /// the bundled one.
    #[test]
    fn a_system_tmux_is_used_from_three_point_two() {
        assert_eq!(TMUX_MIN, (3, 2));
        let bundled = || Some(probe_of(Kind::Tmux, "3.7c", "/app/multiplexer/tmux"));

        for recent in ["3.2", "3.2a", "3.4", "4.0", "next-3.6"] {
            let chosen = choose(
                Some(probe_of(Kind::Tmux, recent, "/usr/bin/tmux")),
                bundled(),
                false,
            )
            .expect("a multiplexer");
            assert_eq!(chosen.path, PathBuf::from("/usr/bin/tmux"), "{recent}");
            assert!(!chosen.bundled, "{recent}");
            assert_eq!(chosen.version, recent);
        }
    }

    #[test]
    fn an_older_system_tmux_is_passed_over_for_the_bundled_one() {
        for old in ["3.1", "3.1c", "2.9a", "1.8"] {
            let chosen = choose(
                Some(probe_of(Kind::Tmux, old, "/usr/bin/tmux")),
                Some(probe_of(Kind::Tmux, "3.7c", "/app/multiplexer/tmux")),
                false,
            )
            .expect("a multiplexer");
            assert!(chosen.bundled, "{old}");
            assert_eq!(chosen.version, "3.7c");
            assert_eq!(chosen.path, PathBuf::from("/app/multiplexer/tmux"));
        }
    }

    #[test]
    fn with_no_system_tmux_the_bundled_one_is_used() {
        let chosen = choose(
            None,
            Some(probe_of(Kind::Tmux, "3.7c", "/app/multiplexer/tmux")),
            false,
        )
        .expect("a multiplexer");

        assert!(chosen.bundled);
    }

    /// With none that can run, the terminal is a plain one, and nothing is said.
    #[test]
    fn with_neither_there_is_none_and_an_old_one_alone_is_not_used() {
        assert_eq!(choose(None, None, false), None);
        assert_eq!(choose(None, None, true), None);
        assert_eq!(
            choose(
                Some(probe_of(Kind::Tmux, "3.1", "/usr/bin/tmux")),
                None,
                false
            ),
            None
        );
    }

    #[test]
    fn windows_uses_the_psmux_that_shipped_and_otherwise_one_on_the_path() {
        let shipped = probe_of(Kind::Psmux, "3.3.8", "C:/app/multiplexer/psmux.exe");
        let own = probe_of(Kind::Psmux, "3.4.0", "C:/tools/psmux.exe");

        let chosen = choose(Some(own.clone()), Some(shipped), true).expect("a multiplexer");
        assert!(chosen.bundled);
        assert_eq!(chosen.kind, Kind::Psmux);
        assert_eq!(chosen.version, "3.3.8");

        let chosen = choose(Some(own), None, true).expect("a multiplexer");
        assert!(!chosen.bundled);
        assert_eq!(chosen.path, PathBuf::from("C:/tools/psmux.exe"));
    }

    #[test]
    fn a_session_is_named_for_the_repositorys_folder() {
        /// The name without its six letters, and that it has them.
        fn folder_of(path: &str) -> String {
            let name = session_name(Path::new(path));
            let (folder, mark) = name.rsplit_once('-').expect("a mark");
            assert_eq!(mark.len(), 6, "{name}");
            assert!(mark.bytes().all(|b| b.is_ascii_hexdigit()), "{name}");
            folder.to_string()
        }

        assert_eq!(folder_of("/home/ada/Projects/spagitty"), "spagitty");
        // A Windows path is one only on Windows: elsewhere a backslash is a
        // letter in a name, not what separates two.
        #[cfg(windows)]
        assert_eq!(folder_of("C:\\Users\\ada\\quiblo"), "quiblo");
        // tmux reads `.` and `:` as separators in a target.
        assert_eq!(folder_of("/srv/my.app"), "my_app");
        assert_eq!(folder_of("/srv/a:b"), "a_b");
        assert_eq!(folder_of("/srv/core banking"), "core banking");
        assert_eq!(folder_of("/"), "spagitty");
        // No repository has no path, and nothing to tell apart.
        assert_eq!(session_name(Path::new("")), "spagitty");
        // Nothing in a name is one of tmux's separators.
        for path in ["/srv/my.app", "/srv/a:b", "/home/ada/Projects/spagitty"] {
            let name = session_name(Path::new(path));
            assert!(!name.contains('.') && !name.contains(':'), "{name}");
        }
    }

    /// `new-session -A` joins a session that is there and ignores the folder
    /// it was given, so a name two repositories share is a shell in the wrong
    /// one.
    #[test]
    fn two_repositories_with_one_folder_name_have_sessions_of_their_own() {
        let work = session_name(Path::new("/work/api"));
        let personal = session_name(Path::new("/personal/api"));

        assert_ne!(work, personal);
        assert!(work.starts_with("api-") && personal.starts_with("api-"));
        // The same repository is the same session, every time it is asked.
        assert_eq!(work, session_name(Path::new("/work/api")));
        assert_eq!(work, session_name(Path::new("/work/api/")));
        // And the same on every machine and in every run: the name is kept
        // by a server that outlives the application. These are FNV-1a's own
        // published values.
        assert_eq!(fnv(b""), 0x811c_9dc5);
        assert_eq!(fnv(b"a"), 0xe40c_292c);
    }

    /// On Windows a path is written with either slash, and it is one folder.
    #[cfg(windows)]
    #[test]
    fn on_windows_a_path_written_with_either_slash_is_one_session() {
        assert_eq!(
            session_name(Path::new("C:\\Users\\Ada\\Quiblo")),
            session_name(Path::new("C:/Users/Ada/Quiblo"))
        );
        // The six letters do not depend on the case it was typed in. The
        // folder's own name is kept as it is spelt.
        let spelt = session_name(Path::new("C:\\Users\\Ada\\Quiblo"));
        let lower = session_name(Path::new("c:\\users\\ada\\quiblo"));
        assert_eq!(
            spelt.rsplit_once('-').map(|n| n.1),
            lower.rsplit_once('-').map(|n| n.1)
        );
        assert!(spelt.starts_with("Quiblo-"));
    }

    /// The item: `new-session -A -s <name> -c <path>`, on Spagitty's own socket.
    #[test]
    fn attaching_makes_the_session_if_it_is_not_there_on_spagittys_own_socket() {
        assert_eq!(SOCKET, "spagitty");
        assert_eq!(
            attach_args("spagitty", Path::new("/repos/spagitty"), None),
            [
                "-L",
                "spagitty",
                "new-session",
                "-A",
                "-s",
                "spagitty",
                "-c",
                "/repos/spagitty"
            ]
        );
        // The person's own shell, when they chose one, and not a blank one.
        assert_eq!(
            attach_args("s", Path::new("/r"), Some("fish"))
                .last()
                .map(String::as_str),
            Some("fish")
        );
        assert_eq!(attach_args("s", Path::new("/r"), Some("  ")).len(), 8);
    }

    /// Closing Spagitty detaches and never kills: nothing here can kill a
    /// session or the server.
    #[test]
    fn no_action_kills_a_session() {
        let actions = [
            Action::NewWindow {
                cwd: PathBuf::from("/r"),
            },
            Action::SelectWindow { index: 2 },
            Action::SplitRight,
            Action::SplitDown,
            Action::Detach,
        ];
        for action in &actions {
            let args = action_args("spagitty", action);
            assert!(
                !args.iter().any(|arg| arg.starts_with("kill")),
                "{action:?} → {args:?}"
            );
        }
        assert_eq!(
            action_args("spagitty", &Action::Detach),
            ["detach-client", "-s", "spagitty"]
        );
        assert_eq!(
            action_args("spagitty", &Action::SplitRight),
            ["split-window", "-h", "-t", "spagitty:"]
        );
        assert_eq!(action_args("spagitty", &Action::SplitDown)[1], "-v");
        assert_eq!(
            action_args("spagitty", &Action::SelectWindow { index: 2 }),
            ["select-window", "-t", "spagitty:2"]
        );
        assert_eq!(
            action_args("spagitty", &actions[0]),
            ["new-window", "-t", "spagitty:", "-c", "/r"]
        );
    }

    #[test]
    fn sessions_and_windows_are_read_from_what_the_multiplexer_lists() {
        let sessions = parse_sessions("spagitty\t3\t1\nquiblo\t1\t0\n\nnot a session\n");
        assert_eq!(
            sessions,
            [
                Session {
                    name: "spagitty".into(),
                    windows: 3,
                    attached: true
                },
                Session {
                    name: "quiblo".into(),
                    windows: 1,
                    attached: false
                }
            ]
        );

        let windows = parse_windows("1\tshell\t1\n2\ttests\t0\n3\tmy logs\t0\n");
        assert_eq!(windows.len(), 3);
        assert!(windows[0].active);
        assert_eq!(windows[2].name, "my logs");
        assert_eq!(windows[2].index, 3);
        assert!(parse_sessions("").is_empty());
        assert!(parse_windows("no server running on /tmp/tmux-1000/spagitty").is_empty());
    }

    #[test]
    fn a_file_that_is_not_a_multiplexer_is_not_one() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let fake = dir
            .path()
            .join(if cfg!(windows) { "tmux.exe" } else { "tmux" });
        std::fs::write(&fake, b"not a program").expect("a file");

        assert_eq!(probe(&fake), None);
        assert_eq!(probe(&dir.path().join("missing")), None);
        // Found by name, and then not used because it will not answer.
        assert_eq!(
            find_in(
                std::iter::once(dir.path().to_path_buf()),
                names(cfg!(windows))
            ),
            Some(fake)
        );
    }

    #[test]
    fn the_names_looked_for_are_the_ones_each_platform_has() {
        assert_eq!(names(true), ["psmux.exe", "tmux.exe", "pmux.exe"]);
        assert_eq!(names(false), ["tmux"]);
    }

    /// The whole of it against a real multiplexer, on Spagitty's own socket:
    /// a client in a PTY makes the session and attaches, the session is asked
    /// for a window and a split, and detaching ends the client and leaves the
    /// session. The session is the test's own, and it is the only thing
    /// removed at the end.
    ///
    /// Not run by default, because it needs a multiplexer:
    /// `SPAGITTY_MUX=<path> cargo test -p spagitty-process -- --ignored live`.
    #[test]
    #[ignore = "needs a multiplexer: set SPAGITTY_MUX to its path"]
    fn live_a_client_attaches_through_a_pty_and_detaching_leaves_the_session() {
        use crate::pty::{Pty, PtySpec};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        const WAIT: Duration = Duration::from_secs(30);
        let path = PathBuf::from(std::env::var("SPAGITTY_MUX").expect("SPAGITTY_MUX is set"));
        let found = probe(&path).expect("SPAGITTY_MUX is a multiplexer");
        let mux = Mux {
            kind: found.kind,
            path: found.path,
            version: found.version,
            bundled: true,
        };
        let session = format!("spagitty-live-{}", std::process::id());
        let here = std::env::temp_dir();
        let until = |what: &str, met: &dyn Fn() -> bool| {
            let deadline = Instant::now() + WAIT;
            while !met() {
                assert!(Instant::now() < deadline, "never: {what}");
                std::thread::sleep(Duration::from_millis(200));
            }
        };
        let ours = |mux: &Mux| mux.sessions().into_iter().find(|one| one.name == session);

        let (send, output) = mpsc::channel::<Vec<u8>>();
        let (done, exit) = mpsc::channel();
        let pty = Pty::spawn(
            &PtySpec {
                program: mux.path.to_string_lossy().into_owned(),
                args: attach_args(&session, &here, None),
                cwd: Some(here.clone()),
                env: Vec::new(),
                env_remove: Vec::new(),
                cols: 100,
                rows: 30,
            },
            move |bytes| {
                let _ = send.send(bytes.to_vec());
            },
            move |code| {
                let _ = done.send(code);
            },
        )
        .expect("a client in a pty");

        // The client draws: ConPTY asks where the cursor is first, and waits.
        let first = output.recv_timeout(WAIT).expect("the client printed");
        if String::from_utf8_lossy(&first).contains("[6n") {
            pty.write(b"\x1b[1;1R").expect("an answer");
        }
        until("the session is there and attached", &|| {
            ours(&mux).is_some_and(|one| one.attached)
        });
        assert_eq!(mux.windows(&session).len(), 1);

        mux.act(&session, &Action::NewWindow { cwd: here.clone() })
            .expect("a new window");
        until("a second window", &|| mux.windows(&session).len() == 2);
        let windows = mux.windows(&session);
        assert_eq!(windows.iter().filter(|window| window.active).count(), 1);
        mux.act(
            &session,
            &Action::SelectWindow {
                index: windows[0].index,
            },
        )
        .expect("the first window selected");
        until("the first window is the one shown", &|| {
            mux.windows(&session)
                .first()
                .is_some_and(|window| window.active)
        });
        mux.act(&session, &Action::SplitRight).expect("a split");
        mux.act(&session, &Action::SplitDown).expect("another");

        // Letting go ends the client, and only the client.
        mux.act(&session, &Action::Detach).expect("detached");
        exit.recv_timeout(WAIT).expect("the client ended");
        until("the session is still there, with nobody attached", &|| {
            ours(&mux).is_some_and(|one| !one.attached && one.windows == 2)
        });

        // A client that is killed, as when Spagitty closes, leaves it too.
        let (send, output) = mpsc::channel::<Vec<u8>>();
        let again = Pty::spawn(
            &PtySpec {
                program: mux.path.to_string_lossy().into_owned(),
                args: attach_args(&session, &here, None),
                cwd: Some(here),
                env: Vec::new(),
                env_remove: Vec::new(),
                cols: 100,
                rows: 30,
            },
            move |bytes| {
                let _ = send.send(bytes.to_vec());
            },
            |_| {},
        )
        .expect("a second client");
        let first = output
            .recv_timeout(WAIT)
            .expect("the second client printed");
        if String::from_utf8_lossy(&first).contains("[6n") {
            again.write(b"\x1b[1;1R").expect("an answer");
        }
        until("attached again", &|| {
            ours(&mux).is_some_and(|one| one.attached)
        });
        drop(again);
        until("the session outlives a client that was killed", &|| {
            ours(&mux).is_some_and(|one| !one.attached)
        });

        mux.run(&["kill-session".into(), "-t".into(), session.clone()])
            .expect("the test's own session removed");
        assert!(ours(&mux).is_none());
    }
}
