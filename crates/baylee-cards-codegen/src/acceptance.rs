//! Parser for `data/acceptance-decks.txt` — the architecture acceptance
//! suite: both Commander decks whose full implementation proves the engine.

use crate::error::CodegenError;
use std::collections::BTreeSet;

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
/// [`CodegenError::DeckLine`] on malformed rows.
pub fn parse_decks(text: &str) -> Result<Vec<DeckRow>, CodegenError> {
    let mut rows = Vec::new();
    let mut deck = String::new();
    let mut zone = Zone::Main;
    for (idx, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(section) = line.strip_circumfix('[', ']') {
            if let Some(name) = section.strip_prefix("deck:") {
                deck = name.to_string();
                zone = Zone::Main;
            } else if section.eq_ignore_ascii_case("sideboard") {
                zone = Zone::Sideboard;
            } else if section.eq_ignore_ascii_case("commander") {
                zone = Zone::Commander;
            } else {
                return Err(CodegenError::DeckLine {
                    line: idx + 1,
                    text: line.to_string(),
                });
            }
            continue;
        }
        let (count, name) = line.split_once(' ').ok_or_else(|| CodegenError::DeckLine {
            line: idx + 1,
            text: line.to_string(),
        })?;
        let count: u32 = count.parse().map_err(|_| CodegenError::DeckLine {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections() {
        let text = "# comment\n[deck:A]\n1 Forest\n2 Lightning Bolt\n[sideboard]\n1 Karakas\n[commander]\n1 General Tazri\n";
        let rows = parse_decks(text).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[1].count, 2);
        assert_eq!(rows[2].zone, Zone::Sideboard);
        assert_eq!(rows[3].zone, Zone::Commander);
        assert_eq!(unique_names(&rows).len(), 4);
    }
}
