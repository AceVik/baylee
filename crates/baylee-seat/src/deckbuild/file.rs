//! A deck as its file holds it: Baylee text (`docs/deck-format.md`), one
//! of the four formats a player exports and [`crate::deck::Deck`] reads,
//! with the deck's name and format in its head and plain `N Name` rows.

use super::check::Rows;
use baylee_account::ranked::Format;
use baylee_core::deckrow;
use baylee_deckio::{Document, FormatId};

/// `rows` of `format` as a deck file's text.
///
/// # Errors
/// A row that does not read (never one [`super::check`] passed).
pub fn text(rows: &Rows, format: Format) -> Result<String, String> {
    let parse = |lines: &[String]| -> Result<Vec<deckrow::Row>, String> {
        lines
            .iter()
            .map(|line| deckrow::parse(line).map_err(|e| format!("«{line}»: {e}")))
            .collect()
    };
    let document = Document::from_stored(
        Some(&rows.name),
        Some(format.as_str()),
        &parse(&rows.main)?,
        &parse(&rows.sideboard)?,
        &rows.commanders,
    );
    Ok(baylee_deckio::export(FormatId::Baylee, &document).text)
}

/// A deck file's text as its rows, and the format its head names.
///
/// # Errors
/// A file no deck format reads, a link, or a line the reader skipped.
pub fn read(text: &str, fallback_name: &str) -> Result<(Rows, Option<String>), String> {
    let read = match baylee_deckio::import(text).map_err(|e| format!("the deck: {e:?}"))? {
        baylee_deckio::Import::Read { read, .. } => read,
        baylee_deckio::Import::Source(_) | baylee_deckio::Import::UnknownLink(_) => {
            return Err("that is a link to a deck, not a deck".into());
        }
    };
    if let Some(skipped) = read.skipped.first() {
        return Err(format!(
            "line {} of the deck cannot be read: {}",
            skipped.line, skipped.text
        ));
    }
    let format = read.document.format.clone();
    let stored = read.document.stored();
    // A stored deck lists each commander among its cards; a file's rows
    // name them apart, once.
    let mut leaders = stored.commanders.clone();
    let mut main = Vec::with_capacity(stored.cards.len());
    for mut row in stored.cards {
        if let Some(at) = leaders.iter().position(|name| *name == row.name) {
            leaders.remove(at);
            row.count = row.count.saturating_sub(1);
        }
        if row.count > 0 {
            main.push(row.to_string());
        }
    }
    Ok((
        Rows {
            name: stored.name.unwrap_or_else(|| fallback_name.to_string()),
            main,
            sideboard: stored.sideboard.iter().map(ToString::to_string).collect(),
            commanders: stored.commanders,
        },
        format,
    ))
}

/// The deck file at `path` as its rows, held to name `format` in its head.
///
/// # Errors
/// As [`read`], a file that cannot be read, or one of another format.
pub fn load(path: &std::path::Path, format: Format) -> Result<Rows, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Deck");
    let (rows, named) = read(&text, stem).map_err(|e| format!("{}: {e}", path.display()))?;
    match named.as_deref() {
        Some(named) if named == format.as_str() => Ok(rows),
        Some(named) => Err(format!(
            "{} is a {named} deck, not {}",
            path.display(),
            format.as_str()
        )),
        None => Err(format!(
            "{} names no format; a ranked deck's file says `# format: {}`",
            path.display(),
            format.as_str()
        )),
    }
}
