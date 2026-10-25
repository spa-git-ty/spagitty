// SPDX-License-Identifier: GPL-3.0-or-later

//! `{{name}}` in a URL, a header, a body or an envelope (proposal §5.2).
//!
//! A name is looked up in the active environment, then in the variables every
//! environment shares, then in the collection's own. A name found nowhere is
//! an error that says which, so a request is never sent with `{{x}}` in it.
//!
//! Four built-ins are read-only and start with `$`: `$timestamp` (seconds
//! since 1970), `$isoNow`, `$uuid` and `$randomInt` (0 to 999).
//!
//! What goes into history is the same text resolved with every secret and
//! kept value left as its `{{name}}` ([`Scope::redacted`]).

use std::collections::HashMap;

use crate::calls::model::{random_u64, Environment, Shared, Variable};

/// One `{{name}}` in a text: where it is, in bytes, and the name inside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub name: String,
}

/// Every `{{name}}` in `text`. Spaces inside the braces are allowed and not
/// part of the name; empty braces are not a variable.
pub fn find(text: &str) -> Vec<Found> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("{{") {
        let start = at + open;
        let Some(close) = text[start + 2..].find("}}") else {
            break;
        };
        let end = start + 2 + close + 2;
        let inner = &text[start + 2..start + 2 + close];
        let name = inner.trim();
        if !name.is_empty() && !name.contains('{') && !name.contains('\n') {
            out.push(Found {
                start,
                end,
                name: name.to_string(),
            });
            at = end;
        } else {
            at = start + 2;
        }
    }
    out
}

/// A variable's value as resolution sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub value: String,
    /// A secret or a kept value: never written to history, a file or a
    /// share.
    pub private: bool,
}

/// What `{{name}}` can be: the layers in the order they are asked.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// The active environment's name, for the message about a missing one.
    pub environment: Option<String>,
    pub layers: Vec<HashMap<String, Value>>,
}

/// A name nothing defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub name: String,
    pub environment: Option<String>,
}

impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.environment {
            Some(env) => write!(f, "{{{{{}}}}} has no value in {env}", self.name),
            None => write!(
                f,
                "{{{{{}}}}} has no value: choose an environment that has it",
                self.name
            ),
        }
    }
}

impl Scope {
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.layers.iter().find_map(|layer| layer.get(name))
    }

    /// `text` with every variable replaced by its value.
    pub fn resolve(&self, text: &str) -> Result<String, Missing> {
        self.replace(text, false)
    }

    /// `text` with every plain variable replaced, and every secret and kept
    /// one left as `{{name}}`. What history records.
    pub fn redacted(&self, text: &str) -> Result<String, Missing> {
        self.replace(text, true)
    }

    /// The names in `text` that nothing defines, each once, in order.
    pub fn missing(&self, text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for found in find(text) {
            if builtin(&found.name).is_none()
                && self.lookup(&found.name).is_none()
                && !out.contains(&found.name)
            {
                out.push(found.name);
            }
        }
        out
    }

    /// Every private value in the scope, longest first, to cut out of an
    /// error message or a response that echoes one back.
    pub fn private_values(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .layers
            .iter()
            .flat_map(|layer| layer.values())
            .filter(|value| value.private && value.value.len() >= 4)
            .map(|value| value.value.clone())
            .collect();
        out.sort_by_key(|value| std::cmp::Reverse(value.len()));
        out.dedup();
        out
    }

    fn replace(&self, text: &str, redact: bool) -> Result<String, Missing> {
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for found in find(text) {
            out.push_str(&text[at..found.start]);
            at = found.end;
            if let Some(value) = builtin(&found.name) {
                out.push_str(&value);
                continue;
            }
            match self.lookup(&found.name) {
                Some(value) if redact && value.private => {
                    out.push_str(&text[found.start..found.end])
                }
                Some(value) => out.push_str(&value.value),
                None => {
                    return Err(Missing {
                        name: found.name,
                        environment: self.environment.clone(),
                    })
                }
            }
        }
        out.push_str(&text[at..]);
        Ok(out)
    }
}

/// One layer: each variable with its value, a private one's from `private`
/// (the keychain's). A private variable with nothing stored for it is left
/// out, so a call that uses it says it has no value rather than sending an
/// empty one.
pub fn layer(vars: &[Variable], private: &HashMap<String, String>) -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for var in vars.iter().filter(|var| !var.name.is_empty()) {
        let value = if var.private() {
            private.get(&var.name).cloned()
        } else {
            Some(var.value.clone().unwrap_or_default())
        };
        if let Some(value) = value {
            out.insert(
                var.name.clone(),
                Value {
                    value,
                    private: var.private(),
                },
            );
        }
    }
    out
}

/// The scope a call is resolved in: the environment, then what every
/// environment shares, then the collection's own.
pub fn scope_of(
    environment: Option<(&Environment, &HashMap<String, String>)>,
    shared: (&Shared, &HashMap<String, String>),
    collection: &[Variable],
) -> Scope {
    let mut layers = Vec::new();
    if let Some((env, private)) = environment {
        layers.push(layer(&env.vars, private));
    }
    layers.push(layer(&shared.0.vars, shared.1));
    layers.push(layer(collection, &HashMap::new()));
    Scope {
        environment: environment.map(|(env, _)| env.name.clone()),
        layers,
    }
}

/// The value of a built-in, or `None` for any other name.
pub fn builtin(name: &str) -> Option<String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    match name {
        "$timestamp" => Some(now.as_secs().to_string()),
        "$isoNow" => Some(iso(now.as_millis() as i64)),
        "$uuid" => Some(uuid()),
        "$randomInt" => Some((random_u64() % 1000).to_string()),
        _ => None,
    }
}

/// The built-ins, documented where they are offered.
pub const BUILTINS: [&str; 4] = ["$timestamp", "$isoNow", "$uuid", "$randomInt"];

/// A version 4 UUID.
pub fn uuid() -> String {
    let high = random_u64();
    let low = random_u64();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&high.to_be_bytes());
    bytes[8..].copy_from_slice(&low.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Milliseconds since 1970 as `2026-10-25T14:03:00.123Z`.
pub fn iso(millis: i64) -> String {
    let secs = millis.div_euclid(1000);
    let ms = millis.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Howard Hinnant's `civil_from_days`: the date `days` after 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> Scope {
        let mut env = HashMap::new();
        env.insert(
            "baseUrl".to_string(),
            Value {
                value: "https://staging".into(),
                private: false,
            },
        );
        env.insert(
            "password".to_string(),
            Value {
                value: "hunter22".into(),
                private: true,
            },
        );
        let mut shared = HashMap::new();
        shared.insert(
            "baseUrl".to_string(),
            Value {
                value: "https://shared".into(),
                private: false,
            },
        );
        shared.insert(
            "tenant".to_string(),
            Value {
                value: "acme".into(),
                private: false,
            },
        );
        Scope {
            environment: Some("staging".into()),
            layers: vec![env, shared],
        }
    }

    #[test]
    fn names_are_found_with_their_places() {
        let found = find("{{baseUrl}}/x/{{ id }}?a={{}}&b={{b");
        assert_eq!(
            found,
            vec![
                Found {
                    start: 0,
                    end: 11,
                    name: "baseUrl".into()
                },
                Found {
                    start: 14,
                    end: 22,
                    name: "id".into()
                },
            ]
        );
    }

    #[test]
    fn the_environment_wins_over_shared() {
        let resolved = scope().resolve("{{baseUrl}}/t/{{tenant}}").unwrap();
        assert_eq!(resolved, "https://staging/t/acme");
    }

    #[test]
    fn an_unknown_name_is_said_with_its_environment() {
        let error = scope().resolve("{{baseUrl}}/{{orderId}}").unwrap_err();
        assert_eq!(error.name, "orderId");
        assert_eq!(error.to_string(), "{{orderId}} has no value in staging");
        assert_eq!(scope().missing("{{a}}{{a}}{{tenant}}{{$uuid}}"), vec!["a"]);
    }

    #[test]
    fn history_keeps_secrets_as_their_names() {
        let text = scope()
            .redacted("{\"p\":\"{{password}}\",\"u\":\"{{baseUrl}}\"}")
            .unwrap();
        assert_eq!(text, "{\"p\":\"{{password}}\",\"u\":\"https://staging\"}");
        assert_eq!(scope().private_values(), vec!["hunter22".to_string()]);
    }

    #[test]
    fn builtins_have_their_shapes() {
        let id = builtin("$uuid").unwrap();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert!(builtin("$timestamp").unwrap().parse::<u64>().is_ok());
        assert!(builtin("$randomInt").unwrap().parse::<u32>().unwrap() < 1000);
        assert!(builtin("$isoNow").unwrap().ends_with('Z'));
        assert!(builtin("other").is_none());
    }

    #[test]
    fn iso_dates_are_right() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_793_016_180_123), "2026-10-26T12:03:00.123Z");
        assert_eq!(iso(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }
}
