// SPDX-License-Identifier: GPL-3.0-or-later

//! Spagitty's own preferences.
//!
//! Application state rather than repository state, so it lives here beside
//! [`crate::recents`] rather than in `spagitty-core`: none of it is a fact about
//! a repository, and none of it belongs in `.git/config` where every other tool
//! would read it.
//!
//! The identity — `user.name` and `user.email` — is deliberately *not* here. It
//! is git's own configuration, every tool reads it, and it is handled by
//! `spagitty_core::identity` through `git config`.
//!
//! Commit signing left this file for the same reason (FEAT-019). It was here,
//! as `signCommits`, and nothing read it — but wiring it up would have been the
//! defect rather than the fix: `commit.gpgsign` is the same preference in the
//! place every other tool looks, and two switches for one behaviour disagree
//! the moment one of them is changed outside this application. A settings file
//! written before that still carrying the key is not a problem; unknown keys
//! are ignored on the way in, and the next write drops it.
//!
//! A hand-edited file must not stop the application starting, so anything that
//! does not parse reads as the defaults. That is the same treatment the
//! repository list gets, for the same reason.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use tauri::{AppHandle, Manager, Runtime};

const FILE: &str = "settings.json";

/// How much personality Spagitty is allowed to show (FEAT-072).
///
/// The delight layer is opt-in *intensity*, never opt-in existence: badges are
/// earned at every level and the badge screen works at every level. What this
/// changes is how loudly an unlock arrives — a line in the corner, a reward
/// moment, or the full thing with the jokes and the Hall of Shame.
///
/// `Balanced` is the default because a tool with no acknowledgement at all is
/// the thing this feature exists to fix, and because the level that respects
/// concentration is one step away rather than the starting point.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Personality {
    /// Badges, quietly. No reward moment, no jokes, no Hall of Shame.
    Professional,
    #[default]
    Balanced,
    /// Everything, including the anti-badges and the easter eggs.
    FullSpagitty,
}

/// How loud, if at all (FEAT-072).
///
/// `Off` by default, and that is not a hedge. Every other preference in this
/// file that changes what the application *does* is off until somebody asks for
/// it, and a desktop application that makes a noise the first time it is used
/// gets muted at the operating system — which would lose the sounds worth
/// having along with the one that annoyed somebody.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoundLevel {
    #[default]
    Off,
    Subtle,
    Full,
}

/// Read a value that has a default, falling back rather than failing.
///
/// A hand-edited `"personality": "loud"` must cost the personality, not every
/// other setting in the file. Without this the whole object fails to parse and
/// [`parse`] hands back the defaults for everything — so one typo would silently
/// undo an unrelated toggle somebody had set months earlier.
fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

/// The behaviour toggles on the Settings screen.
///
/// Every field is optional on the way in and written in full on the way out, so
/// a settings file from an older build gains new keys at their defaults instead
/// of being rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Ask the project whether there is a newer Spagitty, at startup.
    ///
    /// On by default, which is the one preference here that changes what the
    /// application *does* rather than what it checks — the others are off for
    /// exactly that reason. It earns the exception by being the only way a
    /// person finds out their client is old: there is no package manager
    /// behind an AppImage or a bare `.exe`, and a security fix nobody hears
    /// about is not a fix. It can be turned off, and turning it off stops
    /// every request.
    pub check_for_updates: bool,
    /// Ask before anything that rewrites history.
    pub confirm_history_rewrite: bool,
    /// Show the `git` command behind each action.
    pub show_git_commands: bool,
    /// Delete remote-tracking refs the remote no longer has, when fetching.
    ///
    /// A setting rather than something that always happens (FEAT-018).
    /// Pruning deletes refs, and a destructive step that nobody chose is the
    /// thing Amendment 6 exists to stop — it was passed on every fetch before
    /// this was added.
    pub prune_on_fetch: bool,
    /// Fetch each author's real picture for the graph's nodes (FEAT-079).
    ///
    /// On by default, and the second preference here to earn that — the
    /// reasoning is the same shape as the update check's. What it costs is one
    /// request per author, once, to a host that already knows the address; what
    /// it buys is a graph where a node says *who*, which is the question a node
    /// is looked at to answer. Off falls back to the generated face, which is
    /// what the graph drew before this existed and is never worse than nothing.
    ///
    /// Turning it off stops every request and empties the cache: a picture
    /// fetched before somebody opted out must not go on being shown from disk
    /// afterwards.
    pub fetch_avatars: bool,
    /// How much personality the delight layer shows (FEAT-072).
    #[serde(deserialize_with = "lenient")]
    pub personality: Personality,
    /// Whether Spagitty makes a sound, and how loud (FEAT-072).
    #[serde(deserialize_with = "lenient")]
    pub sound: SoundLevel,
}

impl Default for Settings {
    /// Off, except the one that asks first and the one that looks for updates.
    ///
    /// A confirmation defaults to on because the cost of asking is a click and
    /// the cost of not asking is a rewritten history. The update check is the
    /// deliberate exception to "a preference the user did not set should not
    /// change what the application does" — its reasoning is on the field.
    /// The rest are off.
    fn default() -> Self {
        Settings {
            check_for_updates: true,
            confirm_history_rewrite: true,
            show_git_commands: false,
            // Off, like the other two that change what the application does.
            // A branch that vanishes from the graph because a fetch pruned it
            // is a surprise, and one nobody asked for.
            prune_on_fetch: false,
            // On, for the reasoning on the field: a node that cannot say who
            // somebody is has not answered the question it is there for.
            fetch_avatars: true,
            // The middle setting: an unlock is acknowledged, and nothing
            // blocks or interrupts. See the type for why this one is not off.
            personality: Personality::Balanced,
            // Silent until asked, like every other preference here that
            // changes what the application does.
            sound: SoundLevel::Off,
        }
    }
}

/// The stored settings, or the defaults.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let Some(path) = file(app) else {
        return Settings::default();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    parse(&text)
}

/// Write the settings, reporting whether they reached the disk.
///
/// A failed write is worth saying out loud here, unlike the repository list: a
/// toggle that silently did not persist looks exactly like one that did until
/// the next restart.
pub fn save<R: Runtime>(app: &AppHandle<R>, settings: Settings) -> Result<(), String> {
    let path =
        file(app).ok_or_else(|| "there is no configuration directory to write to".to_string())?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    std::fs::write(path, text).map_err(|error| error.to_string())
}

/// Anything that is not a settings object reads as the defaults, and a partial
/// object keeps the keys it does carry.
fn parse(text: &str) -> Settings {
    serde_json::from_str(text).unwrap_or_default()
}

fn file<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_reads_as_the_defaults() {
        // The file does not exist until something is toggled, which is the
        // ordinary case rather than an error.
        assert_eq!(parse(""), Settings::default());
    }

    #[test]
    fn asking_before_a_history_rewrite_is_on_by_default() {
        // The cost of asking is a click. The cost of not asking is a rewritten
        // history, so this is the one that defaults to on.
        assert!(Settings::default().confirm_history_rewrite);
        assert!(!Settings::default().show_git_commands);
        // The one other exception, and the field says why it earns it.
        assert!(Settings::default().check_for_updates);
    }

    #[test]
    fn a_hand_edited_file_that_is_not_settings_does_not_stop_the_application() {
        // The file sits in the user's config directory and invites editing.
        for corrupt in ["{", "[]", "null", "not json", "42"] {
            assert_eq!(parse(corrupt), Settings::default(), "for {corrupt:?}");
        }
    }

    #[test]
    fn a_file_from_an_older_build_keeps_what_it_carries_and_defaults_the_rest() {
        let partial = parse(r#"{"showGitCommands": true}"#);

        assert!(partial.show_git_commands);
        assert!(
            partial.confirm_history_rewrite,
            "a key the file predates arrives at its default"
        );
    }

    #[test]
    fn the_delight_layer_is_balanced_and_silent_until_it_is_asked_otherwise() {
        // Badges are earned at every level; what defaults here is how loudly
        // one arrives. Sound is the one that must never start on.
        assert_eq!(Settings::default().personality, Personality::Balanced);
        assert_eq!(Settings::default().sound, SoundLevel::Off);
    }

    #[test]
    fn the_personality_levels_are_stored_as_the_names_the_screen_uses() {
        let text = serde_json::to_string(&Settings {
            personality: Personality::FullSpagitty,
            sound: SoundLevel::Subtle,
            ..Settings::default()
        })
        .expect("serialising");

        assert!(text.contains("\"personality\":\"fullSpagitty\""), "{text}");
        assert!(text.contains("\"sound\":\"subtle\""), "{text}");
    }

    #[test]
    fn a_personality_this_build_does_not_know_costs_only_the_personality() {
        // The file invites hand-editing, and a bad enum value used to fail the
        // whole object — which would quietly undo every other setting in it.
        let settings = parse(r#"{"personality": "loud", "showGitCommands": true}"#);

        assert_eq!(settings.personality, Personality::Balanced);
        assert!(
            settings.show_git_commands,
            "an unreadable personality must not cost an unrelated toggle"
        );
    }

    #[test]
    fn a_key_the_build_does_not_know_is_ignored_rather_than_fatal() {
        // Going back a version must not cost the settings that still apply.
        // `signCommits` is one such key now: it lived here until FEAT-019 moved
        // the preference to `commit.gpgsign`, and a file written before that
        // still carries it.
        let settings = parse(r#"{"signCommits": true, "showGitCommands": true, "later": "on"}"#);

        assert!(settings.show_git_commands);
        assert!(settings.confirm_history_rewrite);
    }

    #[test]
    fn settings_read_back_as_they_were_written() {
        let written = Settings {
            check_for_updates: false,
            confirm_history_rewrite: false,
            show_git_commands: true,
            prune_on_fetch: true,
            fetch_avatars: false,
            personality: Personality::FullSpagitty,
            sound: SoundLevel::Full,
        };
        let text = serde_json::to_string_pretty(&written).expect("serialising");

        assert_eq!(parse(&text), written);
    }

    #[test]
    fn author_pictures_are_fetched_by_default() {
        // The second preference here that changes what the application does
        // and is still on: a node that cannot say who somebody is has not
        // answered the question a node is looked at to answer (FEAT-079).
        assert!(Settings::default().fetch_avatars);
    }

    #[test]
    fn the_stored_keys_are_the_camel_case_names_the_screen_uses() {
        // The file is meant to be readable by hand, and the frontend types name
        // these keys. A rename on either side has to be a deliberate one.
        let text = serde_json::to_string(&Settings::default()).expect("serialising");

        assert!(text.contains("checkForUpdates"), "{text}");
        assert!(text.contains("confirmHistoryRewrite"), "{text}");
        assert!(text.contains("showGitCommands"), "{text}");
        assert!(text.contains("pruneOnFetch"), "{text}");
        assert!(text.contains("fetchAvatars"), "{text}");
        assert!(text.contains("personality"), "{text}");
        assert!(text.contains("sound"), "{text}");
        // And the one that left: writing it again would recreate a second
        // switch for a preference `commit.gpgsign` already holds.
        assert!(!text.contains("signCommits"), "{text}");
    }
}
