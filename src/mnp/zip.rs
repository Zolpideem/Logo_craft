use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
};

use flate2::read::DeflateDecoder;

const LIMIT: usize = 32 * 1024 * 1024;

pub(super) struct Archive {
    bytes: Vec<u8>,
    entries: HashMap<String, Entry>,
}

struct Entry {
    offset: usize,
    compressed: usize,
    length: usize,
    method: u16,
    flags: u16,
    checksum: u32,
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Archive MNP invalide ou format protégé non pris en charge.",
    )
}

fn part(bytes: &[u8], offset: usize, length: usize) -> io::Result<&[u8]> {
    bytes
        .get(offset..offset.checked_add(length).ok_or_else(invalid)?)
        .ok_or_else(invalid)
}

fn u16_at(bytes: &[u8], offset: usize) -> io::Result<u16> {
    Ok(u16::from_le_bytes(
        part(bytes, offset, 2)?.try_into().unwrap(),
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        part(bytes, offset, 4)?.try_into().unwrap(),
    ))
}

impl Archive {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > LIMIT || !bytes.starts_with(b"PK") {
            return Err(invalid());
        }
        let end = (bytes.len().saturating_sub(65_557)..bytes.len().saturating_sub(21))
            .rev()
            .find(|&pos| {
                bytes[pos..].starts_with(b"PK\x05\x06")
                    && u16_at(&bytes, pos + 20).is_ok_and(|n| pos + 22 + n as usize == bytes.len())
            })
            .ok_or_else(invalid)?;
        let count = u16_at(&bytes, end + 10)?;
        if u32_at(&bytes, end + 4)? != 0 || count != u16_at(&bytes, end + 8)? || count > 1024 {
            return Err(invalid());
        }
        let mut pos = u32_at(&bytes, end + 16)? as usize;
        let central_end = pos
            .checked_add(u32_at(&bytes, end + 12)? as usize)
            .ok_or_else(invalid)?;
        if central_end > end {
            return Err(invalid());
        }
        let mut entries = HashMap::new();
        for _ in 0..count {
            if !part(&bytes, pos, 46)?.starts_with(b"PK\x01\x02") {
                return Err(invalid());
            }
            let name_len = u16_at(&bytes, pos + 28)? as usize;
            let name = std::str::from_utf8(part(&bytes, pos + 46, name_len)?)
                .map_err(|_| invalid())?
                .replace('\\', "/");
            let entry = Entry {
                offset: u32_at(&bytes, pos + 42)? as usize,
                compressed: u32_at(&bytes, pos + 20)? as usize,
                length: u32_at(&bytes, pos + 24)? as usize,
                method: u16_at(&bytes, pos + 10)?,
                flags: u16_at(&bytes, pos + 8)?,
                checksum: u32_at(&bytes, pos + 16)?,
            };
            if entries.insert(name, entry).is_some() {
                return Err(invalid());
            }
            pos += 46
                + name_len
                + u16_at(&bytes, pos + 30)? as usize
                + u16_at(&bytes, pos + 32)? as usize;
        }
        if pos != central_end {
            return Err(invalid());
        }
        Ok(Self { bytes, entries })
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn read(&self, name: &str, limit: usize) -> io::Result<Vec<u8>> {
        let entry = self.entries.get(name).ok_or_else(invalid)?;
        let offset = entry.offset;
        if entry.flags & 1 != 0
            || entry.length > limit
            || !part(&self.bytes, offset, 30)?.starts_with(b"PK\x03\x04")
        {
            return Err(invalid());
        }
        let start = offset
            + 30
            + u16_at(&self.bytes, offset + 26)? as usize
            + u16_at(&self.bytes, offset + 28)? as usize;
        let compressed = part(&self.bytes, start, entry.compressed)?;
        let data = match entry.method {
            0 if entry.compressed == entry.length => compressed.to_vec(),
            8 => {
                let mut data = Vec::with_capacity(entry.length);
                DeflateDecoder::new(compressed)
                    .take(entry.length as u64 + 1)
                    .read_to_end(&mut data)?;
                data
            }
            _ => return Err(invalid()),
        };
        if data.len() != entry.length || crc32fast::hash(&data) != entry.checksum {
            return Err(invalid());
        }
        Ok(data)
    }
}
