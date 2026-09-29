//! Import and export in the deck builder: the decisions, none of the
//! drawing.
//!
//! Formats, detection and URL sources are `baylee_deckio`'s; this is what the
//! builder does with them. An import is read in two steps — the paste is
//! *understood* first (which format, how many rows, which lines were not
//! rows) and taken only when the player confirms — and taking it goes
//! through the same held-rows path a stored deck is loaded through
//! ([`DeckBuilder::load`]), so an imported deck resolves against the pool
//! exactly as a saved one does, the pool may still be on its way, and a card
//! the pool does not have lands in [`DeckBuilder::missing`] where the save
//! refuses over it rather than disappearing.
//!
//! Export is computed from the deck as it stands, every time it is asked:
//! it is a few hundred rows of string formatting, and a cached copy is one
//! that can disagree with the list beside it.

use super::{DeckBuilder, Zone};
use crate::i18n::{Lang, Phrase};
use baylee_core::deckrow::Row;
use baylee_deckio::document::CardError;
use baylee_deckio::source::{Answer, Instruction, SourceId};
use baylee_deckio::{Document, Import, LossKind, Read, ReadError};
pub use baylee_deckio::{FormatId, Written};

/// The import or export dialog, whichever is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transfer {
    /// Importing a deck.
    Import(Importing),
    /// Exporting the deck.
    Export(Exporting),
}

/// The import dialog.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Importing {
    /// What was pasted, as it was pasted.
    pasted: String,
    /// What it turned out to be.
    stage: Stage,
}

impl Importing {
    /// The pasted text.
    #[must_use]
    pub fn pasted(&self) -> &str {
        &self.pasted
    }

    /// Where the import stands.
    #[must_use]
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// Whether confirming would take a deck.
    #[must_use]
    pub fn ready(&self) -> bool {
        matches!(self.stage, Stage::Ready { .. })
    }
}

/// Where an import stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    /// Nothing pasted yet.
    #[default]
    Empty,
    /// A deck, read and waiting for the player to take it.
    Ready {
        /// The format that read it.
        format: FormatId,
        /// What it read.
        read: Read,
    },
    /// A link to a deck on a site that cannot be read from here, and what to
    /// do instead.
    Instruction(SourceId, Instruction),
    /// A link to a site that has an open API, which this build does not
    /// fetch yet.
    Unfetched(SourceId),
    /// A link to a site no source knows.
    UnknownLink(String),
    /// Text that is no deck, and why.
    Refused(ReadError),
    /// Taken into the builder.
    Done(Report),
}

/// What an import did, for the player to check.
///
/// The cards the pool does not know are **not** here: they are the
/// builder's [`DeckBuilder::missing`], read when the report is drawn,
/// because the pool may arrive after the import did and resolve more of
/// them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The format it was read as.
    pub format: FormatId,
    /// Rows taken.
    pub rows: usize,
    /// Copies across those rows.
    pub copies: u64,
    /// Lines that were not rows, by number.
    pub skipped: Vec<baylee_deckio::Skipped>,
    /// Maybeboard rows, which a stored deck has no place for.
    pub maybe: usize,
    /// Whether the file named the deck; if not, a name was made up.
    pub named: bool,
    /// Rows that chose no printing, which play the pool's default one.
    pub unprinted: usize,
    /// Rows that named no language, which are English.
    pub unlanguaged: usize,
    /// The file named no commander, and its main deck is a Commander deck's
    /// hundred cards: the player probably wants to set one.
    pub no_commander: bool,
}

/// The export dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exporting {
    /// The format being shown.
    pub format: FormatId,
    /// What the last copy or save did, if anything.
    pub said: Option<Said>,
}

/// What happened to the exported text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// It is on the clipboard.
    Copied,
    /// It could not be put on the clipboard.
    CopyFailed,
    /// It was written to this file.
    Saved(String),
    /// It was handed to the browser as a download.
    Downloaded(String),
    /// It could not be saved, and why (the system's words).
    SaveFailed(String),
}

/// One line of what a dialog tells the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// How it should look.
    pub tone: Tone,
    /// What it says.
    pub text: String,
}

/// How a line of the report should look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Plain information.
    Info,
    /// Done as asked.
    Good,
    /// Something the player should look at.
    Warn,
}

impl Line {
    fn new(tone: Tone, text: impl Into<String>) -> Self {
        Self {
            tone,
            text: text.into(),
        }
    }
}

/// The most skipped lines, unknown cards or losses a report lists one by
/// one; past that it says how many more.
pub const LISTED: usize = 8;

/// A pasted text is cut to this before it is kept: one byte past the
/// document limit, so a paste that was too large is still refused as too
/// large, and a clipboard holding a film is not copied into the builder.
const KEPT: usize = baylee_deckio::MAX_DOCUMENT_BYTES + 1;

/// The format's name as a player would say it.
#[must_use]
pub fn format_label(format: FormatId, lang: Lang) -> &'static str {
    match format {
        FormatId::Baylee => Phrase::FormatBaylee.text(lang),
        FormatId::Json => "JSON",
        FormatId::Yaml => "YAML",
        FormatId::Moxfield => "Moxfield",
    }
}

impl DeckBuilder {
    /// The open import or export dialog.
    #[must_use]
    pub fn transfer(&self) -> Option<&Transfer> {
        self.transfer.as_ref()
    }

    /// Opens the import dialog, empty.
    pub fn open_import(&mut self) {
        self.transfer = Some(Transfer::Import(Importing::default()));
    }

    /// Opens the export dialog on Baylee's own format, or on the last one
    /// chosen this session.
    pub fn open_export(&mut self) {
        let format = self.export_format.unwrap_or(FormatId::Baylee);
        self.transfer = Some(Transfer::Export(Exporting { format, said: None }));
    }

    /// Closes whichever dialog is open.
    pub fn close_transfer(&mut self) {
        self.transfer = None;
    }

    /// Takes a paste into the import dialog and reads it.
    ///
    /// The whole text is replaced, not appended to: a paste box that grew
    /// with every paste would import a deck twice the second time somebody
    /// pressed the key.
    pub fn import_paste(&mut self, text: &str) {
        let Some(Transfer::Import(importing)) = &mut self.transfer else {
            return;
        };
        let mut end = text.len().min(KEPT);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        importing.pasted = text[..end].to_string();
        importing.stage = understand(&importing.pasted);
    }

    /// Empties the import dialog.
    pub fn import_clear(&mut self) {
        if let Some(Transfer::Import(importing)) = &mut self.transfer {
            *importing = Importing::default();
        }
    }

    /// Takes the read deck into the builder, as a new unsaved deck.
    ///
    /// Returns whether anything was taken. The deck being edited is
    /// replaced, never overwritten on the gateway: the import is a *new*
    /// deck (`editing` is cleared), so saving it creates one rather than
    /// replacing the deck that happened to be open.
    pub fn import_confirm(&mut self, lang: Lang) -> bool {
        let Some(Transfer::Import(importing)) = &self.transfer else {
            return false;
        };
        let Stage::Ready { format, read } = &importing.stage else {
            return false;
        };
        let (format, read) = (*format, read.clone());
        let transfer = self.transfer.take();
        let report = self.take_document(format, read, lang);
        self.transfer = transfer;
        if let Some(Transfer::Import(importing)) = &mut self.transfer {
            importing.stage = Stage::Done(report);
        }
        true
    }

    /// Loads a document into the builder through the held-rows path.
    fn take_document(&mut self, format: FormatId, read: Read, lang: Lang) -> Report {
        let Read { document, skipped } = read;
        let stored = document.stored();
        let report = Report {
            format,
            rows: document.cards.len() - stored.maybe.len(),
            copies: document
                .cards
                .iter()
                .filter(|card| card.zone != baylee_deckio::Zone::Maybe)
                .map(|card| u64::from(card.count))
                .sum(),
            skipped,
            maybe: stored.maybe.len(),
            named: stored.name.as_deref().is_some_and(|n| !n.trim().is_empty()),
            unprinted: stored
                .cards
                .iter()
                .chain(&stored.sideboard)
                .filter(|row| row.print.set.is_none() && row.print.scryfall_id.is_none())
                .count(),
            unlanguaged: stored
                .cards
                .iter()
                .chain(&stored.sideboard)
                .filter(|row| row.print.lang.is_none())
                .count(),
            no_commander: stored.commanders.is_empty()
                && stored
                    .cards
                    .iter()
                    .map(|row| u64::from(row.count))
                    .sum::<u64>()
                    == 100,
        };
        let name = stored
            .name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| Phrase::ImportedDeckName.text(lang).to_string());
        self.start_new();
        self.name = crate::textbuf::TextBuffer::new(name.trim());
        self.pending_commander = stored.commanders;
        self.hold_rows(stored.cards, Zone::Main);
        self.hold_rows(stored.sideboard, Zone::Side);
        self.resolve_pending();
        // Unlike a stored deck, nothing of this is saved yet.
        self.dirty = true;
        self.focus_on(super::BuildField::Search);
        report
    }

    /// Chooses the format the export dialog shows.
    pub fn export_choose(&mut self, format: FormatId) {
        self.export_format = Some(format);
        if let Some(Transfer::Export(exporting)) = &mut self.transfer {
            exporting.format = format;
            exporting.said = None;
        }
    }

    /// Steps the export format through [`FormatId::ALL`].
    pub fn export_step(&mut self, forward: bool) {
        let Some(Transfer::Export(exporting)) = &self.transfer else {
            return;
        };
        let all = FormatId::ALL;
        let at = all.iter().position(|f| *f == exporting.format).unwrap_or(0);
        let next = if forward {
            (at + 1) % all.len()
        } else {
            (at + all.len() - 1) % all.len()
        };
        self.export_choose(all[next]);
    }

    /// Records what the last copy or save did.
    pub fn export_said(&mut self, said: Said) {
        if let Some(Transfer::Export(exporting)) = &mut self.transfer {
            exporting.said = Some(said);
        }
    }

    /// The deck as a document.
    ///
    /// Names are the pool's English ones, the form a stored row takes. With
    /// `whole_faces`, a double-faced card is named by both faces
    /// (`Front // Back`), which is what a site that spells faces its own way
    /// (Moxfield's `Front / Back`) needs to recognise it.
    #[must_use]
    pub fn export_document(&self, whole_faces: bool) -> Document {
        let rows = |zone: Zone| -> Vec<Row> {
            self.entries(zone)
                .iter()
                .filter_map(|entry| {
                    let card = self.pool.get(entry.slot)?;
                    Some(Row {
                        count: u32::from(entry.count),
                        name: card.english_name.clone(),
                        print: entry.print.clone(),
                        note: entry.note.clone(),
                    })
                })
                .collect()
        };
        let name = self.name.text().trim();
        let commanders = self.commander_names();
        let mut document = Document::from_stored(
            (!name.is_empty()).then_some(name),
            (!commanders.is_empty()).then_some("commander"),
            &rows(Zone::Main),
            &rows(Zone::Side),
            &commanders,
        );
        if whole_faces {
            for card in &mut document.cards {
                if let Some(whole) = self.whole_name(&card.name) {
                    card.name = whole;
                }
            }
        }
        document
    }

    /// Scryfall's two-face spelling of a double-faced pool card, by its
    /// English name.
    fn whole_name(&self, english: &str) -> Option<String> {
        let card = self
            .pool
            .iter()
            .find(|card| card.english_name == english && card.double_faced)?;
        let prefix = format!("{english} // ");
        card.alt_names
            .iter()
            .find(|alt| alt.starts_with(&prefix))
            .cloned()
    }

    /// The deck written in one format.
    #[must_use]
    pub fn export(&self, format: FormatId) -> Written {
        baylee_deckio::export(format, &self.export_document(format == FormatId::Moxfield))
    }

    /// A file name for the export: the deck's name made safe for every file
    /// system, and the format's extension.
    #[must_use]
    pub fn export_file_name(&self, format: FormatId) -> String {
        let stem: String = self
            .name
            .text()
            .trim()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .take(60)
            .collect();
        let stem = stem.trim();
        let stem = if stem.is_empty() { "deck" } else { stem };
        let suffix = match format {
            FormatId::Moxfield => "-moxfield",
            _ => "",
        };
        format!("{stem}{suffix}.{}", format.extension())
    }

    /// What the import dialog says under the paste box.
    #[must_use]
    pub fn import_lines(&self, lang: Lang) -> Vec<Line> {
        let Some(Transfer::Import(importing)) = &self.transfer else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        match &importing.stage {
            Stage::Empty => lines.push(Line::new(Tone::Info, Phrase::ImportHowTo.text(lang))),
            Stage::Ready { format, read } => {
                let rows = read.document.cards.len();
                let copies: u64 = read.document.cards.iter().map(|c| u64::from(c.count)).sum();
                lines.push(Line::new(
                    Tone::Good,
                    Phrase::ImportReadAs.fill(
                        lang,
                        &[
                            format_label(*format, lang),
                            &rows.to_string(),
                            &copies.to_string(),
                        ],
                    ),
                ));
                skipped_lines(&mut lines, &read.skipped, lang);
                if self.dirty() || !self.main.is_empty() || !self.side.is_empty() {
                    lines.push(Line::new(Tone::Warn, Phrase::ImportReplaces.text(lang)));
                }
            }
            Stage::Instruction(source, Instruction::MoxfieldExport) => {
                lines.push(Line::new(
                    Tone::Warn,
                    Phrase::ImportMoxfieldLink.fill(lang, &[source.name()]),
                ));
            }
            Stage::Unfetched(source) => lines.push(Line::new(
                Tone::Warn,
                Phrase::ImportNotFetched.fill(lang, &[source.name()]),
            )),
            Stage::UnknownLink(host) => lines.push(Line::new(
                Tone::Warn,
                Phrase::ImportUnknownLink.fill(lang, &[host]),
            )),
            Stage::Refused(error) => lines.push(Line::new(Tone::Warn, refusal(error, lang))),
            Stage::Done(report) => self.report_lines(report, lang, &mut lines),
        }
        lines
    }

    fn report_lines(&self, report: &Report, lang: Lang, lines: &mut Vec<Line>) {
        lines.push(Line::new(
            Tone::Good,
            Phrase::ImportTook.fill(
                lang,
                &[
                    &report.rows.to_string(),
                    &report.copies.to_string(),
                    format_label(report.format, lang),
                ],
            ),
        ));
        if !self.loaded() {
            lines.push(Line::new(
                Tone::Info,
                Phrase::ImportWaitingForPool.text(lang),
            ));
        }
        let missing = self.missing();
        if !missing.is_empty() {
            lines.push(Line::new(
                Tone::Warn,
                Phrase::ImportUnknownCards.fill(lang, &[&missing.len().to_string()]),
            ));
            for name in missing.iter().take(LISTED) {
                lines.push(Line::new(Tone::Warn, format!("  {name}")));
            }
            more(lines, missing.len(), lang);
        }
        skipped_lines(lines, &report.skipped, lang);
        if report.maybe > 0 {
            lines.push(Line::new(
                Tone::Warn,
                Phrase::ImportMaybeNotKept.fill(lang, &[&report.maybe.to_string()]),
            ));
        }
        if let Some(name) = &self.stale_commander {
            lines.push(Line::new(
                Tone::Warn,
                Phrase::ImportNotALeader.fill(lang, &[name]),
            ));
        }
        if !report.named {
            lines.push(Line::new(
                Tone::Info,
                Phrase::ImportNamedForYou.fill(lang, &[self.name()]),
            ));
        }
        if report.unprinted > 0 {
            lines.push(Line::new(
                Tone::Info,
                Phrase::ImportDefaultPrinting.fill(lang, &[&report.unprinted.to_string()]),
            ));
        }
        if report.unlanguaged > 0 {
            lines.push(Line::new(
                Tone::Info,
                Phrase::ImportDefaultLanguage.fill(lang, &[&report.unlanguaged.to_string()]),
            ));
        }
        if report.no_commander && self.commanders().is_empty() {
            lines.push(Line::new(Tone::Info, Phrase::ImportNoCommander.text(lang)));
        }
    }

    /// What the export dialog says about the chosen format: what it could
    /// not write, and what the last copy or save did.
    #[must_use]
    pub fn export_lines(&self, written: &Written, lang: Lang) -> Vec<Line> {
        let Some(Transfer::Export(exporting)) = &self.transfer else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        if written.losses.is_empty() {
            lines.push(Line::new(Tone::Good, Phrase::ExportComplete.text(lang)));
        } else {
            lines.push(Line::new(
                Tone::Warn,
                Phrase::ExportLeavesOut.fill(lang, &[format_label(exporting.format, lang)]),
            ));
            for loss in &written.losses {
                lines.push(Line::new(
                    Tone::Warn,
                    format!(
                        "  {}",
                        loss_phrase(loss.kind).fill(lang, &[&loss.rows.to_string()])
                    ),
                ));
            }
        }
        if let Some(said) = &exporting.said {
            lines.push(match said {
                Said::Copied => Line::new(Tone::Good, Phrase::ExportCopied.text(lang)),
                Said::CopyFailed => Line::new(Tone::Warn, Phrase::ExportCopyFailed.text(lang)),
                Said::Saved(path) => {
                    Line::new(Tone::Good, Phrase::ExportSavedTo.fill(lang, &[path]))
                }
                Said::Downloaded(file) => {
                    Line::new(Tone::Good, Phrase::ExportDownloaded.fill(lang, &[file]))
                }
                Said::SaveFailed(why) => {
                    Line::new(Tone::Warn, Phrase::ExportSaveFailed.fill(lang, &[why]))
                }
            });
        }
        lines
    }

    /// Rows held by name until the pool resolves them: a stored deck's and an
    /// imported one's alike.
    pub(super) fn hold_rows(&mut self, rows: Vec<Row>, zone: Zone) {
        for Row {
            count,
            name,
            print,
            note,
        } in rows
        {
            self.pending.push(super::Held {
                count: u16::try_from(count).unwrap_or(u16::MAX),
                name,
                zone,
                print,
                note,
            });
        }
    }
}

/// What a paste is.
fn understand(text: &str) -> Stage {
    if text.trim().is_empty() {
        return Stage::Empty;
    }
    match baylee_deckio::import(text) {
        Ok(Import::Read { format, read }) => Stage::Ready { format, read },
        Ok(Import::Source(found)) => match found.answer {
            Answer::Instruction(instruction) => Stage::Instruction(found.source, instruction),
            // No source answers with a fetch today; when one does, the shell
            // makes the request and hands the body to `import_paste`.
            Answer::Fetch(_) => Stage::Unfetched(found.source),
        },
        Ok(Import::UnknownLink(host)) => Stage::UnknownLink(host),
        Err(error) => Stage::Refused(error),
    }
}

fn skipped_lines(lines: &mut Vec<Line>, skipped: &[baylee_deckio::Skipped], lang: Lang) {
    if skipped.is_empty() {
        return;
    }
    lines.push(Line::new(
        Tone::Warn,
        Phrase::ImportSkipped.fill(lang, &[&skipped.len().to_string()]),
    ));
    for line in skipped.iter().take(LISTED) {
        lines.push(Line::new(
            Tone::Warn,
            Phrase::ImportSkippedLine.fill(lang, &[&line.line.to_string(), &line.text]),
        ));
    }
    more(lines, skipped.len(), lang);
}

fn more(lines: &mut Vec<Line>, total: usize, lang: Lang) {
    if total > LISTED {
        lines.push(Line::new(
            Tone::Warn,
            Phrase::AndMore.fill(lang, &[&(total - LISTED).to_string()]),
        ));
    }
}

/// Why a paste is no deck, in the player's language.
///
/// A JSON or YAML parser's own words stay as they are, after our sentence:
/// they name a line and a column, and a translation of them would be ours,
/// not the parser's (the rule gateway errors follow too).
#[must_use]
pub fn refusal(error: &ReadError, lang: Lang) -> String {
    match error {
        ReadError::TooLarge { .. } => Phrase::ImportTooLarge.text(lang).to_string(),
        ReadError::TooManyRows(rows) => Phrase::ImportTooManyRows.fill(lang, &[&rows.to_string()]),
        ReadError::Empty => Phrase::ImportNothing.text(lang).to_string(),
        ReadError::Unreadable(line) => {
            Phrase::ImportUnreadable.fill(lang, &[&line.line.to_string(), &line.text])
        }
        ReadError::Syntax(said) => Phrase::ImportSyntax.fill(lang, &[said]),
        ReadError::Version(version) => Phrase::ImportVersion.fill(lang, &[&version.to_string()]),
        ReadError::DeckName => Phrase::ImportDeckName.text(lang).to_string(),
        ReadError::Card { at, why } => Phrase::ImportCardRefused.fill(
            lang,
            &[
                &(at + 1).to_string(),
                match why {
                    CardError::Name => Phrase::ImportCardName,
                    CardError::Count => Phrase::ImportCardCount,
                    CardError::Note => Phrase::ImportCardNote,
                    CardError::Unstorable => Phrase::ImportCardUnstorable,
                }
                .text(lang),
            ],
        ),
    }
}

/// The sentence for one kind of loss; `{0}` is how many rows it hit.
fn loss_phrase(kind: LossKind) -> Phrase {
    match kind {
        LossKind::Name => Phrase::LossName,
        LossKind::Lang => Phrase::LossLang,
        LossKind::ScryfallId => Phrase::LossScryfallId,
        LossKind::Note => Phrase::LossNote,
        LossKind::Finish => Phrase::LossFinish,
        LossKind::CollectorNumber => Phrase::LossCollectorNumber,
        LossKind::Maybeboard => Phrase::LossMaybeboard,
    }
}

/// The export as a `data:` URL, for a browser to download: what an anchor
/// with a `download` name saves without the page holding a file of its own.
///
/// Percent-encoded byte by byte, every byte but the unreserved ones, so
/// nothing in a card name or a note can end the URL or start a second one.
#[must_use]
pub fn data_url(media_type: &str, text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = format!("data:{media_type};charset=utf-8,");
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// Writes an export into `dir` under `name`, never over a file that is
/// already there: `Deck.txt`, then `Deck (2).txt`, and so on.
///
/// # Errors
/// The file system's, when the folder cannot be made or the file written.
pub fn save_to(
    dir: &std::path::Path,
    name: &str,
    text: &str,
) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write as _;
    std::fs::create_dir_all(dir)?;
    let path = std::path::Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("deck");
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("txt");
    for n in 1..1000 {
        let candidate = if n == 1 {
            dir.join(format!("{stem}.{extension}"))
        } else {
            dir.join(format!("{stem} ({n}).{extension}"))
        };
        // `create_new` asks the file system, which is the one place the
        // question "is it there" and the act "make it" cannot be raced.
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(text.as_bytes())?;
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "a thousand exports of this deck are already there",
    ))
}
