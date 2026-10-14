// SPDX-License-Identifier: GPL-3.0-or-later

//! A program run in a pseudo-terminal (FEAT-130).
//!
//! The Terminal screen runs a shell — or a multiplexer's client — the way a
//! terminal emulator does: through a PTY, so the program believes it is at a
//! terminal and behaves like it. `portable-pty` gives one interface over
//! ConPTY on Windows and `openpty` everywhere else.
//!
//! What comes out is bytes, handed to a callback as they are read. They are
//! not decoded here: a terminal's output is escape sequences and text in
//! whatever the program writes, cut wherever a read happened to end, and the
//! emulator is the one that knows how to join it back up.
//!
//! Dropping a [`Pty`] kills what it started. A terminal nobody holds is a
//! process nobody can see.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};

/// How much is read from the terminal at once. A busy program fills this many
/// times a frame; the screen gathers the pieces and writes once a frame.
const READ_BYTES: usize = 32 * 1024;

/// The smallest a terminal is made. A program told it has no columns divides
/// by them.
const MIN_COLS: u16 = 2;
const MIN_ROWS: u16 = 1;

/// What to run, where, and how big the terminal is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtySpec {
    pub program: String,
    pub args: Vec<String>,
    /// The directory it starts in. `None` is this process's own.
    pub cwd: Option<PathBuf>,
    /// Set in the program's environment, over what this process has.
    pub env: Vec<(String, String)>,
    /// Taken out of the program's environment, though this process has them.
    /// `TMUX`, for one: a multiplexer started with it set believes it is
    /// inside another and refuses to attach.
    pub env_remove: Vec<String>,
    pub cols: u16,
    pub rows: u16,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        cols: cols.max(MIN_COLS),
        rows: rows.max(MIN_ROWS),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// A running program and the terminal it is in.
pub struct Pty {
    master: Mutex<Box<dyn MasterPty + Send>>,
    /// The queue the writing thread takes from.
    writer: Mutex<std::sync::mpsc::Sender<Vec<u8>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// Set once the program has exited, so a late kill is not an error.
    exited: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for Pty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pty").finish_non_exhaustive()
    }
}

impl Pty {
    /// Start `spec`'s program in a new terminal.
    ///
    /// `on_output` is called on a thread of its own with each piece read, in
    /// order. `on_exit` is called once, after the last piece, with the
    /// program's exit code — `None` when it was killed and has none.
    pub fn spawn(
        spec: &PtySpec,
        mut on_output: impl FnMut(&[u8]) + Send + 'static,
        on_exit: impl FnOnce(Option<u32>) + Send + 'static,
    ) -> std::io::Result<Pty> {
        let pair = native_pty_system()
            .openpty(size(spec.cols, spec.rows))
            .map_err(other)?;

        let mut command = CommandBuilder::new(&spec.program);
        command.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            command.cwd(cwd);
        }
        // What every terminal program looks at to decide what it may draw.
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        for (key, value) in &spec.env {
            command.env(key, value);
        }
        for key in &spec.env_remove {
            command.env_remove(key);
        }

        let mut child = pair.slave.spawn_command(command).map_err(other)?;
        // The program has its end now. Keeping ours open would keep the
        // terminal alive after the program is gone, and the read would never end.
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(other)?;
        let writer = pair.master.take_writer().map_err(other)?;
        let killer = child.clone_killer();
        let exited = Arc::new(Mutex::new(false));

        let (done, read_done) = std::sync::mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("spagitty-pty-read".into())
            .spawn(move || {
                let mut buffer = vec![0u8; READ_BYTES];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => on_output(&buffer[..count]),
                    }
                }
                let _ = done.send(());
            })?;

        // What is typed is written on a thread of its own, in the order it
        // was typed. A write to a terminal waits when the program is not
        // reading: a paste into `sleep 60` fills what the terminal will hold.
        // Whoever called `write` must not wait with it, because that is the
        // window's own thread. The thread ends when the terminal is dropped,
        // which drops the queue, or when the program's end has closed.
        let (queue, queued) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::Builder::new()
            .name("spagitty-pty-write".into())
            .spawn(move || {
                let mut writer = writer;
                for bytes in queued {
                    if writer
                        .write_all(&bytes)
                        .and_then(|()| writer.flush())
                        .is_err()
                    {
                        break;
                    }
                }
            })?;

        let flag = exited.clone();
        std::thread::Builder::new()
            .name("spagitty-pty-wait".into())
            .spawn(move || {
                let code = child.wait().ok().map(|status| status.exit_code());
                *flag.lock().unwrap_or_else(|p| p.into_inner()) = true;
                // The last of the output first, then the exit: an emulator
                // told "exited" before the final line would lose the line. On
                // Windows the read only ends when the terminal is closed, so
                // it is given a moment and not waited on for ever.
                let _ = read_done.recv_timeout(std::time::Duration::from_millis(400));
                on_exit(code);
            })?;

        Ok(Pty {
            master: Mutex::new(pair.master),
            writer: Mutex::new(queue),
            killer: Mutex::new(killer),
            exited,
        })
    }

    /// Send keystrokes — or anything else — to the program.
    ///
    /// Returns at once: the bytes are queued, and written in order by the
    /// terminal's own thread. An error means the terminal has closed.
    pub fn write(&self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(bytes.to_vec())
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the terminal has closed")
            })
    }

    /// Tell the program its terminal is now this big.
    pub fn resize(&self, cols: u16, rows: u16) -> std::io::Result<()> {
        self.master
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .resize(size(cols, rows))
            .map_err(other)
    }

    /// True once the program has exited.
    pub fn has_exited(&self) -> bool {
        *self.exited.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Stop the program. Nothing to do, and no error, when it has already gone.
    pub fn kill(&self) {
        if self.has_exited() {
            return;
        }
        let _ = self.killer.lock().unwrap_or_else(|p| p.into_inner()).kill();
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.kill();
    }
}

/// `portable-pty` reports failures in its own error type. They leave this
/// module as `io::Error`, which is what the rest of the crate speaks.
fn other(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    const WAIT: Duration = Duration::from_secs(20);

    /// A shell that reads commands from its terminal, on this platform.
    fn shell() -> PtySpec {
        let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
            ("cmd.exe", vec!["/Q", "/D"])
        } else {
            ("/bin/sh", vec![])
        };
        PtySpec {
            program: program.to_string(),
            args: args.into_iter().map(str::to_string).collect(),
            cwd: None,
            env: Vec::new(),
            env_remove: Vec::new(),
            cols: 80,
            rows: 24,
        }
    }

    struct Running {
        pty: Pty,
        output: mpsc::Receiver<Vec<u8>>,
        exit: mpsc::Receiver<Option<u32>>,
        seen: String,
    }

    fn start(spec: &PtySpec) -> Running {
        let (send, output) = mpsc::channel();
        let (done, exit) = mpsc::channel();
        let pty = Pty::spawn(
            spec,
            move |bytes| {
                let _ = send.send(bytes.to_vec());
            },
            move |code| {
                let _ = done.send(code);
            },
        )
        .expect("a pty");
        Running {
            pty,
            output,
            exit,
            seen: String::new(),
        }
    }

    impl Running {
        /// Read until `wanted` has been seen, or fail with what was.
        fn until(&mut self, wanted: &str) {
            let deadline = std::time::Instant::now() + WAIT;
            while !self.seen.contains(wanted) {
                let left = deadline.saturating_duration_since(std::time::Instant::now());
                match self.output.recv_timeout(left) {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        self.answer(&text);
                        self.seen.push_str(&text);
                    }
                    Err(_) => panic!("never saw {wanted:?} in {:?}", self.seen),
                }
            }
        }

        /// Be the emulator, as far as a program needs one to start.
        ///
        /// ConPTY opens by asking where the cursor is (`ESC [ 6 n`) and waits
        /// for the answer before it runs anything. xterm.js answers by
        /// itself; a test that reads bytes has to.
        fn answer(&self, text: &str) {
            if text.contains("\x1b[6n") {
                self.pty.write(b"\x1b[1;1R").expect("an answer");
            }
        }

        fn line(&self, text: &str) {
            let end = if cfg!(windows) { "\r\n" } else { "\n" };
            self.pty
                .write(format!("{text}{end}").as_bytes())
                .expect("a write");
        }
    }

    #[test]
    fn what_is_written_to_it_is_run_and_what_it_prints_comes_back() {
        let mut running = start(&shell());

        // The parts are joined by the shell, so the word is only ever seen
        // whole in what the command printed, never in the echo of typing it.
        if cfg!(windows) {
            running.line("set A=spag&& set B=itty");
            running.line("echo %A%%B%-ran");
        } else {
            running.line("A=spag; B=itty; echo \"$A$B-ran\"");
        }
        running.until("spagitty-ran");
    }

    #[test]
    fn its_program_is_told_the_size_and_the_new_one() {
        let mut spec = shell();
        spec.cols = 91;
        spec.rows = 17;
        let mut running = start(&spec);

        let ask = if cfg!(windows) {
            "mode con"
        } else {
            "stty size"
        };
        running.line(ask);
        // `stty size` prints "rows cols"; `mode con` prints them on two lines.
        if cfg!(windows) {
            running.until("91");
        } else {
            running.until("17 91");
        }

        running.pty.resize(123, 31).expect("a resize");
        running.line(ask);
        if cfg!(windows) {
            running.until("123");
        } else {
            running.until("31 123");
        }
    }

    #[test]
    fn it_says_when_its_program_exits_and_with_what() {
        let mut running = start(&shell());
        // Wait for the shell to be reading before it is told to go.
        running.line(if cfg!(windows) {
            "echo re^ady"
        } else {
            "echo re\"\"ady"
        });
        running.until("ready");

        running.line("exit 7");

        let code = running.exit.recv_timeout(WAIT).expect("an exit");
        assert_eq!(code, Some(7));
        assert!(running.pty.has_exited());
        // Killing what has gone is nothing, not an error.
        running.pty.kill();
        let _ = &mut running.seen;
    }

    #[test]
    fn killing_it_ends_its_program() {
        let running = start(&shell());

        running.pty.kill();

        running
            .exit
            .recv_timeout(WAIT)
            .expect("an exit after a kill");
    }

    #[test]
    fn dropping_it_ends_its_program() {
        let Running { pty, exit, .. } = start(&shell());

        drop(pty);

        exit.recv_timeout(WAIT).expect("an exit after the drop");
    }

    #[test]
    fn it_starts_where_it_is_told_and_with_what_it_is_given() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut spec = shell();
        spec.cwd = Some(dir.path().to_path_buf());
        spec.env = vec![("SPAGITTY_PTY_TEST".into(), "given".into())];
        let mut running = start(&spec);

        if cfg!(windows) {
            running.line("echo %SPAGITTY_PTY_TEST%-%TERM%");
            running.until("given-xterm-256color");
            running.line("cd");
        } else {
            running.line("echo \"$SPAGITTY_PTY_TEST-$TERM\"");
            running.until("given-xterm-256color");
            running.line("pwd");
        }
        let name = dir
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a name")
            .to_string();
        running.until(&name);
    }

    /// `TMUX` is the one this is for: a multiplexer that finds it set
    /// believes it is inside another.
    #[test]
    fn what_is_taken_out_of_its_environment_is_not_there() {
        let mut spec = shell();
        spec.env = vec![("SPAGITTY_PTY_GONE".into(), "here".into())];
        spec.env_remove = vec!["SPAGITTY_PTY_GONE".into()];
        let mut running = start(&spec);

        // What is typed is echoed, so the answer is spelt in a way the
        // question is not.
        if cfg!(windows) {
            running.line("if not defined SPAGITTY_PTY_GONE echo gone-y^es");
        } else {
            running.line("echo \"gone-${SPAGITTY_PTY_GONE:-yes}\"");
        }
        running.until("gone-yes");
    }

    /// A paste into a program that is not reading must not hold the caller:
    /// the caller is the window's own thread.
    #[test]
    fn a_write_does_not_wait_for_a_program_that_is_not_reading() {
        let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
            ("cmd.exe", vec!["/D", "/C", "ping -n 8 127.0.0.1 >nul"])
        } else {
            ("/bin/sh", vec!["-c", "sleep 6"])
        };
        let spec = PtySpec {
            program: program.to_string(),
            args: args.into_iter().map(str::to_string).collect(),
            ..shell()
        };
        let running = start(&spec);
        let pasted = vec![b'x'; 2 * 1024 * 1024];

        let began = std::time::Instant::now();
        running.pty.write(&pasted).expect("queued");
        running.pty.write(b"more").expect("queued");

        assert!(
            began.elapsed() < Duration::from_secs(2),
            "writing waited {:?} for a program that was not reading",
            began.elapsed()
        );
    }

    #[test]
    fn a_program_that_does_not_exist_is_an_error_and_not_a_terminal() {
        let mut spec = shell();
        spec.program = "spagitty-no-such-program".into();

        assert!(Pty::spawn(&spec, |_| {}, |_| {}).is_err());
    }

    #[test]
    fn a_terminal_is_never_made_with_no_room_in_it() {
        assert_eq!(size(0, 0).cols, MIN_COLS);
        assert_eq!(size(0, 0).rows, MIN_ROWS);
        assert_eq!(size(120, 40).cols, 120);
    }
}
