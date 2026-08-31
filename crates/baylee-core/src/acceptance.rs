//! Parser for `data/acceptance-decks.txt` — the architecture acceptance
//! suite: both Commander decks whose full implementation proves the engine.

use std::collections::BTreeSet;

/// Deck-file line is malformed.
#[derive(Debug, thiserror::Error)]
pub enum DeckParseError {
    /// A row that is neither a section header nor `N Card Name`.
    #[error("deck line {line}: {text}")]
    DeckLine {
        /// 1-based line number.
        line: usize,
        /// The offending line.
        text: String,
    },
}

/// Deck zone of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Main deck.
    Main,
    /// Sideboard.
    Sideboard,
    /// Commander(s).
    Commander,
}

/// One parsed row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeckRow {
    /// Deck name (section `[deck:Name]`).
    pub deck: String,
    /// Zone.
    pub zone: Zone,
    /// Copies.
    pub count: u32,
    /// Card name as written.
    pub name: String,
}

/// Parses the acceptance deck file.
///
/// Format: `[deck:Name]` / `[sideboard]` / `[commander]` sections, rows
/// `N Card Name`, `#` comments, empty lines ignored.
///
/// # Errors
/// [`DeckParseError::DeckLine`] on malformed rows.
pub fn parse_decks(text: &str) -> Result<Vec<DeckRow>, DeckParseError> {
    let mut rows = Vec::new();
    let mut deck = String::new();
    let mut zone = Zone::Main;
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(name) = section.strip_prefix("deck:") {
                deck = name.to_string();
                zone = Zone::Main;
            } else if section.eq_ignore_ascii_case("sideboard") {
                zone = Zone::Sideboard;
            } else if section.eq_ignore_ascii_case("commander") {
                zone = Zone::Commander;
            } else {
                return Err(DeckParseError::DeckLine {
                    line: idx + 1,
                    text: line.to_string(),
                });
            }
            continue;
        }
        let (count, name) = line
            .split_once(' ')
            .ok_or_else(|| DeckParseError::DeckLine {
                line: idx + 1,
                text: line.to_string(),
            })?;
        let count: u32 = count.parse().map_err(|_| DeckParseError::DeckLine {
            line: idx + 1,
            text: line.to_string(),
        })?;
        rows.push(DeckRow {
            deck: deck.clone(),
            zone,
            count,
            name: name.trim().to_string(),
        });
    }
    Ok(rows)
}

/// Unique card names across all rows, sorted.
#[must_use]
pub fn unique_names(rows: &[DeckRow]) -> Vec<String> {
    rows.iter()
        .map(|r| r.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Card names from the standalone pool file (`data/card-pool.txt`): one name
/// per line, `#` comments and blank lines ignored.
///
/// The acceptance decks are the architecture proof and say what they say; a
/// card implemented because someone wants to play it does not belong in them.
/// This is where those live. Both lists feed the same registry, and the
/// `CardIndex` ledger keeps either from disturbing the other.
#[must_use]
pub fn pool_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Every card the registry should contain: the acceptance decks plus the
/// pool file, deduplicated. A card in both is one card.
#[must_use]
pub fn all_names(rows: &[DeckRow], pool_text: &str) -> Vec<String> {
    unique_names(rows)
        .into_iter()
        .chain(pool_names(pool_text))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pool file is a plain list, and a card named in both places is one
    /// card — otherwise it would take two indices and the deck resolver would
    /// have to pick.
    #[test]
    fn the_pool_file_and_the_decks_make_one_list() {
        let rows = parse_decks("[deck:A]\n1 Island\n2 Counterspell\n").expect("decks parse");
        let pool = "# a comment\n\nBrainstorm\nCounterspell\n  Island  \n";
        assert_eq!(
            pool_names(pool),
            vec!["Brainstorm", "Counterspell", "Island"]
        );
        assert_eq!(
            all_names(&rows, pool),
            vec!["Brainstorm", "Counterspell", "Island"],
            "Counterspell is in both and is still one card"
        );
    }

    /// A missing pool file is an empty pool, not an error: the file is
    /// optional and the acceptance decks stand on their own.
    #[test]
    fn an_empty_pool_leaves_the_decks_alone() {
        let rows = parse_decks("[deck:A]\n1 Island\n").expect("decks parse");
        assert_eq!(all_names(&rows, ""), vec!["Island"]);
    }

    #[test]
    fn parses_sections() {
        let text = "# comment\n[deck:A]\n1 Island\n\n[sideboard]\n2 Swamp\n[commander]\n1 Boss\n";
        let rows = parse_decks(text).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].deck, "A");
        assert_eq!(rows[0].zone, Zone::Main);
        assert_eq!(rows[1].zone, Zone::Sideboard);
        assert_eq!(rows[1].count, 2);
        assert_eq!(rows[2].zone, Zone::Commander);
        assert_eq!(unique_names(&rows), vec!["Boss", "Island", "Swamp"]);
    }
}
