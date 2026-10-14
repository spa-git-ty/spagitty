// SPDX-License-Identifier: GPL-3.0-or-later

//! The closed hook catalogue (FEAT-137). These values describe a standalone
//! script, so the builder and a terminal commit use the same rules.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    #[serde(rename = "pre-commit")]
    PreCommit,
    #[serde(rename = "commit-msg")]
    CommitMsg,
    #[serde(rename = "pre-push")]
    PrePush,
}

impl Stage {
    pub const ALL: [Self; 3] = [Self::PreCommit, Self::CommitMsg, Self::PrePush];

    pub fn name(self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::CommitMsg => "commit-msg",
            Self::PrePush => "pre-push",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    Tests {
        paths: Vec<String>,
        command: String,
    },
    Format {
        preset: String,
        paths: Vec<String>,
        command: String,
        restage: bool,
    },
    Message {
        style: String,
        types: Vec<String>,
        scope_required: bool,
        subject_max: usize,
        pattern: String,
        ticket_pattern: String,
    },
    Script {
        command: String,
    },
    Secrets {
        patterns: Vec<String>,
    },
    LargeFiles {
        bytes: u64,
    },
    ProtectBranches {
        branches: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Block {
    pub id: String,
    pub label: String,
    pub stage: Stage,
    pub enabled: bool,
    pub warn: bool,
    pub folder: String,
    pub timeout_seconds: u32,
    pub skip_markdown: bool,
    pub skip_wip: bool,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub version: u32,
    pub shared: bool,
    pub active_stages: Vec<Stage>,
    pub blocks: Vec<Block>,
}

impl Default for Definition {
    fn default() -> Self {
        Self {
            version: 1,
            shared: false,
            active_stages: Stage::ALL.to_vec(),
            blocks: Vec::new(),
        }
    }
}

/// Run ids and block ids are also filenames. Never accept a path here.
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}

pub fn validate(definition: &Definition) -> Result<(), String> {
    if definition.version != 1 {
        return Err("This hook definition has an unsupported version.".into());
    }
    if definition.blocks.len() > 100 {
        return Err("A hook definition can contain at most 100 blocks.".into());
    }
    let mut ids = std::collections::HashSet::new();
    for block in &definition.blocks {
        if !identifier(&block.id) || !ids.insert(&block.id) {
            return Err("Each hook block needs a unique filename-safe id.".into());
        }
        if block.label.trim().is_empty() || block.label.len() > 200 {
            return Err("Each hook block needs a name of at most 200 characters.".into());
        }
        if block.timeout_seconds == 0 || block.timeout_seconds > 86400 {
            return Err("A hook timeout must be between 1 second and 24 hours.".into());
        }
        if block.folder.contains(['\0', '\n', '\r'])
            || block.folder.starts_with(['/', '\\'])
            || std::path::Path::new(&block.folder).is_absolute()
            || block
                .folder
                .replace('\\', "/")
                .split('/')
                .any(|p| p == ".." || p.contains(':'))
        {
            return Err("A hook folder must stay inside the repository.".into());
        }
        let text = serde_json::to_string(block).map_err(|e| e.to_string())?;
        if text.len() > 65536 || text.contains("\\u0000") {
            return Err("A hook block is too long or contains a NUL character.".into());
        }
        match &block.action {
            Action::Tests { paths, command } | Action::Format { paths, command, .. } => {
                if paths.is_empty() || paths.iter().any(|p| p.is_empty() || p.starts_with(':')) {
                    return Err(
                        "Choose at least one path glob; pathspec magic is added for you.".into(),
                    );
                }
                if command.trim().is_empty() {
                    return Err("This block needs a command.".into());
                }
            }
            Action::Script { command } if command.trim().is_empty() => {
                return Err("This block needs a command.".into())
            }
            Action::Message {
                style,
                types,
                subject_max,
                pattern,
                ..
            } => {
                if block.stage != Stage::CommitMsg {
                    return Err("Commit message rules belong to commit-msg.".into());
                }
                if !["conventional", "ticket", "regex"].contains(&style.as_str())
                    || *subject_max == 0
                    || *subject_max > 1000
                    || (style == "regex" && pattern.trim().is_empty())
                {
                    return Err(
                        "Choose a message style and a subject limit between 1 and 1000.".into(),
                    );
                }
                if types
                    .iter()
                    .any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_lowercase()))
                {
                    return Err("Commit types use lowercase letters only.".into());
                }
            }
            Action::LargeFiles { bytes: 0 } => {
                return Err("The file size limit must be greater than zero.".into())
            }
            Action::ProtectBranches { branches } if branches.is_empty() => {
                return Err("Choose at least one branch to protect.".into())
            }
            _ => {}
        }
    }
    Ok(())
}

/// The same Conventional Commits shape used to group release notes. Message
/// acceptance itself is performed by the generated commit-msg script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conventional<'a> {
    pub kind: &'a str,
    pub scope: Option<&'a str>,
    pub breaking: bool,
    pub subject: &'a str,
}

pub fn conventional(subject: &str) -> Option<Conventional<'_>> {
    let (prefix, subject) = subject.split_once(": ")?;
    if subject.trim().is_empty() || subject.contains(['\r', '\n']) {
        return None;
    }
    let breaking = prefix.ends_with('!');
    let prefix = prefix.strip_suffix('!').unwrap_or(prefix);
    let (kind, scope) = if let Some((kind, scope)) = prefix.split_once('(') {
        let scope = scope.strip_suffix(')')?;
        if scope.is_empty() || scope.contains(['(', ')', '\n', '\r']) {
            return None;
        }
        (kind, Some(scope))
    } else {
        (prefix, None)
    };
    if kind.is_empty() || !kind.bytes().all(|c| c.is_ascii_lowercase()) {
        return None;
    }
    Some(Conventional {
        kind,
        scope,
        breaking,
        subject,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn script(command: &str) -> Block {
        Block {
            id: "check".into(),
            label: "Check".into(),
            stage: Stage::PreCommit,
            enabled: true,
            warn: false,
            folder: ".".into(),
            timeout_seconds: 30,
            skip_markdown: false,
            skip_wip: false,
            action: Action::Script {
                command: command.into(),
            },
        }
    }

    #[test]
    fn block_ids_are_unique_and_never_paths() {
        let block = script("true");
        let mut definition = Definition {
            blocks: vec![block.clone()],
            ..Definition::default()
        };
        assert!(validate(&definition).is_ok());
        definition.blocks.push(block);
        assert!(validate(&definition).is_err());
        for id in ["../x", "a/b", "a\\b", "", "a\n"] {
            assert!(!identifier(id));
        }
    }

    #[test]
    fn commands_cannot_escape_by_the_folder_and_message_rules_need_the_message_stage() {
        let mut definition = Definition {
            blocks: vec![script("true")],
            ..Definition::default()
        };
        for folder in ["../other", "C:\\other", "/tmp"] {
            definition.blocks[0].folder = folder.into();
            assert!(validate(&definition).is_err());
        }
        definition.blocks[0].folder = ".".into();
        definition.blocks[0].action = Action::Message {
            style: "conventional".into(),
            types: vec!["feat".into()],
            scope_required: false,
            subject_max: 72,
            pattern: String::new(),
            ticket_pattern: String::new(),
        };
        assert!(validate(&definition).is_err());
        definition.blocks[0].stage = Stage::CommitMsg;
        assert!(validate(&definition).is_ok());
    }

    #[test]
    fn release_notes_recognise_the_message_shape_without_rewriting_the_subject() {
        let parsed = conventional("feat(api)!: accept refunds (#12)").expect("conventional");
        assert_eq!(parsed.kind, "feat");
        assert_eq!(parsed.scope, Some("api"));
        assert!(parsed.breaking);
        assert_eq!(parsed.subject, "accept refunds (#12)");
        for bad in [
            "Feat: x",
            "feat(): x",
            "feat: ",
            "feat:no space",
            "fix: x\ny",
        ] {
            assert!(conventional(bad).is_none(), "{bad}");
        }
    }
}
