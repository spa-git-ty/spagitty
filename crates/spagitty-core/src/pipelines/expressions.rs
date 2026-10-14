// SPDX-License-Identifier: GPL-3.0-or-later

//! Three-valued expressions: an unavailable context is never a false result.
use super::Simulation;
use regex::RegexBuilder;

#[derive(Clone, Debug)]
enum Token {
    Text(String),
    Name(String),
    Regex(String, bool),
    Op(&'static str),
    End,
}
#[derive(Clone, Debug)]
enum Value {
    Text(String),
    Bool(bool),
    Pattern(String, bool),
    Unknown,
}
impl Value {
    fn truth(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            Self::Text(v) => Some(!v.is_empty()),
            _ => None,
        }
    }
}
fn tokens(text: &str) -> Option<Vec<Token>> {
    if text.len() > 4096 {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '\'' || c == '"' {
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == c {
                    if c == '\'' && chars.get(i + 1) == Some(&c) {
                        value.push(c);
                        i += 2;
                        continue;
                    }
                    i += 1;
                    closed = true;
                    break;
                }
                value.push(chars[i]);
                i += 1;
            }
            if !closed {
                return None;
            }
            out.push(Token::Text(value));
            continue;
        }
        if c == '/' {
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '/' {
                    i += 1;
                    closed = true;
                    break;
                }
                if chars[i] == '\\' && chars.get(i + 1) == Some(&'/') {
                    value.push('/');
                    i += 2;
                } else {
                    value.push(chars[i]);
                    i += 1;
                }
            }
            if !closed || value.len() > 512 {
                return None;
            }
            let insensitive = chars.get(i) == Some(&'i');
            if insensitive {
                i += 1;
            }
            out.push(Token::Regex(value, insensitive));
            continue;
        }
        let pair: String = chars[i..chars.len().min(i + 2)].iter().collect();
        if let Some(op) = match pair.as_str() {
            "==" => Some("=="),
            "!=" => Some("!="),
            "&&" => Some("&&"),
            "||" => Some("||"),
            "=~" => Some("=~"),
            "!~" => Some("!~"),
            _ => None,
        } {
            out.push(Token::Op(op));
            i += 2;
            continue;
        }
        if let Some(op) = match c {
            '!' => Some("!"),
            '(' => Some("("),
            ')' => Some(")"),
            ',' => Some(","),
            _ => None,
        } {
            out.push(Token::Op(op));
            i += 1;
            continue;
        }
        if c.is_ascii_alphanumeric() || c == '$' || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '.' | '_'))
            {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
            continue;
        }
        return None;
    }
    out.push(Token::End);
    Some(out)
}
struct Reader<'a> {
    tokens: Vec<Token>,
    at: usize,
    context: &'a Simulation,
    depth: usize,
    github: bool,
}
impl Reader<'_> {
    fn take(&mut self, op: &str) -> bool {
        if matches!(self.tokens.get(self.at),Some(Token::Op(v)) if *v==op) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Value {
        let mut v = self.and();
        while self.take("||") {
            let b = self.and();
            v = match (v.truth(), b.truth()) {
                (Some(true), _) | (_, Some(true)) => Value::Bool(true),
                (Some(false), Some(false)) => Value::Bool(false),
                _ => Value::Unknown,
            };
        }
        v
    }
    fn and(&mut self) -> Value {
        let mut v = self.compare();
        while self.take("&&") {
            let b = self.compare();
            v = match (v.truth(), b.truth()) {
                (Some(false), _) | (_, Some(false)) => Value::Bool(false),
                (Some(true), Some(true)) => Value::Bool(true),
                _ => Value::Unknown,
            };
        }
        v
    }
    fn compare(&mut self) -> Value {
        let a = self.atom();
        let op = match self.tokens.get(self.at) {
            Some(Token::Op(op @ ("==" | "!=" | "=~" | "!~"))) => *op,
            _ => return a,
        };
        self.at += 1;
        let b = self.atom();
        let result = match (&a, &b) {
            (Value::Text(a), Value::Text(b)) if op == "==" || op == "!=" => Some(if self.github {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }),
            (Value::Bool(a), Value::Bool(b)) if op == "==" || op == "!=" => Some(a == b),
            (Value::Text(a), Value::Pattern(b, case)) if op == "=~" || op == "!~" => {
                RegexBuilder::new(b)
                    .case_insensitive(*case)
                    .size_limit(256 * 1024)
                    .build()
                    .ok()
                    .map(|r| r.is_match(a))
            }
            _ => None,
        };
        result
            .map(|v| Value::Bool(if op == "!=" || op == "!~" { !v } else { v }))
            .unwrap_or(Value::Unknown)
    }
    fn atom(&mut self) -> Value {
        self.depth += 1;
        if self.depth > 64 {
            return Value::Unknown;
        }
        let value = self.inner();
        self.depth -= 1;
        value
    }
    fn inner(&mut self) -> Value {
        if self.take("!") {
            return self
                .atom()
                .truth()
                .map(|v| Value::Bool(!v))
                .unwrap_or(Value::Unknown);
        }
        if self.take("(") {
            let v = self.or();
            if !self.take(")") {
                return Value::Unknown;
            }
            return v;
        }
        let token = self.tokens.get(self.at).cloned().unwrap_or(Token::End);
        self.at += 1;
        match token {
            Token::Text(v) => Value::Text(v),
            Token::Regex(v, c) => Value::Pattern(v, c),
            Token::Name(v) => {
                if self.take("(") {
                    let mut args = Vec::new();
                    if !self.take(")") {
                        loop {
                            args.push(self.or());
                            if self.take(")") {
                                break;
                            }
                            if !self.take(",") || args.len() > 8 {
                                return Value::Unknown;
                            }
                        }
                    }
                    return match (v.to_ascii_lowercase().as_str(), args.as_slice()) {
                        ("always" | "success", []) => Value::Bool(true),
                        ("startswith", [Value::Text(a), Value::Text(b)]) => {
                            Value::Bool(a.to_lowercase().starts_with(&b.to_lowercase()))
                        }
                        ("endswith", [Value::Text(a), Value::Text(b)]) => {
                            Value::Bool(a.to_lowercase().ends_with(&b.to_lowercase()))
                        }
                        ("contains", [Value::Text(a), Value::Text(b)]) => {
                            Value::Bool(a.to_lowercase().contains(&b.to_lowercase()))
                        }
                        _ => Value::Unknown,
                    };
                }
                let c = self.context;
                match v.as_str() {
                    "true" => Value::Bool(true),
                    "false" | "null" => Value::Bool(false),
                    "github.ref" if c.event != "pull_request" => Value::Text(format!(
                        "refs/{}/{}",
                        if c.event == "tag" { "tags" } else { "heads" },
                        c.reference
                    )),
                    "github.ref_name" if c.event != "pull_request" => {
                        Value::Text(c.reference.clone())
                    }
                    "github.base_ref" => Value::Text(if c.event == "pull_request" {
                        c.base.clone()
                    } else {
                        String::new()
                    }),
                    "github.event_name" => Value::Text(c.github_event().into()),
                    "$CI_COMMIT_BRANCH" => {
                        Value::Text(if c.event == "tag" || c.event == "pull_request" {
                            String::new()
                        } else {
                            c.reference.clone()
                        })
                    }
                    "$CI_COMMIT_TAG" => Value::Text(if c.event == "tag" {
                        c.reference.clone()
                    } else {
                        String::new()
                    }),
                    "$CI_PIPELINE_SOURCE" => Value::Text(c.gitlab_event().into()),
                    "$CI_DEFAULT_BRANCH" if !c.default_branch.is_empty() => {
                        Value::Text(c.default_branch.clone())
                    }
                    _ => Value::Unknown,
                }
            }
            _ => Value::Unknown,
        }
    }
}
pub fn evaluate(text: &str, c: &Simulation) -> Option<bool> {
    let text = text
        .trim()
        .strip_prefix("${{")
        .map(|s| s.trim_end().strip_suffix("}}").unwrap_or(s).trim())
        .unwrap_or(text.trim());
    if text.is_empty() {
        return Some(true);
    }
    let mut r = Reader {
        tokens: tokens(text)?,
        at: 0,
        context: c,
        depth: 0,
        github: !text.contains("$CI_"),
    };
    let v = r.or();
    if !matches!(r.tokens.get(r.at), Some(Token::End)) {
        return None;
    }
    v.truth()
}

pub fn glob(pattern: &str, text: &str, github: bool) -> Option<bool> {
    if pattern.len() > 512 {
        return None;
    }
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if chars.peek() == Some(&'/') {
                        chars.next();
                        regex.push_str("(?:.*/)?");
                    } else {
                        regex.push_str(".*");
                    }
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' if github => regex.push('?'),
            '?' => regex.push_str("[^/]"),
            '+' if github => regex.push('+'),
            '[' | ']' | '{' | '}' | '(' | ')' | '@' | '!' => return None,
            '\\' => {
                let next = chars.next()?;
                regex.push_str(&regex::escape(&next.to_string()));
            }
            _ => regex.push_str(&regex::escape(&c.to_string())),
        }
    }
    regex.push('$');
    RegexBuilder::new(&regex)
        .size_limit(256 * 1024)
        .build()
        .ok()
        .map(|r| r.is_match(text))
}
pub fn patterns(patterns: &[String], text: &str, github: bool) -> Option<bool> {
    let mut result = Some(false);
    for p in patterns {
        let (exclude, p) = p
            .strip_prefix('!')
            .map(|p| (true, p))
            .unwrap_or((false, p.as_str()));
        match glob(p, text, github) {
            Some(true) => result = Some(!exclude),
            Some(false) => {}
            None => result = None,
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expressions_are_conservative_and_keep_boolean_precedence() {
        let c = Simulation {
            event: "push".into(),
            reference: "main".into(),
            default_branch: "main".into(),
            ..Default::default()
        };
        assert_eq!(
            evaluate(
                "github.ref == 'refs/heads/main' && startsWith(github.ref_name, 'ma')",
                &c
            ),
            Some(true)
        );
        assert_eq!(evaluate("false && unknown() || always()", &c), Some(true));
        assert_eq!(
            evaluate("github.event.pull_request.number == '1'", &c),
            None
        );
        assert_eq!(
            evaluate(
                "$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH && $CI_COMMIT_BRANCH =~ /^ma/",
                &c
            ),
            Some(true)
        );
        assert_eq!(evaluate("$CI_COMMIT_BRANCH =~ /[invalid/", &c), None);
        assert_eq!(glob("src/**/test.rs", "src/test.rs", true), Some(true));
        assert_eq!(
            patterns(&["**".into(), "!docs/**".into()], "docs/a.md", true),
            Some(false)
        );
    }
}
