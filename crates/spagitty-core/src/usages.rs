// SPDX-License-Identifier: GPL-3.0-or-later

//! Where a name is defined and used, at a revision (FEAT-154).
//!
//! Files reads git's objects at any branch, tag or commit, so there is no
//! checked-out project for a language server to build. This answers the
//! question the way code hosts' search-based navigation does: every place
//! the name stands as a whole word, case and all, and then each place read
//! with its file's tree-sitter grammar —
//!
//! - a **definition** where the grammar's tags query names one there: a
//!   function, a class, a method, a module, an interface;
//! - a **mention** where it sits in a comment or a string, or anywhere in a
//!   file that is data or prose (JSON, YAML, TOML, Markdown, CSS);
//! - a **use** everywhere else, with what the tags query calls it when it
//!   says (`call`, `implementation`, `type`…).
//!
//! It is by name, not by type: two methods called `get` are one name here,
//! and the screen says so.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator};

use crate::error::{Error, Result};
use crate::files::{self, Hit};
use crate::highlight;

/// The longest name looked for. Past this it is not a name someone clicked.
const NAME_MAX: usize = 200;

/// Languages whose files only mention names: they hold no code that calls one.
const PROSE: &[&str] = &["json", "yaml", "toml", "markdown", "markdown_inline", "css"];

/// What a name is at one place it appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Definition,
    Use,
    Mention,
}

/// One place the name appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(flatten)]
    pub hit: Hit,
    pub role: Role,
    /// What the grammar's tags query calls it — `function`, `class`,
    /// `method`, `call` — or `None` where it says nothing.
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usages {
    pub name: String,
    /// Definitions first, then uses, then mentions; each in path and line order.
    pub usages: Vec<Usage>,
    /// How many files the name appears in.
    pub files: u32,
    /// More were found than were returned, or the walk stopped at its limit.
    pub more: bool,
}

/// Is `c` part of a name, in the languages Files reads? `$` is, for
/// JavaScript and Svelte.
fn in_name(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// `name` as something to look for, or why it is not one.
fn checked(name: &str) -> Result<&str> {
    let name = name.trim();
    let first = name.chars().next();
    if name.is_empty()
        || name.len() > NAME_MAX
        || !name.chars().all(in_name)
        || first.is_some_and(|c| c.is_numeric())
    {
        return Err(Error::NotAName(name.to_string()));
    }
    Ok(name)
}

/// The byte ranges in `line` where `name` stands as a whole word.
fn whole_words(line: &str, name: &str) -> Vec<(usize, usize)> {
    line.match_indices(name)
        .map(|(start, _)| (start, start + name.len()))
        .filter(|&(start, end)| {
            !line[..start].chars().next_back().is_some_and(in_name)
                && !line[end..].chars().next().is_some_and(in_name)
        })
        .collect()
}

/// A grammar as this module reads with it: the language, and its tags query
/// when it has one that compiles.
struct Grammar {
    language: Language,
    tags: Option<Query>,
}

fn build(name: &str) -> Option<Grammar> {
    use tree_sitter_javascript as js;
    use tree_sitter_typescript as ts;

    let (language, tags): (Language, Option<String>) = match name {
        "bash" => (tree_sitter_bash::LANGUAGE.into(), None),
        "c" => (
            tree_sitter_c::LANGUAGE.into(),
            Some(tree_sitter_c::TAGS_QUERY.into()),
        ),
        "c_sharp" => (
            tree_sitter_c_sharp::LANGUAGE.into(),
            Some(tree_sitter_c_sharp::TAGS_QUERY.into()),
        ),
        "cpp" => (
            tree_sitter_cpp::LANGUAGE.into(),
            Some(tree_sitter_cpp::TAGS_QUERY.into()),
        ),
        "go" => (
            tree_sitter_go::LANGUAGE.into(),
            Some(tree_sitter_go::TAGS_QUERY.into()),
        ),
        "html" => (tree_sitter_html::LANGUAGE.into(), None),
        "java" => (
            tree_sitter_java::LANGUAGE.into(),
            Some(tree_sitter_java::TAGS_QUERY.into()),
        ),
        "javascript" => (js::LANGUAGE.into(), Some(js::TAGS_QUERY.into())),
        "python" => (
            tree_sitter_python::LANGUAGE.into(),
            Some(tree_sitter_python::TAGS_QUERY.into()),
        ),
        "rust" => (
            tree_sitter_rust::LANGUAGE.into(),
            Some(tree_sitter_rust::TAGS_QUERY.into()),
        ),
        "sql" => (tree_sitter_sequel::LANGUAGE.into(), None),
        "svelte" => (tree_sitter_svelte_ng::LANGUAGE.into(), None),
        // TypeScript's query names only what TypeScript adds to JavaScript,
        // as its highlight query does.
        "typescript" => (
            ts::LANGUAGE_TYPESCRIPT.into(),
            Some(format!("{}\n{}", js::TAGS_QUERY, ts::TAGS_QUERY)),
        ),
        "tsx" => (
            ts::LANGUAGE_TSX.into(),
            Some(format!("{}\n{}", js::TAGS_QUERY, ts::TAGS_QUERY)),
        ),
        _ => return None,
    };
    // A query that does not compile against its grammar tells nothing apart;
    // the grammar still finds comments and strings.
    let tags = tags.and_then(|source| Query::new(&language, &source).ok());
    Some(Grammar { language, tags })
}

/// A grammar, built the first time it is asked for and kept for the life of
/// the process, as `highlight` keeps its own.
fn grammar(name: &str) -> Option<&'static Grammar> {
    static BUILT: OnceLock<Mutex<HashMap<String, Option<&'static Grammar>>>> = OnceLock::new();
    let mut built = BUILT
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *built
        .entry(name.to_string())
        .or_insert_with(|| build(name).map(|grammar| &*Box::leak(Box::new(grammar))))
}

/// The names the tags query captures, by the byte they start at: a
/// definition wins over a reference at the same place.
fn tagged(grammar: &Grammar, root: Node<'_>, text: &str) -> HashMap<usize, (Role, String)> {
    let mut found: HashMap<usize, (Role, String)> = HashMap::new();
    let Some(query) = &grammar.tags else {
        return found;
    };
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, root, text.as_bytes());
    while let Some(each) = matches.next() {
        let mut at = None;
        let mut what = None;
        for capture in each.captures() {
            let capture_name = names[capture.index as usize];
            if capture_name == "name" {
                at = Some(capture.node.start_byte());
            } else if let Some(kind) = capture_name.strip_prefix("definition.") {
                what = Some((Role::Definition, kind));
            } else if let Some(kind) = capture_name.strip_prefix("reference.") {
                what = Some((Role::Use, kind));
            }
        }
        let (Some(at), Some((role, kind))) = (at, what) else {
            continue;
        };
        match found.get(&at) {
            Some((held, _)) if *held <= role => {}
            _ => {
                found.insert(at, (role, kind.to_string()));
            }
        }
    }
    found
}

/// Is the text at `start..end` inside a comment or a string? A substitution
/// inside a string — `${name}`, `{name}` in an f-string — is code again.
fn quoted(root: Node<'_>, start: usize, end: usize) -> bool {
    let mut node = root.descendant_for_byte_range(start, end);
    while let Some(here) = node {
        let kind = here.kind();
        if kind.contains("substitution") || kind.contains("interpolation") {
            return false;
        }
        if kind.contains("comment") || kind.contains("string") || kind == "char_literal" {
            return true;
        }
        node = here.parent();
    }
    false
}

/// Every place `name` stands as a whole word in the text files at `rev`,
/// each told apart as a definition, a use or a mention (FEAT-154). Binary
/// files, Git LFS pointers and files over 2 MB are left out, as *Search in
/// files* leaves them.
pub fn usages(repo: &gix::Repository, rev: &str, name: &str, limit: usize) -> Result<Usages> {
    let name = checked(name)?;
    let (blobs, mut more) = files::blobs_at(repo, rev)?;

    let mut usages: Vec<Usage> = Vec::new();
    let mut files = 0u32;
    let mut parser = Parser::new();
    'files: for (path, id) in blobs {
        let Some(text) = files::searchable_text(repo, id)? else {
            continue;
        };
        if !text.contains(name) {
            continue;
        }

        // Every whole word first, by byte in the file; most files that hold
        // the text hold it as part of a longer name, and are never parsed.
        let mut places = Vec::new();
        let mut offset = 0usize;
        for (number, piece) in text.split_inclusive('\n').enumerate() {
            let line = piece.trim_end_matches('\n').trim_end_matches('\r');
            for (start, end) in whole_words(line, name) {
                places.push((number, offset, start, end));
            }
            offset += piece.len();
        }
        if places.is_empty() {
            continue;
        }
        files += 1;

        let language = highlight::grammar_for(&path);
        let prose = language.is_some_and(|language| PROSE.contains(&language));
        let read = language
            .filter(|_| !prose)
            .and_then(grammar)
            .and_then(|grammar| {
                parser.set_language(&grammar.language).ok()?;
                let tree = parser.parse(&text, None)?;
                let tags = tagged(grammar, tree.root_node(), &text);
                Some((tree, tags))
            });

        let lines: Vec<&str> = text.lines().collect();
        for (number, offset, start, end) in places {
            if usages.len() == limit {
                more = true;
                break 'files;
            }
            let (role, kind) = if prose {
                (Role::Mention, None)
            } else if let Some((tree, tags)) = &read {
                match tags.get(&(offset + start)) {
                    Some((role, kind)) => (*role, Some(kind.clone())),
                    None if quoted(tree.root_node(), offset + start, offset + end) => {
                        (Role::Mention, None)
                    }
                    None => (Role::Use, None),
                }
            } else {
                (Role::Use, None)
            };
            let line = lines.get(number).copied().unwrap_or_default();
            usages.push(Usage {
                hit: files::hit_on(&path, number, line, start, end),
                role,
                kind,
            });
        }
    }

    // Stable: within a role, the walk's path and line order stands.
    usages.sort_by_key(|usage| usage.role);
    Ok(Usages {
        name: name.to_string(),
        usages,
        files,
        more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    fn a_project() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write(
            "src/lib.rs",
            "/// Says hello to `greet`'s caller.\n\
             pub fn greet(name: &str) -> String {\n    \
                 format!(\"greet {name}\")\n\
             }\n\
             \n\
             pub struct Greeter;\n\
             \n\
             impl Greeter {\n    \
                 pub fn wave(&self) -> String {\n        \
                     greet(\"you\")\n    \
                 }\n\
             }\n",
        );
        fixture.write(
            "web/app.ts",
            "import { greet } from './greet';\n\
             export function greeting(): string {\n  \
               return greet('web') + `${greet('x')}`;\n\
             }\n\
             const greeted = 'greet';\n",
        );
        fixture.write("docs/greet.md", "Call greet to say hello.\n");
        fixture.write("config.json", "{ \"greet\": true }\n");
        fixture.write("notes.txt", "greet, greeting, greeter\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("A project");
        fixture
    }

    fn roles(found: &Usages) -> Vec<(String, u32, Role, Option<&str>)> {
        found
            .usages
            .iter()
            .map(|usage| {
                (
                    usage.hit.path.clone(),
                    usage.hit.line,
                    usage.role,
                    usage.kind.as_deref(),
                )
            })
            .collect()
    }

    #[test]
    fn finds_the_definition_the_uses_and_the_mentions_in_that_order() {
        let fixture = a_project();
        let repo = fixture.open();
        let found = usages(&repo, "HEAD", "greet", 100).expect("usages");

        let rows = roles(&found);
        assert_eq!(
            rows.first(),
            Some(&(
                "src/lib.rs".to_string(),
                2,
                Role::Definition,
                Some("function")
            ))
        );
        assert!(rows.contains(&("src/lib.rs".to_string(), 10, Role::Use, Some("call"))));
        // In the doc comment and in the format string: mentions.
        assert!(rows.contains(&("src/lib.rs".to_string(), 1, Role::Mention, None)));
        assert!(rows.contains(&("src/lib.rs".to_string(), 3, Role::Mention, None)));
        // Prose and data only mention it.
        assert!(rows.contains(&("docs/greet.md".to_string(), 1, Role::Mention, None)));
        assert!(rows.contains(&("config.json".to_string(), 1, Role::Mention, None)));

        let roles_in_order: Vec<Role> = rows.iter().map(|row| row.2).collect();
        let mut sorted = roles_in_order.clone();
        sorted.sort();
        assert_eq!(roles_in_order, sorted);
    }

    #[test]
    fn reads_typescript_calls_and_a_substitution_as_uses() {
        let fixture = a_project();
        let repo = fixture.open();
        let found = usages(&repo, "HEAD", "greet", 100).expect("usages");

        let in_ts: Vec<(u32, Role)> = found
            .usages
            .iter()
            .filter(|usage| usage.hit.path == "web/app.ts")
            .map(|usage| (usage.hit.line, usage.role))
            .collect();
        // The import, the call, the call inside `${…}`, then the string.
        assert_eq!(
            in_ts.iter().filter(|(line, _)| *line == 3).count(),
            2,
            "{in_ts:?}"
        );
        assert!(in_ts
            .iter()
            .filter(|(line, _)| *line == 3)
            .all(|(_, role)| *role == Role::Use));
        assert!(in_ts.contains(&(1, Role::Use)));
        assert!(in_ts.contains(&(5, Role::Mention)));
    }

    #[test]
    fn matches_whole_words_with_their_case_only() {
        let fixture = a_project();
        let repo = fixture.open();
        let found = usages(&repo, "HEAD", "greet", 100).expect("usages");

        let notes: Vec<&Usage> = found
            .usages
            .iter()
            .filter(|usage| usage.hit.path == "notes.txt")
            .collect();
        assert_eq!(notes.len(), 1, "greeting and greeter are other names");
        assert_eq!((notes[0].hit.column, notes[0].hit.length), (0, 5));
        // No grammar: a use, since nothing says otherwise.
        assert_eq!(notes[0].role, Role::Use);

        let other_case = usages(&repo, "HEAD", "Greet", 100).expect("usages");
        assert!(other_case.usages.is_empty());
        let greeter = usages(&repo, "HEAD", "Greeter", 100).expect("usages");
        assert_eq!(
            roles(&greeter).first(),
            Some(&("src/lib.rs".to_string(), 6, Role::Definition, Some("class")))
        );
    }

    #[test]
    fn says_when_there_is_more_and_refuses_what_is_not_a_name() {
        let fixture = a_project();
        let repo = fixture.open();
        let few = usages(&repo, "HEAD", "greet", 2).expect("usages");
        assert_eq!(few.usages.len(), 2);
        assert!(few.more);

        assert!(usages(&repo, "HEAD", "greet(", 10).is_err());
        assert!(usages(&repo, "HEAD", "", 10).is_err());
        assert!(usages(&repo, "HEAD", "9lives", 10).is_err());
        assert_eq!(whole_words("$store store", "store"), vec![(7, 12)]);
    }

    #[test]
    fn every_tags_query_compiles_against_its_grammar() {
        for name in [
            "c",
            "c_sharp",
            "cpp",
            "go",
            "java",
            "javascript",
            "python",
            "rust",
            "typescript",
            "tsx",
        ] {
            let grammar = grammar(name).unwrap_or_else(|| panic!("{name} has no grammar"));
            assert!(
                grammar.tags.is_some(),
                "{name}'s tags query does not compile"
            );
        }
    }
}
