//! Files in the data folder: the write log (one JSON line per write), profiles and sites (one JSON
//! file each). Written whole to a temporary file and renamed, so a crash leaves the old file.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::write::Entry;

pub const JOURNAL: &str = "writes.jsonl";
pub const PROFILES: &str = "profiles";
pub const SITES: &str = "sites";
pub const SETTINGS: &str = "settings.json";
pub const RECORDING: &str = "recording.json";

/// The user's local application data, as bacnetio uses it.
pub fn default_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        return std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("modbusio"));
    }
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(state).join("modbusio"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state/modbusio"))
}

pub fn append_entry(path: &Path, entry: &Entry) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut line = serde_json::to_string(entry).map_err(io::Error::other)?;
    line.push('\n');
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(line.as_bytes())
}

/// The write log; lines that cannot be read (a crash in the middle of one) are left out and
/// counted.
pub fn load_entries(path: &Path) -> io::Result<(Vec<Entry>, usize)> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok((Vec::new(), 0)),
        Err(error) => return Err(error),
    };
    let mut skipped = 0;
    let entries = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let entry = serde_json::from_str(line).ok();
            skipped += usize::from(entry.is_none());
            entry
        })
        .collect();
    Ok((entries, skipped))
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// `None` when the file does not exist.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: {error}", path.display()),
            )
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Every `*.json` in a folder, loaded; files that cannot be read are named in the second list.
pub fn load_all<T: DeserializeOwned>(dir: &Path) -> (Vec<T>, Vec<String>) {
    let Ok(read) = fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };
    let mut paths: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    let mut loaded = Vec::new();
    let mut broken = Vec::new();
    for path in paths {
        match load_json(&path) {
            Ok(Some(value)) => loaded.push(value),
            Ok(None) => {}
            Err(error) => broken.push(error.to_string()),
        }
    }
    (loaded, broken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Area;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("modbusio-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_log_survives_a_line_cut_off_by_a_crash() {
        let dir = folder("journal");
        let path = dir.join(JOURNAL);
        let entry = Entry {
            id: 1,
            time: 1,
            device: "tcp:10.0.0.5:502/1".into(),
            point: Some("Set point".into()),
            area: Area::HoldingRegister,
            address: 1,
            before: Some(vec![210]),
            written: vec![220],
            after: Some(vec![220]),
            text: "22 °C".into(),
            before_text: Some("21 °C".into()),
            restores: None,
            error: None,
        };
        append_entry(&path, &entry).expect("written");
        OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(b"{\"id\":2,\"ti")
            .expect("written");
        let (entries, skipped) = load_entries(&path).expect("read");
        assert_eq!(entries, [entry]);
        assert_eq!(skipped, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn json_files_are_replaced_whole() {
        let dir = folder("json");
        let path = dir.join("a.json");
        save_json(&path, &vec![1, 2]).expect("saved");
        save_json(&path, &vec![3]).expect("saved");
        assert_eq!(load_json::<Vec<u8>>(&path).expect("read"), Some(vec![3]));
        fs::write(dir.join("broken.json"), "{").expect("written");
        let (loaded, broken) = load_all::<Vec<u8>>(&dir);
        assert_eq!(loaded, [vec![3]]);
        assert_eq!(broken.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}
