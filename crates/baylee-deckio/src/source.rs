//! Places a deck can be linked from, and what to do with a link to one.
//!
//! A pasted URL is not a deck; it names one somewhere else. A [`Source`]
//! recognises its own links and answers in one of two ways:
//!
//! - [`Answer::Fetch`]: the site has an open API. The answer is a plan — the
//!   URL to `GET` and the [`FormatId`] that reads the body — and the *client*
//!   carries it out through its own HTTP path, so the request goes from the
//!   player's machine to the site and the gateway never fetches a third
//!   party on anybody's behalf (`docs/privacy.md`). No source answers this
//!   way yet; the variant is the door a future one comes in by.
//! - [`Answer::Instruction`]: the site has no open API, or does not want to
//!   be fetched. The answer is what the player should do instead, which the
//!   client says in the player's language.
//!
//! Moxfield is the second kind. It has no public API and stands behind bot
//! protection that refuses a plain request, and fetching it anyway — by
//! proxy, by a borrowed browser's user agent — is exactly what that
//! protection says not to do. So a Moxfield link is recognised, and the
//! answer is how to copy the deck out of Moxfield by hand
//! ([`Instruction::MoxfieldExport`]); the text that produces is
//! [`crate::FormatId::Moxfield`].
//!
//! Adding a source is one type implementing [`Source`] and one line in
//! [`SOURCES`]. Nothing here makes a request.

use crate::format::FormatId;

/// The sites this build recognises.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// moxfield.com.
    Moxfield,
}

impl SourceId {
    /// The site's name as it writes it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Moxfield => "Moxfield",
        }
    }
}

/// What to do with a recognised link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Fetch this, and read the body with that format.
    Fetch(FetchPlan),
    /// Tell the player how to get the deck out by hand.
    Instruction(Instruction),
}

/// A request the client may make for a source with an open API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchPlan {
    /// The URL to `GET`, `https` only.
    pub url: String,
    /// The format the response body is in.
    pub format: FormatId,
}

/// What the player is asked to do instead of a fetch.
///
/// An enum and not a sentence, because the sentence is the client's to say
/// in the player's language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    /// Open the deck on Moxfield, choose More → Export → "Copy for
    /// Moxfield", and paste what that copies.
    MoxfieldExport,
}

/// A link a source recognised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recognised {
    /// Whose it is.
    pub source: SourceId,
    /// The deck's id on that site, as the link spells it.
    pub deck: String,
    /// What to do about it.
    pub answer: Answer,
}

/// A link, taken apart as far as a source needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link<'a> {
    /// The host, lower-case, without `www.` or a port.
    pub host: &'a str,
    /// The path, from its first `/`, without query or fragment.
    pub path: &'a str,
}

/// Takes a pasted text apart as a link: one token, with or without an
/// `http(s)://` in front, whose host has a dot in it. The host comes back
/// owned because it is lower-cased.
fn split_link(text: &str) -> Option<(String, &str)> {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) {
        return None;
    }
    let rest = strip_scheme(text);
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') {
        return None;
    }
    let path_end = tail.find(['?', '#']).unwrap_or(tail.len());
    Some((host, &tail[..path_end]))
}

fn strip_scheme(text: &str) -> &str {
    for scheme in ["https://", "http://"] {
        if text
            .get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
        {
            return &text[scheme.len()..];
        }
    }
    text
}

/// One site decks can be linked from.
pub trait Source: Sync {
    /// Which one.
    fn id(&self) -> SourceId;
    /// The deck id a link names, when the link is one of this site's deck
    /// links, and what to do about it.
    fn recognise(&self, link: Link<'_>) -> Option<(String, Answer)>;
}

/// Every source, asked in order.
pub static SOURCES: [&dyn Source; 1] = [&Moxfield];

/// The source a pasted text links to, or `None` for a text that is not a
/// link to a deck any source knows.
#[must_use]
pub fn recognise(text: &str) -> Option<Recognised> {
    let (host, path) = split_link(text)?;
    let link = Link { host: &host, path };
    SOURCES.iter().find_map(|source| {
        source.recognise(link).map(|(deck, answer)| Recognised {
            source: source.id(),
            deck,
            answer,
        })
    })
}

/// The host of a pasted link no source recognised, for saying which site
/// Baylee cannot read from. `None` for a text that is no link at all.
#[must_use]
pub fn unknown_link(text: &str) -> Option<String> {
    if recognise(text).is_some() {
        return None;
    }
    let trimmed = text.trim();
    let has_scheme = strip_scheme(trimmed).len() != trimmed.len();
    // Without a scheme a single token with a dot is too often something
    // else — `1x` is not, but `Mox.` could be — so only a written scheme
    // makes an unknown text a link.
    has_scheme
        .then(|| split_link(trimmed).map(|(host, _)| host))
        .flatten()
}

/// moxfield.com: no public API, bot protection, so an instruction.
struct Moxfield;

impl Source for Moxfield {
    fn id(&self) -> SourceId {
        SourceId::Moxfield
    }

    fn recognise(&self, link: Link<'_>) -> Option<(String, Answer)> {
        if link.host != "moxfield.com" {
            return None;
        }
        let mut parts = link.path.split('/').filter(|part| !part.is_empty());
        if parts.next()? != "decks" {
            return None;
        }
        let deck = parts.next()?;
        let well_formed = (1..=64).contains(&deck.len())
            && deck
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        well_formed.then(|| {
            (
                deck.to_string(),
                Answer::Instruction(Instruction::MoxfieldExport),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moxfield_deck_link_is_answered_with_the_export_instruction() {
        for text in [
            "https://moxfield.com/decks/Xq3ExampleDeck0000000a",
            "  https://www.moxfield.com/decks/Xq3ExampleDeck0000000a/primer?x=1#top \n",
            "moxfield.com/decks/Xq3ExampleDeck0000000a",
            "HTTP://MOXFIELD.COM/decks/Xq3ExampleDeck0000000a",
        ] {
            let found = recognise(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(found.source, SourceId::Moxfield);
            assert_eq!(found.deck, "Xq3ExampleDeck0000000a");
            assert_eq!(
                found.answer,
                Answer::Instruction(Instruction::MoxfieldExport)
            );
        }
    }

    #[test]
    fn what_is_not_a_moxfield_deck_link_is_not_one() {
        for text in [
            "https://moxfield.com/",
            "https://moxfield.com/users/someone",
            "https://moxfield.com/decks/",
            "https://moxfield.com.evil.example/decks/abc",
            "https://notmoxfield.com/decks/abc",
            "https://moxfield.com/decks/abc def",
            "1 Sol Ring",
            "1 Archangel Avacyn / Avacyn, the Purifier (SOI) 5",
        ] {
            assert_eq!(recognise(text), None, "{text}");
        }
    }

    #[test]
    fn an_unknown_link_names_its_host_and_a_deck_list_is_no_link() {
        assert_eq!(
            unknown_link("https://archidekt.com/decks/123"),
            Some("archidekt.com".to_string())
        );
        assert_eq!(unknown_link("https://moxfield.com/decks/abc"), None);
        assert_eq!(unknown_link("1 Sol Ring"), None);
        assert_eq!(unknown_link("Sol.Ring"), None);
    }

    /// `moxfield.com@evil.example` is the host `evil.example`, whatever the
    /// text before the `@` says: a deck link is judged by where it goes.
    #[test]
    fn a_link_cannot_borrow_a_host_through_its_userinfo() {
        assert_eq!(
            recognise("https://moxfield.com@evil.example/decks/abc"),
            None
        );
        assert!(recognise("https://user@moxfield.com/decks/abc").is_some());
        assert!(recognise("https://moxfield.com:443/decks/abc").is_some());
    }

    #[test]
    fn a_deck_id_is_one_to_sixty_four_url_safe_characters() {
        let at = |id: &str| recognise(&format!("https://moxfield.com/decks/{id}"));
        assert!(at("a").is_some());
        assert!(at(&"a".repeat(64)).is_some());
        assert!(at("A_b-9").is_some());
        assert_eq!(at(&"a".repeat(65)), None);
        assert_eq!(at("a%20b"), None);
        assert_eq!(at("a.b"), None);
    }

    #[test]
    fn a_host_must_be_a_dotted_name_with_no_empty_label_at_its_edges() {
        for text in [
            "https://localhost/decks/abc",
            "https://.moxfield.com/decks/abc",
            "https://moxfield.com./decks/abc",
            "",
            "   ",
        ] {
            assert_eq!(recognise(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_source_says_its_name_and_its_only_answer_is_an_instruction() {
        assert_eq!(SourceId::Moxfield.name(), "Moxfield");
        assert_eq!(SOURCES.len(), 1);
        assert_eq!(SOURCES[0].id(), SourceId::Moxfield);
    }

    #[test]
    fn a_link_with_a_scheme_but_no_dot_is_no_link_at_all() {
        assert_eq!(unknown_link("https://localhost/x"), None);
        assert_eq!(
            unknown_link("  HTTPS://WWW.Archidekt.com/decks/1  "),
            Some("archidekt.com".to_string()),
            "host is lower-cased and www is dropped"
        );
    }
}
