// SPDX-License-Identifier: GPL-3.0-or-later

//! Read existing manager configuration without taking its hooks over. YAML
//! quoting, folded commands and aliases are the parser's job. Remote
//! pre-commit hooks keep using their installed manager environment.

use super::{
    blocks::{Action, Block, Definition, Stage},
    Manager,
};
use crate::recipes::script::sh_arg;
use std::path::Path;
use yaml_rust2::{Yaml, YamlLoader};

type Result<T> = std::result::Result<T, String>;

fn strings(value: &Yaml) -> Vec<String> {
    if let Some(values) = value.as_vec() {
        values
            .iter()
            .filter_map(Yaml::as_str)
            .map(str::to_string)
            .collect()
    } else {
        value
            .as_str()
            .map(|s| vec![s.to_string()])
            .unwrap_or_default()
    }
}

fn add(
    definition: &mut Definition,
    stage: Stage,
    label: String,
    command: String,
    settings: &Yaml,
) -> Result<()> {
    if command.trim().is_empty() {
        return Err(format!("{label} has no command to import."));
    }
    let mut paths = strings(&settings["glob"]);
    let has_files = command.contains("{staged_files}");
    let command = command
        .replace("\"{staged_files}\"", "\"$@\"")
        .replace("'{staged_files}'", "\"$@\"")
        .replace("{staged_files}", "\"$@\"");
    if command.contains("{all_files}")
        || command.contains("{push_files}")
        || command.contains("{files}")
    {
        return Err(format!("{label} uses a manager file placeholder that cannot be preserved. Its original configuration was left alone."));
    }
    let preset = ["prettier", "eslint", "ruff", "gofmt", "rustfmt", "spotless"]
        .into_iter()
        .find(|tool| {
            command
                .split(|c: char| c.is_whitespace() || c == ':')
                .any(|word| word == *tool)
        });
    let action = if has_files || preset.is_some() {
        if paths.is_empty() {
            paths.push("**".into());
        }
        Action::Format {
            preset: preset.unwrap_or("custom").into(),
            paths,
            command,
            restage: false,
        }
    } else if !paths.is_empty() {
        Action::Tests { paths, command }
    } else {
        Action::Script { command }
    };
    let folder = settings["root"].as_str().unwrap_or(".").to_string();
    definition.blocks.push(Block {
        id: format!("imported-{}", definition.blocks.len() + 1),
        label,
        stage,
        enabled: true,
        warn: false,
        folder,
        timeout_seconds: 300,
        skip_markdown: false,
        skip_wip: false,
        action,
    });
    Ok(())
}

pub fn definition(repo: &Path) -> Result<Definition> {
    let info = super::list(repo).map_err(|e| e.to_string())?;
    let mut definition = Definition::default();
    if let Some(config) = &info.config {
        let documents =
            YamlLoader::load_from_str(&config.text).map_err(|e| format!("{}: {e}", config.path))?;
        let document = documents
            .first()
            .ok_or("The manager configuration is empty.")?;
        match info.manager {
            Manager::Lefthook => {
                for stage in Stage::ALL {
                    if let Some(commands) = document[stage.name()]["commands"].as_hash() {
                        for (name, settings) in commands {
                            let label = name
                                .as_str()
                                .ok_or("A hook command needs a name.")?
                                .to_string();
                            let command = settings["run"]
                                .as_str()
                                .ok_or_else(|| format!("{label} has no run command."))?;
                            add(&mut definition, stage, label, command.into(), settings)?;
                        }
                    }
                    if let Some(scripts) = document[stage.name()]["scripts"].as_hash() {
                        for (file, settings) in scripts {
                            let file = file.as_str().ok_or("A manager script needs a filename.")?;
                            let path = repo.join(".lefthook").join(stage.name()).join(file);
                            let command = format!(
                                "{} {}",
                                settings["runner"].as_str().unwrap_or("sh"),
                                sh_arg(&crate::shell::hook_path(&path))
                            );
                            add(&mut definition, stage, file.into(), command, settings)?;
                        }
                    }
                }
            }
            Manager::PreCommit => {
                for repository in document["repos"].as_vec().into_iter().flatten() {
                    for hook in repository["hooks"].as_vec().into_iter().flatten() {
                        let id = hook["id"]
                            .as_str()
                            .ok_or("A pre-commit hook needs an id.")?;
                        let label = hook["name"].as_str().unwrap_or(id).to_string();
                        let stages = strings(&hook["stages"]);
                        for stage in Stage::ALL.into_iter().filter(|stage| {
                            if stages.is_empty() {
                                *stage == Stage::PreCommit
                            } else {
                                stages.iter().any(|name| name == stage.name())
                            }
                        }) {
                            // The original manager supplies remote environments, file
                            // regexes and arguments. Keep that contract for remote hooks.
                            let command = if repository["repo"].as_str() == Some("local")
                                && hook["language"].as_str() == Some("system")
                                && hook["files"].is_badvalue()
                                && hook["args"].is_badvalue()
                                && hook["pass_filenames"].as_bool() == Some(false)
                            {
                                hook["entry"]
                                    .as_str()
                                    .ok_or("A local hook needs an entry.")?
                                    .to_string()
                            } else {
                                let command = format!(
                                    "pre-commit run {} --hook-stage {}",
                                    sh_arg(id),
                                    stage.name()
                                );
                                match stage {
                                    Stage::CommitMsg => format!("{command} --commit-msg-filename \"$HOOK_ARG1\""),
                                    Stage::PrePush => format!("if [ ! -s \"$RUN_DIR/push\" ]; then {command} --all-files; else\nwhile read -r local_ref local_id remote_ref remote_id; do\ncase \"$local_id\" in *[!0]*) ;; *) continue ;; esac\ncase \"$remote_id\" in *[!0]*) {command} --from-ref \"$remote_id\" --to-ref \"$local_id\" ;; *) {command} --all-files ;; esac || exit $?\ndone < \"$RUN_DIR/push\"\nfi"),
                                    Stage::PreCommit => command,
                                }
                            };
                            add(
                                &mut definition,
                                stage,
                                label.clone(),
                                command,
                                &Yaml::BadValue,
                            )?;
                        }
                    }
                }
            }
            _ => {}
        }
    } else {
        for hook in &info.hooks {
            if let Some(stage) = Stage::ALL
                .into_iter()
                .find(|stage| stage.name() == hook.name)
            {
                // Run the original file with its own $0, so a Husky script's
                // relative helper paths keep meaning the same thing.
                let path = Path::new(&hook.path);
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    repo.join(path)
                };
                let command = format!(
                    "sh {} \"$HOOK_ARG1\" \"$HOOK_ARG2\"",
                    sh_arg(&crate::shell::hook_path(&path))
                );
                add(
                    &mut definition,
                    stage,
                    hook.name.clone(),
                    command,
                    &Yaml::BadValue,
                )?;
            }
        }
    }
    if definition.blocks.is_empty() {
        return Err(
            "No supported hook commands were found to import. The original hooks were left alone."
                .into(),
        );
    }
    super::blocks::validate(&definition)?;
    Ok(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn lefthook_import_preserves_quotes_anchors_multiline_commands_and_stages() {
        let fixture = Fixture::linear(1);
        std::fs::write(
            fixture.path().join(".git/hooks/pre-commit"),
            "#!/bin/sh\nlefthook run pre-commit\n",
        )
        .expect("hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                fixture.path().join(".git/hooks/pre-commit"),
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("executable");
        }
        let source = "pre-commit:\n  commands:\n    quoted: &check\n      run: |\n        printf 'a \\\"quote\\\"'\n        true\npre-push:\n  commands:\n    same: *check\n";
        std::fs::write(fixture.path().join("lefthook.yml"), source).expect("config");
        let imported = definition(fixture.path()).expect("import");
        assert_eq!(imported.blocks.len(), 2);
        assert_eq!(imported.blocks[0].stage, Stage::PreCommit);
        assert_eq!(imported.blocks[1].stage, Stage::PrePush);
        assert_eq!(imported.blocks[0].action, imported.blocks[1].action);
        let Action::Script { command } = &imported.blocks[0].action else {
            panic!("script")
        };
        assert!(command.contains("printf 'a"));
        assert!(command.contains("\ntrue\n"));
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("lefthook.yml")).expect("original"),
            source
        );
    }
}
