//! Baylee's own text format (`docs/deck-format.md` §"Text format").
//!
//! ```text
//! # baylee deck export v1
//! # name: Allytifact
//! CMD: 1 Aminatou, the Fateshifter (C18) 37
//! 1 Lightning Bolt (M11) 149 [de] *F* scryfall=e3285e6a-… # die deutsche
//! SB: 1 Karakas (EMA) 240
//! MB: 1 Sol Ring
//! ```
//!
//! Every row is a `deckrow` row, so this says every finish (`*H*`, `*G*`,
//! `*S*` included), language, printing id and note a stored row can. The
//! one thing it cannot say is a collector number with no set in front of it
//! ([`LossKind::CollectorNumber`]).

use super::text::{self, Dialect};
use crate::document::{Document, Zone};
use crate::format::{Format, FormatId, LossKind, Read, ReadError, Sniff, Tally, Written};
use std::fmt::Write as _;

/// The first line of every file this format writes, and how it is known.
pub(crate) const MAGIC: &str = "# baylee deck export v1";

/// Baylee's own text format.
pub(crate) struct Baylee;

const DIALECT: Dialect = Dialect {
    single_slash_faces: false,
    header_fields: true,
};

impl Format for Baylee {
    fn id(&self) -> FormatId {
        FormatId::Baylee
    }

    fn sniff(&self, text: &str) -> Sniff {
        if text
            .trim_start()
            .get(..MAGIC.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(MAGIC))
        {
            Sniff::Certain
        } else if text::has_prefix(text) {
            Sniff::Likely
        } else if text::starts_like_a_list(text) {
            Sniff::Plain
        } else {
            Sniff::No
        }
    }

    fn read(&self, text: &str) -> Result<Read, ReadError> {
        Ok(text::read(text, DIALECT))
    }

    fn write(&self, document: &Document) -> Written {
        let mut out = String::from(MAGIC);
        out.push('\n');
        if let Some(name) = &document.name {
            let _ = writeln!(out, "# name: {name}");
        }
        if let Some(format) = &document.format {
            let _ = writeln!(out, "# format: {format}");
        }
        let mut losses = Tally::default();
        for (zone, prefix) in [
            (Zone::Commander, "CMD: "),
            (Zone::Main, ""),
            (Zone::Side, "SB: "),
            (Zone::Maybe, "MB: "),
        ] {
            for card in document.zone(zone) {
                if card.set.is_none() && card.collector_number.is_some() {
                    losses.add(LossKind::CollectorNumber);
                }
                let _ = writeln!(out, "{prefix}{}", card.row());
            }
        }
        Written {
            text: out,
            losses: losses.losses(),
        }
    }
}
