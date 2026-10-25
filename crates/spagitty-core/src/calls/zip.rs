// SPDX-License-Identifier: GPL-3.0-or-later

//! Zip, as far as a shared collection needs it (FEAT-164): files written
//! deflated, and read back whether deflated or stored. No encryption, no
//! ZIP64 (a collection is far below 4 GB), no extra fields.

use std::io::{Read, Write};

use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::{Compression, Crc};

use crate::{Error, Result};

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// 2026-01-01 00:00, in MS-DOS time: the same bytes for the same files.
const DOS_TIME: u16 = 0;
const DOS_DATE: u16 = ((2026 - 1980) << 9) | (1 << 5) | 1;

fn crc(bytes: &[u8]) -> u32 {
    let mut crc = Crc::new();
    crc.update(bytes);
    crc.sum()
}

/// `files` as a zip, each deflated, in the order given.
pub fn write(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, bytes) in files {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes)?;
        let packed = encoder.finish()?;
        let sum = crc(bytes);
        let offset = out.len() as u32;
        let name = name.as_bytes();
        // Bit 11: the name is UTF-8.
        let flags: u16 = 1 << 11;
        out.extend_from_slice(&LOCAL.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&DOS_TIME.to_le_bytes());
        out.extend_from_slice(&DOS_DATE.to_le_bytes());
        out.extend_from_slice(&sum.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&packed);

        central.extend_from_slice(&CENTRAL.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&flags.to_le_bytes());
        central.extend_from_slice(&8u16.to_le_bytes());
        central.extend_from_slice(&DOS_TIME.to_le_bytes());
        central.extend_from_slice(&DOS_DATE.to_le_bytes());
        central.extend_from_slice(&sum.to_le_bytes());
        central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        central.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let start = out.len() as u32;
    let size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&END.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    Ok(out)
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| broken("it ends too early"))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| broken("it ends too early"))
}

fn broken(why: &str) -> Error {
    Error::Calls(format!("this is not a zip Calls can read: {why}"))
}

/// The files in a zip, by name. A file bigger than `limit` once unpacked
/// is refused, so a crafted archive cannot fill the memory.
pub fn read(bytes: &[u8], limit: usize) -> Result<Vec<(String, Vec<u8>)>> {
    let end = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&at| u32_at(bytes, at).ok() == Some(END))
        .ok_or_else(|| broken("it has no end record"))?;
    let count = u16_at(bytes, end + 10)? as usize;
    let mut at = u32_at(bytes, end + 16)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(bytes, at)? != CENTRAL {
            return Err(broken("its directory is damaged"));
        }
        let method = u16_at(bytes, at + 10)?;
        let packed = u32_at(bytes, at + 20)? as usize;
        let size = u32_at(bytes, at + 24)? as usize;
        let name_len = u16_at(bytes, at + 28)? as usize;
        let extra = u16_at(bytes, at + 30)? as usize;
        let comment = u16_at(bytes, at + 32)? as usize;
        let local = u32_at(bytes, at + 42)? as usize;
        let name = String::from_utf8_lossy(
            bytes
                .get(at + 46..at + 46 + name_len)
                .ok_or_else(|| broken("a name is cut"))?,
        )
        .into_owned();
        at += 46 + name_len + extra + comment;
        if name.ends_with('/') {
            continue;
        }
        if size > limit {
            return Err(Error::Calls(format!(
                "{name} is bigger than a collection could be"
            )));
        }
        if u32_at(bytes, local)? != LOCAL {
            return Err(broken("a file's header is damaged"));
        }
        let start =
            local + 30 + u16_at(bytes, local + 26)? as usize + u16_at(bytes, local + 28)? as usize;
        let data = bytes
            .get(start..start + packed)
            .ok_or_else(|| broken("a file is cut"))?;
        let content = match method {
            0 => data.to_vec(),
            8 => {
                let mut content = Vec::with_capacity(size);
                DeflateDecoder::new(data)
                    .take(limit as u64 + 1)
                    .read_to_end(&mut content)?;
                content
            }
            other => {
                return Err(Error::Calls(format!(
                    "{name} is packed in a way Calls cannot read ({other})"
                )))
            }
        };
        if content.len() > limit {
            return Err(Error::Calls(format!(
                "{name} is bigger than a collection could be"
            )));
        }
        out.push((name, content));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zip_round_trips_and_is_the_same_bytes_each_time() {
        let files = vec![
            ("collection.json".to_string(), br#"{"v":1}"#.to_vec()),
            ("env/staging.json".to_string(), "é".repeat(500).into_bytes()),
            ("empty.txt".to_string(), Vec::new()),
        ];
        let packed = write(&files).unwrap();
        assert_eq!(read(&packed, 1 << 20).unwrap(), files);
        assert_eq!(write(&files).unwrap(), packed);
        assert!(packed.len() < 600);
        assert!(read(b"not a zip at all, just some text", 1 << 20).is_err());
        assert!(read(&packed, 10).is_err());
    }
}
