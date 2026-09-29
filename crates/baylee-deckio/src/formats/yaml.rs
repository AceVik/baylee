//! The deck document as YAML: the same [`crate::Document`] as the JSON
//! format, through the same serde model, so the two say exactly the same
//! things and a file converts between them without a word lost.

use crate::document::Document;
use crate::format::{Format, FormatId, Read, ReadError, Sniff, Written};

/// YAML.
pub(crate) struct Yaml;

impl Format for Yaml {
    fn id(&self) -> FormatId {
        FormatId::Yaml
    }

    fn sniff(&self, text: &str) -> Sniff {
        // Any deck list is valid YAML — as one long string — so "it parses"
        // says nothing. What says YAML is the document's own shape: a
        // `version:` or `cards:` key at the start of a line, or the document
        // marker. No deck row starts with either word and a colon.
        let first = text
            .lines()
            .map(str::trim_end)
            .find(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'));
        match first {
            Some(line) if line == "---" || line.starts_with("version:") => Sniff::Certain,
            _ if text
                .lines()
                .any(|line| line.starts_with("cards:") || line.starts_with("version:")) =>
            {
                Sniff::Likely
            }
            _ => Sniff::No,
        }
    }

    fn read(&self, text: &str) -> Result<Read, ReadError> {
        // The parser's default budget bounds alias expansion, nesting and
        // node count; the size of the text was bounded before it got here.
        let document: Document = serde_saphyr::from_str(text).map_err(|e| ReadError::syntax(&e))?;
        Ok(Read {
            document,
            skipped: Vec::new(),
        })
    }

    fn write(&self, document: &Document) -> Written {
        let text = serde_saphyr::to_string(document).unwrap_or_default();
        Written {
            text,
            losses: Vec::new(),
        }
    }
}
