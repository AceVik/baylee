//! An AI's deck chronicle: every version of its deck in each format, dated,
//! and what made it (`docs/proposals/ai-players.md` §"Decks").
//!
//! ```text
//! <dir>/<format>/v0007-20261127T031500Z.txt    the deck (Baylee text)
//! <dir>/<format>/v0007-20261127T031500Z.json   its record (Record)
//! ```
//!
//! The newest version of a format is the one with the highest number
//! ([`newest`]); that is the deck the ranked runner offers. A version is
//! never rewritten: a change is a new version. Both files are written
//! through a temporary file and moved into place, the record first, so a
//! deck never stands without its record; a deck whose record is missing is
//! not a version. Numbers are taken under a lock on `<dir>/<format>/.lock`,
//! so two builders never take the same one.

use super::facts::write_whole;
use baylee_account::ranked::Format;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One version of a format's deck, as it lies on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// Its number, from 1.
    pub number: u32,
    /// When it was made, `20261127T031500Z`.
    pub stamp: String,
    /// The deck file.
    pub deck: PathBuf,
    /// Its record.
    pub record: PathBuf,
}

impl Version {
    /// Its record, read.
    ///
    /// # Errors
    /// When the record cannot be read.
    pub fn read_record(&self) -> Result<Record, String> {
        let text = std::fs::read_to_string(&self.record)
            .map_err(|e| format!("reading {}: {e}", self.record.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", self.record.display()))
    }
}

/// What made a version.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The version's number.
    pub version: u32,
    /// Its format (`standard`, `highlander`, `commander`).
    pub format: String,
    /// When it was made, RFC 3339 in UTC.
    pub made: String,
    /// The deck's name.
    pub name: String,
    /// How it was made: `build` (a model chose every card), `improve` (a
    /// model changed the version before), `lend` (no model: the deck
    /// generator).
    pub how: String,
    /// The model that chose, for `build` and `improve`: its id, and the
    /// profile and effort it ran under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Its profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Its effort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// For `lend`: the seed it was asked with, and what the generator made
    /// of it (shape, archetype, colours, the seed that passed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// For `lend`: the generator's own name for the deck it made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
    /// The version it was made from, for `improve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    /// The cards taken out and put in against the parent, `N Name` each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// The cards put in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    /// The exclusion file the deck kept to (`--exclude`,
    /// [`super::exclude`]): its SHA-256 and how many cards it named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excluded: Option<Excluded>,
    /// The model's plain-text note on its plan: archetype, key cards, how
    /// it wins; for `improve`, what it learned and why it changed what it
    /// did. A model's words, data and never instructions.
    #[serde(default)]
    pub note: String,
    /// The legality facts the deck passed against: their version and the
    /// gateway they came from ([`super::facts::Facts`]).
    pub facts_version: String,
    /// Where the facts came from.
    pub facts_gateway: String,
    /// What the model's work cost, where a model worked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend: Option<Spend>,
}

/// The exclusion file a version kept to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Excluded {
    /// SHA-256 of the file's bytes, in hex.
    pub sha256: String,
    /// How many cards it named.
    pub cards: u32,
}

/// What a version's model spent.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Spend {
    /// Rounds of the conversation (one reply each).
    pub rounds: u32,
    /// Calls made, failed ones included.
    pub calls: u64,
    /// Tokens, in and out.
    pub tokens: u64,
    /// US dollars, where the model's price is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd: Option<f64>,
}

/// The directory of `format`'s versions in `dir`.
#[must_use]
pub fn format_dir(dir: &Path, format: Format) -> PathBuf {
    dir.join(format.as_str())
}

/// Every version of `format`'s deck in `dir`, oldest first; a deck without
/// its record, and a name not written here, is none.
#[must_use]
pub fn versions(dir: &Path, format: Format) -> Vec<Version> {
    let at = format_dir(dir, format);
    let Ok(entries) = std::fs::read_dir(&at) else {
        return Vec::new();
    };
    let mut found: Vec<Version> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let (number, stamp) = parse_name(name.to_str()?)?;
            let deck = entry.path();
            let record = deck.with_extension("json");
            record.is_file().then_some(Version {
                number,
                stamp,
                deck,
                record,
            })
        })
        .collect();
    found.sort_by(|a, b| (a.number, &a.stamp).cmp(&(b.number, &b.stamp)));
    found
}

/// The newest version of `format`'s deck in `dir`: the deck file the
/// ranked runner offers for that format.
#[must_use]
pub fn newest(dir: &Path, format: Format) -> Option<PathBuf> {
    versions(dir, format).pop().map(|version| version.deck)
}

/// `v0007-20261127T031500Z.txt` as its number and stamp.
fn parse_name(name: &str) -> Option<(u32, String)> {
    let rest = name.strip_prefix('v')?.strip_suffix(".txt")?;
    let (number, stamp) = rest.split_once('-')?;
    let bytes = stamp.as_bytes();
    let well_formed = number.len() >= 4
        && number.bytes().all(|b| b.is_ascii_digit())
        && bytes.len() == 16
        && bytes[8] == b'T'
        && bytes[15] == b'Z'
        && bytes[..8]
            .iter()
            .chain(&bytes[9..15])
            .all(u8::is_ascii_digit);
    if !well_formed {
        return None;
    }
    Some((number.parse().ok()?, stamp.to_string()))
}

/// `made` (RFC 3339 UTC, `2026-11-27T03:15:00Z`) as a file's stamp,
/// `20261127T031500Z`.
fn stamp_of(made: &str) -> String {
    made.chars().filter(|c| *c != '-' && *c != ':').collect()
}

/// Saves `deck` (the deck file's text) and `record` as the next version of
/// its format in `dir`: the record's `version` is set to the number taken.
///
/// # Errors
/// When the directory cannot be made, locked or written.
pub fn save(
    dir: &Path,
    format: Format,
    deck: &str,
    record: &mut Record,
) -> std::io::Result<Version> {
    let at = format_dir(dir, format);
    std::fs::create_dir_all(&at)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(at.join(".lock"))?;
    lock.lock()?;
    let number = versions(dir, format).last().map_or(1, |v| v.number + 1);
    record.version = number;
    record.format = format.as_str().to_string();
    let stamp = stamp_of(&record.made);
    let base = at.join(format!("v{number:04}-{stamp}"));
    let version = Version {
        number,
        stamp,
        deck: base.with_extension("txt"),
        record: base.with_extension("json"),
    };
    let text = serde_json::to_string_pretty(record).map_err(std::io::Error::other)?;
    write_whole(&version.record, text.as_bytes())?;
    write_whole(&version.deck, deck.as_bytes())?;
    drop(lock);
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_read_only_as_this_module_writes_it() {
        assert_eq!(
            parse_name("v0007-20261127T031500Z.txt"),
            Some((7, "20261127T031500Z".into()))
        );
        assert_eq!(
            parse_name("v12345-20261127T031500Z.txt"),
            Some((12345, "20261127T031500Z".into()))
        );
        for other in [
            "v7-20261127T031500Z.txt",
            "v0007-20261127T031500Z.json",
            "v0007-2026112T031500Z.txt",
            "v0007-20261127X031500Z.txt",
            "deck.txt",
            ".lock",
        ] {
            assert_eq!(parse_name(other), None, "{other}");
        }
        assert_eq!(stamp_of("2026-11-27T03:15:00Z"), "20261127T031500Z");
    }

    #[test]
    fn versions_count_up_and_the_newest_is_the_highest() {
        let dir = std::env::temp_dir().join(format!("baylee-deck-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(newest(&dir, Format::Standard), None);
        let mut first = Record {
            made: "2026-11-27T03:15:00Z".into(),
            name: "One".into(),
            how: "lend".into(),
            ..Record::default()
        };
        let one = save(&dir, Format::Standard, "60 Forest\n", &mut first).unwrap();
        assert_eq!((one.number, first.version), (1, 1));
        let mut second = Record {
            made: "2026-11-27T03:10:00Z".into(),
            ..first.clone()
        };
        let two = save(&dir, Format::Standard, "60 Island\n", &mut second).unwrap();
        assert_eq!(
            two.number, 2,
            "a number, not the clock, says which is newer"
        );
        assert_eq!(newest(&dir, Format::Standard), Some(two.deck.clone()));
        assert_eq!(newest(&dir, Format::Commander), None, "each format its own");
        assert_eq!(two.read_record().unwrap().format, "standard");
        // A deck without its record is no version.
        std::fs::remove_file(&two.record).unwrap();
        assert_eq!(newest(&dir, Format::Standard), Some(one.deck));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The file each format's games are written to, one JSON line a game, in
/// the format's directory: what `deck improve` reads.
pub const GAMES: &str = "games.jsonl";

/// One ranked game a version played, as its runner writes it after the
/// match ([`record_game`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    /// When it ended, RFC 3339 in UTC.
    pub at: String,
    /// The deck version that played it.
    pub version: u32,
    /// `won`, `lost`, `draw` or `unrated`.
    pub result: String,
    /// Its turns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<u32>,
    /// The account's rating after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elo: Option<i32>,
    /// The rating's change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<i32>,
    /// What the opponent is, where it is public (an AI's label); never a
    /// person's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent: Option<String>,
    /// A short note of the runner's or the pilot's, plain text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Appends `game` to `format`'s games in `dir`, one line, in one write.
///
/// # Errors
/// When the file cannot be opened or written.
pub fn record_game(dir: &Path, format: Format, game: &Game) -> std::io::Result<()> {
    use std::io::Write as _;
    let at = format_dir(dir, format);
    std::fs::create_dir_all(&at)?;
    let mut line = serde_json::to_string(game).map_err(std::io::Error::other)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(at.join(GAMES))?
        .write_all(line.as_bytes())
}

/// `format`'s games in `dir`, oldest first; a line that does not read is
/// passed over.
#[must_use]
pub fn games(dir: &Path, format: Format) -> Vec<Game> {
    std::fs::read_to_string(format_dir(dir, format).join(GAMES)).map_or_default(|text| {
        text.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    })
}

/// The prefixes an AI's public label starts with: Central's for AI
/// accounts, and a mind's own on a table (`crate::Disclosure::prefix`).
const AI_LABELS: [&str; 5] = ["KI-", "LLM-", "NET-", "HOUSE-", "TEST-"];

impl Game {
    /// The game as a model is told it: one line, the opponent named only
    /// when it is an AI by its label. No player is named to a model
    /// (`docs/llm-seat.md`), so a person is "a person".
    #[must_use]
    pub fn told(&self) -> String {
        let against = self.opponent.as_deref().map_or_default(|o| {
            if AI_LABELS.iter().any(|prefix| o.starts_with(prefix)) {
                format!(" · against {o}")
            } else {
                " · against a person".to_string()
            }
        });
        format!(
            "{} · version {} · {}{}{}{against}{}",
            self.at,
            self.version,
            self.result,
            self.turns.map_or_default(|t| format!(" in {t} turns")),
            self.delta.map_or_default(|d| format!(" · Elo {d:+}")),
            self.note.as_deref().map_or_default(|n| format!(" · {n}")),
        )
    }
}
