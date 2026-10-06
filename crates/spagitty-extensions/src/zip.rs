// SPDX-License-Identifier: GPL-3.0-or-later

//! The `.spagitty-extension` container: a ZIP file, read strictly.
//!
//! Only what a package needs: the central directory, stored and deflated
//! entries, no encryption, no ZIP64, no multi-disk archives. Every entry is
//! checked against the rules below **when the archive is opened**, before a
//! byte of it is written anywhere, and every read checks the entry's CRC-32 and
//! size. An archive that breaks a rule is refused whole.
//!
//! - **Names** are relative UTF-8 paths with forward slashes: no absolute path,
//!   no drive letter, no UNC prefix, no backslash, no `.` or `..` segment, no
//!   colon (an NTFS stream), no control character, no segment Windows would
//!   silently rewrite (a trailing dot or space) or refuse (`CON`, `NUL`, …).
//! - **No links.** A Unix symlink or a Windows reparse point is refused: a link
//!   is how an archive that looks contained writes outside its directory.
//! - **No duplicates**, compared case-insensitively, because two names that
//!   differ only in case are one file on the platforms most people use.
//! - **No bombs.** At most [`MAX_ENTRIES`] entries, [`MAX_TOTAL`] bytes
//!   expanded, and a compression ratio of at most [`MAX_RATIO`] for any entry
//!   larger than 1 MiB.

use std::collections::BTreeSet;
use std::io::Read;

pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_TOTAL: u64 = 256 * 1024 * 1024;
pub const MAX_RATIO: u64 = 200;
/// The largest archive read into memory.
pub const MAX_ARCHIVE: u64 = 300 * 1024 * 1024;

const EOCD: u32 = 0x0605_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;

const STORED: u16 = 0;
const DEFLATED: u16 = 8;

/// One entry, as the central directory describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    method: u16,
    crc: u32,
    compressed: u64,
    pub size: u64,
    local_offset: u64,
    /// The Unix mode, when the archive was made on Unix.
    pub unix_mode: Option<u32>,
}

impl Entry {
    /// Whether the archive marks this entry executable.
    pub fn is_executable(&self) -> bool {
        self.unix_mode.is_some_and(|mode| mode & 0o111 != 0)
    }
}

#[derive(Debug)]
pub struct Archive {
    data: Vec<u8>,
    entries: Vec<Entry>,
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

impl Archive {
    /// Read an archive's directory and check every entry against the rules.
    pub fn open(data: Vec<u8>) -> Result<Archive, String> {
        if data.len() as u64 > MAX_ARCHIVE {
            return Err("the package is larger than 300 MiB".into());
        }
        let eocd = find_eocd(&data).ok_or("it is not a ZIP archive")?;
        let disk = u16_at(&data, eocd + 4).ok_or("truncated directory")?;
        let cd_disk = u16_at(&data, eocd + 6).ok_or("truncated directory")?;
        let on_disk = u16_at(&data, eocd + 8).ok_or("truncated directory")?;
        let total = u16_at(&data, eocd + 10).ok_or("truncated directory")?;
        let cd_size = u32_at(&data, eocd + 12).ok_or("truncated directory")? as usize;
        let cd_offset = u32_at(&data, eocd + 16).ok_or("truncated directory")? as usize;
        if disk != 0 || cd_disk != 0 || on_disk != total {
            return Err("multi-part archives are not supported".into());
        }
        if total == 0xFFFF || cd_offset == 0xFFFF_FFFF || cd_size == 0xFFFF_FFFF {
            return Err("ZIP64 archives are not supported".into());
        }
        if total as usize > MAX_ENTRIES {
            return Err(format!("it has more than {MAX_ENTRIES} entries"));
        }
        if cd_offset
            .checked_add(cd_size)
            .map_or(true, |end| end > eocd)
        {
            return Err("its directory points outside the file".into());
        }

        let mut entries = Vec::with_capacity(total as usize);
        let mut seen = BTreeSet::new();
        let mut expanded: u64 = 0;
        let mut at = cd_offset;
        for _ in 0..total {
            if u32_at(&data, at) != Some(CENTRAL) {
                return Err("its directory is damaged".into());
            }
            let made_by = u16_at(&data, at + 4).ok_or("damaged entry")?;
            let flags = u16_at(&data, at + 8).ok_or("damaged entry")?;
            let method = u16_at(&data, at + 10).ok_or("damaged entry")?;
            let crc = u32_at(&data, at + 16).ok_or("damaged entry")?;
            let compressed = u32_at(&data, at + 20).ok_or("damaged entry")? as u64;
            let size = u32_at(&data, at + 24).ok_or("damaged entry")? as u64;
            let name_len = u16_at(&data, at + 28).ok_or("damaged entry")? as usize;
            let extra_len = u16_at(&data, at + 30).ok_or("damaged entry")? as usize;
            let comment_len = u16_at(&data, at + 32).ok_or("damaged entry")? as usize;
            let external = u32_at(&data, at + 38).ok_or("damaged entry")?;
            let local_offset = u32_at(&data, at + 42).ok_or("damaged entry")? as u64;
            let name_bytes = data
                .get(at + 46..at + 46 + name_len)
                .ok_or("damaged entry")?;
            at += 46 + name_len + extra_len + comment_len;

            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| "an entry's name is not UTF-8".to_string())?
                .to_string();
            if flags & 0x0001 != 0 {
                return Err(format!("{name}: encrypted entries are not supported"));
            }
            if method != STORED && method != DEFLATED {
                return Err(format!(
                    "{name}: compression method {method} is not supported"
                ));
            }
            if compressed == 0xFFFF_FFFF || size == 0xFFFF_FFFF {
                return Err(format!("{name}: ZIP64 entries are not supported"));
            }

            let is_dir = name.ends_with('/');
            let path = name.strip_suffix('/').unwrap_or(&name);
            check_name(path).map_err(|why| format!("{name}: {why}"))?;

            let host = made_by >> 8;
            let unix_mode = (host == 3).then_some(external >> 16);
            if let Some(mode) = unix_mode {
                if mode & 0o170000 == 0o120000 {
                    return Err(format!("{name}: links are not allowed in a package"));
                }
            }
            // FILE_ATTRIBUTE_REPARSE_POINT in the MS-DOS attribute byte range.
            if matches!(host, 0 | 10 | 14) && external & 0x400 != 0 {
                return Err(format!("{name}: links are not allowed in a package"));
            }

            if !seen.insert(path.to_lowercase()) {
                return Err(format!("{name}: the package names this file twice"));
            }
            if is_dir {
                if size != 0 {
                    return Err(format!("{name}: a directory cannot have content"));
                }
            } else {
                expanded = expanded.saturating_add(size);
                if expanded > MAX_TOTAL {
                    return Err("it expands to more than 256 MiB".into());
                }
                if size > 1024 * 1024 && size / compressed.max(1) > MAX_RATIO {
                    return Err(format!("{name}: compressed more than {MAX_RATIO} to one"));
                }
                if method == STORED && compressed != size {
                    return Err(format!("{name}: stored entry sizes disagree"));
                }
            }

            entries.push(Entry {
                name: path.to_string(),
                is_dir,
                method,
                crc,
                compressed,
                size,
                local_offset,
                unix_mode,
            });
        }

        // Every entry's data must lie inside the file, before the directory.
        for entry in &entries {
            let start =
                data_start(&data, entry).ok_or_else(|| format!("{}: damaged entry", entry.name))?;
            if start
                .checked_add(entry.compressed)
                .map_or(true, |end| end > cd_offset as u64)
            {
                return Err(format!(
                    "{}: its data runs past the end of the archive",
                    entry.name
                ));
            }
        }

        Ok(Archive { data, entries })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name && !e.is_dir)
    }

    /// The bytes of one entry, CRC- and size-checked.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>, String> {
        let start = data_start(&self.data, entry).ok_or("damaged entry")? as usize;
        let raw = self
            .data
            .get(start..start + entry.compressed as usize)
            .ok_or("damaged entry")?;
        let bytes = match entry.method {
            STORED => raw.to_vec(),
            _ => {
                let mut out = Vec::with_capacity(entry.size as usize);
                flate2::read::DeflateDecoder::new(raw)
                    .take(entry.size + 1)
                    .read_to_end(&mut out)
                    .map_err(|e| format!("{}: {e}", entry.name))?;
                out
            }
        };
        if bytes.len() as u64 != entry.size {
            return Err(format!(
                "{}: its size does not match the directory",
                entry.name
            ));
        }
        if crc32fast::hash(&bytes) != entry.crc {
            return Err(format!(
                "{}: its checksum does not match — the package is damaged",
                entry.name
            ));
        }
        Ok(bytes)
    }
}

fn find_eocd(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    let lowest = data.len().saturating_sub(22 + 65_535);
    (lowest..=data.len() - 22)
        .rev()
        .find(|&at| u32_at(data, at) == Some(EOCD))
}

fn data_start(data: &[u8], entry: &Entry) -> Option<u64> {
    let at = entry.local_offset as usize;
    if u32_at(data, at)? != LOCAL {
        return None;
    }
    let name_len = u16_at(data, at + 26)? as u64;
    let extra_len = u16_at(data, at + 28)? as u64;
    Some(entry.local_offset + 30 + name_len + extra_len)
}

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Why `path` cannot be a file in a package, if it cannot.
pub fn check_name(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("an empty name");
    }
    if path.len() > 512 {
        return Err("the name is too long");
    }
    if path.contains('\\') {
        return Err("backslashes are not allowed in names");
    }
    if path.starts_with('/') {
        return Err("absolute paths are not allowed");
    }
    if path.contains(':') {
        return Err("drive letters and streams are not allowed");
    }
    if path.chars().any(char::is_control) {
        return Err("control characters are not allowed in names");
    }
    for segment in path.split('/') {
        if segment.is_empty() {
            return Err("empty path segments are not allowed");
        }
        if segment == "." || segment == ".." {
            return Err("'.' and '..' are not allowed in names");
        }
        if segment.ends_with('.') || segment.ends_with(' ') {
            return Err("a name may not end in a dot or a space");
        }
        let stem = segment.split('.').next().unwrap_or("").to_ascii_lowercase();
        if RESERVED.contains(&stem.as_str()) {
            return Err("that name is reserved on Windows");
        }
    }
    Ok(())
}

/// A minimal writer: stored or deflated entries, Unix modes. Used by the
/// tests to build hostile archives the reader must refuse, and by the
/// desktop's tests to build packages.
#[derive(Default)]
pub struct Writer {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Writer {
    pub fn new() -> Writer {
        Writer::default()
    }

    /// Add a file. `mode` is a Unix mode (`0o100755` for an executable); `None`
    /// writes an MS-DOS entry with no mode.
    pub fn file(&mut self, name: &str, data: &[u8], mode: Option<u32>, deflate: bool) {
        let (method, body) = if deflate {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            std::io::Write::write_all(&mut encoder, data).expect("deflate into memory");
            (DEFLATED, encoder.finish().expect("deflate into memory"))
        } else {
            (STORED, data.to_vec())
        };
        self.raw(
            name,
            data.len() as u32,
            crc32fast::hash(data),
            method,
            &body,
            mode,
            0,
        );
    }

    /// Add an entry exactly as described, lies included.
    #[allow(clippy::too_many_arguments)]
    pub fn raw(
        &mut self,
        name: &str,
        size: u32,
        crc: u32,
        method: u16,
        body: &[u8],
        mode: Option<u32>,
        dos_attributes: u32,
    ) {
        let offset = self.out.len() as u32;
        let name = name.as_bytes();
        let mut local = Vec::new();
        local.extend_from_slice(&LOCAL.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes());
        local.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
        local.extend_from_slice(&method.to_le_bytes());
        local.extend_from_slice(&[0, 0, 0x21, 0]); // time, date
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(body.len() as u32).to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&(name.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name);
        self.out.extend_from_slice(&local);
        self.out.extend_from_slice(body);

        let (made_by, external) = match mode {
            Some(mode) => ((3u16 << 8) | 20, mode << 16),
            None => (20u16, dos_attributes),
        };
        let c = &mut self.central;
        c.extend_from_slice(&CENTRAL.to_le_bytes());
        c.extend_from_slice(&made_by.to_le_bytes());
        c.extend_from_slice(&20u16.to_le_bytes());
        c.extend_from_slice(&0x0800u16.to_le_bytes());
        c.extend_from_slice(&method.to_le_bytes());
        c.extend_from_slice(&[0, 0, 0x21, 0]);
        c.extend_from_slice(&crc.to_le_bytes());
        c.extend_from_slice(&(body.len() as u32).to_le_bytes());
        c.extend_from_slice(&size.to_le_bytes());
        c.extend_from_slice(&(name.len() as u16).to_le_bytes());
        c.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]); // extra, comment, disk, internal
        c.extend_from_slice(&external.to_le_bytes());
        c.extend_from_slice(&offset.to_le_bytes());
        c.extend_from_slice(name);
        self.count += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let offset = self.out.len() as u32;
        let size = self.central.len() as u32;
        self.out.extend_from_slice(&self.central);
        self.out.extend_from_slice(&EOCD.to_le_bytes());
        self.out.extend_from_slice(&[0, 0, 0, 0]);
        self.out.extend_from_slice(&self.count.to_le_bytes());
        self.out.extend_from_slice(&self.count.to_le_bytes());
        self.out.extend_from_slice(&size.to_le_bytes());
        self.out.extend_from_slice(&offset.to_le_bytes());
        self.out.extend_from_slice(&0u16.to_le_bytes());
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(build: impl FnOnce(&mut Writer)) -> Result<Archive, String> {
        let mut writer = Writer::new();
        build(&mut writer);
        Archive::open(writer.finish())
    }

    #[test]
    fn stored_and_deflated_entries_read_back() {
        let a = archive(|w| {
            w.file("extension.json", b"{}", None, false);
            w.file("bin/worker", &vec![7u8; 5000], Some(0o100755), true);
        })
        .unwrap();
        assert_eq!(a.entries().len(), 2);
        let worker = a.find("bin/worker").unwrap();
        assert!(worker.is_executable());
        assert_eq!(a.read(worker).unwrap(), vec![7u8; 5000]);
        assert!(!a.find("extension.json").unwrap().is_executable());
    }

    #[test]
    fn every_unsafe_name_is_refused() {
        for name in [
            "../evil",
            "bin/../../evil",
            "/etc/passwd",
            "C:/Windows/x",
            "C:evil",
            "\\\\server\\share",
            "bin\\worker.exe",
            "a//b",
            "./a",
            "file.txt:stream",
            "con.txt",
            "bin/NUL",
            "trailing.",
            "space ",
            "tab\there",
        ] {
            let result = archive(|w| w.file(name, b"x", None, false));
            assert!(result.is_err(), "{name:?} was accepted");
        }
    }

    #[test]
    fn links_are_refused_whatever_made_them() {
        assert!(archive(|w| w.file("link", b"/etc/passwd", Some(0o120777), false)).is_err());
        assert!(
            archive(|w| w.raw("junction", 1, crc32fast::hash(b"x"), 0, b"x", None, 0x400)).is_err()
        );
    }

    #[test]
    fn two_names_that_differ_only_in_case_are_one_file_too_many() {
        let result = archive(|w| {
            w.file("README.md", b"a", None, false);
            w.file("readme.md", b"b", None, false);
        });
        assert!(result.unwrap_err().contains("twice"));
    }

    #[test]
    fn a_lying_checksum_or_size_is_caught_on_read() {
        let a = archive(|w| w.raw("x", 1, 12345, 0, b"x", None, 0)).unwrap();
        assert!(a.read(&a.entries()[0]).unwrap_err().contains("checksum"));

        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, b"hello world").unwrap();
        let body = encoder.finish().unwrap();
        let a = archive(|w| w.raw("y", 5, crc32fast::hash(b"hello"), 8, &body, None, 0)).unwrap();
        assert!(a.read(&a.entries()[0]).is_err());
    }

    #[test]
    fn a_bomb_is_refused_from_its_directory_before_anything_is_inflated() {
        let result = archive(|w| w.raw("bomb", 200 * 1024 * 1024, 0, 8, &[0u8; 100], None, 0));
        assert!(result.unwrap_err().contains("compressed more than"));
        let result = archive(|w| {
            w.raw("a", 200 * 1024 * 1024, 0, 0, &[], None, 0);
            w.raw("b", 100 * 1024 * 1024, 0, 0, &[], None, 0);
        });
        assert!(result.is_err());
    }

    #[test]
    fn things_that_are_not_archives_are_refused() {
        assert!(Archive::open(b"not a zip at all, just some text".to_vec()).is_err());
        assert!(Archive::open(Vec::new()).is_err());
        let mut truncated = {
            let mut w = Writer::new();
            w.file("a", b"abc", None, false);
            w.finish()
        };
        truncated.drain(0..10);
        assert!(Archive::open(truncated).is_err());
    }

    #[test]
    fn encryption_and_unknown_methods_are_refused() {
        assert!(archive(|w| w.raw("x", 1, 0, 12, b"x", None, 0)).is_err());
    }
}
