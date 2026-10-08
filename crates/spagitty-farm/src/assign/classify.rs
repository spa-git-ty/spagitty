// SPDX-License-Identifier: GPL-3.0-or-later

//! Spagitty names the choice, not the agent.
//!
//! An agent proposes the text a conflict region should become. Spagitty reads
//! that text against the two sides: if it is exactly A, exactly B, A then B, B
//! then A, or some of A's lines followed by some of B's, it *is* that choice —
//! the resolver's own vocabulary, which the person already knows how to read.
//! Anything else is *Edit*: the agent's own text, badged as the agent's.
//!
//! The order of the tests matters only where a text could be two things at
//! once — two empty sides, say — and the first match is the plainest name.

use serde::{Deserialize, Serialize};

/// One region's choice, in the resolver's words. Serialises to the same shape
/// as the webview's `Choice` (`src/lib/resolver/model.ts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Choice {
    A,
    B,
    Ab,
    Ba,
    Pick { a: Vec<bool>, b: Vec<bool> },
    Edit { text: String },
}

impl Choice {
    /// The name on the conflict card.
    pub fn label(&self) -> &'static str {
        match self {
            Choice::A => "Take A",
            Choice::B => "Take B",
            Choice::Ab => "Both, A first",
            Choice::Ba => "Both, B first",
            Choice::Pick { .. } => "Pick lines",
            Choice::Edit { .. } => "Edit",
        }
    }
}

/// The lines of a proposed text, as the resolver splits an edit.
pub fn lines_of(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    trimmed
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_string())
        .collect()
}

/// Name the choice `text` makes between `a` and `b`.
pub fn classify(a: &[String], b: &[String], text: &str) -> Choice {
    let got = lines_of(text);
    if got == a {
        return Choice::A;
    }
    if got == b {
        return Choice::B;
    }
    if got.len() == a.len() + b.len() {
        if got[..a.len()] == *a && got[a.len()..] == *b {
            return Choice::Ab;
        }
        if got[..b.len()] == *b && got[b.len()..] == *a {
            return Choice::Ba;
        }
    }
    if let Some((ticks_a, ticks_b)) = pick(a, b, &got) {
        return Choice::Pick {
            a: ticks_a,
            b: ticks_b,
        };
    }
    Choice::Edit {
        text: text.to_string(),
    }
}

/// Some of A's lines in order, then some of B's, and nothing else.
///
/// Every split point is tried: a line both sides share can belong to either,
/// and the first split that works is as good as any other — the result is the
/// same text.
fn pick(a: &[String], b: &[String], got: &[String]) -> Option<(Vec<bool>, Vec<bool>)> {
    for split in 0..=got.len() {
        let (head, tail) = got.split_at(split);
        if let (Some(ticks_a), Some(ticks_b)) = (subsequence(a, head), subsequence(b, tail)) {
            return Some((ticks_a, ticks_b));
        }
    }
    None
}

/// Which of `side`'s lines, in order, make `wanted`. Greedy is enough: taking
/// the earliest match never prevents a later one.
fn subsequence(side: &[String], wanted: &[String]) -> Option<Vec<bool>> {
    let mut ticks = vec![false; side.len()];
    let mut at = 0;
    for line in wanted {
        while at < side.len() && side[at] != *line {
            at += 1;
        }
        if at == side.len() {
            return None;
        }
        ticks[at] = true;
        at += 1;
    }
    Some(ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn exactly_one_side_is_that_side() {
        let (a, b) = (v(&["x", "y"]), v(&["z"]));
        assert_eq!(classify(&a, &b, "x\ny\n"), Choice::A);
        assert_eq!(classify(&a, &b, "z"), Choice::B);
    }

    #[test]
    fn both_sides_in_either_order() {
        let (a, b) = (v(&["x"]), v(&["z"]));
        assert_eq!(classify(&a, &b, "x\nz"), Choice::Ab);
        assert_eq!(classify(&a, &b, "z\nx"), Choice::Ba);
    }

    #[test]
    fn some_of_a_then_some_of_b_is_a_pick() {
        let (a, b) = (v(&["one", "two", "three"]), v(&["four", "five"]));
        assert_eq!(
            classify(&a, &b, "one\nthree\nfive"),
            Choice::Pick {
                a: vec![true, false, true],
                b: vec![false, true]
            }
        );
    }

    #[test]
    fn b_before_a_with_lines_left_out_is_an_edit() {
        let (a, b) = (v(&["one", "two"]), v(&["three", "four"]));
        assert!(matches!(classify(&a, &b, "four\none"), Choice::Edit { .. }));
    }

    #[test]
    fn a_new_line_is_an_edit_with_the_agents_text() {
        let (a, b) = (v(&["fn f(x)"]), v(&["fn g()"]));
        assert_eq!(
            classify(&a, &b, "fn g(x)"),
            Choice::Edit {
                text: "fn g(x)".into()
            }
        );
    }

    #[test]
    fn crlf_lines_compare_as_their_text() {
        let (a, b) = (v(&["x"]), v(&["y"]));
        assert_eq!(classify(&a, &b, "x\r\n"), Choice::A);
    }

    #[test]
    fn choices_serialise_like_the_resolvers() {
        assert_eq!(
            serde_json::to_string(&Choice::Ab).unwrap(),
            r#"{"mode":"ab"}"#
        );
        assert_eq!(
            serde_json::to_string(&Choice::Edit { text: "t".into() }).unwrap(),
            r#"{"mode":"edit","text":"t"}"#
        );
        assert_eq!(
            serde_json::to_string(&Choice::Pick {
                a: vec![true],
                b: vec![]
            })
            .unwrap(),
            r#"{"mode":"pick","a":[true],"b":[]}"#
        );
    }
}
