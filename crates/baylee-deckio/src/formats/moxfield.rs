//! Moxfield's text export: what "More → Export → Copy for Moxfield" puts on
//! the clipboard, and what Moxfield's own import box reads.
//!
//! ```text
//! 1 Archangel Avacyn / Avacyn, the Purifier (SOI) 5
//! 1 Crop Rotation (PLST) DDR-7
//!
//! SIDEBOARD:
//! 1 Jetmir's Garden (PSNC) 250s *F*
//! ```
//!
//! Every row is a `deckrow` row already — `(SET) number` with Scryfall's
//! own collector numbers (`196p`, `DDR-7`), `*F*` foil, `*E*` etched — so the
//! reader is the shared text reader with two differences: a double-faced
//! card is `Front / Back` (Moxfield) rather than `Front // Back` (Scryfall,
//! Baylee), and the plain lists other sites write (`1x Name`, Arena's
//! `Commander`/`Deck`/`Sideboard` sections) read here too.
//!
//! # What it cannot say
//!
//! No deck name, language, printing id or note, and of the finishes only
//! foil and etched. The writer leaves those out and names each in
//! [`Written::losses`]; a holographic, glitter or galaxy card is written
//! with no marker, which Moxfield reads as non-foil, and the loss says so
//! rather than the row claiming a foil it is not.
//!
//! The commander is written under a `COMMANDER:` header, **first**, ended by
//! a blank line. Moxfield's own export of the sample deck in
//! `tests/fixtures/moxfield-export.txt` names no commander, so this header is
//! not yet checked against Moxfield's importer; putting it first means that
//! an importer which does not know it reads the leader as a main-deck card,
//! never as a sideboard one.

use super::text::{self, Dialect};
use crate::document::{Document, Zone};
use crate::format::{Format, FormatId, LossKind, Read, ReadError, Sniff, Tally, Written};
use baylee_core::deckrow::{PrintChoice, Row};
use baylee_core::preset::Finish;
use std::fmt::Write as _;

/// Moxfield's text format.
pub(crate) struct Moxfield;

const DIALECT: Dialect = Dialect {
    single_slash_faces: true,
    header_fields: false,
};

impl Format for Moxfield {
    fn id(&self) -> FormatId {
        FormatId::Moxfield
    }

    fn sniff(&self, text: &str) -> Sniff {
        if !text::starts_like_a_list(text) {
            return Sniff::No;
        }
        let rows = || {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
        };
        let x_counts = rows().any(|line| {
            line.split_once(' ').is_some_and(|(count, _)| {
                count.len() > 1
                    && count.ends_with(['x', 'X'])
                    && count[..count.len() - 1].chars().all(|c| c.is_ascii_digit())
            })
        });
        if text::has_header(text) || x_counts || rows().any(|line| line.contains(" / ")) {
            Sniff::Likely
        } else {
            Sniff::Plain
        }
    }

    fn read(&self, text: &str) -> Result<Read, ReadError> {
        Ok(text::read(text, DIALECT))
    }

    fn write(&self, document: &Document) -> Written {
        let mut losses = Tally::default();
        if document.name.is_some() {
            losses.add(LossKind::Name);
        }
        let mut out = String::new();
        let mut row = |out: &mut String, card: &crate::Card| {
            let row = moxfield_row(card, &mut losses);
            let _ = writeln!(out, "{row}");
        };
        let commanders: Vec<_> = document.zone(Zone::Commander).collect();
        if !commanders.is_empty() {
            out.push_str("COMMANDER:\n");
            for card in commanders {
                row(&mut out, card);
            }
            out.push('\n');
        }
        for card in document.zone(Zone::Main) {
            row(&mut out, card);
        }
        let side: Vec<_> = document.zone(Zone::Side).collect();
        if !side.is_empty() {
            out.push_str("\nSIDEBOARD:\n");
            for card in side {
                row(&mut out, card);
            }
        }
        for _ in document.zone(Zone::Maybe) {
            losses.add(LossKind::Maybeboard);
        }
        Written {
            text: out,
            losses: losses.losses(),
        }
    }
}

/// One card as Moxfield writes it, tallying what it had to leave out.
fn moxfield_row(card: &crate::Card, losses: &mut Tally) -> Row {
    let finish = match card.finish {
        None | Some(Finish::Normal) => None,
        Some(kept @ (Finish::Foil | Finish::Etched)) => Some(kept),
        Some(Finish::Holographic | Finish::Glitter | Finish::Galaxy) => {
            losses.add(LossKind::Finish);
            None
        }
    };
    if card.lang.is_some() {
        losses.add(LossKind::Lang);
    }
    if card.scryfall_id.is_some() {
        losses.add(LossKind::ScryfallId);
    }
    if card.note.is_some() {
        losses.add(LossKind::Note);
    }
    if card.set.is_none() && card.collector_number.is_some() {
        losses.add(LossKind::CollectorNumber);
    }
    Row {
        count: card.count,
        // Moxfield's spelling of a double-faced card.
        name: card.name.replace(" // ", " / "),
        print: PrintChoice {
            set: card.set.clone(),
            collector_number: card.collector_number.clone(),
            lang: None,
            finish,
            scryfall_id: None,
        },
        note: None,
    }
}
