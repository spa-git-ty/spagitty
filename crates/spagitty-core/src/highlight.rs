// SPDX-License-Identifier: GPL-3.0-or-later

//! Colour for Files, from tree-sitter (FEAT-131).
//!
//! `tree-sitter-highlight` with the grammars compiled in. A grammar reads a
//! whole file to colour any of it, so the whole blob is coloured once, kept
//! per line, and a request is a slice of what is kept. A blob never changes —
//! its id is its content — so the cache is never invalidated, only trimmed.
//!
//! # What crosses the wire
//!
//! Not text. The screen already has the file; a line's colour is pairs of
//! *token class, length*, the length in UTF-16 units because that is what the
//! webview cuts a string by. The classes are the twelve Diff already has
//! (`src/lib/diff/highlight.ts`), so Files and Diff read the same.
//!
//! # A language with no grammar here
//!
//! Falls back to the frontend's own highlighter, the one Diff uses. That is
//! Kotlin and Dockerfile today: the Kotlin crate that builds against this
//! tree-sitter ships no highlight query, and the Dockerfile crate is built for
//! a tree-sitter two major versions back.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::error::{Error, Result};

/// Above this a file is shown without colour (FEAT-131).
pub const COLOUR_MAX_BYTES: u64 = 5 * 1024 * 1024;

/// How many coloured blobs are kept. A person has a handful of tabs open.
const CACHE_BLOBS: usize = 24;

/// The token classes, in the order their numbers cross the wire.
///
/// `src/lib/files/colour.ts` carries the same list, and a test there reads
/// this file to keep the two in step.
pub const CLASSES: [&str; 12] = [
    "plain",
    "keyword",
    "string",
    "number",
    "comment",
    "fn",
    "type",
    "operator",
    "punctuation",
    "tag",
    "attr",
    "meta",
];

const PLAIN: u32 = 0;
const KEYWORD: u32 = 1;
const STRING: u32 = 2;
const NUMBER: u32 = 3;
const COMMENT: u32 = 4;
const FUNCTION: u32 = 5;
const TYPE: u32 = 6;
const OPERATOR: u32 = 7;
const PUNCTUATION: u32 = 8;
const TAG: u32 = 9;
const ATTR: u32 = 10;
const META: u32 = 11;

/// What a query calls a thing, and the class it is drawn in.
///
/// The highlighter matches a capture to the longest name here that prefixes
/// it, so `function.macro` wins over `function` and an unknown
/// `keyword.control.flow` still lands on `keyword`. Both the names the
/// tree-sitter project uses and the ones Neovim's queries use are listed: the
/// grammars' own queries are written in either.
const CAPTURES: &[(&str, u32)] = &[
    ("attribute", META),
    ("boolean", NUMBER),
    ("character", STRING),
    ("comment", COMMENT),
    ("conditional", KEYWORD),
    ("constant", NUMBER),
    ("constructor", TYPE),
    ("define", META),
    ("embedded", PLAIN),
    ("escape", STRING),
    ("exception", KEYWORD),
    ("field", ATTR),
    ("float", NUMBER),
    ("function", FUNCTION),
    ("function.macro", META),
    ("include", KEYWORD),
    ("keyword", KEYWORD),
    ("label", PLAIN),
    ("macro", META),
    ("markup.heading", KEYWORD),
    ("markup.link", STRING),
    ("markup.raw", STRING),
    ("method", FUNCTION),
    ("module", TYPE),
    ("namespace", TYPE),
    ("number", NUMBER),
    ("operator", OPERATOR),
    ("parameter", PLAIN),
    ("preproc", META),
    ("property", ATTR),
    ("punctuation", PUNCTUATION),
    ("repeat", KEYWORD),
    ("storageclass", KEYWORD),
    ("string", STRING),
    ("tag", TAG),
    ("tag.attribute", ATTR),
    ("text.literal", STRING),
    ("text.reference", FUNCTION),
    ("text.title", KEYWORD),
    ("text.uri", STRING),
    ("type", TYPE),
    ("variable", PLAIN),
    ("variable.builtin", KEYWORD),
];

/// In a markup file an `attribute` is the name in `name="value"`, not an
/// annotation, so it takes the class the frontend's highlighter gives it.
const MARKUP: &[&str] = &["html", "svelte"];

/// The grammar that reads `path`, by the names the injections use too.
///
/// By file name first, then by extension. `None` means no grammar is compiled
/// in for it, and the frontend's highlighter takes over.
pub fn grammar_for(path: &str) -> Option<&'static str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name {
        "Cargo.lock" | "Pipfile" | "poetry.lock" | "uv.lock" => return Some("toml"),
        ".bashrc" | ".bash_profile" | ".zshrc" | ".profile" | "PKGBUILD" => return Some("bash"),
        _ => {}
    }
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())?;
    Some(match extension.as_str() {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "svelte" => "svelte",
        "py" | "pyi" => "python",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "cs" => "c_sharp",
        "json" | "jsonc" | "webmanifest" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" | "markdown" => "markdown",
        "css" => "css",
        "html" | "htm" => "html",
        "sh" | "bash" | "zsh" => "bash",
        "sql" => "sql",
        _ => return None,
    })
}

/// Every grammar compiled in, by name. The measure the item asks for is taken
/// against this list.
pub const GRAMMARS: [&str; 20] = [
    "bash",
    "c",
    "c_sharp",
    "cpp",
    "css",
    "go",
    "html",
    "java",
    "javascript",
    "json",
    "markdown",
    "markdown_inline",
    "python",
    "rust",
    "sql",
    "svelte",
    "toml",
    "tsx",
    "typescript",
    "yaml",
];

/// The name an injection asks for, as the name a grammar is kept under.
fn canonical(name: &str) -> &str {
    match name {
        "js" | "jsx" | "ecma" => "javascript",
        "ts" => "typescript",
        "sh" | "shell" | "zsh" => "bash",
        "rs" => "rust",
        "py" => "python",
        "md" => "markdown",
        "yml" => "yaml",
        "c++" => "cpp",
        "cs" | "csharp" | "c-sharp" => "c_sharp",
        "htm" => "html",
        "scss" => "css",
        other => other,
    }
}

fn build(name: &str) -> Option<HighlightConfiguration> {
    use tree_sitter_javascript as js;
    use tree_sitter_typescript as ts;

    let made = match name {
        "bash" => HighlightConfiguration::new(
            tree_sitter_bash::LANGUAGE.into(),
            name,
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        ),
        "c" => HighlightConfiguration::new(
            tree_sitter_c::LANGUAGE.into(),
            name,
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        ),
        "c_sharp" => HighlightConfiguration::new(
            tree_sitter_c_sharp::LANGUAGE.into(),
            name,
            tree_sitter_c_sharp::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        // C++'s query is written on top of C's.
        "cpp" => HighlightConfiguration::new(
            tree_sitter_cpp::LANGUAGE.into(),
            name,
            &format!(
                "{}\n{}",
                tree_sitter_c::HIGHLIGHT_QUERY,
                tree_sitter_cpp::HIGHLIGHT_QUERY
            ),
            "",
            "",
        ),
        "css" => HighlightConfiguration::new(
            tree_sitter_css::LANGUAGE.into(),
            name,
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "go" => HighlightConfiguration::new(
            tree_sitter_go::LANGUAGE.into(),
            name,
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "html" => HighlightConfiguration::new(
            tree_sitter_html::LANGUAGE.into(),
            name,
            tree_sitter_html::HIGHLIGHTS_QUERY,
            tree_sitter_html::INJECTIONS_QUERY,
            "",
        ),
        "java" => HighlightConfiguration::new(
            tree_sitter_java::LANGUAGE.into(),
            name,
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "javascript" => HighlightConfiguration::new(
            js::LANGUAGE.into(),
            name,
            &format!("{}\n{}", js::HIGHLIGHT_QUERY, js::JSX_HIGHLIGHT_QUERY),
            js::INJECTIONS_QUERY,
            js::LOCALS_QUERY,
        ),
        "json" => HighlightConfiguration::new(
            tree_sitter_json::LANGUAGE.into(),
            name,
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "markdown" => HighlightConfiguration::new(
            tree_sitter_md::LANGUAGE.into(),
            name,
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            tree_sitter_md::INJECTION_QUERY_BLOCK,
            "",
        ),
        "markdown_inline" => HighlightConfiguration::new(
            tree_sitter_md::INLINE_LANGUAGE.into(),
            name,
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
            "",
        ),
        "python" => HighlightConfiguration::new(
            tree_sitter_python::LANGUAGE.into(),
            name,
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "rust" => HighlightConfiguration::new(
            tree_sitter_rust::LANGUAGE.into(),
            name,
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            "",
        ),
        "sql" => HighlightConfiguration::new(
            tree_sitter_sequel::LANGUAGE.into(),
            name,
            tree_sitter_sequel::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        "svelte" => HighlightConfiguration::new(
            tree_sitter_svelte_ng::LANGUAGE.into(),
            name,
            tree_sitter_svelte_ng::HIGHLIGHTS_QUERY,
            tree_sitter_svelte_ng::INJECTIONS_QUERY,
            tree_sitter_svelte_ng::LOCALS_QUERY,
        ),
        "toml" => HighlightConfiguration::new(
            tree_sitter_toml_ng::LANGUAGE.into(),
            name,
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        // TypeScript's query names only what TypeScript adds to JavaScript.
        "typescript" => HighlightConfiguration::new(
            ts::LANGUAGE_TYPESCRIPT.into(),
            name,
            &format!("{}\n{}", js::HIGHLIGHT_QUERY, ts::HIGHLIGHTS_QUERY),
            "",
            &format!("{}\n{}", js::LOCALS_QUERY, ts::LOCALS_QUERY),
        ),
        "tsx" => HighlightConfiguration::new(
            ts::LANGUAGE_TSX.into(),
            name,
            &format!(
                "{}\n{}\n{}",
                js::HIGHLIGHT_QUERY,
                js::JSX_HIGHLIGHT_QUERY,
                ts::HIGHLIGHTS_QUERY
            ),
            "",
            &format!("{}\n{}", js::LOCALS_QUERY, ts::LOCALS_QUERY),
        ),
        "yaml" => HighlightConfiguration::new(
            tree_sitter_yaml::LANGUAGE.into(),
            name,
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        _ => return None,
    };

    // A query that does not compile against its grammar is a grammar we do
    // not have: the frontend's highlighter takes the file.
    let mut configuration = made.ok()?;
    let names: Vec<&str> = CAPTURES.iter().map(|(capture, _)| *capture).collect();
    configuration.configure(&names);
    Some(configuration)
}

/// A grammar, built the first time it is asked for and kept for the life of
/// the process. Twenty of them at most, so they are leaked rather than counted.
fn configuration(name: &str) -> Option<&'static HighlightConfiguration> {
    static BUILT: OnceLock<Mutex<HashMap<String, Option<&'static HighlightConfiguration>>>> =
        OnceLock::new();
    let name = canonical(name);
    let mut built = BUILT
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(found) = built.get(name) {
        return *found;
    }
    let made = build(name).map(|configuration| &*Box::leak(Box::new(configuration)));
    built.insert(name.to_string(), made);
    made
}

/// True when a grammar for `name` is compiled in and its query builds.
pub fn has_grammar(name: &str) -> bool {
    configuration(name).is_some()
}

/// One line's colour: `[class, length, class, length, …]`, lengths in UTF-16
/// units, covering the line without its terminator.
pub type Line = Vec<u32>;

/// Colour `text` with `grammar`, a line at a time.
///
/// Lines are counted the way a file viewer counts them: a trailing newline
/// ends the last line rather than starting an empty one, and a `\r` before a
/// `\n` belongs to the terminator.
pub fn colour(text: &str, grammar: &str) -> Result<Vec<Line>> {
    let top =
        configuration(grammar).ok_or_else(|| Error::Diff(format!("no grammar for {grammar}")))?;
    let markup = MARKUP.contains(&canonical(grammar));

    let mut highlighter = Highlighter::new();
    // A closure and not the function itself: passed by name, the function's
    // `'static` answer is asked of every borrow in the call.
    #[allow(clippy::redundant_closure)]
    let events = highlighter
        .highlight(top, text.as_bytes(), None, None, |name| configuration(name))
        .map_err(|error| Error::Diff(error.to_string()))?;

    let mut lines: Vec<Line> = Vec::new();
    let mut current: Line = Vec::new();
    let mut stack: Vec<u32> = Vec::new();

    for event in events {
        match event.map_err(|error| Error::Diff(error.to_string()))? {
            HighlightEvent::HighlightStart(highlight) => {
                let (capture, class) = CAPTURES[highlight.0];
                stack.push(if markup && capture == "attribute" {
                    ATTR
                } else {
                    class
                });
            }
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                let class = stack.last().copied().unwrap_or(PLAIN);
                // Offsets are bytes into `text`, on character boundaries: the
                // grammar read the same bytes.
                let Some(piece) = text.get(start..end) else {
                    continue;
                };
                let mut rest = piece;
                while let Some(at) = rest.find('\n') {
                    push(&mut current, class, &rest[..at]);
                    lines.push(std::mem::take(&mut current));
                    rest = &rest[at + 1..];
                }
                push(&mut current, class, rest);
            }
        }
    }
    if !text.is_empty() && !text.ends_with('\n') {
        lines.push(current);
    }

    // A `\r` left at a line's end is part of its terminator.
    for (line, source) in lines.iter_mut().zip(text.split('\n')) {
        if source.ends_with('\r') {
            trim_one(line);
        }
    }
    Ok(lines)
}

/// Add `piece` to a line in `class`, joining it to the run before when that is
/// the same class.
fn push(line: &mut Line, class: u32, piece: &str) {
    let length = piece.encode_utf16().count() as u32;
    if length == 0 {
        return;
    }
    let count = line.len();
    if count >= 2 && line[count - 2] == class {
        line[count - 1] += length;
    } else {
        line.push(class);
        line.push(length);
    }
}

/// Take one unit off the end of a line.
fn trim_one(line: &mut Line) {
    let count = line.len();
    if count < 2 {
        return;
    }
    if line[count - 1] > 1 {
        line[count - 1] -= 1;
    } else {
        line.truncate(count - 2);
    }
}

/// A blob's id and the grammar it was coloured with.
type Key = (String, String);
/// The coloured blobs, least recently used first.
type Kept = VecDeque<(Key, Arc<Vec<Line>>)>;

fn cache() -> &'static Mutex<Kept> {
    static CACHE: OnceLock<Mutex<Kept>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn cached(key: &Key) -> Option<Arc<Vec<Line>>> {
    let mut cache = cache().lock().unwrap_or_else(|p| p.into_inner());
    let at = cache.iter().position(|(held, _)| held == key)?;
    // Most recently used at the back.
    let entry = cache.remove(at)?;
    let lines = entry.1.clone();
    cache.push_back(entry);
    Some(lines)
}

fn keep(key: Key, lines: Arc<Vec<Line>>) {
    let mut cache = cache().lock().unwrap_or_else(|p| p.into_inner());
    cache.retain(|(held, _)| held != &key);
    cache.push_back((key, lines));
    while cache.len() > CACHE_BLOBS {
        cache.pop_front();
    }
}

/// How many coloured blobs are held. For the tests.
pub fn cached_blobs() -> usize {
    cache().lock().unwrap_or_else(|p| p.into_inner()).len()
}

/// The colour of lines `start..end` (0-based, end exclusive) of blob `blob`.
///
/// The whole blob is coloured on the first request and kept by its id, so the
/// next request — the next screenful — is a slice. A blob over
/// [`COLOUR_MAX_BYTES`] gets no colour: an empty answer, not an error.
pub fn spans(
    repo: &gix::Repository,
    blob: &str,
    grammar: &str,
    start: usize,
    end: usize,
) -> Result<Vec<Line>> {
    let key = (blob.to_string(), canonical(grammar).to_string());
    let lines = match cached(&key) {
        Some(lines) => lines,
        None => {
            let id = gix::ObjectId::from_hex(blob.as_bytes())
                .map_err(|_| Error::UnknownCommit(blob.to_string()))?;
            let header = repo
                .find_header(id)
                .map_err(|error| Error::Diff(error.to_string()))?;
            if header.size() > COLOUR_MAX_BYTES {
                return Ok(Vec::new());
            }
            let object = repo
                .find_object(id)
                .map_err(|error| Error::Diff(error.to_string()))?;
            let text = String::from_utf8_lossy(&object.data);
            let lines = Arc::new(colour(&text, grammar)?);
            keep(key, lines.clone());
            lines
        }
    };
    let end = end.min(lines.len());
    let start = start.min(end);
    Ok(lines[start..end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    /// Stage everything, new files included, and commit.
    fn save(fixture: &Fixture, message: &str) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit(message)
    }

    /// A line's runs as `(class name, text)`, cut from the line by UTF-16 length.
    fn painted(line: &Line, source: &str) -> Vec<(&'static str, String)> {
        let units: Vec<u16> = source.encode_utf16().collect();
        let mut at = 0;
        line.chunks(2)
            .map(|run| {
                let length = run[1] as usize;
                let text = String::from_utf16_lossy(&units[at..at + length]);
                at += length;
                (CLASSES[run[0] as usize], text)
            })
            .collect()
    }

    fn class_of(lines: &[Line], source: &str, row: usize, word: &str) -> &'static str {
        let line = source.split('\n').nth(row).expect("a line");
        painted(&lines[row], line)
            .into_iter()
            .find(|(_, text)| text.contains(word))
            .map(|(class, _)| class)
            .unwrap_or_else(|| panic!("{word} is not on line {row}"))
    }

    #[test]
    fn every_grammar_listed_builds_with_its_query() {
        for grammar in GRAMMARS {
            assert!(has_grammar(grammar), "{grammar} did not build");
        }
    }

    #[test]
    fn a_language_with_no_grammar_says_so() {
        assert!(!has_grammar("kotlin"));
        assert!(!has_grammar("dockerfile"));
        assert!(!has_grammar("cobol"));
        assert!(colour("x", "cobol").is_err());
    }

    #[test]
    fn a_path_finds_its_grammar() {
        assert_eq!(grammar_for("crates/core/src/lib.rs"), Some("rust"));
        assert_eq!(grammar_for("src/App.tsx"), Some("tsx"));
        assert_eq!(grammar_for("src/app.ts"), Some("typescript"));
        assert_eq!(grammar_for("src/lib/Thing.svelte"), Some("svelte"));
        assert_eq!(grammar_for("Cargo.lock"), Some("toml"));
        assert_eq!(grammar_for("a/b/Cargo.toml"), Some("toml"));
        assert_eq!(grammar_for("README.MD"), Some("markdown"));
        assert_eq!(grammar_for("query.sql"), Some("sql"));
        assert_eq!(grammar_for("Main.kt"), None);
        assert_eq!(grammar_for("Dockerfile"), None);
        assert_eq!(grammar_for("LICENSE"), None);
        // Every name it gives is a grammar that builds.
        for path in [
            "a.rs", "a.ts", "a.tsx", "a.js", "a.svelte", "a.py", "a.go", "a.java", "a.c", "a.cpp",
            "a.cs", "a.json", "a.toml", "a.yaml", "a.md", "a.css", "a.html", "a.sh", "a.sql",
        ] {
            let grammar = grammar_for(path).expect(path);
            assert!(has_grammar(grammar), "{path} → {grammar}");
        }
    }

    #[test]
    fn rust_is_coloured_by_what_each_word_is() {
        let source = "use std::time::Duration;\n\n/// The avatar.\npub fn avatar(email: &str) -> Result<Avatar> {\n    let key = format!(\"{email}.png\");\n    Ok(42)\n}\n";
        let lines = colour(source, "rust").expect("coloured");

        assert_eq!(lines.len(), 7);
        assert_eq!(class_of(&lines, source, 0, "use"), "keyword");
        assert_eq!(class_of(&lines, source, 0, "Duration"), "type");
        assert_eq!(class_of(&lines, source, 2, "avatar"), "comment");
        assert_eq!(class_of(&lines, source, 3, "fn"), "keyword");
        assert_eq!(class_of(&lines, source, 3, "avatar"), "fn");
        assert_eq!(class_of(&lines, source, 3, "Result"), "type");
        assert_eq!(class_of(&lines, source, 4, "format"), "meta");
        assert_eq!(class_of(&lines, source, 4, "png"), "string");
        assert_eq!(class_of(&lines, source, 5, "42"), "number");
        // An empty line has no runs, and is still a line.
        assert!(lines[1].is_empty());
    }

    #[test]
    fn every_line_is_covered_exactly_whatever_is_in_it() {
        // Two-unit characters, a CRLF, and no newline at the end.
        let source = "let a = \"héllo 🍝\";\r\nlet b = 1;";
        let lines = colour(source, "javascript").expect("coloured");
        let expected = ["let a = \"héllo 🍝\";", "let b = 1;"];

        assert_eq!(lines.len(), 2);
        for (line, text) in lines.iter().zip(expected) {
            let length: u32 = line.chunks(2).map(|run| run[1]).sum();
            assert_eq!(length as usize, text.encode_utf16().count(), "{text}");
            let joined: String = painted(line, text).into_iter().map(|(_, t)| t).collect();
            assert_eq!(joined, text);
        }
        assert_eq!(class_of(&lines, "let a = \"héllo 🍝\";", 0, "🍝"), "string");
    }

    #[test]
    fn a_trailing_newline_ends_the_last_line_and_starts_none() {
        assert_eq!(colour("a = 1\n", "toml").expect("coloured").len(), 1);
        assert_eq!(colour("a = 1\n\n", "toml").expect("coloured").len(), 2);
        assert_eq!(colour("a = 1", "toml").expect("coloured").len(), 1);
        assert!(colour("", "toml").expect("coloured").is_empty());
    }

    #[test]
    fn a_markup_files_script_is_coloured_as_its_own_language() {
        let source = "<script>\n\tconst n = 1;\n</script>\n<p class=\"x\">hi</p>\n";
        let lines = colour(source, "html").expect("coloured");

        assert_eq!(class_of(&lines, source, 0, "script"), "tag");
        assert_eq!(class_of(&lines, source, 1, "const"), "keyword");
        assert_eq!(class_of(&lines, source, 3, "class"), "attr");
        assert_eq!(class_of(&lines, source, 3, "x"), "string");
    }

    #[test]
    fn config_files_colour_their_keys() {
        let toml = "name = \"spagitty\"\n";
        assert_eq!(
            class_of(&colour(toml, "toml").unwrap(), toml, 0, "name"),
            "attr"
        );
        let json = "{ \"name\": true }\n";
        let lines = colour(json, "json").unwrap();
        assert_eq!(class_of(&lines, json, 0, "true"), "number");
    }

    #[test]
    fn a_blob_is_coloured_once_and_served_in_slices() {
        let fixture = Fixture::empty();
        let source: String = (0..50).map(|n| format!("let v{n} = {n};\n")).collect();
        fixture.write("many.rs", &source);
        save(&fixture, "Add many");
        let repo = fixture.open();
        let blob = fixture.rev("HEAD:many.rs");

        let first = spans(&repo, &blob, "rust", 0, 10).expect("spans");
        assert_eq!(first.len(), 10);
        let middle = spans(&repo, &blob, "rust", 20, 25).expect("spans");
        assert_eq!(middle.len(), 5);
        assert_eq!(
            painted(&middle[0], "let v20 = 20;")
                .into_iter()
                .map(|(_, text)| text)
                .collect::<String>(),
            "let v20 = 20;"
        );
        // Past the end is the end, not an error.
        assert_eq!(
            spans(&repo, &blob, "rust", 45, 999).expect("spans").len(),
            5
        );
        assert!(spans(&repo, &blob, "rust", 80, 90)
            .expect("spans")
            .is_empty());

        // Kept by blob id: the object can go and the colour is still served.
        let held = cached(&(blob.clone(), "rust".to_string())).expect("kept");
        assert_eq!(held.len(), 50);
    }

    #[test]
    fn the_cache_keeps_the_newest_and_lets_the_oldest_go() {
        let first: Key = ("first-blob".into(), "rust".into());
        keep(first.clone(), Arc::new(Vec::new()));
        for n in 0..CACHE_BLOBS {
            keep((format!("blob-{n}"), "rust".into()), Arc::new(Vec::new()));
        }
        assert!(cached(&first).is_none(), "the oldest was let go");
        assert!(cached_blobs() <= CACHE_BLOBS);
        assert!(cached(&(format!("blob-{}", CACHE_BLOBS - 1), "rust".into())).is_some());
    }

    #[test]
    fn a_blob_that_does_not_exist_is_refused() {
        let fixture = Fixture::linear(1);
        let repo = fixture.open();

        assert!(spans(&repo, "not-an-id", "rust", 0, 1).is_err());
        assert!(spans(&repo, &"0".repeat(40), "rust", 0, 1).is_err());
    }
}
