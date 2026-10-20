//! The gateway's terms of use (WG-1): Markdown, in one language for
//! everyone or in several, versioned, and published to Baylee Central, where
//! players of Central accounts accept them (`docs/protocol.md` §"Terms of
//! use (WG-1)").
//!
//! Where they come from, first answer wins: a version an admin published in
//! the console (kept in the database, the newest one current); else the file
//! or directory `BAYLEE_TERMS_PATH` names; else the default terms every
//! gateway ships (`docs/gateway-legal/`). `BAYLEE_TERMS_PATH=off` is a
//! gateway without terms, and then nothing a client sees changes: no
//! `terms_stale` at sign-in, `"terms": null` in `/info`, `404` on `/terms`.
//! The path names either
//!
//! - **a file**: one text, shown to every player whatever their language
//!   (Central knows it as English); or
//! - **a directory** of `terms.<lang>.md` files, `<lang>` a lowercase primary
//!   language tag (`de`, `en`). `terms.en.md` must be among them, since
//!   English is what a player whose language the gateway lacks is shown.
//!   Files not named `terms.*.md` are not read.
//!
//! Every file must be there, be UTF-8, hold some text and be at most
//! [`MAX_TERMS_BYTES`]; anything else refuses startup, because a gateway that
//! quietly ran without the terms its operator meant to show would be worse
//! than one that did not start. Central's bounds hold too: at most
//! [`MAX_LANGUAGES`] languages, no control character but newline and tab
//! (a `\r\n` line end is read as `\n`), a version of printable ASCII.
//!
//! **The version** is what an account accepts, together with the digest of
//! every language's text. It is the first 16 hex digits of the file's
//! SHA-256, so any edit asks every player again — unless the file's first
//! line is `<!-- version: 2026-10 -->`, which names it outright. In a
//! directory every language must carry the same version, or the gateway does
//! not start: accepting the terms in one language accepts that version, so
//! the languages have to be one text. Two files without a named version hash
//! apart, so a directory's files name theirs. An
//! `<!-- updated: 2026-10-06 -->` line among the leading comment lines is
//! shown as the date; the file's mtime is not, since that is the deploy's
//! date. Those leading comment lines are taken off the Markdown that is
//! served, which is what the client renders.

use crate::auth;
use baylee_account::terms::{self as central, TermsText, TermsVersion};
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

/// The most languages one version carries, as Central takes them.
pub(crate) const MAX_LANGUAGES: usize = central::MAX_TEXTS;

/// The terms every gateway ships, used when no admin published any and
/// `BAYLEE_TERMS_PATH` names none: German binding, English translation.
const DEFAULT: [(&str, &str); 2] = [
    (
        "de",
        include_str!("../../../docs/gateway-legal/gateway-terms.de.md"),
    ),
    (
        "en",
        include_str!("../../../docs/gateway-legal/gateway-terms.en.md"),
    ),
];

/// Where the terms in force came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// An admin published them in the console.
    Published,
    /// `BAYLEE_TERMS_PATH` named them.
    File,
    /// The default terms every gateway ships.
    Default,
}

impl Source {
    /// How the console names it.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::File => "file",
            Self::Default => "default",
        }
    }
}

/// The terms in force.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Terms {
    /// What an account accepts, and `/info` and `/terms` name: one for every
    /// language.
    pub(crate) version: String,
    /// The texts, never empty: one without a language (a file), or one per
    /// language, sorted by it, `en` among them (a directory).
    texts: Vec<Text>,
    /// Central's digest of every text: what an acceptance names beside the
    /// version.
    pub(crate) digest: [u8; 32],
    /// Where they came from.
    pub(crate) source: Source,
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
    /// Terms from a version and its texts (sorted by language, a file's one
    /// text without one), held to Central's bounds.
    fn new(version: String, texts: Vec<Text>, source: Source) -> Result<Self, String> {
        if version == central::DRAFT || !central::valid_version(&version) {
            return Err(format!(
                "`{version}` cannot be a version: printable ASCII without spaces, at most \
                 {MAX_VERSION_CHARS} characters, and not `{}`",
                central::DRAFT
            ));
        }
        if texts.len() > MAX_LANGUAGES {
            return Err(format!(
                "{} languages, and terms carry {MAX_LANGUAGES} at most",
                texts.len()
            ));
        }
        let mut terms = Self {
            version,
            texts,
            digest: [0; 32],
            source,
        };
        let shown = terms.central_texts();
        central::validate(&shown).map_err(|_| {
            "the terms are outside what Baylee Central takes (a language tag, a text \
             longer than 64 KiB or a control character)"
                .to_owned()
        })?;
        terms.digest = central::digest(&shown);
        Ok(terms)
    }

    /// Terms an admin published: a version and its languages.
    pub(crate) fn published(
        version: String,
        texts: Vec<(String, Option<String>, String)>,
    ) -> Result<Self, String> {
        let mut texts: Vec<Text> = texts
            .into_iter()
            .map(|(lang, updated, markdown)| {
                if !is_primary_tag(&lang) {
                    return Err(format!(
                        "`{lang}` is not a lowercase primary language tag such as `de` or `en`"
                    ));
                }
                Ok(Text {
                    lang: Some(lang),
                    updated: updated.filter(|d| !d.is_empty()),
                    markdown: normalized(&markdown)?,
                })
            })
            .collect::<Result<_, String>>()?;
        texts.sort_by(|a, b| a.lang.cmp(&b.lang));
        if texts.windows(2).any(|pair| pair[0].lang == pair[1].lang) {
            return Err("a language appears twice".to_owned());
        }
        if !texts.iter().any(|t| t.lang.as_deref() == Some(FALLBACK)) {
            return Err(format!(
                "the terms hold no `{FALLBACK}` text, and English is what a player in any other \
                 language is shown"
            ));
        }
        Self::new(version, texts, Source::Published)
    }

    /// Terms an admin published earlier, as the database holds them; refused
    /// when they no longer hash to the digest stored with them.
    pub(crate) fn from_published(published: baylee_db::terms::Published) -> Result<Self, String> {
        let stored = published.digest;
        let terms = Self::published(
            published.version,
            published
                .texts
                .into_iter()
                .map(|text| (text.lang, text.updated, text.markdown))
                .collect(),
        )?;
        if terms.digest.as_slice() != stored.as_slice() {
            return Err(format!(
                "the published terms {} no longer hash to the digest stored with them",
                terms.version
            ));
        }
        Ok(terms)
    }

    /// The texts as Central takes them: a file's one text is English.
    pub(crate) fn central_texts(&self) -> Vec<TermsText> {
        self.texts
            .iter()
            .map(|text| TermsText {
                lang: text.lang.clone().unwrap_or_else(|| FALLBACK.to_owned()),
                updated: text.updated.clone(),
                markdown: text.markdown.clone(),
            })
            .collect()
    }

    /// The version and digest an acceptance names.
    pub(crate) fn central_version(&self) -> TermsVersion {
        TermsVersion {
            version: self.version.clone(),
            digest: self.digest,
        }
    }

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

/// The terms `BAYLEE_TERMS_PATH` names: the default terms when it names
/// none, `None` for `off`, or why the gateway must not start.
pub(crate) fn from_env(path: Option<&OsStr>) -> Result<Option<Terms>, String> {
    let Some(path) = path.filter(|p| !p.to_string_lossy().trim().is_empty()) else {
        return default().map(Some);
    };
    if path.to_string_lossy().trim() == "off" {
        return Ok(None);
    }
    let path = Path::new(path);
    let meta =
        std::fs::metadata(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if meta.is_dir() {
        return from_dir(path).map(Some);
    }
    let (version, text) = read_file(path)?;
    Terms::new(version, vec![text], Source::File)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The default terms every gateway ships.
pub(crate) fn default() -> Result<Terms, String> {
    let mut version = None;
    let mut texts = Vec::with_capacity(DEFAULT.len());
    for (lang, source) in DEFAULT {
        let (named, mut text) = read(source.as_bytes())?;
        if version.as_ref().is_some_and(|v| *v != named) {
            return Err("the default terms name two versions".to_owned());
        }
        version = Some(named);
        text.lang = Some(lang.to_owned());
        texts.push(text);
    }
    let version = version.ok_or_else(|| "no default terms".to_owned())?;
    Terms::new(version, texts, Source::Default)
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
    Terms::new(version, texts, Source::File).map_err(|e| format!("{}: {e}", dir.display()))
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
    let markdown = normalized(body.trim_start_matches(['\r', '\n']))?;
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

/// A text with `\r\n` read as `\n`, refused when it holds any other control
/// character but tab, as Central refuses it.
fn normalized(text: &str) -> Result<String, String> {
    let text = text.replace("\r\n", "\n");
    if let Some(bad) = text
        .chars()
        .find(|&c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(format!(
            "the text contains the control character U+{:04X}",
            u32::from(bad)
        ));
    }
    Ok(text)
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
    if let Some(bad) = named.chars().find(|&c| !c.is_ascii_graphic()) {
        return Err(format!("the version contains U+{:04X}", u32::from(bad)));
    }
    if named == central::DRAFT {
        return Err(format!("`{}` is never a version in force", central::DRAFT));
    }
    Ok(named.to_owned())
}

/// Whether a session's account accepted the terms in force, and if not,
/// whose: a session that has not is refused everything but reading the
/// terms, signing out and deleting its data here (`crate::terms_admit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Standing {
    /// Both Central's terms and this gateway's, or the session is local.
    #[default]
    Accepted,
    /// Baylee Central's terms in force are not accepted.
    Central,
    /// This gateway's terms in force are not accepted.
    Gateway,
}

impl Standing {
    /// What a refusal names, so a client knows which sheet to show.
    pub(crate) const fn document(self) -> Option<&'static str> {
        match self {
            Self::Accepted => None,
            Self::Central => Some("central"),
            Self::Gateway => Some("gateway"),
        }
    }

    /// From Central's two answers; Central's own terms come first.
    pub(crate) const fn of(central_terms: bool, gateway_terms: bool) -> Self {
        match (central_terms, gateway_terms) {
            (false, _) => Self::Central,
            (true, false) => Self::Gateway,
            (true, true) => Self::Accepted,
        }
    }
}

/// The terms in force and the version Baylee Central confirmed it holds,
/// shared by every route. Players of Central accounts can accept only what
/// Central holds, so the gateway enforces a version only once Central
/// confirmed it ([`Book::confirmed`]).
#[derive(Debug, Default)]
pub(crate) struct Book {
    current: parking_lot::RwLock<Option<std::sync::Arc<Terms>>>,
    confirmed: parking_lot::RwLock<Option<TermsVersion>>,
}

impl Book {
    /// A book holding `terms`, none confirmed yet.
    pub(crate) fn new(terms: Option<Terms>) -> Self {
        Self {
            current: parking_lot::RwLock::new(terms.map(std::sync::Arc::new)),
            confirmed: parking_lot::RwLock::new(None),
        }
    }

    /// The terms in force, if any.
    pub(crate) fn current(&self) -> Option<std::sync::Arc<Terms>> {
        self.current.read().clone()
    }

    /// Put `terms` in force; Central has to confirm them again.
    pub(crate) fn replace(&self, terms: Terms) {
        *self.current.write() = Some(std::sync::Arc::new(terms));
        *self.confirmed.write() = None;
    }

    /// The version and digest Central confirmed for the terms in force.
    pub(crate) fn confirmed(&self) -> Option<TermsVersion> {
        self.confirmed.read().clone()
    }

    /// Central holds `version`; it counts only while it is still the one in
    /// force (an admin may have published another meanwhile).
    pub(crate) fn confirm(&self, version: &TermsVersion) {
        let current = self.current.read();
        if current
            .as_ref()
            .is_some_and(|terms| terms.central_version() == *version)
        {
            *self.confirmed.write() = Some(version.clone());
        }
    }
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
        assert!(read(b"<!-- version: draft -->\nText").is_err(), "draft");
        assert!(
            read(b"<!-- version: v\xc3\xa4 -->\nText").is_err(),
            "not ASCII"
        );
        assert!(read(b"Text with a \x07 bell").is_err(), "control");
        assert!(read(b"A lone \r carriage return").is_err(), "lone CR");
    }

    /// Unset is the default terms every gateway ships; `off` is none.
    #[test]
    fn unset_is_the_default_terms_and_off_is_none() {
        for unset in [None, Some(OsStr::new("")), Some(OsStr::new(" "))] {
            let terms = from_env(unset).unwrap().expect("the default terms");
            assert_eq!(terms.source, Source::Default);
            assert_eq!(terms, default().unwrap());
        }
        assert_eq!(from_env(Some(OsStr::new("off"))), Ok(None));
        assert_eq!(from_env(Some(OsStr::new(" off "))), Ok(None));
    }

    /// The shipped default reads, in German and English, names one version
    /// and hashes as Central hashes it.
    #[test]
    fn the_default_terms_read_in_both_languages() {
        let terms = default().unwrap();
        assert_eq!(terms.langs(), ["de", "en"]);
        assert!(central::valid_version(&terms.version), "{}", terms.version);
        assert_eq!(terms.digest, central::digest(&terms.central_texts()));
        assert!(
            terms
                .text(Some("de"))
                .markdown
                .contains("Nutzungsbedingungen")
        );
        assert!(terms.text(Some("en")).markdown.contains("Terms of use"));
        for text in terms.central_texts() {
            for placeholder in ["[Betreiber", "[operator", "[14]"] {
                assert!(
                    !text.markdown.contains(placeholder),
                    "{placeholder} left in {}",
                    text.lang
                );
            }
        }
    }

    /// A Windows line end is a line end; the digest does not see it.
    #[test]
    fn crlf_reads_as_lf() {
        let (_, crlf) = read(b"<!-- version: 1 -->\r\n# Terms\r\n\r\nBe kind.\r\n").unwrap();
        let (_, lf) = read(b"<!-- version: 1 -->\n# Terms\n\nBe kind.\n").unwrap();
        assert_eq!(crlf.markdown, lf.markdown);
    }

    /// Any edit to any language is a new digest; the version alone is not
    /// what an acceptance names.
    #[test]
    fn the_digest_follows_every_language() {
        let one = Terms::published(
            "7".into(),
            vec![
                ("en".into(), None, "# Terms\n".into()),
                (
                    "de".into(),
                    Some("2026-10-20".into()),
                    "# Bedingungen\n".into(),
                ),
            ],
        )
        .unwrap();
        assert_eq!(one.langs(), ["de", "en"], "sorted");
        let two = Terms::published(
            "7".into(),
            vec![
                ("en".into(), None, "# Terms\n".into()),
                (
                    "de".into(),
                    Some("2026-10-20".into()),
                    "# Bedingungen.\n".into(),
                ),
            ],
        )
        .unwrap();
        assert_eq!(one.version, two.version);
        assert_ne!(one.digest, two.digest);
        assert_eq!(one.central_version().digest, one.digest);
    }

    /// Only the version in force is confirmed, and a new one needs its own.
    #[test]
    fn a_book_confirms_only_the_version_in_force() {
        let one = default().unwrap();
        let book = Book::new(Some(one.clone()));
        assert_eq!(book.confirmed(), None);
        let mut other = one.central_version();
        other.digest[0] ^= 1;
        book.confirm(&other);
        assert_eq!(book.confirmed(), None, "a digest that is not in force");
        book.confirm(&one.central_version());
        assert_eq!(book.confirmed(), Some(one.central_version()));
        let two = Terms::published("2".into(), vec![("en".into(), None, "# T\n".into())]).unwrap();
        book.replace(two.clone());
        assert_eq!(book.confirmed(), None, "a new version is unconfirmed");
        book.confirm(&one.central_version());
        assert_eq!(book.confirmed(), None, "the old one no longer counts");
        assert_eq!(book.current().as_deref(), Some(&two));
    }

    #[test]
    fn what_an_admin_cannot_publish() {
        let text = |lang: &str| (lang.to_owned(), None, "# T\n".to_owned());
        for (version, texts) in [
            ("1", vec![text("de")]),
            ("1", vec![text("en"), text("en")]),
            ("1", vec![text("EN")]),
            (
                "1",
                vec![text("en"), text("de"), text("fr"), text("es"), text("it")],
            ),
            ("draft", vec![text("en")]),
            ("two words", vec![text("en")]),
            ("", vec![text("en")]),
        ] {
            assert!(
                Terms::published(version.into(), texts.clone()).is_err(),
                "{version} {}",
                texts.len()
            );
        }
        assert!(Terms::published("1".into(), vec![("en".into(), None, String::new())]).is_err());
        assert!(Terms::published("1".into(), vec![text("en")]).is_ok());
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
        let why = refused(
            "many",
            &[
                ("terms.de.md", "<!-- version: 1 -->\nT"),
                ("terms.en.md", "<!-- version: 1 -->\nT"),
                ("terms.es.md", "<!-- version: 1 -->\nT"),
                ("terms.fr.md", "<!-- version: 1 -->\nT"),
                ("terms.it.md", "<!-- version: 1 -->\nT"),
            ],
        );
        assert!(why.contains("languages"), "{why}");
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
