// SPDX-License-Identifier: GPL-3.0-or-later

//! Commit signing: what git would do, and whether it can.
//!
//! # `commit.gpgsign` is the authority
//!
//! Settings once carried a "Sign my commits" toggle of its own, in Spagitty's
//! own preferences file, that nothing read. Even wired up it would have been a
//! defect: `commit.gpgsign` is the same preference expressed in the place every
//! other git tool reads, and two switches for one behaviour disagree the moment
//! one of them is changed outside this application.
//!
//! So there is one switch. [`read`] reports `commit.gpgsign` as git resolves
//! it, [`set`] writes it with `git config`, and a repository configured on the
//! command line shows as configured here. It is scoped exactly like the
//! identity, and for the same reason — a signing preference is often global and
//! sometimes overridden for one repository.
//!
//! # This module does not verify anything
//!
//! [`signed_by_header`] says a commit *carries* a signature, which is a fact
//! about the object and costs nothing to read. Whether that signature is valid,
//! and whether the key behind it is one you trust, is a different question with
//! a different answer per machine, and answering it means a subprocess per
//! commit. The graph says "signed"; it does not say "verified", and the screens
//! are careful to keep those apart.
//!
//! # Nothing here waits for a human
//!
//! `shell::run` sets `GIT_TERMINAL_PROMPT=0`, so a signing program that wants a
//! passphrase on a terminal fails rather than hanging the application. A GPG
//! agent with a graphical pinentry still asks, which is the behaviour a user
//! configured on purpose and gets everywhere else.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::identity::{Origin, Scope};
use crate::shell;
use crate::{Error, Result};

/// The config key that decides whether a commit is signed.
pub const KEY: &str = "commit.gpgsign";

/// Which signing machinery git is configured to use — `gpg.format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// The default. Signs with GPG, and can find a key from the committer's
    /// email when `user.signingkey` is not set.
    OpenPgp,
    /// Signs with an ssh key. Needs `user.signingkey`: there is no email to
    /// look a key up by.
    Ssh,
    /// S/MIME, through `gpgsm`.
    X509,
}

impl Format {
    fn parse(raw: &str) -> Format {
        match raw.trim() {
            "ssh" => Format::Ssh,
            "x509" => Format::X509,
            _ => Format::OpenPgp,
        }
    }

    /// The config key naming this format's program, and the program git falls
    /// back to when that key is unset.
    fn program(self) -> (&'static str, &'static str) {
        match self {
            Format::OpenPgp => ("gpg.program", "gpg"),
            Format::Ssh => ("gpg.ssh.program", "ssh-keygen"),
            Format::X509 => ("gpg.x509.program", "gpgsm"),
        }
    }
}

/// Why a commit that is meant to be signed would not be.
///
/// Both are checkable before the commit is attempted, which is the point: the
/// item this comes from asks that a repository with no working signer be told
/// so *at* the point of commit rather than after it fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "detail")]
pub enum Problem {
    /// The signing program is not on `PATH`.
    MissingProgram(String),
    /// SSH signing with no `user.signingkey`. Unlike GPG, ssh has no email to
    /// find a key by, so there is nothing for it to fall back to.
    NoSigningKey,
}

impl Problem {
    /// What the screen says. Plain, specific, and it names the program.
    pub fn message(&self) -> String {
        match self {
            Problem::MissingProgram(program) => {
                format!("signing is on, but {program} is not installed")
            }
            Problem::NoSigningKey => {
                "signing is on with an ssh key, but user.signingkey is not set".into()
            }
        }
    }
}

/// Signing, as git would resolve it here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signing {
    /// `commit.gpgsign`, effective. What a commit made now would do.
    pub enabled: bool,
    /// Which file `enabled` came from. [`Origin::Unset`] means nothing sets it
    /// and the default — off — stands.
    pub origin: Origin,
    pub format: Format,
    /// `user.signingkey`, effective.
    pub key: Option<String>,
    /// The program git would run for this format.
    pub program: String,
    /// What would stop a signature happening. `None` when signing is off, since
    /// a signer that is not there cannot fail to be used.
    pub problem: Option<Problem>,
    /// False when no repository is open: the local scope is then neither
    /// readable nor writable, and the screen offers the global one alone.
    pub repository: bool,
    /// `commit.gpgsign` as each writable scope holds it, so the screen can show
    /// a repository override beside the global value it hides.
    pub global: Option<bool>,
    pub local: Option<bool>,
}

/// Read signing from a repository's full configuration cascade.
pub fn read(repo: &gix::Repository) -> Signing {
    let snapshot = repo.config_snapshot();
    build(snapshot.plumbing(), true)
}

/// Read signing with no repository open: system, global and environment.
pub fn read_global() -> Result<Signing> {
    let globals =
        gix::config::File::from_globals().map_err(|error| Error::Config(error.to_string()))?;
    Ok(build(&globals, false))
}

/// Turn signing on or off in one scope.
///
/// Written rather than unset in both directions. `commit.gpgsign = false` is a
/// deliberate "not here", which is exactly what turning it off in a repository
/// whose global setting is on has to mean; unsetting would let the global value
/// through and the switch would spring back on.
pub fn set(dir: &Path, scope: Scope, on: bool) -> Result<()> {
    shell::set_config(dir, scope.flag(), KEY, if on { "true" } else { "false" })
}

/// Clear the key in one scope, so the next one up decides again.
pub fn clear(dir: &Path, scope: Scope) -> Result<()> {
    shell::unset_config(dir, scope.flag(), KEY)
}

/// Does this commit carry a signature?
///
/// The `gpgsig` header on the commit object. Present means it was signed;
/// absent means it was not. It says nothing about whether the signature is
/// valid — see the module header.
pub fn signed(commit: &gix::Commit<'_>) -> bool {
    commit
        .decode()
        .map(|commit| signed_by_header(&commit))
        .unwrap_or(false)
}

/// The header test on its own, so it can be exercised without a repository.
pub fn signed_by_header(commit: &gix::objs::CommitRef<'_>) -> bool {
    // `gpgsig` for GPG and S/MIME, `gpgsig-sha256` in a sha256 repository.
    // An ssh signature is written under `gpgsig` too — the header name is
    // about the field, not the algorithm.
    commit
        .extra_headers()
        .find("gpgsig")
        .or_else(|| commit.extra_headers().find("gpgsig-sha256"))
        .is_some()
}

/// Tell a signing failure apart from every other reason a commit fails.
///
/// git reports one as an ordinary commit failure with a line about signing in
/// the stderr, so an unrecognised failure stays what it was rather than being
/// relabelled — a hook that rejected the commit must not be reported as a
/// signing problem.
pub fn as_signing_failure(error: Error, program: &str) -> Error {
    let Error::Git { command, stderr } = error else {
        return error;
    };

    let lowered = stderr.to_lowercase();
    let signing = lowered.contains("failed to sign")
        || lowered.contains("gpg failed")
        || lowered.contains("error: unable to sign")
        || lowered.contains("signing failed");

    if signing {
        Error::Signing {
            program: program.to_string(),
            stderr,
        }
    } else {
        Error::Git { command, stderr }
    }
}

/// Fold a configuration file into what the screen shows.
fn build(config: &gix::config::File, repository: bool) -> Signing {
    let (enabled, origin, global, local) = boolean(config, KEY);
    let format = Format::parse(&string(config, "gpg.format").unwrap_or_default());

    let (program_key, fallback) = format.program();
    let program = string(config, program_key)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| fallback.to_string());

    let key = string(config, "user.signingkey").filter(|value| !value.trim().is_empty());

    let problem = if !enabled {
        None
    } else if format == Format::Ssh && key.is_none() {
        Some(Problem::NoSigningKey)
    } else if !on_path(&program) {
        Some(Problem::MissingProgram(program.clone()))
    } else {
        None
    };

    Signing {
        enabled,
        origin,
        format,
        key,
        program,
        problem,
        repository,
        global,
        local,
    }
}

/// The last value of `key` in the cascade, as a string.
fn string(config: &gix::config::File, key: &str) -> Option<String> {
    let occurrences = config
        .values_with_sections::<gix::bstr::BString>(key)
        .ok()?;
    occurrences.last().map(|(raw, _)| raw.to_string())
}

/// The last value of `key` as a boolean, plus where it came from and what each
/// writable scope holds.
///
/// Same rule as the identity's: `values_with_sections` yields in ascending
/// precedence, so keeping the last of each thing is what `git config --get`
/// does.
fn boolean(config: &gix::config::File, key: &str) -> (bool, Origin, Option<bool>, Option<bool>) {
    let mut effective = false;
    let mut source = Origin::Unset;
    let mut global = None;
    let mut local = None;

    let Ok(occurrences) = config.values_with_sections::<gix::bstr::BString>(key) else {
        return (effective, source, global, local);
    };

    for (raw, section) in occurrences {
        let Some(value) = git_boolean(&raw.to_string()) else {
            // A key set to something that is not a boolean is git's error to
            // report when it runs, not a reason for this read to fail.
            continue;
        };

        match crate::identity::writable(section.meta().source) {
            Some(Scope::Global) => global = Some(value),
            Some(Scope::Local) => local = Some(value),
            None => {}
        }

        source = crate::identity::origin(section.meta().source);
        effective = value;
    }

    (effective, source, global, local)
}

/// git's own idea of a boolean, which is wider than `"true"`.
///
/// A key present with no value at all — `[commit]\n\tgpgsign` — is true, the
/// same as a bare flag on a command line.
fn git_boolean(raw: &str) -> Option<bool> {
    match raw.trim().to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" | "" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Is `program` something that can be run?
///
/// Asked by running it with `--version` rather than by searching `PATH`
/// ourselves: a configured program can be an absolute path, a name on `PATH`,
/// or a name that resolves differently under the user's shell, and the only
/// answer that matters is whether spawning it works. Every signing program in
/// use understands `--version` and returns promptly.
///
/// A program that exists but exits non-zero still counts as present. The
/// question here is "is it installed", and a bad exit code from `--version` is
/// a different problem git will report in its own words.
///
/// Answered once per program per session. This read happens on every status
/// refresh, and the filesystem watcher can make that several a second — a
/// subprocess each time would be a real cost for an answer that does not change
/// while the application is running. Installing gpg mid-session and expecting
/// Spagitty to notice without a restart is not a case worth paying for.
fn on_path(program: &str) -> bool {
    static ANSWERED: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

    let mut guard = ANSWERED.lock().expect("the program cache");
    let cache = guard.get_or_insert_with(HashMap::new);
    if let Some(known) = cache.get(program) {
        return *known;
    }

    let found = crate::shell::program(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok();

    cache.insert(program.to_string(), found);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// A configuration file with a chosen source, so a cascade can be assembled
    /// without a real `~/.gitconfig` anywhere near it.
    fn layer(source: gix::config::Source, text: &str) -> gix::config::File {
        let mut bytes = text.as_bytes().to_vec();
        gix::config::File::from_bytes_owned(
            &mut bytes,
            gix::config::file::Metadata::from(source),
            Default::default(),
        )
        .expect("a parsable configuration layer")
    }

    fn cascade(layers: Vec<gix::config::File>) -> gix::config::File {
        let mut merged = gix::config::File::default();
        for layer in layers {
            merged.append(layer).expect("appending a layer");
        }
        merged
    }

    fn of(text: &str) -> Signing {
        build(&layer(gix::config::Source::Local, text), true)
    }

    #[test]
    fn signing_is_off_when_nothing_sets_it() {
        let signing = of("");

        assert!(!signing.enabled);
        assert_eq!(signing.origin, Origin::Unset);
        assert_eq!(signing.problem, None);
    }

    #[test]
    fn a_repository_that_signs_says_so_and_says_where_from() {
        let signing = of("[commit]\n\tgpgsign = true\n");

        assert!(signing.enabled);
        assert_eq!(signing.origin, Origin::Local);
        assert_eq!(signing.local, Some(true));
        assert_eq!(signing.global, None);
    }

    #[test]
    fn a_repository_can_turn_signing_off_against_a_global_yes() {
        // The reason `set` writes `false` rather than unsetting: the local value
        // has to be able to say "not here".
        let signing = build(
            &cascade(vec![
                layer(gix::config::Source::User, "[commit]\n\tgpgsign = true\n"),
                layer(gix::config::Source::Local, "[commit]\n\tgpgsign = false\n"),
            ]),
            true,
        );

        assert!(!signing.enabled);
        assert_eq!(signing.origin, Origin::Local);
        assert_eq!(signing.global, Some(true));
        assert_eq!(signing.local, Some(false));
    }

    #[test]
    fn every_spelling_git_accepts_for_a_boolean_is_accepted_here() {
        for on in ["true", "yes", "on", "1", "TRUE", "On"] {
            assert!(
                of(&format!("[commit]\n\tgpgsign = {on}\n")).enabled,
                "for {on}"
            );
        }
        for off in ["false", "no", "off", "0", "FALSE"] {
            assert!(
                !of(&format!("[commit]\n\tgpgsign = {off}\n")).enabled,
                "for {off}"
            );
        }
    }

    #[test]
    fn a_bare_key_with_no_value_is_true_as_it_is_on_a_command_line() {
        assert!(of("[commit]\n\tgpgsign\n").enabled);
    }

    #[test]
    fn a_value_that_is_not_a_boolean_is_left_for_git_to_complain_about() {
        // Not an error here: this read exists to describe the configuration,
        // and refusing to describe it would take the screen away as well.
        let signing = of("[commit]\n\tgpgsign = maybe\n");

        assert!(!signing.enabled);
        assert_eq!(signing.origin, Origin::Unset);
    }

    #[test]
    fn the_format_decides_which_program_git_would_run() {
        assert_eq!(of("").program, "gpg");
        assert_eq!(of("[gpg]\n\tformat = ssh\n").program, "ssh-keygen");
        assert_eq!(of("[gpg]\n\tformat = x509\n").program, "gpgsm");
    }

    #[test]
    fn a_configured_program_wins_over_the_default_for_its_format() {
        let signing =
            of("[gpg]\n\tformat = ssh\n\t[gpg \"ssh\"]\n\tprogram = /usr/bin/ssh-keygen\n");

        assert_eq!(signing.format, Format::Ssh);
        assert_eq!(signing.program, "/usr/bin/ssh-keygen");
    }

    #[test]
    fn an_unknown_format_reads_as_openpgp_which_is_gits_default() {
        assert_eq!(of("[gpg]\n\tformat = smoke\n").format, Format::OpenPgp);
    }

    #[test]
    fn ssh_signing_with_no_key_is_a_problem_named_before_it_happens() {
        // ssh has no email to look a key up by, so this cannot work and can be
        // said in advance rather than discovered by a failed commit.
        let signing = of("[commit]\n\tgpgsign = true\n[gpg]\n\tformat = ssh\n");

        assert_eq!(signing.problem, Some(Problem::NoSigningKey));
        assert!(signing
            .problem
            .unwrap()
            .message()
            .contains("user.signingkey"));
    }

    #[test]
    fn ssh_signing_with_a_key_gets_as_far_as_looking_for_the_program() {
        let signing = of(
            "[commit]\n\tgpgsign = true\n[gpg]\n\tformat = ssh\n[user]\n\tsigningkey = /k/id_ed25519.pub\n",
        );

        assert_eq!(signing.key.as_deref(), Some("/k/id_ed25519.pub"));
        // Whichever it is, it is no longer the missing-key problem.
        assert_ne!(signing.problem, Some(Problem::NoSigningKey));
    }

    #[test]
    fn a_signing_program_that_is_not_installed_is_named() {
        let signing =
            of("[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = spagitty-no-such-signer\n");

        assert_eq!(
            signing.problem,
            Some(Problem::MissingProgram("spagitty-no-such-signer".into()))
        );
        assert!(signing
            .problem
            .unwrap()
            .message()
            .contains("spagitty-no-such-signer"));
    }

    #[test]
    fn signing_that_is_off_has_no_problem_to_report() {
        // A signer that is not going to be used cannot fail to be used, and a
        // warning about one would be noise on every commit.
        let signing = of("[gpg]\n\tprogram = spagitty-no-such-signer\n");

        assert!(!signing.enabled);
        assert_eq!(signing.problem, None);
    }

    #[test]
    fn an_empty_program_setting_falls_back_to_the_default() {
        assert_eq!(of("[gpg]\n\tprogram = \n").program, "gpg");
    }

    #[test]
    fn a_hook_refusing_a_commit_is_not_reported_as_a_signing_failure() {
        // The whole reason the classifier looks at the message rather than
        // assuming: a failed commit is usually a failed commit.
        let refused = Error::Git {
            command: "commit -m x".into(),
            stderr: "pre-commit hook refused this commit".into(),
        };

        assert!(matches!(
            as_signing_failure(refused, "gpg"),
            Error::Git { .. }
        ));
    }

    #[test]
    fn gits_own_signing_failure_is_reported_as_one_and_names_the_program() {
        let failed = Error::Git {
            command: "commit -m x".into(),
            stderr: "error: gpg failed to sign the data\nfatal: failed to write commit object"
                .into(),
        };

        match as_signing_failure(failed, "gpg") {
            Error::Signing { program, stderr } => {
                assert_eq!(program, "gpg");
                assert!(stderr.contains("gpg failed to sign"));
            }
            other => panic!("expected a signing failure, got {other:?}"),
        }
    }

    #[test]
    fn a_failure_that_is_not_gits_is_left_exactly_as_it_was() {
        let io = Error::EmptyMessage;
        assert!(matches!(as_signing_failure(io, "gpg"), Error::EmptyMessage));
    }

    #[test]
    fn an_ordinary_commit_carries_no_signature() {
        let fixture = Fixture::woven();
        let repo = fixture.open();
        let head = repo.head_commit().expect("a head commit");

        assert!(!signed(&head));
    }

    #[test]
    fn a_commit_with_a_gpgsig_header_reads_as_signed() {
        // Written by hand with `git hash-object`, because making a real
        // signature in a test would need a key, an agent and a passphrase —
        // none of which a fixture may have. What is under test is the header,
        // and the header is the whole of what this reports.
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("Unsigned");

        let tree = fixture
            .git(&["rev-parse", "HEAD^{tree}"])
            .trim()
            .to_string();
        let raw = format!(
            "tree {tree}\n\
             author Ada Lovelace <ada@example.com> 1700000000 +0000\n\
             committer Ada Lovelace <ada@example.com> 1700000000 +0000\n\
             gpgsig -----BEGIN PGP SIGNATURE-----\n \n \
             not a real signature\n \
             -----END PGP SIGNATURE-----\n\
             \n\
             Signed by hand\n"
        );
        fixture.write("commit.raw", &raw);
        let id = fixture
            .git(&["hash-object", "-t", "commit", "-w", "commit.raw"])
            .trim()
            .to_string();

        let repo = fixture.open();
        let commit = repo
            .find_object(gix::ObjectId::from_hex(id.as_bytes()).expect("an object id"))
            .expect("the handmade commit")
            .into_commit();

        assert!(signed(&commit));
    }

    #[test]
    fn writing_the_key_turns_signing_on_and_off_in_the_repository() {
        let fixture = Fixture::woven();

        set(fixture.path(), Scope::Local, true).expect("turning signing on");
        assert!(read(&fixture.open()).enabled);
        assert_eq!(
            fixture.git(&["config", "--local", KEY]).trim(),
            "true",
            "git must agree, because git is the one that reads it"
        );

        set(fixture.path(), Scope::Local, false).expect("turning signing off");
        assert!(!read(&fixture.open()).enabled);

        clear(fixture.path(), Scope::Local).expect("clearing the key");
        assert_eq!(read(&fixture.open()).local, None);
    }
}
