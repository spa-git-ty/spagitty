// SPDX-License-Identifier: GPL-3.0-or-later

//! WS-Security's username token (FEAT-163, proposal §7.1): a
//! `wsse:Security` header with the user, the password as text or as a
//! digest, a nonce and the time it was made. *Signed* is FEAT-167.
//!
//! As text, the password is put in as the call has it — usually
//! `{{password}}` — so it is resolved when sent and stays a name in
//! history. As a digest, Base64(SHA-1(nonce + created + password)), it is
//! resolved here to be hashed, and only the hash is sent or kept.

use super::super::build::base64;
use super::super::xml::escape;
use super::Version;

const WSSE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
const WSU: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";
const TEXT: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";
const DIGEST: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";
const BASE64: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";

/// SHA-1, for the password digest the username token profile defines.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut message = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());
    for block in message.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The password, as the token carries it.
pub enum Password<'a> {
    /// As the call has it, `{{password}}` most often.
    Text(&'a str),
    /// The resolved password, to be hashed.
    Digest(&'a str),
}

/// `envelope` with a username token in its header. `nonce` is random
/// bytes and `created` the time, `2026-01-31T09:30:00.000Z`.
pub fn with_username(
    envelope: &str,
    version: Version,
    user: &str,
    password: Password,
    nonce: &[u8],
    created: &str,
) -> String {
    let must = match version {
        Version::V11 => "1",
        Version::V12 => "true",
    };
    let (kind, value) = match password {
        Password::Text(text) => (TEXT, text.to_string()),
        Password::Digest(secret) => {
            let mut input = nonce.to_vec();
            input.extend_from_slice(created.as_bytes());
            input.extend_from_slice(secret.as_bytes());
            (DIGEST, base64(&sha1(&input)))
        }
    };
    // The envelope's own prefix, from its root tag.
    let prefix = envelope
        .find("Envelope")
        .and_then(|at| {
            envelope[..at]
                .rfind('<')
                .map(|open| &envelope[open + 1..at])
        })
        .unwrap_or("soapenv:");
    let header = format!(
        "\n    <wsse:Security xmlns:wsse=\"{WSSE}\" xmlns:wsu=\"{WSU}\" {prefix}mustUnderstand=\"{must}\">\n      <wsse:UsernameToken wsu:Id=\"UsernameToken-1\">\n        <wsse:Username>{}</wsse:Username>\n        <wsse:Password Type=\"{kind}\">{}</wsse:Password>\n        <wsse:Nonce EncodingType=\"{BASE64}\">{}</wsse:Nonce>\n        <wsu:Created>{created}</wsu:Created>\n      </wsse:UsernameToken>\n    </wsse:Security>\n  ",
        escape(user),
        escape(&value),
        base64(nonce),
    );
    let empty = format!("<{prefix}Header/>");
    let open = format!("<{prefix}Header>");
    if envelope.contains(&empty) {
        return envelope.replacen(
            &empty,
            &format!("<{prefix}Header>{header}</{prefix}Header>"),
            1,
        );
    }
    if let Some(at) = envelope.find(&open) {
        let end = at + open.len();
        return format!("{}{}{}", &envelope[..end], header, &envelope[end..]);
    }
    let body = format!("<{prefix}Body");
    match envelope.find(&body) {
        Some(at) => format!(
            "{}<{prefix}Header>{header}</{prefix}Header>\n  {}",
            &envelope[..at],
            &envelope[at..]
        ),
        None => envelope.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn sha1_matches_the_standard_vectors() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&sha1(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
    }

    const ENVELOPE: &str = "<soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\">\n  <soapenv:Header/>\n  <soapenv:Body>\n    <x/>\n  </soapenv:Body>\n</soapenv:Envelope>";

    #[test]
    fn a_text_token_keeps_the_password_as_the_call_has_it() {
        let out = with_username(
            ENVELOPE,
            Version::V11,
            "{{user}}",
            Password::Text("{{password}}"),
            b"nonce-16-bytes!!",
            "2026-01-31T09:30:00.000Z",
        );
        assert!(out.contains("<soapenv:Header>\n    <wsse:Security"));
        assert!(out.contains("soapenv:mustUnderstand=\"1\""));
        assert!(out.contains("<wsse:Username>{{user}}</wsse:Username>"));
        assert!(out.contains("#PasswordText\">{{password}}</wsse:Password>"));
        assert!(out.contains(&format!(">{}</wsse:Nonce>", base64(b"nonce-16-bytes!!"))));
        assert!(out.contains("<wsu:Created>2026-01-31T09:30:00.000Z</wsu:Created>"));
        assert!(out.contains("</soapenv:Header>"));
    }

    #[test]
    fn a_digest_is_the_profiles_hash() {
        let nonce = b"0123456789abcdef";
        let created = "2026-01-31T09:30:00.000Z";
        let out = with_username(
            ENVELOPE,
            Version::V12,
            "mira",
            Password::Digest("hunter2"),
            nonce,
            created,
        );
        let mut input = nonce.to_vec();
        input.extend_from_slice(created.as_bytes());
        input.extend_from_slice(b"hunter2");
        assert!(out.contains(&format!(
            "#PasswordDigest\">{}</wsse:Password>",
            base64(&sha1(&input))
        )));
        assert!(!out.contains("hunter2"));
        assert!(out.contains("mustUnderstand=\"true\""));
        // No header at all: one is made before the body.
        let bare = "<s:Envelope xmlns:s=\"x\"><s:Body/></s:Envelope>";
        assert!(
            with_username(bare, Version::V11, "u", Password::Text("p"), b"n", "c")
                .contains("<s:Header>")
        );
    }
}
