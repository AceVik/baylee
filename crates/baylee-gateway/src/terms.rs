//! The gateway's terms of use (WG-1): one Markdown file, read once at start.
//!
//! `BAYLEE_TERMS_PATH` names it. Unset (or blank) is a gateway without terms,
//! and then nothing a client sees changes: no `terms_stale` at sign-in,
//! `"terms": null` in `/info`, `404` on `/terms`. Set, the file must be there,
//! be UTF-8, hold some text and be at most [`MAX_TERMS_BYTES`]; anything else
//! refuses startup, because a gateway that quietly ran without the terms its
//! operator meant to show would be worse than one that did not start.
//!
//! **The version** is what an account accepts. It is the first 16 hex digits
//! of the file's SHA-256, so any edit asks every player again — unless the
//! file's first line is `<!-- version: 2026-10 -->`, which names it outright,
//! so an editorial fix (a typo, a link) need not be accepted twice. An
//! `<!-- updated: 2026-10-06 -->` line among the leading comment lines is
//! shown as the date; the file's mtime is not, since that is the deploy's
//! date. Those leading comment lines are taken off the Markdown that is
//! served, which is what the client renders.

use crate::auth;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;

/// The largest terms file a gateway takes, in bytes.
pub(crate) const MAX_TERMS_BYTES: usize = 64 * 1024;

/// The longest version a `<!-- version: … -->` line may name, in characters.
const MAX_VERSION_CHARS: usize = 64;

/// The terms, as read at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Terms {
    /// What an account accepts, and `/info` and `/terms` name.
    pub(crate) version: String,
    /// The date the file says it was last changed, if it says.
    pub(crate) updated: Option<String>,
    /// The text to show, without the leading comment lines.
    pub(crate) markdown: String,
}

/// The terms `BAYLEE_TERMS_PATH` names, `None` when it names none, or why
/// the gateway must not start.
pub(crate) fn from_env(path: Option<&OsStr>) -> Result<Option<Terms>, String> {
    let Some(path) = path.filter(|p| !p.to_string_lossy().trim().is_empty()) else {
        return Ok(None);
    };
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    read(&bytes).map(Some)
}

/// Reads a terms file's bytes.
pub(crate) fn read(bytes: &[u8]) -> Result<Terms, String> {
    if bytes.len() > MAX_TERMS_BYTES {
        return Err(format!(
            "{} bytes, and terms are {MAX_TERMS_BYTES} at most",
            bytes.len()
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| format!("not UTF-8: {e}"))?;
    let mut version = None;
    let mut updated = None;
    let mut body = text;
    // The leading comment lines, and only those: a comment further down is
    // the author's, and is left where it is.
    while let Some((line, rest)) = split_line(body) {
        let Some(inner) = line
            .trim()
            .strip_prefix("<!--")
            .and_then(|l| l.strip_suffix("-->"))
        else {
            break;
        };
        let inner = inner.trim();
        if let Some(named) = inner.strip_prefix("version:") {
            if body.as_ptr() != text.as_ptr() {
                return Err("`<!-- version: … -->` must be the first line".to_owned());
            }
            version = Some(checked_version(named.trim())?);
        } else if let Some(date) = inner.strip_prefix("updated:") {
            updated = Some(date.trim().to_owned()).filter(|d| !d.is_empty());
        }
        body = rest;
    }
    let markdown = body.trim_start_matches(['\r', '\n']).to_owned();
    if markdown.trim().is_empty() {
        return Err("the file holds no text".to_owned());
    }
    let version = version.unwrap_or_else(|| {
        let digest = Sha256::digest(bytes);
        auth::hex_lower(&digest[..8])
    });
    Ok(Terms {
        version,
        updated,
        markdown,
    })
}

/// The first line and what follows it, `None` at the end of the text.
fn split_line(text: &str) -> Option<(&str, &str)> {
    if text.is_empty() {
        return None;
    }
    Some(text.split_once('\n').unwrap_or((text, "")))
}

/// A version the file names, as an account will store it and a client show it.
fn checked_version(named: &str) -> Result<String, String> {
    if named.is_empty() {
        return Err("`<!-- version: -->` names no version".to_owned());
    }
    if named.chars().count() > MAX_VERSION_CHARS {
        return Err(format!(
            "the version is longer than {MAX_VERSION_CHARS} characters"
        ));
    }
    if let Some(bad) = named
        .chars()
        .find(|&c| c.is_whitespace() || c.is_control() || crate::is_bidi_control(c))
    {
        return Err(format!("the version contains U+{:04X}", u32::from(bad)));
    }
    Ok(named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_named_version_any_edit_is_a_new_one() {
        let one = read(b"# Terms\n\nBe kind.\n").unwrap();
        let two = read(b"# Terms\n\nBe kind!\n").unwrap();
        assert_eq!(one.version.len(), 16, "{one:?}");
        assert!(one.version.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(one.version, two.version, "an edit kept its version");
        assert_eq!(one.markdown, "# Terms\n\nBe kind.\n");
        assert_eq!(one.updated, None);
    }

    #[test]
    fn a_named_version_survives_an_editorial_fix_and_the_comments_are_not_shown() {
        let one =
            read(b"<!-- version: 2026-10 -->\n<!-- updated: 2026-10-06 -->\n\n# Terms\n").unwrap();
        let two =
            read(b"<!-- version: 2026-10 -->\n<!-- updated: 2026-10-07 -->\n\n# Terms.\n").unwrap();
        assert_eq!(one.version, "2026-10");
        assert_eq!(two.version, "2026-10");
        assert_eq!(one.updated.as_deref(), Some("2026-10-06"));
        assert_eq!(one.markdown, "# Terms\n");
        // A comment in the text is the author's, not a header.
        let three = read(b"# Terms\n<!-- version: 9 -->\n").unwrap();
        assert_ne!(three.version, "9");
        assert!(three.markdown.contains("<!-- version: 9 -->"));
    }

    #[test]
    fn what_cannot_be_the_terms_refuses_startup() {
        assert!(read(&vec![b'a'; MAX_TERMS_BYTES + 1]).is_err(), "oversize");
        assert!(read(&vec![b'a'; MAX_TERMS_BYTES]).is_ok(), "at the limit");
        assert!(read(&[0xff, 0xfe, b'a']).is_err(), "not UTF-8");
        assert!(read(b"  \n\n").is_err(), "empty");
        assert!(read(b"<!-- version: 1 -->\n").is_err(), "only a header");
        assert!(read(b"<!-- updated: x -->\n<!-- version: 1 -->\nText").is_err());
        assert!(read(b"<!-- version: two words -->\nText").is_err());
        assert!(
            read(b"<!-- version: \xe2\x80\xae1 -->\nText").is_err(),
            "bidi"
        );
        assert!(from_env(Some(OsStr::new("/nonexistent/terms.md"))).is_err());
        assert_eq!(from_env(None), Ok(None));
        assert_eq!(from_env(Some(OsStr::new(" "))), Ok(None));
    }

    /// The placeholder in the repository reads, and says on its first visible
    /// line that it is not legal text.
    #[test]
    fn the_placeholder_reads_and_says_what_it_is() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/terms-placeholder.md"
        );
        let terms = from_env(Some(OsStr::new(path))).unwrap().expect("terms");
        assert!(
            terms.markdown.contains("PLACEHOLDER") && terms.markdown.contains("not legal text"),
            "{}",
            terms.markdown
        );
    }
}
