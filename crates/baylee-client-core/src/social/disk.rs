//! The chat's files (§10.2, §11.1): the sealed state file and one sealed,
//! length-prefixed history log per conversation, in the account's chat
//! folder `<config>/chat/<gw>/<tag>/`.
//!
//! The social machine reads through a [`ChatDisk`] when it restores and
//! writes by [`super::Effect`]s, which [`apply`] carries out on any
//! [`ChatDisk`]: the client routes its disk effects there without looking
//! inside them. Every record is sealed before it leaves the machine, so a
//! disk sees only ciphertext.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use super::Effect;

/// The state file's name, which is also its record's at-rest name.
pub const STATE_FILE: &str = "state.bin";

/// The folder history logs live in.
pub const HISTORY_DIR: &str = "history";

/// A history log's path from its at-rest name (`<h>.log`).
#[must_use]
pub fn history_path(name: &str) -> String {
    format!("{HISTORY_DIR}/{name}")
}

/// The chat folder of the account `tag` at `gateway`, relative to the
/// client's config directory: `chat/<gw>/<tag>`, where `<gw>` is the first
/// twelve hex characters of SHA-256 of the gateway's normalised base URL
/// (lower case, no trailing `/`); for the live gateway, of the address its
/// keys are bound to (`gateway_list::keyed`).
#[must_use]
pub fn chat_folder(gateway: &str, tag: &str) -> String {
    let normal = gateway.trim().trim_end_matches('/').to_ascii_lowercase();
    let normal = crate::lobby::gateway_list::keyed(&normal);
    let hash = Sha256::digest(normal.as_bytes());
    let gw = super::sealing::hex(&hash[..6]);
    let tag: String = tag.chars().filter(char::is_ascii_alphanumeric).collect();
    format!("chat/{gw}/{tag}")
}

/// Where the chat's files are kept: paths are relative to the account's
/// chat folder and use `/`.
pub trait ChatDisk {
    /// The whole file at `path`, if it is there.
    fn read(&self, path: &str) -> Option<Vec<u8>>;

    /// The names of the files in the folder `dir`, sorted; `""` selects the root.
    fn list(&self, dir: &str) -> Vec<String>;

    /// Replaces the file at `path` with `bytes` (temp file, then rename).
    ///
    /// # Errors
    ///
    /// A sentence when it can't be written.
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String>;

    /// Adds `bytes` to the end of the file at `path`, making it if needed.
    ///
    /// # Errors
    ///
    /// A sentence when it can't be written.
    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), String>;

    /// Removes the file at `path`; a missing file is not an error.
    ///
    /// # Errors
    ///
    /// A sentence when it can't be removed.
    fn remove(&mut self, path: &str) -> Result<(), String>;

    /// Removes every file of the chat folder.
    ///
    /// # Errors
    ///
    /// A sentence when something can't be removed.
    fn remove_all(&mut self) -> Result<(), String>;
}

/// Carries out a disk effect ([`Effect::SaveState`],
/// [`Effect::AppendHistory`], [`Effect::RewriteHistory`],
/// [`Effect::DeleteAllLocal`]) on `disk`; `None` for any other effect.
pub fn apply(effect: &Effect, disk: &mut dyn ChatDisk) -> Option<Result<(), String>> {
    Some(match effect {
        Effect::SaveState(bytes) => disk.write(STATE_FILE, bytes),
        Effect::AppendHistory { file, record } => disk.append(file, record),
        Effect::RewriteHistory { file, bytes } if bytes.is_empty() => disk.remove(file),
        Effect::RewriteHistory { file, bytes } => disk.write(file, bytes),
        Effect::DeleteAllLocal => disk.remove_all(),
        _ => return None,
    })
}

/// A history log's frames: each record is `u32be(len) ‖ sealed`. A frame
/// whose length runs past the end is the end (a torn last write).
#[must_use]
pub fn frames(bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while rest.len() >= 4 {
        let len = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let Some(frame) = rest.get(4..4 + len) else {
            break;
        };
        out.push(frame);
        rest = &rest[4 + len..];
    }
    out
}

/// One frame of a history log: `u32be(len) ‖ sealed`.
#[must_use]
pub fn frame(sealed: &[u8]) -> Vec<u8> {
    let len = u32::try_from(sealed.len()).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(4 + sealed.len());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(sealed);
    out
}

/// A disk in memory, for tests and the client's fake transport.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryDisk {
    /// Every file, by path.
    pub files: BTreeMap<String, Vec<u8>>,
}

impl MemoryDisk {
    /// An empty disk.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChatDisk for MemoryDisk {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.files.get(path).cloned()
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{}/", dir.trim_end_matches('/'))
        };
        self.files
            .keys()
            .filter_map(|p| p.strip_prefix(&prefix))
            .filter(|rest| !rest.contains('/'))
            .map(str::to_string)
            .collect()
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.files.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.files
            .entry(path.to_string())
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }

    fn remove(&mut self, path: &str) -> Result<(), String> {
        self.files.remove(path);
        Ok(())
    }

    fn remove_all(&mut self) -> Result<(), String> {
        self.files.clear();
        Ok(())
    }
}

/// The chat folder on a real file system: the folder is made `0700` on
/// unix, a write goes through a temp file and a rename, and a path that
/// would leave the folder is refused.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct DirDisk {
    root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl DirDisk {
    /// The disk rooted at the account's chat folder.
    #[must_use]
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, path: &str) -> Result<std::path::PathBuf, String> {
        let fine = !path.is_empty()
            && path.split('/').all(|part| {
                !part.is_empty() && part != "." && part != ".." && !part.contains('\\')
            });
        if !fine {
            return Err("that is not a chat file's name".into());
        }
        Ok(path
            .split('/')
            .fold(self.root.clone(), |p, part| p.join(part)))
    }

    fn make_dir(dir: &std::path::Path) -> Result<(), String> {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            builder.mode(0o700);
        }
        builder
            .create(dir)
            .map_err(|e| format!("the chat folder could not be made: {}", e.kind()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ChatDisk for DirDisk {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(path).ok()?).ok()
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let path = if dir.is_empty() {
            self.root.clone()
        } else {
            let Ok(path) = self.path(dir) else {
                return Vec::new();
            };
            path
        };
        let mut names: Vec<String> = std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| !n.starts_with('.'))
            .collect();
        names.sort();
        names
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write as _;
        let target = self.path(path)?;
        let dir = target.parent().ok_or("a chat file has no folder")?;
        Self::make_dir(dir)?;
        let name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");
        let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let written = options
            .open(&temp)
            .and_then(|mut file| {
                file.write_all(bytes)?;
                file.sync_all()
            })
            .and_then(|()| std::fs::rename(&temp, &target));
        if let Err(e) = written {
            let _ = std::fs::remove_file(&temp);
            return Err(format!("a chat file could not be written: {}", e.kind()));
        }
        Ok(())
    }

    fn append(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        use std::io::Write as _;
        let target = self.path(path)?;
        Self::make_dir(target.parent().ok_or("a chat file has no folder")?)?;
        let mut options = std::fs::OpenOptions::new();
        options.append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        options
            .open(&target)
            .and_then(|mut file| file.write_all(bytes))
            .map_err(|e| format!("a chat file could not be written: {}", e.kind()))
    }

    fn remove(&mut self, path: &str) -> Result<(), String> {
        match std::fs::remove_file(self.path(path)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("a chat file could not be removed: {}", e.kind())),
        }
    }

    fn remove_all(&mut self) -> Result<(), String> {
        match std::fs::remove_dir_all(&self.root) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                "the chat folder could not be removed: {}",
                e.kind()
            )),
        }
    }
}
