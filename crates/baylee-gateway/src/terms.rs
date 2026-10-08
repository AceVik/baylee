//! The gateway's terms of use (WG-1): Markdown, read once at start, in one
//! language for everyone or in several.
//!
//! `BAYLEE_TERMS_PATH` names them. Unset (or blank) is a gateway without
//! terms, and then nothing a client sees changes: no `terms_stale` at sign-in,
//! `"terms": null` in `/info`, `404` on `/terms`. It names either
//!
//! - **a file**: one text, shown to every player whatever their language; or
//! - **a directory** of `terms.<lang>.md` files, `<lang>` a lowercase primary
//!   language tag (`de`, `en`). `terms.en.md` must be among them, since
//!   English is what a player whose language the gateway lacks is shown.
//!   Files not named `terms.*.md` are not read.
//!
//! Every file must be there, be UTF-8, hold some text and be at most
//! [`MAX_TERMS_BYTES`]; anything else refuses startup, because a gateway that
//! quietly ran without the terms its operator meant to show would be worse
//! than one that did not start.
//!
//! **The version** is what an account accepts. It is the first 16 hex digits
//! of the file's SHA-256, so any edit asks every player again — unless the
//! file's first line is `<!-- version: 2026-10 -->`, which names it outright,
//! so an editorial fix (a typo, a link) need not be accepted twice. In a
//! directory every language must carry the same version, or the gateway does
//! not start: accepting the terms in one language accepts that version, so
//! the languages have to be one text. Two files without a named version hash
//! apart, so a directory's files name theirs. An
//! `<!-- updated: 2026-10-06 -->` line among the leading comment lines is
//! shown as the date; the file's mtime is not, since that is the deploy's
//! date. Those leading comment lines are taken off the Markdown that is
//! served, which is what the client renders.

use crate::auth;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::path::Path;

/// The largest terms file a gateway takes, in bytes.
pub(crate) const MAX_TERMS_BYTES: usize = 64 * 1024;

/// The longest version a `<!-- version: … -->` line may name, in characters.
const MAX_VERSION_CHARS: usize = 64;

/// The language a directory must hold, and the one shown when the asked one
/// is not there.
const FALLBACK: &str = "en";

/// The terms, as read at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Terms {
    /// What an account accepts, and `/info` and `/terms` name: one for every
    /// language.
    pub(crate) version: String,
    /// The texts, never empty: one without a language (a file), or one per
    /// language, sorted by it, `en` among them (a directory).
    texts: Vec<Text>,
}

/// The terms in one language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Text {
    /// Its primary language tag; `None` for the one file every language is
    /// shown.
    pub(crate) lang: Option<String>,
    /// The date the file says it was last changed, if it says.
    pub(crate) updated: Option<String>,
    /// The text to show, without the leading comment lines.
    pub(crate) markdown: String,
}

impl Terms {
    /// The text for a client that asked for `asked` (`?lang=`, read by its
    /// primary tag, so `de-AT` is `de`): that language, else English, else
    /// the one file.
    pub(crate) fn text(&self, asked: Option<&str>) -> &Text {
        let of = |lang: &str| {
            self.texts.iter().find(|t| {
                t.lang
                    .as_deref()
                    .is_some_and(|l| l.eq_ignore_ascii_case(lang))
            })
        };
        asked
            .and_then(|a| a.split(['-', '_']).next())
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .and_then(of)
            .or_else(|| of(FALLBACK))
            .unwrap_or(&self.texts[0])
    }

    /// The languages the terms are in; empty for one file for all.
    #[cfg(test)]
    fn langs(&self) -> Vec<&str> {
        self.texts
            .iter()
            .filter_map(|t| t.lang.as_deref())
            .collect()
    }
}

/// The terms `BAYLEE_TERMS_PATH` names, `None` when it names none, or why
/// the gateway must not start.
pub(crate) fn from_env(path: Option<&OsStr>) -> Result<Option<Terms>, String> {
    let Some(path) = path.filter(|p| !p.to_string_lossy().trim().is_empty()) else {
        return Ok(None);
    };
    let path = Path::new(path);
    let meta =
        std::fs::metadata(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if meta.is_dir() {
        return from_dir(path).map(Some);
    }
    let (version, text) = read_file(path)?;
    Ok(Some(Terms {
        version,
        texts: vec![text],
    }))
}

/// One file's version and text, or why it cannot be the terms.
fn read_file(path: &Path) -> Result<(String, Text), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    read(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// A directory of `terms.<lang>.md` files.
fn from_dir(dir: &Path) -> Result<Terms, String> {
    let cannot = |e: std::io::Error| format!("cannot read {}: {e}", dir.display());
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(cannot)? {
        let entry = entry.map_err(cannot)?;
        let name = entry.file_name();
        let Some(lang) = name
            .to_str()
            .and_then(|n| n.strip_prefix("terms."))
            .and_then(|n| n.strip_suffix(".md"))
        else {
            continue;
        };
        if !is_primary_tag(lang) {
            return Err(format!(
                "{}: `{lang}` is not a lowercase primary language tag such as `de` or `en`",
                entry.path().display()
            ));
        }
        found.push((lang.to_owned(), entry.path()));
    }
    // `read_dir` answers in the file system's order; sorted, the texts and
    // every refusal below come out the same on every machine.
    found.sort();
    if !found.iter().any(|(lang, _)| lang == FALLBACK) {
        return Err(format!(
            "{} holds no terms.{FALLBACK}.md, and English is what a player in any other language is shown",
            dir.display()
        ));
    }
    let mut first: Option<(String, String)> = None;
    let mut texts = Vec::with_capacity(found.len());
    for (lang, path) in found {
        let (version, mut text) = read_file(&path)?;
        match &first {
            Some((named, by)) if *named != version => {
                return Err(format!(
                    "terms.{by}.md is version {named} and terms.{lang}.md is version {version}: \
                     every language must name the same version on its first line \
                     (`<!-- version: … -->`)"
                ));
            }
            Some(_) => {}
            None => first = Some((version, lang.clone())),
        }
        text.lang = Some(lang);
        texts.push(text);
    }
    // `en` was found, so there is a first.
    let (version, _) = first.ok_or_else(|| format!("{} holds no terms", dir.display()))?;
    Ok(Terms { version, texts })
}

/// A BCP-47 primary language subtag as a file names it: two or three
/// lowercase ASCII letters.
fn is_primary_tag(lang: &str) -> bool {
    (2..=3).contains(&lang.len()) && lang.bytes().all(|b| b.is_ascii_lowercase())
}

/// Reads a terms file's bytes: its version and its text (no language yet).
pub(crate) fn read(bytes: &[u8]) -> Result<(String, Text), String> {
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
    Ok((
        version,
        Text {
            lang: None,
            updated,
            markdown,
        },
    ))
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

    /// A fresh directory for one test, named after it.
    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("baylee-terms-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn dir_with(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = temp_dir(name);
        for (file, text) in files {
            std::fs::write(dir.join(file), text).expect("write");
        }
        dir
    }

    fn load(dir: &Path) -> Result<Terms, String> {
        from_env(Some(dir.as_os_str())).map(|t| t.expect("terms"))
    }

    #[test]
    fn without_a_named_version_any_edit_is_a_new_one() {
        let (one, text) = read(b"# Terms\n\nBe kind.\n").unwrap();
        let (two, _) = read(b"# Terms\n\nBe kind!\n").unwrap();
        assert_eq!(one.len(), 16, "{one:?}");
        assert!(one.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(one, two, "an edit kept its version");
        assert_eq!(text.markdown, "# Terms\n\nBe kind.\n");
        assert_eq!(text.updated, None);
    }

    #[test]
    fn a_named_version_survives_an_editorial_fix_and_the_comments_are_not_shown() {
        let (one, text) =
            read(b"<!-- version: 2026-10 -->\n<!-- updated: 2026-10-06 -->\n\n# Terms\n").unwrap();
        let (two, _) =
            read(b"<!-- version: 2026-10 -->\n<!-- updated: 2026-10-07 -->\n\n# Terms.\n").unwrap();
        assert_eq!(one, "2026-10");
        assert_eq!(two, "2026-10");
        assert_eq!(text.updated.as_deref(), Some("2026-10-06"));
        assert_eq!(text.markdown, "# Terms\n");
        // A comment in the text is the author's, not a header.
        let (three, text) = read(b"# Terms\n<!-- version: 9 -->\n").unwrap();
        assert_ne!(three, "9");
        assert!(text.markdown.contains("<!-- version: 9 -->"));
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

    /// A file is one text for every language: whatever is asked, it is
    /// served, and it names no language.
    #[test]
    fn a_file_is_shown_in_every_language() {
        let dir = dir_with("file", &[("terms.md", "<!-- version: 1 -->\n# One\n")]);
        let terms = load(&dir.join("terms.md")).unwrap();
        assert!(terms.langs().is_empty());
        for asked in [None, Some("de"), Some("en"), Some("fr"), Some("")] {
            let text = terms.text(asked);
            assert_eq!(text.lang, None, "{asked:?}");
            assert_eq!(text.markdown, "# One\n");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A directory: the asked language by its primary tag, English for any
    /// other, one version for all.
    #[test]
    fn a_directory_serves_the_asked_language_and_english_for_the_rest() {
        let dir = dir_with(
            "langs",
            &[
                ("terms.en.md", "<!-- version: 3 -->\n# Terms\n"),
                (
                    "terms.de.md",
                    "<!-- version: 3 -->\n<!-- updated: 2026-10-08 -->\n# Bedingungen\n",
                ),
                ("README.txt", "not read"),
                ("terms.md", "not a language, not read"),
            ],
        );
        let terms = load(&dir).unwrap();
        assert_eq!(terms.version, "3");
        assert_eq!(terms.langs(), ["de", "en"]);
        let shown = |asked: Option<&str>| {
            let text = terms.text(asked);
            (text.lang.clone().unwrap(), text.markdown.clone())
        };
        let de = ("de".to_owned(), "# Bedingungen\n".to_owned());
        let en = ("en".to_owned(), "# Terms\n".to_owned());
        assert_eq!(shown(Some("de")), de);
        assert_eq!(shown(Some("DE-at")), de, "by its primary tag");
        assert_eq!(shown(Some("de_CH")), de);
        assert_eq!(shown(Some("en")), en);
        assert_eq!(shown(Some("fr")), en, "a language not here is English");
        assert_eq!(shown(None), en);
        assert_eq!(shown(Some("")), en);
        assert_eq!(
            terms.text(Some("de")).updated.as_deref(),
            Some("2026-10-08")
        );
        assert_eq!(terms.text(Some("en")).updated, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_directory_that_cannot_be_the_terms_refuses_startup() {
        let refused = |name: &str, files: &[(&str, &str)]| {
            let dir = dir_with(name, files);
            let why = load(&dir).expect_err(name);
            let _ = std::fs::remove_dir_all(&dir);
            why
        };
        let why = refused(
            "versions",
            &[
                ("terms.en.md", "<!-- version: 2 -->\nTerms"),
                ("terms.de.md", "<!-- version: 1 -->\nBedingungen"),
            ],
        );
        assert!(
            why.contains("terms.de.md is version 1 and terms.en.md is version 2")
                && why.contains("same version"),
            "{why}"
        );
        // Unnamed, two languages hash apart: the sentence says what to do.
        let why = refused(
            "unnamed",
            &[("terms.en.md", "Terms"), ("terms.de.md", "Bedingungen")],
        );
        assert!(why.contains("<!-- version: … -->"), "{why}");
        let why = refused("no-en", &[("terms.de.md", "<!-- version: 1 -->\nText")]);
        assert!(why.contains("no terms.en.md"), "{why}");
        assert!(refused("empty", &[]).contains("no terms.en.md"));
        for bad in [
            "terms.DE.md",
            "terms.de-AT.md",
            "terms.d.md",
            "terms.deutsch.md",
        ] {
            let why = refused(
                "badlang",
                &[
                    ("terms.en.md", "<!-- version: 1 -->\nT"),
                    (bad, "<!-- version: 1 -->\nT"),
                ],
            );
            assert!(why.contains("primary language tag"), "{bad}: {why}");
        }
        // Each file is held to what a single file is.
        let why = refused(
            "emptyfile",
            &[
                ("terms.en.md", "<!-- version: 1 -->\nT"),
                ("terms.de.md", " \n"),
            ],
        );
        assert!(
            why.contains("terms.de.md") && why.contains("no text"),
            "{why}"
        );
    }

    /// The placeholders in the repository read, and say on their first
    /// visible line that they are not legal text.
    #[test]
    fn the_placeholders_read_and_say_what_they_are() {
        let docs = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs");
        let file = from_env(Some(OsStr::new(&format!("{docs}/terms-placeholder.md"))))
            .unwrap()
            .expect("terms");
        let markdown = &file.text(None).markdown;
        assert!(
            markdown.contains("PLACEHOLDER") && markdown.contains("not legal text"),
            "{markdown}"
        );
        let dir = load(Path::new(&format!("{docs}/terms-placeholder"))).unwrap();
        assert_eq!(dir.langs(), ["de", "en"]);
        let en = &dir.text(Some("en")).markdown;
        assert!(
            en.contains("PLACEHOLDER") && en.contains("not legal text"),
            "{en}"
        );
        let de = &dir.text(Some("de")).markdown;
        assert!(
            de.contains("PLATZHALTER") && de.contains("kein Rechtstext"),
            "{de}"
        );
    }
}
