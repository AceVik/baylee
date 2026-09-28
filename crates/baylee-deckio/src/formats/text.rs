//! What the text formats share: reading a deck list line by line.
//!
//! Every row goes through `baylee_core::deckrow::parse`, the one row grammar
//! a stored deck is read with. What a text format adds around the rows is
//! only *where* a row goes — section headers, zone prefixes — and a few
//! spellings other sites use for the same row (`1x Name`, `Front / Back`),
//! which are rewritten into the row grammar's before it reads them.

use crate::document::{Card, Document, Zone};
use crate::format::{Read, Skipped};
use baylee_core::deckrow;

/// What differs between the text formats when reading.
#[derive(Clone, Copy)]
pub(crate) struct Dialect {
    /// `Front / Back` is a double-faced card (Moxfield's spelling), read as
    /// `Front // Back` (Scryfall's, and Baylee's).
    pub(crate) single_slash_faces: bool,
    /// `# name: …` and `# format: …` comment lines carry the document's
    /// fields (Baylee's own header).
    pub(crate) header_fields: bool,
}

/// The zone a section header line switches to, or `None` for a line that is
/// not one.
///
/// A header is the whole line: `SIDEBOARD:` (Moxfield), `Sideboard` (Arena),
/// `// Sideboard` (the comment form some sites write), in any case, with or
/// without the colon. No card is called any of these words, so a header can
/// never be a row somebody meant.
pub(crate) fn header(line: &str) -> Option<Header> {
    let word = line.trim();
    let word = word.strip_prefix("//").unwrap_or(word).trim();
    let word = word.strip_suffix(':').unwrap_or(word).trim();
    let zone = match word.to_ascii_lowercase().as_str() {
        "deck" | "main" | "mainboard" | "main deck" => Zone::Main,
        // A companion (CR 702.139a) starts the game outside it, which in a
        // constructed deck is the sideboard.
        "sideboard" | "side" | "companion" => Zone::Side,
        "commander" | "commanders" => Zone::Commander,
        "maybeboard" | "maybe" | "considering" => Zone::Maybe,
        "about" => return Some(Header::About),
        _ => return None,
    };
    Some(Header::Zone(zone))
}

/// A section header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Header {
    /// Rows after it go to this zone.
    Zone(Zone),
    /// Arena's `About` block, whose `Name …` line names the deck.
    About,
}

/// A zone prefix on a row (`SB: 1 Karakas`), and the row after it.
pub(crate) fn prefixed(line: &str) -> Option<(Zone, &str)> {
    let (head, rest) = line.split_once(':')?;
    let zone = match head.trim().to_ascii_uppercase().as_str() {
        "SB" => Zone::Side,
        "CMD" => Zone::Commander,
        "MB" => Zone::Maybe,
        _ => return None,
    };
    Some((zone, rest.trim_start()))
}

/// Rewrites the other sites' spellings of a row into the row grammar's.
fn normalise(line: &str) -> String {
    let mut line = line.trim().to_string();
    // `1x Name` and `1X Name`: the count's `x` is the one difference.
    if let Some((count, rest)) = line.split_once(' ')
        && let Some(digits) = count.strip_suffix(['x', 'X'])
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        line = format!("{digits} {rest}");
    }
    line
}

/// `Front / Back` as `Front // Back`, when the dialect spells faces so.
///
/// Only a name holding exactly one ` / ` and no ` // ` is rewritten: that is
/// the one shape Moxfield writes a double-faced card in, and no printed card
/// name contains a lone slash between spaces.
fn faces(name: &str, dialect: Dialect) -> String {
    if dialect.single_slash_faces && !name.contains(" // ") && name.matches(" / ").count() == 1 {
        name.replacen(" / ", " // ", 1)
    } else {
        name.to_string()
    }
}

/// Reads a deck list.
pub(crate) fn read(text: &str, dialect: Dialect) -> Read {
    let mut document = Document::default();
    let mut skipped = Vec::new();
    let mut zone = Zone::Main;
    let mut about = false;
    for (at, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            // A commander or companion section is a few lines long and ends
            // at the blank line after it; what follows is the deck again.
            // Every other section runs until the next header.
            if matches!(zone, Zone::Commander) || about {
                zone = Zone::Main;
            }
            about = false;
            continue;
        }
        if let Some(found) = header(line) {
            match found {
                Header::Zone(to) => {
                    zone = to;
                    about = false;
                }
                Header::About => about = true,
            }
            continue;
        }
        if about {
            if let Some(name) = line.strip_prefix("Name ") {
                document.name = Some(name.trim().to_string());
            }
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if dialect.header_fields {
                let comment = comment.trim();
                if let Some(name) = comment.strip_prefix("name:") {
                    document.name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
                } else if let Some(format) = comment.strip_prefix("format:") {
                    document.format = Some(format.trim().to_string()).filter(|f| !f.is_empty());
                }
            }
            continue;
        }
        let (row_zone, row) = match prefixed(line) {
            Some((prefixed_zone, rest)) => (prefixed_zone, rest),
            None => (zone, line),
        };
        match deckrow::parse(&normalise(row)) {
            Ok(mut parsed) => {
                parsed.name = faces(&parsed.name, dialect);
                document.cards.push(Card::from_row(row_zone, parsed));
            }
            Err(_) => skipped.push(Skipped::new(at + 1, raw)),
        }
        // Checked as the rows arrive, so a text of a million one-line rows
        // stops at the limit rather than after building all of them.
        if document.cards.len() > crate::MAX_ROWS {
            break;
        }
    }
    Read { document, skipped }
}

/// Whether some line of the text is a row with a zone prefix.
pub(crate) fn has_prefix(text: &str) -> bool {
    text.lines().any(|line| prefixed(line.trim()).is_some())
}

/// Whether some line of the text is a section header.
pub(crate) fn has_header(text: &str) -> bool {
    text.lines().any(|line| header(line).is_some())
}

/// Whether the first line that says anything starts like a row: a count.
pub(crate) fn starts_like_a_list(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .is_some_and(|line| {
            header(line).is_some()
                || prefixed(line).is_some()
                || line.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Dialect = Dialect {
        single_slash_faces: true,
        header_fields: false,
    };

    #[test]
    fn a_header_is_the_whole_line_in_any_of_its_spellings() {
        for line in ["SIDEBOARD:", "Sideboard", "// Sideboard", "sideboard :"] {
            assert_eq!(header(line), Some(Header::Zone(Zone::Side)), "{line}");
        }
        assert_eq!(header("COMMANDER:"), Some(Header::Zone(Zone::Commander)));
        assert_eq!(header("Deck"), Some(Header::Zone(Zone::Main)));
        assert_eq!(header("1 Sideboard"), None);
    }

    #[test]
    fn other_sites_counts_read_as_counts() {
        let read = read("1x Aesi, Tyrant of Gyre Strait (DSC)\n4X Forest", PLAIN);
        assert!(read.skipped.is_empty(), "{:?}", read.skipped);
        assert_eq!(read.document.cards[0].name, "Aesi, Tyrant of Gyre Strait");
        assert_eq!(read.document.cards[0].set.as_deref(), Some("DSC"));
        assert_eq!(read.document.cards[1].count, 4);
    }

    #[test]
    fn a_commander_section_ends_at_the_blank_line() {
        let read = read("Commander\n1 Atraxa, Grand Unifier\n\n1 Sol Ring\n", PLAIN);
        let zones: Vec<Zone> = read.document.cards.iter().map(|c| c.zone).collect();
        assert_eq!(zones, [Zone::Commander, Zone::Main]);
    }

    #[test]
    fn a_sideboard_runs_past_a_blank_line() {
        let read = read("1 A\n\nSIDEBOARD:\n1 B\n\n1 C\n", PLAIN);
        let zones: Vec<Zone> = read.document.cards.iter().map(|c| c.zone).collect();
        assert_eq!(zones, [Zone::Main, Zone::Side, Zone::Side]);
    }

    #[test]
    fn arenas_about_block_names_the_deck() {
        let read = read("About\nName Ossi's Deck\n\nDeck\n1 Forest\n", PLAIN);
        assert_eq!(read.document.name.as_deref(), Some("Ossi's Deck"));
        assert_eq!(read.document.cards.len(), 1);
    }

    #[test]
    fn a_line_that_is_no_row_is_reported_with_its_number() {
        let read = read("1 Forest\nForest of Doom\n", PLAIN);
        assert_eq!(read.document.cards.len(), 1);
        assert_eq!(read.skipped, [Skipped::new(2, "Forest of Doom")]);
    }

    #[test]
    fn only_a_lone_single_slash_is_a_pair_of_faces() {
        assert_eq!(faces("Fire // Ice", PLAIN), "Fire // Ice");
        assert_eq!(faces("A / B", PLAIN), "A // B");
        assert_eq!(faces("A / B / C", PLAIN), "A / B / C");
        let baylee = Dialect {
            single_slash_faces: false,
            header_fields: true,
        };
        assert_eq!(faces("A / B", baylee), "A / B");
    }
}
