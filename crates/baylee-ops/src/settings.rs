//! The services' settings files: systemd `EnvironmentFile=` syntax, one
//! `KEY=value` a line, `#` comments, values optionally quoted.
use std::path::Path;

/// Every `KEY=value` of a file, in order; an unreadable file gives none.
pub fn read(path: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return None;
            }
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The last value any of `files` gives `key` (later files win).
pub fn last(files: &[&Path], key: &str) -> Option<String> {
    files
        .iter()
        .flat_map(|f| read(f))
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v)
        .next_back()
}

#[cfg(test)]
mod tests {
    #[test]
    fn settings_read_like_a_systemd_environment_file() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        std::fs::write(&a, "# c\nDATABASE_URL=postgres://a\nPORT = 1\nbad key=x\n").unwrap();
        std::fs::write(&b, "export DATABASE_URL=\"postgres://b\"\nNAME='x y'\n").unwrap();
        assert_eq!(
            super::last(&[&a, &b], "DATABASE_URL").as_deref(),
            Some("postgres://b")
        );
        assert_eq!(super::last(&[&a], "PORT").as_deref(), Some("1"));
        assert_eq!(super::last(&[&b], "NAME").as_deref(), Some("x y"));
        assert_eq!(super::last(&[&a, &dir.path().join("absent")], "NAME"), None);
    }
}
