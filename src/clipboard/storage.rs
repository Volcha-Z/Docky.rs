use std::{
    env,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
};

use super::history::{ClipboardEntry, StoredEntry};

const MAGIC: &[u8; 8] = b"DOCKCLP1";
const MAX_FILE_SIZE: usize = 12 * 1024 * 1024;
const MAX_ITEMS: usize = 50;
const MAX_TEXT_SIZE: usize = 256 * 1024;

pub fn load() -> Vec<ClipboardEntry> {
    let Ok(file) = File::open(path()) else {
        return Vec::new();
    };
    if file.metadata().is_ok_and(|metadata| metadata.len() > MAX_FILE_SIZE as u64) {
        return Vec::new();
    }
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut magic = [0; MAGIC.len()];
    if reader.read_exact(&mut magic).is_err() || &magic != MAGIC {
        return Vec::new();
    }
    let Some(count) = read_u32(&mut reader).map(|value| value as usize) else {
        return Vec::new();
    };
    if count > MAX_ITEMS {
        return Vec::new();
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(entry) = read_entry(&mut reader) else {
            return Vec::new();
        };
        entries.push(entry);
    }
    entries
}

pub fn save(entries: &[ClipboardEntry]) {
    if encoded_size(entries).is_none_or(|size| size > MAX_FILE_SIZE) {
        return;
    }
    let path = path();
    let Some(directory) = path.parent() else {
        return;
    };
    if !directory.exists() && fs::DirBuilder::new().recursive(true).mode(0o700).create(directory).is_err() {
        return;
    }
    let temporary = directory.join("clipboard.tmp");
    let Ok(file) = fs::OpenOptions::new().create(true).truncate(true).write(true).mode(0o600).open(&temporary) else {
        return;
    };
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    let written = writer.write_all(MAGIC).is_ok()
        && write_u32(&mut writer, entries.len() as u32).is_ok()
        && entries.iter().all(|entry| write_entry(&mut writer, entry.stored()).is_ok());
    let Ok(file) = writer.into_inner() else {
        return;
    };
    if written && file.sync_data().is_ok() {
        let _ = fs::rename(temporary, path);
    }
}

fn read_entry(reader: &mut impl Read) -> Option<ClipboardEntry> {
    let mime = read_string(reader, MAX_TEXT_SIZE)?;
    let title = read_string(reader, MAX_TEXT_SIZE)?;
    let description = read_string(reader, MAX_TEXT_SIZE)?;
    let data = read_bytes(reader, MAX_FILE_SIZE)?;
    let preview = read_bytes(reader, MAX_FILE_SIZE)?;
    ClipboardEntry::restore(mime, data, title, description, preview)
}

fn write_entry(output: &mut impl Write, entry: StoredEntry<'_>) -> std::io::Result<()> {
    write_bytes(output, entry.mime.as_bytes())?;
    write_bytes(output, entry.title.as_bytes())?;
    write_bytes(output, entry.description.as_bytes())?;
    write_bytes(output, entry.data)?;
    write_bytes(output, entry.preview.unwrap_or_default())
}

fn write_bytes(output: &mut impl Write, bytes: &[u8]) -> std::io::Result<()> {
    write_u32(output, bytes.len() as u32)?;
    output.write_all(bytes)
}

fn write_u32(output: &mut impl Write, value: u32) -> std::io::Result<()> {
    output.write_all(&value.to_le_bytes())
}

fn read_u32(input: &mut impl Read) -> Option<u32> {
    let mut bytes = [0; 4];
    input.read_exact(&mut bytes).ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_bytes(input: &mut impl Read, limit: usize) -> Option<Vec<u8>> {
    let length = read_u32(input)? as usize;
    if length > limit {
        return None;
    }
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

fn read_string(input: &mut impl Read, limit: usize) -> Option<String> {
    String::from_utf8(read_bytes(input, limit)?).ok()
}

fn encoded_size(entries: &[ClipboardEntry]) -> Option<usize> {
    let mut size = MAGIC.len().checked_add(4)?;
    for entry in entries {
        let entry = entry.stored();
        for bytes in [
            entry.mime.as_bytes(),
            entry.title.as_bytes(),
            entry.description.as_bytes(),
            entry.data,
            entry.preview.unwrap_or_default(),
        ] {
            size = size.checked_add(4)?.checked_add(bytes.len())?;
        }
    }
    Some(size)
}

fn path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dockyrs/clipboard.bin")
}
