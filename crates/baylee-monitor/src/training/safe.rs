//! The bounded, text-safe readers every training reader shares: a small
//! regular file (never a symlink) parsed for a few fields, and display text
//! with control characters removed and absolute paths reduced.

use std::{
    borrow::Cow,
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

pub(super) const MAX_TEXT: usize = 240;

/// Reads a regular file (not a symlink) of at most `cap` bytes and parses the
/// wanted fields.
pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path, cap: u64) -> io::Result<T> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "keine Datei"));
    }
    if meta.len() > cap {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "zu groß"));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(cap).read_to_end(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(super) fn unix_ms(seconds: f64) -> Option<u64> {
    (seconds.is_finite() && seconds > 0.0).then_some((seconds * 1000.0) as u64)
}

/// A name that is safe to join below a directory and to show.
pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Display text: control characters removed, absolute paths reduced to
/// `…/name`, at most `MAX_TEXT` characters.
pub(super) fn clean(text: &str) -> String {
    let text: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let redacted = text
        .split(' ')
        .map(redact_path)
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = redacted.chars();
    let mut clipped: String = chars.by_ref().take(MAX_TEXT).collect();
    if chars.next().is_some() {
        clipped.pop();
        clipped.push('…');
    }
    clipped
}

/// Replaces an absolute (or home-relative) path inside one token with
/// `…/last-component`, keeping what precedes it (`--out=`, `(`) and the
/// punctuation that follows it.
fn redact_path(token: &str) -> Cow<'_, str> {
    let starts_path = |at: usize| {
        let rest = &token[at..];
        let begins = (rest.starts_with('/') && rest.len() > 1) || rest.starts_with("~/");
        let after_boundary = token[..at]
            .chars()
            .next_back()
            .is_none_or(|p| matches!(p, '=' | ':' | '(' | '"' | '\'' | '[' | ','));
        begins && after_boundary
    };
    let Some(at) = token
        .char_indices()
        .map(|(i, _)| i)
        .find(|&i| starts_path(i))
    else {
        return Cow::Borrowed(token);
    };
    let rest = &token[at..];
    let end = rest
        .find([')', ']', '"', '\'', ',', ';'])
        .unwrap_or(rest.len());
    let (path, tail) = rest.split_at(end);
    let (path, colon) = path.strip_suffix(':').map_or((path, ""), |p| (p, ":"));
    let name = path
        .rsplit('/')
        .find(|s| !s.is_empty() && *s != "~")
        .unwrap_or("");
    Cow::Owned(format!(
        "{}…/{name}{colon}{}",
        &token[..at],
        redact_path(tail)
    ))
}

/// A scratch directory for tests, removed on drop.
#[cfg(test)]
pub(super) struct Scratch(pub std::path::PathBuf);

#[cfg(test)]
impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("baylee-pulse-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn write(&self, relative: &str, text: &str) {
        use std::io::Write;
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::File::create(path)
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }
}

#[cfg(test)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_reduced_and_text_is_bounded() {
        assert_eq!(
            clean("already owned: /home/ace/baylee-data/v5/gpu-work.lock: lock failed"),
            "already owned: …/gpu-work.lock: lock failed"
        );
        assert_eq!(
            clean("--out=/a/b/c.json (/x/y) 1/2"),
            "--out=…/c.json (…/y) 1/2"
        );
        assert_eq!(clean("runs /a/x1,/a/x2 ~/k"), "runs …/x1,…/x2 …/k");
        assert_eq!(clean("raw / tactical"), "raw / tactical");
        assert_eq!(clean("tab\there\x1b[0m"), "tab here [0m");
        let long = clean(&"é".repeat(500));
        assert_eq!(long.chars().count(), MAX_TEXT);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn json_is_read_only_from_small_regular_files() {
        let scratch = Scratch::new("safe-json");
        scratch.write("a.json", r#"{"x":1}"#);
        let value: serde_json::Value = read_json(&scratch.0.join("a.json"), 64).unwrap();
        assert_eq!(value["x"], 1);
        assert!(read_json::<serde_json::Value>(&scratch.0.join("a.json"), 3).is_err());
        std::os::unix::fs::symlink(scratch.0.join("a.json"), scratch.0.join("l.json")).unwrap();
        assert!(read_json::<serde_json::Value>(&scratch.0.join("l.json"), 64).is_err());
    }
}
