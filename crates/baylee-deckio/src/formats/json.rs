//! The deck document as JSON (`docs/deck-format.md` §"JSON format").
//!
//! [`crate::Document`] through serde, pretty-printed, every field written —
//! an absent one as `null` — so a file says plainly what it does not know.
//! Lossless: every finish, set, collector number, language, printing id and
//! note comes back exactly.

use crate::document::Document;
use crate::format::{Format, FormatId, Read, ReadError, Sniff, Written};

/// JSON.
pub(crate) struct Json;

impl Format for Json {
    fn id(&self) -> FormatId {
        FormatId::Json
    }

    fn sniff(&self, text: &str) -> Sniff {
        // An object is the only thing a deck document can be, and no deck
        // list starts with a brace. A broken one is still JSON: its reader's
        // error is the one worth showing.
        if text.trim_start().starts_with('{') {
            Sniff::Certain
        } else {
            Sniff::No
        }
    }

    fn read(&self, text: &str) -> Result<Read, ReadError> {
        let document: Document = serde_json::from_str(text).map_err(|e| ReadError::syntax(&e))?;
        Ok(Read {
            document,
            skipped: Vec::new(),
        })
    }

    fn write(&self, document: &Document) -> Written {
        let mut text = serde_json::to_string_pretty(document)
            // A document is strings, numbers and options of them; there is
            // nothing in it serde_json can fail on.
            .unwrap_or_default();
        text.push('\n');
        Written {
            text,
            losses: Vec::new(),
        }
    }
}
