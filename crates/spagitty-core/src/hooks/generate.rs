// SPDX-License-Identifier: GPL-3.0-or-later

//! POSIX hooks from checked blocks. Commands are the person's own shell;
//! every value used as data is quoted independently from that command.

use super::blocks::{validate, Action, Block, Definition, Stage};
use crate::recipes::script::sh_arg;

pub const MARK: &str = "# spagitty-hooks v1";

pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn globs(paths: &[String]) -> String {
    paths
        .iter()
        .map(|path| sh_arg(&format!(":(glob){path}")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn message(action: &Action) -> String {
    let Action::Message {
        style,
        types,
        scope_required,
        subject_max,
        pattern,
        ticket_pattern,
    } = action
    else {
        return String::new();
    };
    let check = match style.as_str() {
        "conventional" => {
            let kinds = if types.is_empty() {
                "[a-z]+".into()
            } else {
                format!("({})", types.join("|"))
            };
            let scope = if *scope_required {
                "\\([^)]+\\)"
            } else {
                "(\\([^)]+\\))?"
            };
            format!("^{kinds}{scope}!?: .+")
        }
        "ticket" => {
            if pattern.is_empty() {
                "^[A-Z]+-[0-9]+[ :].+".into()
            } else {
                pattern.clone()
            }
        }
        _ => pattern.clone(),
    };
    let mut text = format!("subject=$(head -n 1 \"$MESSAGE\")\n[ \"${{#subject}}\" -le {subject_max} ] || {{ printf '%s\\n' 'The subject is too long.'; return 1; }}\nprintf '%s\\n' \"$subject\" | grep -Eq -- {} || {{ printf '%s\\n' 'The subject does not match the message rules.'; return 1; }}\n", sh_arg(&check));
    if !ticket_pattern.is_empty() {
        text.push_str(&format!("awk 'NR > 1 {{ if ($0 == \"\") {{ if (chunk != \"\") {{ last = chunk; chunk = \"\" }} }} else chunk = chunk $0 \"\\n\" }} END {{ printf \"%s\", chunk != \"\" ? chunk : last }}' \"$MESSAGE\" | grep -Eq -- {} || {{ printf '%s\\n' 'The message needs a ticket in its footer.'; return 1; }}\n", sh_arg(ticket_pattern)));
    }
    text
}

fn body(block: &Block) -> String {
    match &block.action {
        Action::Tests { paths, command } => format!(
            "git -C \"$ROOT\" diff --cached --quiet -- {}\nmatched=$?\n[ \"$matched\" = 0 ] && return 77\n[ \"$matched\" = 1 ] || return \"$matched\"\ncount=$(git -C \"$ROOT\" diff --cached --name-only -z --diff-filter=ACMR -- {} | tr -cd '\\000' | wc -c)\nemit \"{{\\\"t\\\":\\\"match\\\",\\\"id\\\":$(json \"$BID\"),\\\"files\\\":$count}}\"\nsh -c {}\n", globs(paths), globs(paths), sh_arg(command)),
        Action::Script { command } => format!("sh -c {}\n", sh_arg(command)),
        Action::Message { .. } => message(&block.action),
        Action::Format { preset, paths, command, restage } => {
            let mut text = format!("git -C \"$ROOT\" diff --cached --name-only -z --diff-filter=ACMR -- {} > \"$RUN_DIR/$BID-matches\"\n[ -s \"$RUN_DIR/$BID-matches\" ] || return 77\n: > \"$RUN_DIR/$BID-safe\"\nxargs -0 sh \"$SELF\" --files safe < \"$RUN_DIR/$BID-matches\" || return 1\n", globs(paths));
            // Spotless accepts absolute Java regex patterns in one Maven
            // property, unlike the other presets' positional file arguments.
            let command = if preset == "spotless" {
                format!("spotless_files=''\nfor file do\nif command -v cygpath >/dev/null 2>&1; then file=$(cygpath -aw \"$file\") || exit 1; fi\ncase \"$file\" in *,*|*'\\E'*) printf '%s\\n' 'This filename needs a custom formatter command.'; exit 1 ;; esac\nspotless_files=\"$spotless_files${{spotless_files:+,}}\\Q$file\\E\"\ndone\n{command} \"-DspotlessFiles=$spotless_files\"")
            } else if command.contains("\"$@\"") { command.clone() } else { format!("{command} \"$@\"") };
            text.push_str(&format!("if [ -s \"$RUN_DIR/$BID-safe\" ]; then\n  xargs -0 sh -c {} _ < \"$RUN_DIR/$BID-safe\" || return 1\n", sh_arg(&command)));
            if *restage { text.push_str("  xargs -0 sh \"$SELF\" --files restage < \"$RUN_DIR/$BID-safe\" || return 1\n"); }
            text.push_str("fi\n[ ! -f \"$RUN_DIR/$BID-unsafe\" ]\n");
            text
        }
        Action::Secrets { patterns } => {
            let mut checks = vec!["(AKIA|ASIA)[A-Z0-9]{16}".to_string(), "-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----".into(), "gh[pousr]_[A-Za-z0-9]{20,}".into()];
            checks.extend(patterns.iter().cloned());
            format!("git diff --cached --no-ext-diff -U0 | awk '/^\\+\\+\\+ / {{next}} /^\\+/ {{print substr($0,2)}}' > \"$RUN_DIR/$BID-added\"\ngrep -Eq -- {} \"$RUN_DIR/$BID-added\"\ncode=$?\nrm -f \"$RUN_DIR/$BID-added\"\n[ \"$code\" = 1 ] && return 0\n[ \"$code\" = 0 ] && printf '%s\\n' 'A secret pattern matched the staged diff.'\nreturn 1\n", sh_arg(&checks.join("|")))
        }
        Action::LargeFiles { bytes } => format!("LIMIT={bytes}; export LIMIT\nxargs -0 sh \"$SELF\" --files size < \"$RUN_DIR/staged\" || return 1\n[ ! -f \"$RUN_DIR/$BID-large\" ]\n"),
        Action::ProtectBranches { branches } => {
            let names = branches.iter().map(|b| sh_arg(b)).collect::<Vec<_>>().join("|");
            let refuse = format!("case \"$branch\" in {names}) printf '%s\\n' 'This branch is protected by a hook.'; return 1 ;; esac\n");
            if block.stage == Stage::PrePush {
                format!("if [ -s \"$RUN_DIR/push\" ]; then\nwhile read -r local_ref local_id remote_ref remote_id; do\nbranch=${{remote_ref#refs/heads/}}\n{refuse}done < \"$RUN_DIR/push\"\nelse\nbranch=$(git symbolic-ref --quiet --short HEAD) || return 0\n{refuse}fi\n")
            } else { format!("branch=$(git symbolic-ref --quiet --short HEAD) || return 0\n{refuse}") }
        }
    }
}

pub fn script(definition: &Definition, stage: Stage) -> Result<String, String> {
    validate(definition)?;
    let bytes = serde_json::to_vec(definition).map_err(|e| e.to_string())?;
    let mut text = format!("#!/bin/sh\n{MARK}\n# blocks {:016x}\n", hash(&bytes));
    text.push_str(&include_str!("runner.sh").replace("\r\n", "\n"));
    let blocks: Vec<_> = definition
        .blocks
        .iter()
        .filter(|b| b.stage == stage && b.enabled && definition.active_stages.contains(&stage))
        .collect();
    for (i, block) in blocks.iter().enumerate() {
        text.push_str(&format!("\nblock_{i}() {{\n{}\n}}\n", body(block)));
    }
    text.push_str("\nif [ \"${1-}\" = --block ]; then shift; \"$@\"; exit $?; fi\n");
    let rows: Vec<_> = blocks
        .iter()
        .map(|block| serde_json::json!({"id": block.id, "label": block.label}))
        .collect();
    text.push_str(&format!(
        "\nBLOCKS={}\nMESSAGE=${{1-}}\nHOOK_ARG1=${{1-}}\nHOOK_ARG2=${{2-}}\nexport HOOK_ARG1 HOOK_ARG2\n",
        sh_arg(&serde_json::to_string(&rows).map_err(|e| e.to_string())?)
    ));
    text.push_str(&format!("begin {}\n", stage.name()));
    for (i, b) in blocks.iter().enumerate() {
        text.push_str(&format!(
            "step {} {} {} {} {} {} {} block_{i}\n",
            sh_arg(&b.id),
            sh_arg(&b.label),
            b.warn,
            b.timeout_seconds,
            sh_arg(&b.folder),
            b.skip_markdown,
            b.skip_wip
        ));
    }
    text.push_str("done_run 0\n");
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::super::blocks::tests::script as block;
    use super::*;

    #[test]
    fn each_stage_contains_only_its_enabled_blocks_and_quotes_names() {
        let mut first = block("printf 'hello'");
        first.label = "$(do not run) 'quoted'".into();
        let mut other = block("false");
        other.id = "other".into();
        other.stage = Stage::PrePush;
        let definition = Definition {
            blocks: vec![first, other],
            ..Definition::default()
        };
        let text = script(&definition, Stage::PreCommit).expect("script");
        assert!(text.starts_with("#!/bin/sh\n# spagitty-hooks v1\n# blocks "));
        assert!(text.contains("'$(do not run) '\\''quoted'\\'''"));
        assert!(!text.contains("step other "));
    }
}
