//! Preconstructed decks: the retail products' lists as Baylee text, and which
//! of them this pool can play (`docs/precons.md`).
//!
//! Two commands and one rule.
//!
//! - `decks-import` reads MTGJSON's deck archive and writes one Baylee-text
//!   file per product under `data/decks/precon/<set>/<slug>.txt`. A card is
//!   matched on its **oracle id** through the ledger
//!   (`baylee_cards_index::ROWS`) and written under the ledger's name, never
//!   the source's: MTGJSON spells a reversible card `Sol Ring // Sol Ring`
//!   and a meld card with the name of the card it melds into, and neither is
//!   a card anywhere else. A card the ledger does not know is written as the
//!   source names it and reported — never dropped.
//! - `decks-status` holds every file against the pool and writes
//!   `data/decks/precon/STATUS.tsv` and the gateway's embedded list of the
//!   playable ones (`crates/baylee-db/src/precons/generated.rs`). A test runs
//!   the same comparison, so neither can drift from the pool.
//! - A deck is **playable** when every card in it passes [`verdict`], which is
//!   also what `deck-check` asks: the card is in the pool, `Implemented`, and
//!   named in the engine's test code (`crate::working`, the trained AI's
//!   rule), and a commander may lead.

use crate::working::{Refusal, Working};
use anyhow::{Context as _, bail};
use baylee_core::deckrow::{self, PrintChoice, Row};
use baylee_core::ids::CardIndex;
use baylee_core::preset::Finish;
use baylee_deckio::{Document, FormatId, Zone};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ sources

/// A place deck lists come from, and the terms they come under.
///
/// Every source states its licence here, beside the code that reads it, so a
/// file written from it can say where it came from and under what — and a
/// source whose terms nobody has read cannot be added without writing them
/// down.
#[derive(Debug)]
pub struct Source {
    /// Short key, as a file's `# source:` line names it.
    pub key: &'static str,
    /// Its own name.
    pub name: &'static str,
    /// Its home page.
    pub home: &'static str,
    /// What is downloaded.
    pub archive: &'static str,
    /// The licence the data comes under.
    pub licence: &'static str,
    /// The notice the licence asks every copy to carry (also in `NOTICE`).
    pub attribution: &'static str,
    /// Where the licence says so, read on the day this entry was written.
    pub terms: &'static str,
}

/// MTGJSON: every retail deck list, MIT-licensed.
///
/// The archive and not the per-deck JSON: `https://mtgjson.com/robots.txt`
/// says `Disallow: /api/v5/*.json` to every agent, which covers
/// `DeckList.json` and `decks/<file>.json`; the archive is not a `.json`
/// path, and one download is also kinder than three thousand.
pub const MTGJSON: Source = Source {
    key: "mtgjson",
    name: "MTGJSON",
    home: "https://mtgjson.com",
    archive: "https://mtgjson.com/api/v5/AllDeckFiles.tar.gz",
    licence: "MIT",
    attribution: "Copyright © 2018 – Present, Zach Halpern",
    terms: "https://mtgjson.com/license/",
};

/// Every source there is. Community sources are researched, not read
/// (`docs/precons.md` §"Community decks").
pub const SOURCES: &[&Source] = &[&MTGJSON];

// -------------------------------------------------------------- deck types

/// What the importer does with one MTGJSON deck type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A real deck, sold or given to be played as it comes.
    Import,
    /// Not a deck somebody sits down with, and why.
    Skip(&'static str),
}

/// Every deck type MTGJSON used on 2026-09-29, and what is done with it.
///
/// A type not in this list is reported and not imported: a new product line
/// gets a decision, not a default. The reasons are the ones
/// `docs/precons.md` gives.
pub const TYPES: &[(&str, Verdict)] = &[
    ("Theme Deck", Verdict::Import),
    ("Intro Pack", Verdict::Import),
    ("Commander Deck", Verdict::Import),
    ("Duel Deck", Verdict::Import),
    ("Planeswalker Deck", Verdict::Import),
    ("Challenger Deck", Verdict::Import),
    ("Pioneer Challenger Deck", Verdict::Import),
    ("Starter Deck", Verdict::Import),
    ("Starter Kit", Verdict::Import),
    ("Spellslinger Starter Kit", Verdict::Import),
    ("Welcome Deck", Verdict::Import),
    ("Sample Deck", Verdict::Import),
    ("Event Deck", Verdict::Import),
    ("Modern Event Deck", Verdict::Import),
    ("World Championship Deck", Verdict::Import),
    ("Pro Tour Deck", Verdict::Import),
    ("Shandalar Enemy Deck", Verdict::Import),
    ("Game Night Deck", Verdict::Import),
    ("Arena Starter Deck", Verdict::Import),
    ("Arena Starter Kit", Verdict::Import),
    ("Arena Promotional Deck", Verdict::Import),
    ("Enhanced Deck", Verdict::Import),
    ("Advanced Deck", Verdict::Import),
    ("MTGO Theme Deck", Verdict::Import),
    ("MTGO Duel Deck", Verdict::Import),
    ("MTGO Commander Deck", Verdict::Import),
    ("Guild Kit", Verdict::Import),
    ("Clash Pack", Verdict::Import),
    ("Premium Deck", Verdict::Import),
    ("Duel Of The Planeswalkers Deck", Verdict::Import),
    ("Brawl Deck", Verdict::Import),
    ("Historic Brawl Precon Deck", Verdict::Import),
    ("Planechase Deck", Verdict::Import),
    ("Archenemy Deck", Verdict::Import),
    (
        "Secret Lair Drop",
        Verdict::Skip("a collector's drop of a few alternate-art cards, not a deck"),
    ),
    (
        "Jumpstart",
        Verdict::Skip("a 20-card half, only a deck once shuffled with another half"),
    ),
    (
        "Halfdeck",
        Verdict::Skip("a 30-card half, only a deck once shuffled with another half"),
    ),
    (
        "MTGO Redemption",
        Verdict::Skip("a whole set redeemed for paper cards, not a deck"),
    ),
    (
        "Deck Builder's Toolkit",
        Verdict::Skip("a box of cards and basic lands to build from, not a deck"),
    ),
    (
        "Bundle Land Pack",
        Verdict::Skip("the basic lands of a bundle, not a deck"),
    ),
    (
        "Box Set",
        Verdict::Skip("a collection (land sets, anniversary and gift boxes), not a deck"),
    ),
    (
        "Welcome Booster",
        Verdict::Skip("a booster of ten cards, not a deck"),
    ),
    (
        "Advanced Pack",
        Verdict::Skip("an add-on pack of a few cards, not a deck"),
    ),
    (
        "Demo Deck",
        Verdict::Skip("a scripted tutorial of a dozen cards, not a deck"),
    ),
    (
        "San Diego Comic Con Promos",
        Verdict::Skip("a set of promotional cards, not a deck"),
    ),
    (
        "Challenge Deck",
        Verdict::Skip("an automated opponent with its own rules, not a player's deck"),
    ),
    (
        "Dandan Deck",
        Verdict::Skip("one library two players share, a variant this engine does not play"),
    ),
    ("Enemy Deck", Verdict::Skip("an empty list")),
];

/// What the importer does with a deck of this type; `None` for a type
/// nobody has decided about yet.
#[must_use]
pub fn verdict_for(kind: &str) -> Option<Verdict> {
    TYPES.iter().find(|(k, _)| *k == kind).map(|(_, v)| *v)
}

// ------------------------------------------------------------------- files

/// Where the files go, from the repository root.
pub const DECKS_DIR: &str = "data/decks/precon";
/// The status table, from the repository root.
pub const STATUS_FILE: &str = "data/decks/precon/STATUS.tsv";
/// The gateway's list of the playable decks, from the repository root.
pub const EMBED_FILE: &str = "crates/baylee-db/src/precons/generated.rs";
/// Where the archive is kept between runs (gitignored).
pub const ARCHIVE_CACHE: &str = "data/mtgjson-cache/AllDeckFiles.tar.gz";

/// The line that makes a file the importer's own: only a file carrying it is
/// ever rewritten or removed, so a hand-placed list in the same directory is
/// never touched.
pub const MARKER: &str = "# written by `cargo run -p xtask -- decks-import`; do not edit by hand";

/// The line that names the source build, which is the one line a re-import of
/// an unchanged list may differ in. A file whose every other line is the same
/// is left as it is, so a new MTGJSON build rewrites only the lists it
/// changed.
const SOURCE_LINE: &str = "# source: ";

/// The command that brings the status back in step.
const REGENERATE: &str = "cargo run -p xtask -- decks-status";

/// Windows refuses these as file or directory names in any case, and `CON`
/// is Conflux's set code.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The directory a set's decks live in: its code, with a trailing `_` for a
/// code Windows cannot hold as a name.
#[must_use]
pub fn set_dir(code: &str) -> String {
    let code = code.to_ascii_uppercase();
    if RESERVED.contains(&code.as_str()) {
        format!("{code}_")
    } else {
        code
    }
}

/// A deck name as a file name: lower-case ASCII letters and digits, runs of
/// anything else as one `-`. Accented letters keep their base letter.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.chars().flat_map(fold) {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            dash = true;
        }
    }
    out
}

/// The base letter of the accented letters deck names use, as a string so
/// `ß` can be two.
fn fold(c: char) -> std::vec::IntoIter<char> {
    let base: &str = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' | 'č' | 'Č' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
        'ý' | 'ÿ' | 'Ý' => "y",
        'š' | 'Š' => "s",
        'ž' | 'Ž' => "z",
        'ß' => "ss",
        'æ' | 'Æ' => "ae",
        other => return vec![other].into_iter(),
    };
    base.chars().collect::<Vec<_>>().into_iter()
}

// ------------------------------------------------------------------ import

/// The ledger by oracle id: every card there is.
pub struct Ledger(HashMap<&'static str, &'static baylee_cards_index::Row>);

impl Ledger {
    /// The whole ledger.
    #[must_use]
    pub fn new() -> Self {
        Self(
            baylee_cards_index::ROWS
                .iter()
                .map(|row| (row.oracle_id, row))
                .collect(),
        )
    }

    /// The ledger row for an oracle id.
    #[must_use]
    pub fn get(&self, oracle_id: &str) -> Option<&'static baylee_cards_index::Row> {
        self.0.get(oracle_id).copied()
    }
}

/// A card the ledger does not know, as the source named it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unknown {
    /// The source's name for it.
    pub name: String,
    /// Its oracle id, or empty when the source gave none.
    pub oracle_id: String,
}

/// One deck, converted.
#[derive(Debug)]
pub struct Converted {
    /// `<set dir>/<slug>`: the file's path under [`DECKS_DIR`], without
    /// `.txt`, and the key the gateway syncs it under.
    pub key: String,
    /// The deck type.
    pub kind: String,
    /// The file.
    pub text: String,
    /// Cards the ledger does not know, in the order they appear.
    pub unknown: Vec<Unknown>,
    /// Rows whose printing (set, number or language) could not be written
    /// back exactly and was left out, so the row names the card alone.
    pub printings_dropped: usize,
}

/// A deck's JSON field as a string, or a message naming the field.
fn text_field<'a>(value: &'a Value, field: &str, file: &str) -> anyhow::Result<&'a str> {
    value[field]
        .as_str()
        .with_context(|| format!("{file}: no `{field}`"))
}

/// The Scryfall language code of an MTGJSON language name; `None` for
/// English, which a row does not write, and for a name this does not know.
fn lang_code(language: &str) -> Option<&'static str> {
    Some(match language {
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "Italian" => "it",
        "Portuguese (Brazil)" | "Portuguese" => "pt",
        "Japanese" => "ja",
        "Korean" => "ko",
        "Russian" => "ru",
        "Chinese Simplified" => "zhs",
        "Chinese Traditional" => "zht",
        "Phyrexian" => "ph",
        _ => return None,
    })
}

/// Whether a row reads back as itself.
fn round_trips(row: &Row) -> bool {
    deckrow::parse(&row.to_string()).is_ok_and(|back| back == *row)
}

/// One MTGJSON card entry as a deck row.
///
/// The name is the ledger's for the entry's oracle id. The printing is the
/// entry's own, as far as the row grammar can say it back exactly: a row
/// that would not round-trip loses its language, then its set and number,
/// and says so through `dropped`.
fn card_row(
    card: &Value,
    ledger: &Ledger,
    unknown: &mut Vec<Unknown>,
    dropped: &mut usize,
    file: &str,
) -> anyhow::Result<Row> {
    let count = card["count"]
        .as_u64()
        .and_then(|c| u32::try_from(c).ok())
        .filter(|c| (1..=deckrow::MAX_COUNT).contains(c))
        .with_context(|| format!("{file}: a card with no count"))?;
    let oracle_id = card["identifiers"]["scryfallOracleId"]
        .as_str()
        .unwrap_or_default();
    let name = if let Some(row) = ledger.get(oracle_id) {
        row.name.to_string()
    } else {
        let name = text_field(card, "name", file)?.to_string();
        let missing = Unknown {
            name: name.clone(),
            oracle_id: oracle_id.to_string(),
        };
        if !unknown.contains(&missing) {
            unknown.push(missing);
        }
        name
    };
    let finish = if card["isEtched"].as_bool() == Some(true) {
        Some(Finish::Etched)
    } else if card["isFoil"].as_bool() == Some(true) {
        Some(Finish::Foil)
    } else {
        None
    };
    let print = PrintChoice {
        set: card["setCode"].as_str().map(str::to_ascii_uppercase),
        collector_number: card["number"].as_str().map(str::to_string),
        lang: card["language"]
            .as_str()
            .and_then(lang_code)
            .map(str::to_string),
        finish,
        scryfall_id: None,
    };
    let mut row = Row {
        count,
        name,
        print,
        note: None,
    };
    if round_trips(&row) {
        return Ok(row);
    }
    *dropped += 1;
    row.print.lang = None;
    if round_trips(&row) {
        return Ok(row);
    }
    row.print.set = None;
    row.print.collector_number = None;
    if round_trips(&row) {
        return Ok(row);
    }
    row.print.finish = None;
    if round_trips(&row) {
        return Ok(row);
    }
    bail!("{file}: `{}` cannot be written as a deck row", row.name)
}

/// One board's rows, identical rows merged and the rest in a stable order.
///
/// Sorted rather than in the source's order, so that a source reordering a
/// list changes no file: by name, then printing.
fn board(
    cards: &Value,
    ledger: &Ledger,
    unknown: &mut Vec<Unknown>,
    dropped: &mut usize,
    file: &str,
) -> anyhow::Result<Vec<Row>> {
    let mut rows: Vec<Row> = Vec::new();
    for card in cards.as_array().map(Vec::as_slice).unwrap_or_default() {
        let row = card_row(card, ledger, unknown, dropped, file)?;
        match rows
            .iter_mut()
            .find(|r| r.name == row.name && r.print == row.print)
        {
            Some(same) => same.count += row.count,
            None => rows.push(row),
        }
    }
    rows.sort_by(|a, b| {
        let key = |r: &Row| {
            (
                r.name.clone(),
                r.print.set.clone(),
                r.print.collector_number.clone(),
                r.print.lang.clone(),
                r.print.finish.map(|f| f as u8),
            )
        };
        key(a).cmp(&key(b))
    });
    Ok(rows)
}

/// Converts one MTGJSON deck file. `None` for a type the importer skips or
/// has no decision for; `kinds` counts what was seen either way.
///
/// # Errors
/// A file that is not a deck, or a card that cannot be written as a row.
pub fn convert(
    file: &str,
    json: &Value,
    ledger: &Ledger,
    kinds: &mut BTreeMap<String, usize>,
) -> anyhow::Result<Option<Converted>> {
    let version = text_field(&json["meta"], "version", file)?;
    let deck = &json["data"];
    let kind = text_field(deck, "type", file)?;
    *kinds.entry(kind.to_string()).or_default() += 1;
    if verdict_for(kind) != Some(Verdict::Import) {
        return Ok(None);
    }
    let name = text_field(deck, "name", file)?.trim().to_string();
    let code = text_field(deck, "code", file)?;
    let released = text_field(deck, "releaseDate", file)?;

    let mut unknown = Vec::new();
    let mut dropped = 0;
    let leaders = board(&deck["commander"], ledger, &mut unknown, &mut dropped, file)?;
    let main = board(&deck["mainBoard"], ledger, &mut unknown, &mut dropped, file)?;
    let side = board(&deck["sideBoard"], ledger, &mut unknown, &mut dropped, file)?;
    if main.is_empty() && leaders.is_empty() {
        bail!("{file}: an empty deck");
    }
    let names: Vec<String> = leaders.iter().map(|r| r.name.clone()).collect();

    let mut text = String::from("# baylee deck export v1\n");
    let _ = writeln!(text, "# name: {name}");
    let _ = writeln!(text, "# format: {}", baylee_cards::decks::format_of(&names));
    let _ = writeln!(
        text,
        "{SOURCE_LINE}{} {version}, {}, {file}",
        MTGJSON.name, MTGJSON.home
    );
    let _ = writeln!(
        text,
        "# licence: {} ({}); see NOTICE",
        MTGJSON.licence, MTGJSON.attribution
    );
    let _ = writeln!(text, "# type: {kind}");
    let _ = writeln!(text, "# set: {}", code.to_ascii_uppercase());
    let _ = writeln!(text, "# released: {released}");
    let _ = writeln!(text, "{MARKER}");
    for row in &leaders {
        let _ = writeln!(text, "CMD: {row}");
    }
    for row in &main {
        let _ = writeln!(text, "{row}");
    }
    for row in &side {
        let _ = writeln!(text, "SB: {row}");
    }

    let slug = slug(&name);
    if slug.is_empty() {
        bail!("{file}: `{name}` makes no file name");
    }
    Ok(Some(Converted {
        key: format!("{}/{slug}", set_dir(code)),
        kind: kind.to_string(),
        text,
        unknown,
        printings_dropped: dropped,
    }))
}

/// A file without its source line, which is what "the same list" compares.
fn without_source(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with(SOURCE_LINE))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `.txt` under the decks directory, relative to it, sorted.
fn deck_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for set in fs::read_dir(dir)? {
        let set = set?.path();
        if !set.is_dir() {
            continue;
        }
        for file in fs::read_dir(&set)? {
            let file = file?.path();
            if file.extension().is_some_and(|e| e == "txt") {
                out.push(file.strip_prefix(dir).unwrap_or(&file).to_path_buf());
            }
        }
    }
    out.sort();
    Ok(out)
}

/// A relative deck path as its key: `E02/sun-empire`.
fn key_of(relative: &Path) -> String {
    relative
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

/// What writing the converted decks did to the directory.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Written {
    /// Files created or rewritten.
    pub changed: usize,
    /// Files already holding the same list.
    pub unchanged: usize,
    /// The importer's own files no deck produced any more.
    pub removed: usize,
}

/// Writes the decks under `dir`, leaving alone every file that already holds
/// the same list and every file the importer did not write.
///
/// # Errors
/// When the directory cannot be read or written.
pub fn write_decks(dir: &Path, decks: &BTreeMap<String, String>) -> anyhow::Result<Written> {
    let mut written = Written::default();
    for (key, text) in decks {
        let path = dir.join(format!("{key}.txt"));
        match fs::read_to_string(&path) {
            Ok(old) if without_source(&old) == without_source(text) => written.unchanged += 1,
            _ => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
                written.changed += 1;
            }
        }
    }
    for relative in deck_files(dir)? {
        if decks.contains_key(&key_of(&relative)) {
            continue;
        }
        let path = dir.join(&relative);
        if fs::read_to_string(&path).is_ok_and(|text| text.lines().any(|l| l == MARKER)) {
            fs::remove_file(&path)?;
            written.removed += 1;
            if let Some(parent) = path.parent()
                && fs::read_dir(parent).is_ok_and(|mut d| d.next().is_none())
            {
                fs::remove_dir(parent)?;
            }
        }
    }
    Ok(written)
}

/// Downloads the archive to `path`, through a temporary file so an
/// interrupted download never looks like a finished one.
fn download(url: &str, path: &Path) -> anyhow::Result<()> {
    println!("decks-import: fetching {url}");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let agent = ureq::Agent::new_with_defaults();
    let response = agent
        .get(url)
        .header(
            "User-Agent",
            "baylee-xtask/0.1 (non-commercial fan project; decks-import)",
        )
        .call()
        .with_context(|| format!("fetching {url}"))?;
    let partial = path.with_extension("part");
    let mut out = fs::File::create(&partial)?;
    std::io::copy(&mut response.into_body().into_reader(), &mut out)?;
    fs::rename(&partial, path)?;
    Ok(())
}

/// Every `.json` in the archive, as `(file name, bytes)`, in name order.
fn read_archive(path: &Path) -> anyhow::Result<BTreeMap<String, Vec<u8>>> {
    let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut out = BTreeMap::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry
            .path()?
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !Path::new(&name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        out.insert(name, bytes);
    }
    Ok(out)
}

/// `xtask decks-import`: the archive to files, then the status.
///
/// # Errors
/// When the archive cannot be read, a deck cannot be converted, or two decks
/// would share a file.
pub fn import(root: &Path, archive: Option<&Path>, refresh: bool) -> anyhow::Result<()> {
    let path = archive.map_or_else(|| root.join(ARCHIVE_CACHE), Path::to_path_buf);
    for source in SOURCES {
        println!(
            "decks-import: source `{}`: {} ({}), {}, terms {}",
            source.key, source.name, source.home, source.licence, source.terms
        );
    }
    if archive.is_none() && (refresh || !path.exists()) {
        download(MTGJSON.archive, &path)?;
    }
    let ledger = Ledger::new();
    let mut kinds = BTreeMap::new();
    let mut imported: BTreeMap<String, usize> = BTreeMap::new();
    let mut decks: BTreeMap<String, String> = BTreeMap::new();
    let mut unknown: BTreeMap<Unknown, Vec<String>> = BTreeMap::new();
    let mut dropped = 0;
    let mut version = String::new();
    for (file, bytes) in read_archive(&path)? {
        let json: Value =
            serde_json::from_slice(&bytes).with_context(|| format!("{file}: not JSON"))?;
        if version.is_empty() {
            version = json["meta"]["version"].as_str().unwrap_or("?").to_string();
        }
        let Some(deck) = convert(&file, &json, &ledger, &mut kinds)? else {
            continue;
        };
        *imported.entry(deck.kind.clone()).or_default() += 1;
        dropped += deck.printings_dropped;
        for card in deck.unknown {
            unknown.entry(card).or_default().push(deck.key.clone());
        }
        if decks.insert(deck.key.clone(), deck.text).is_some() {
            bail!("{file}: a second deck at {}", deck.key);
        }
    }
    let written = write_decks(&root.join(DECKS_DIR), &decks)?;

    println!("decks-import: {} (MTGJSON {version})", path.display());
    println!("  imported, by type:");
    for (kind, n) in &imported {
        println!("    {n:5}  {kind}");
    }
    println!("  skipped, by type:");
    for (kind, n) in &kinds {
        if let Some(Verdict::Skip(why)) = verdict_for(kind) {
            println!("    {n:5}  {kind} — {why}");
        }
    }
    let undecided: Vec<_> = kinds
        .iter()
        .filter(|(kind, _)| verdict_for(kind).is_none())
        .collect();
    for (kind, n) in &undecided {
        println!("    {n:5}  {kind} — NO DECISION: add it to precons::TYPES");
    }
    println!(
        "  {} decks: {} written, {} unchanged, {} removed; {dropped} rows without their printing",
        decks.len(),
        written.changed,
        written.unchanged,
        written.removed
    );
    let affected: BTreeSet<&String> = unknown.values().flatten().collect();
    println!(
        "  {} cards missing from the ledger, in {} decks (written as the source names them):",
        unknown.len(),
        affected.len()
    );
    for (card, keys) in &unknown {
        println!(
            "    {} ({}) in {}",
            card.name,
            if card.oracle_id.is_empty() {
                "no oracle id"
            } else {
                &card.oracle_id
            },
            keys.join(", ")
        );
    }
    status(root, false)
}

// ------------------------------------------------------------------ status

/// Why a card keeps a deck from being played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// No card of this name exists (the ledger has no row for it).
    NotInLedger,
    /// A real card this pool compiles nothing for.
    NotInPool,
    /// In the pool, not `Coverage::Implemented`.
    NotImplemented,
    /// Implemented, and no engine test names it.
    Untested,
    /// It leads the deck and may not (CR 903.3), or not beside the other.
    NoLeader,
}

impl Why {
    /// The word the status table writes.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::NotInLedger => "ledger",
            Self::NotInPool => "pool",
            Self::NotImplemented => "stub",
            Self::Untested => "untested",
            Self::NoLeader => "leader",
        }
    }
}

/// Whether this pool plays a card, by the name a deck row writes: the one
/// predicate `deck-check` and the precon status share.
///
/// # Errors
/// The first half of the rule the card fails.
pub fn verdict(name: &str, working: &Working) -> Result<CardIndex, Why> {
    let Some(card) = baylee_cards::decks::by_name(name) else {
        return Err(if baylee_cards_index::row_by_name(name).is_some() {
            Why::NotInPool
        } else {
            Why::NotInLedger
        });
    };
    working
        .check(card)
        .map(|()| card)
        .map_err(|refusal| match refusal {
            Refusal::NotInPool => Why::NotInPool,
            Refusal::NotImplemented => Why::NotImplemented,
            Refusal::Untested => Why::Untested,
        })
}

/// How many failing cards a line names per deck.
const FIRST: usize = 3;

/// How many of the nearest unplayable decks `decks-status` prints.
const NEAREST: usize = 10;

/// One line of the status table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// `<set dir>/<slug>`.
    pub key: String,
    /// Set code.
    pub set: String,
    /// Deck type.
    pub kind: String,
    /// Release date, `YYYY-MM-DD`.
    pub released: String,
    /// The deck's name.
    pub name: String,
    /// Cards in it, commanders and sideboard included.
    pub cards: u32,
    /// Every card passes [`verdict`] and every leader may lead.
    pub playable: bool,
    /// Distinct cards that do not, in the order the file lists them, each
    /// with why.
    pub failing: Vec<(String, Why)>,
}

/// A `# key: value` header line's value.
fn header<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find_map(|line| {
            line.strip_prefix('#')
                .map(str::trim_start)
                .and_then(|l| l.strip_prefix(key))
                .and_then(|l| l.strip_prefix(':'))
        })
        .map_or("", str::trim)
}

/// Reads one precon file as the builder's import would.
///
/// # Errors
/// A file the Baylee-text reader refuses, or one with a line it skips.
pub fn read_deck(text: &str, what: &str) -> anyhow::Result<Document> {
    let read = baylee_deckio::format::read(FormatId::Baylee, text)
        .map_err(|e| anyhow::anyhow!("{what}: {e}"))?;
    if let Some(skipped) = read.skipped.first() {
        bail!(
            "{what}: line {} is not a row: {}",
            skipped.line,
            skipped.text
        );
    }
    Ok(read.document)
}

/// Judges one deck file.
///
/// # Errors
/// A file that does not read.
pub fn judge(key: &str, text: &str, working: &Working) -> anyhow::Result<Status> {
    let document = read_deck(text, key)?;
    let mut failing: Vec<(String, Why)> = Vec::new();
    let mut fail = |name: &str, why: Why| {
        if !failing.iter().any(|(n, _)| n == name) {
            failing.push((name.to_string(), why));
        }
    };
    let mut cards = 0;
    let mut leaders = Vec::new();
    for card in &document.cards {
        cards += card.count;
        match verdict(&card.name, working) {
            Ok(index) if card.zone == Zone::Commander => leaders.push((card.name.clone(), index)),
            Ok(_) => {}
            Err(why) => fail(&card.name, why),
        }
    }
    let leads = |index| baylee_cards::decks::leader_of(index);
    match leaders.as_slice() {
        [] => {}
        [(name, one)] => {
            if !leads(*one).is_some_and(|l| l.eligible) {
                fail(name, Why::NoLeader);
            }
        }
        [(a_name, a), (b_name, b)] => {
            let pair = leads(*a)
                .zip(leads(*b))
                .is_some_and(|(a, b)| baylee_cards::decks::may_lead_together(&a, &b));
            if !pair {
                fail(a_name, Why::NoLeader);
                fail(b_name, Why::NoLeader);
            }
        }
        more => {
            for (name, _) in more {
                fail(name, Why::NoLeader);
            }
        }
    }
    Ok(Status {
        key: key.to_string(),
        set: header(text, "set").to_string(),
        kind: header(text, "type").to_string(),
        released: header(text, "released").to_string(),
        name: document.name.clone().unwrap_or_default(),
        cards,
        playable: failing.is_empty(),
        failing,
    })
}

/// The status of every deck under `dir`, in key order.
///
/// # Errors
/// A file that cannot be read or does not read as a deck.
pub fn statuses(dir: &Path, working: &Working) -> anyhow::Result<Vec<Status>> {
    let mut out = Vec::new();
    for relative in deck_files(dir)? {
        let text = fs::read_to_string(dir.join(&relative))?;
        out.push(judge(&key_of(&relative), &text, working)?);
    }
    Ok(out)
}

/// The status table: a header and one line per deck, tab-separated.
///
/// Every column but `playable` comes from the deck's own file, and
/// `playable` moves only when a deck's offer does. That is deliberate: which
/// cards a deck still lacks moves with nearly every card commit (142 of the
/// 328 commits on main from 27.09. to 30.09.2026 touched a card or a card
/// test), and a committed table that CI holds to the pool would then
/// ask every one of those commits, and every branch merging past them, for
/// a regenerated file. What is missing is printed by `decks-status` and
/// `deck-check` instead, where it is read.
#[must_use]
pub fn render_status(rows: &[Status]) -> String {
    let mut out = String::from("deck\tset\ttype\treleased\tname\tcards\tplayable\n");
    for row in rows {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.key,
            row.set,
            row.kind,
            row.released,
            row.name,
            row.cards,
            if row.playable { "yes" } else { "no" },
        );
    }
    out
}

/// A deck's missing cards as a line: how many, then the first few with why.
fn missing(row: &Status) -> String {
    let first: Vec<String> = row
        .failing
        .iter()
        .take(FIRST)
        .map(|(name, why)| format!("{name} ({})", why.word()))
        .collect();
    let more = row.failing.len().saturating_sub(FIRST);
    if more == 0 {
        first.join("; ")
    } else {
        format!("{}; and {more} more", first.join("; "))
    }
}

/// The gateway's list: every playable deck, embedded by path.
#[must_use]
pub fn render_embed(rows: &[Status]) -> String {
    let mut out = String::from(
        "// GENERATED by `cargo run -p xtask -- decks-status` — do not edit by hand.\n\
         //\n\
         // Every preconstructed deck in `data/decks/precon/STATUS.tsv` marked\n\
         // playable, embedded so the gateway needs no file beside it. The test\n\
         // `the_precon_status_is_the_pools` in xtask holds this to the pool.\n\
         \n\
         use super::Precon;\n\
         \n\
         /// The decks the house offers, in key order.\n\
         #[rustfmt::skip]\n\
         pub static PLAYABLE: &[Precon] = &[\n",
    );
    for row in rows.iter().filter(|r| r.playable) {
        let _ = writeln!(
            out,
            "    Precon {{ key: {:?}, text: include_str!(\"../../../../{DECKS_DIR}/{}.txt\") }},",
            row.key, row.key
        );
    }
    out.push_str("];\n");
    out
}

/// Where the first difference between two texts is, as a sentence.
fn first_difference(what: &str, committed: &str, current: &str) -> String {
    let line = committed
        .lines()
        .zip(current.lines())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| committed.lines().count().min(current.lines().count()));
    format!(
        "{what} is not what the pool says (first difference at line {}):\n  committed: {}\n  current:   {}\n\
         Run `{REGENERATE}` and commit the result.",
        line + 1,
        committed.lines().nth(line).unwrap_or("<end of file>"),
        current.lines().nth(line).unwrap_or("<end of file>"),
    )
}

/// Compares a committed file with what it should hold.
///
/// # Errors
/// A sentence naming the file, the first line that differs, and the command
/// that fixes it.
pub fn compare(what: &str, committed: Option<&str>, current: &str) -> Result<(), String> {
    match committed {
        Some(committed) if committed == current => Ok(()),
        Some(committed) => Err(first_difference(what, committed, current)),
        None => Err(format!(
            "{what} is missing. Run `{REGENERATE}` and commit the result."
        )),
    }
}

/// The two files `decks-status` writes, as `(path, text)`, for the decks
/// under `dir`.
///
/// # Errors
/// A deck file that does not read.
pub fn outputs(
    decks_dir: &Path,
    status_file: &Path,
    embed_file: &Path,
    working: &Working,
) -> anyhow::Result<[(PathBuf, String); 2]> {
    let rows = statuses(decks_dir, working)?;
    Ok([
        (status_file.to_path_buf(), render_status(&rows)),
        (embed_file.to_path_buf(), render_embed(&rows)),
    ])
}

/// Holds both files against the pool, writing nothing.
///
/// # Errors
/// Every file that differs, each with its first difference.
pub fn check(
    decks_dir: &Path,
    status_file: &Path,
    embed_file: &Path,
    working: &Working,
) -> anyhow::Result<()> {
    let mut problems = Vec::new();
    for (path, current) in outputs(decks_dir, status_file, embed_file, working)? {
        let committed = fs::read_to_string(&path).ok();
        if let Err(problem) = compare(&path.display().to_string(), committed.as_deref(), &current) {
            problems.push(problem);
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        bail!("{}", problems.join("\n\n"))
    }
}

/// `xtask decks-status [--check]`.
///
/// # Errors
/// With `--check`, a file that differs; otherwise a deck that does not read.
pub fn status(root: &Path, check_only: bool) -> anyhow::Result<()> {
    let working = Working::scan(root).context("reading the engine's test code")?;
    let (decks, status_file, embed) = (
        root.join(DECKS_DIR),
        root.join(STATUS_FILE),
        root.join(EMBED_FILE),
    );
    if check_only {
        check(&decks, &status_file, &embed, &working)?;
        println!("decks-status: both files are current");
        return Ok(());
    }
    let rows = statuses(&decks, &working)?;
    fs::write(&status_file, render_status(&rows))?;
    fs::write(&embed, render_embed(&rows))?;
    let playable: Vec<&Status> = rows.iter().filter(|r| r.playable).collect();
    let mut by_kind: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for row in &rows {
        let entry = by_kind.entry(&row.kind).or_default();
        entry.0 += 1;
        entry.1 += usize::from(row.playable);
    }
    println!(
        "decks-status: {} decks, {} playable (the pool: {} cards, {} implemented, {} of them \
         tested; working set {}; {} tested ids name no pool card)",
        rows.len(),
        playable.len(),
        working.pool,
        working.implemented,
        working.cards.len(),
        &working.hash()[..12],
        working.unknown.len(),
    );
    for (kind, (all, open)) in &by_kind {
        println!("    {open:4} of {all:4}  {kind}");
    }
    for row in &playable {
        println!("  PLAYABLE {} — {}", row.key, row.name);
    }
    // Why the rest are not, counted once per card and deck, and the decks
    // that are nearest: what the next cards to write would unlock.
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, why) in rows.iter().flat_map(|r| &r.failing) {
        *reasons.entry(why.word()).or_default() += 1;
    }
    let reasons: Vec<String> = reasons.iter().map(|(w, n)| format!("{n} {w}")).collect();
    println!("  missing cards, per deck: {}", reasons.join(", "));
    let mut nearest: Vec<&Status> = rows.iter().filter(|r| !r.playable).collect();
    nearest.sort_by_key(|r| (r.failing.len(), r.key.clone()));
    for row in nearest.iter().take(NEAREST) {
        println!("  NEAR {} — {}: {}", row.key, row.name, missing(row));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Working {
        Working::scan(&crate::working::repo_root()).expect("the engine's sources read")
    }

    /// A card entry as MTGJSON writes one, reduced to what is read.
    fn entry(name: &str, oracle_id: &str, count: u64, set: &str, number: &str) -> Value {
        serde_json::json!({
            "count": count,
            "name": name,
            "setCode": set,
            "number": number,
            "language": "English",
            "isFoil": false,
            "identifiers": { "scryfallOracleId": oracle_id },
        })
    }

    fn deck(kind: &str, main: &[Value], commander: &[Value]) -> Value {
        serde_json::json!({
            "meta": { "date": "2026-09-29", "version": "5.3.0+20260929" },
            "data": {
                "code": "tst",
                "name": "Tëst Deck: One",
                "type": kind,
                "releaseDate": "1996-05-01",
                "mainBoard": main,
                "sideBoard": [],
                "commander": commander,
            }
        })
    }

    fn oracle_of(name: &str) -> &'static str {
        baylee_cards_index::row_by_name(name)
            .expect("a real card")
            .oracle_id
    }

    /// **A card is its oracle id, not the name the source prints.** MTGJSON
    /// names a reversible card `Sol Ring // Sol Ring`, which is no card in
    /// the ledger, the pool or a player's deck; the row is written under the
    /// ledger's name for the id, and the source's spelling is never read.
    #[test]
    fn a_card_is_matched_on_its_oracle_id_and_written_under_the_ledgers_name() {
        let ledger = Ledger::new();
        let sol_ring = oracle_of("Sol Ring");
        let json = deck(
            "Commander Deck",
            &[
                entry("Sol Ring // Sol Ring", sol_ring, 1, "sld", "1011"),
                entry(
                    "A Name MTGJSON Invented",
                    oracle_of("Island"),
                    30,
                    "tst",
                    "7",
                ),
            ],
            &[],
        );
        let converted = convert("Test_TST.json", &json, &ledger, &mut BTreeMap::new())
            .expect("converts")
            .expect("an imported type");
        assert!(
            converted.text.contains("\n1 Sol Ring (SLD) 1011\n"),
            "{}",
            converted.text
        );
        assert!(
            converted.text.contains("\n30 Island (TST) 7\n"),
            "{}",
            converted.text
        );
        assert!(!converted.text.contains("Invented"), "{}", converted.text);
        assert!(converted.unknown.is_empty());
        assert_eq!(converted.key, "TST/test-deck-one");
    }

    /// **A card the ledger does not know is written and reported, never
    /// dropped.** The deck keeps its size, the source's name stands in the
    /// row, and the status then says the deck is not playable and why.
    #[test]
    fn a_card_missing_from_the_ledger_is_written_and_reported() {
        let ledger = Ledger::new();
        let stranger = "00000000-0000-4000-8000-000000000000";
        let json = deck(
            "Theme Deck",
            &[
                entry("Card From Next Week", stranger, 2, "tst", "1"),
                entry("Forest", oracle_of("Forest"), 20, "tst", "2"),
            ],
            &[],
        );
        let converted = convert("Test_TST.json", &json, &ledger, &mut BTreeMap::new())
            .expect("converts")
            .expect("an imported type");
        assert_eq!(
            converted.unknown,
            [Unknown {
                name: "Card From Next Week".into(),
                oracle_id: stranger.into()
            }]
        );
        assert!(converted.text.contains("\n2 Card From Next Week (TST) 1\n"));
        let status = judge(&converted.key, &converted.text, &working()).expect("reads");
        assert_eq!(status.cards, 22, "the deck keeps every card");
        assert!(!status.playable);
        assert_eq!(
            status.failing,
            [("Card From Next Week".to_string(), Why::NotInLedger)]
        );
    }

    /// A type nobody decided about is not imported, and one the table skips
    /// is not either; both are still counted, so the report can name them.
    #[test]
    fn only_a_type_the_table_imports_is_written() {
        let ledger = Ledger::new();
        let mut kinds = BTreeMap::new();
        for kind in ["Jumpstart", "Brand New Product Line"] {
            let json = deck(
                kind,
                &[entry("Forest", oracle_of("Forest"), 20, "tst", "2")],
                &[],
            );
            assert!(
                convert("T.json", &json, &ledger, &mut kinds)
                    .expect("reads")
                    .is_none(),
                "{kind}"
            );
        }
        assert_eq!(kinds.len(), 2);
        assert_eq!(verdict_for("Brand New Product Line"), None);
        assert!(matches!(verdict_for("Jumpstart"), Some(Verdict::Skip(_))));
    }

    /// **The unlock predicate.** A deck is playable when every card passes
    /// [`verdict`]: a deck of one card that works is, and the same deck with
    /// a stub beside it is not, naming the stub and why.
    #[test]
    fn a_deck_unlocks_only_when_every_card_passes_the_rule() {
        let working = working();
        let good = baylee_cards::by_index(*working.cards.first().expect("some card works"))
            .expect("in the pool")
            .name()
            .to_string();
        let stub = baylee_cards::all()
            .find(|d| !d.is_implemented())
            .expect("the pool has a stub")
            .name()
            .to_string();
        let text = |rows: &[&str]| {
            format!(
                "# baylee deck export v1\n# name: Probe\n# set: TST\n# type: Theme Deck\n# released: 1996-05-01\n{}\n",
                rows.join("\n")
            )
        };
        let open = judge("TST/probe", &text(&[&format!("4 {good}")]), &working).expect("reads");
        assert!(open.playable, "{open:?}");
        assert_eq!(open.cards, 4);

        let shut = judge(
            "TST/probe",
            &text(&[&format!("4 {good}"), &format!("1 {stub}")]),
            &working,
        )
        .expect("reads");
        assert!(!shut.playable);
        assert_eq!(shut.failing, [(stub.clone(), Why::NotImplemented)]);
        assert_eq!(verdict(&stub, &working), Err(Why::NotImplemented));
        let elsewhere = baylee_cards_index::ROWS
            .iter()
            .find(|row| baylee_cards::decks::by_name(row.name).is_none())
            .expect("the ledger holds cards the pool does not");
        assert_eq!(verdict(elsewhere.name, &working), Err(Why::NotInPool));
        assert_eq!(
            verdict("No Such Card At All", &working),
            Err(Why::NotInLedger)
        );
    }

    /// **The status check fails on drift.** The table is written for a
    /// directory of decks; a card fixed or broken since then changes a line,
    /// and the check names the file, the line and the command.
    #[test]
    fn the_status_check_fails_when_a_deck_changes_under_it() {
        let working = working();
        let stub = baylee_cards::all()
            .find(|d| !d.is_implemented())
            .expect("the pool has a stub")
            .name()
            .to_string();
        let dir = std::env::temp_dir().join(format!("baylee-precons-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let decks = dir.join("decks");
        fs::create_dir_all(decks.join("TST")).expect("a temp dir");
        let (status, embed) = (dir.join("STATUS.tsv"), dir.join("generated.rs"));
        let file = decks.join("TST/probe.txt");
        let deck = |rows: &str| {
            format!(
                "# baylee deck export v1\n# name: Probe\n# set: TST\n# type: Theme Deck\n# released: 1996-05-01\n{rows}"
            )
        };
        let write_both = || {
            for (path, text) in outputs(&decks, &status, &embed, &working).expect("reads") {
                fs::write(path, text).expect("write");
            }
        };
        fs::write(&file, deck("20 Forest\n")).expect("write");
        write_both();
        check(&decks, &status, &embed, &working).expect("current right after writing");
        assert!(
            fs::read_to_string(&status)
                .expect("the table")
                .contains("\t20\tyes\n"),
            "twenty Forests are playable, or the flip below proves nothing"
        );

        // The same twenty cards, one of them a stub: only `playable` moves.
        fs::write(&file, deck(&format!("19 Forest\n1 {stub}\n"))).expect("write");
        let drift = check(&decks, &status, &embed, &working)
            .expect_err("the deck stopped being playable and the table did not say so")
            .to_string();
        assert!(drift.contains("STATUS.tsv"), "{drift}");
        assert!(drift.contains("TST/probe"), "{drift}");
        assert!(drift.contains("\t20\tno"), "the line that moved: {drift}");
        assert!(drift.contains(REGENERATE), "{drift}");
        assert!(
            drift.contains("generated.rs"),
            "and the embedded list: {drift}"
        );

        // The embedded list alone, edited by hand, is caught as well.
        write_both();
        check(&decks, &status, &embed, &working).expect("current again");
        let edited = fs::read_to_string(&embed).expect("the list").replace(
            "];",
            "    Precon { key: \"TST/sneaked-in\", text: \"\" },\n];",
        );
        fs::write(&embed, edited).expect("write");
        let drift = check(&decks, &status, &embed, &working)
            .expect_err("a hand-edited list")
            .to_string();
        assert!(drift.contains("generated.rs"), "{drift}");
        assert!(!drift.contains("STATUS.tsv"), "{drift}");
        let _ = fs::remove_dir_all(&dir);
    }

    /// **CI's half of "cannot drift":** the committed table and the gateway's
    /// list are what the pool and the engine's tests say today.
    #[test]
    fn the_precon_status_is_the_pools() {
        let root = crate::working::repo_root();
        check(
            &root.join(DECKS_DIR),
            &root.join(STATUS_FILE),
            &root.join(EMBED_FILE),
            &working(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn a_set_code_windows_cannot_hold_gets_a_trailing_underscore() {
        assert_eq!(set_dir("con"), "CON_");
        assert_eq!(set_dir("E02"), "E02");
        assert_eq!(
            slug("Jakub Šlemr - Mono-Black Control"),
            "jakub-slemr-mono-black-control"
        );
        assert_eq!(
            slug("Angels: They're Just Like Us"),
            "angels-they-re-just-like-us"
        );
    }

    /// A row the grammar cannot say back loses the part it cannot say, and
    /// is counted, rather than being written wrong.
    #[test]
    fn a_printing_the_row_cannot_hold_is_left_out_and_counted() {
        let ledger = Ledger::new();
        let mut unknown = Vec::new();
        let mut dropped = 0;
        let card = entry("Forest", oracle_of("Forest"), 1, "pmps06", "1");
        let row = card_row(&card, &ledger, &mut unknown, &mut dropped, "T").expect("a row");
        assert_eq!(row.to_string(), "1 Forest");
        assert_eq!(dropped, 1);
    }

    /// Re-importing the same list from a newer build leaves the file as it
    /// is; only the importer's own files are ever removed.
    #[test]
    fn an_unchanged_list_is_not_rewritten_and_a_hand_placed_file_is_kept() {
        let dir = std::env::temp_dir().join(format!("baylee-precon-write-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let body =
            format!("# baylee deck export v1\n{SOURCE_LINE}MTGJSON 1\n{MARKER}\n20 Forest\n");
        let decks = BTreeMap::from([("TST/a".to_string(), body.clone())]);
        assert_eq!(write_decks(&dir, &decks).expect("writes").changed, 1);
        let newer = BTreeMap::from([("TST/a".to_string(), body.replace("MTGJSON 1", "MTGJSON 2"))]);
        assert_eq!(write_decks(&dir, &newer).expect("writes").unchanged, 1);
        assert!(
            fs::read_to_string(dir.join("TST/a.txt"))
                .unwrap()
                .contains("MTGJSON 1")
        );

        fs::write(dir.join("TST/mine.txt"), "20 Island\n").expect("write");
        let gone = write_decks(&dir, &BTreeMap::new()).expect("writes");
        assert_eq!(gone.removed, 1);
        assert!(
            dir.join("TST/mine.txt").exists(),
            "a file without the marker stays"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
