// SPDX-License-Identifier: GPL-3.0-or-later

//! What a response body is, so the screen knows how to show it.
//!
//! Text is shown, with colour while it is small enough. Anything else is a
//! type and a size and *Save as…* (proposal §5.3).

use serde::Serialize;

/// Past this, a body is shown without colour: 2 MB.
pub const COLOUR_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    Json,
    Xml,
    Html,
    /// `text/event-stream`: shown as events as they arrive.
    Events,
    Text,
    Binary,
}

/// The media type without its parameters, lower case.
pub fn media_type(content_type: Option<&str>) -> String {
    content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

/// What `body`, sent as `content_type`, is.
pub fn shape(content_type: Option<&str>, body: &[u8]) -> Shape {
    let media = media_type(content_type);
    if media == "text/event-stream" {
        return Shape::Events;
    }
    if media.ends_with("json") || media.ends_with("+json") {
        return Shape::Json;
    }
    if media.ends_with("/xml") || media.ends_with("+xml") {
        return Shape::Xml;
    }
    if media == "text/html" {
        return Shape::Html;
    }
    let texty = media.starts_with("text/")
        || media.ends_with("javascript")
        || media == "application/x-www-form-urlencoded"
        || media == "application/yaml"
        || media == "application/graphql";
    if !texty && !looks_like_text(body) {
        return Shape::Binary;
    }
    // A server that said nothing, or `text/plain`, may still have sent JSON
    // or XML; shown formatted it is easier to read and to keep from.
    let start = body.iter().find(|b| !b.is_ascii_whitespace()).copied();
    match start {
        Some(b'{') | Some(b'[') if serde_json::from_slice::<serde_json::Value>(body).is_ok() => {
            Shape::Json
        }
        Some(b'<') if media != "text/plain" || body.starts_with(b"<?xml") => Shape::Xml,
        _ => Shape::Text,
    }
}

/// Valid UTF-8 with no NUL in the first 8 KB.
fn looks_like_text(body: &[u8]) -> bool {
    let head = &body[..body.len().min(8192)];
    if head.contains(&0) {
        return false;
    }
    match std::str::from_utf8(head) {
        Ok(_) => true,
        // A multi-byte character cut by the 8 KB boundary is still text.
        Err(error) => error.error_len().is_none() && error.valid_up_to() + 4 > head.len(),
    }
}

/// Pieces of an event stream turned into text as they arrive, without
/// splitting a character across two pieces.
#[derive(Debug, Default)]
pub struct Utf8Stream {
    held: Vec<u8>,
}

impl Utf8Stream {
    pub fn push(&mut self, piece: &[u8]) -> String {
        self.held.extend_from_slice(piece);
        let valid = match std::str::from_utf8(&self.held) {
            Ok(_) => self.held.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => self.held.len(),
        };
        let rest = self.held.split_off(valid);
        let text = String::from_utf8_lossy(&self.held).into_owned();
        self.held = rest;
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_come_from_the_type_and_then_the_bytes() {
        assert_eq!(
            shape(Some("application/json; charset=utf-8"), b"{}"),
            Shape::Json
        );
        assert_eq!(shape(Some("application/problem+json"), b"{}"), Shape::Json);
        assert_eq!(shape(Some("text/xml"), b"<a/>"), Shape::Xml);
        assert_eq!(shape(Some("application/soap+xml"), b"<a/>"), Shape::Xml);
        assert_eq!(shape(Some("text/event-stream"), b"data: 1"), Shape::Events);
        assert_eq!(shape(Some("text/html"), b"<p>"), Shape::Html);
        assert_eq!(
            shape(Some("application/pdf"), b"%PDF-1.7\0\x01"),
            Shape::Binary
        );
        assert_eq!(
            shape(Some("image/png"), &[0x89, b'P', b'N', b'G', 0, 0]),
            Shape::Binary
        );
        assert_eq!(shape(None, b"{\"a\":1}"), Shape::Json);
        assert_eq!(shape(Some("text/plain"), b"{\"a\":1}"), Shape::Json);
        assert_eq!(shape(Some("text/plain"), b"hello"), Shape::Text);
        assert_eq!(shape(None, b"<?xml version=\"1.0\"?><a/>"), Shape::Xml);
    }

    #[test]
    fn a_stream_never_splits_a_character() {
        let mut stream = Utf8Stream::default();
        let text = "data: é\n";
        let bytes = text.as_bytes();
        let cut = bytes.iter().position(|b| *b >= 0x80).unwrap() + 1;
        let first = stream.push(&bytes[..cut]);
        let second = stream.push(&bytes[cut..]);
        assert_eq!(format!("{first}{second}"), text);
        assert_eq!(first, "data: ");
    }
}
