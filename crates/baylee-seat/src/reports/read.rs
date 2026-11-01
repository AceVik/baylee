//! A report as a model's answer carries it.

use super::{FIELD_CHARS, MOST_REFERENCES, REFERENCE_CHARS};
use serde_json::Value;

/// A rules or engine error a model noticed, as its answer said it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// What happened.
    pub what: String,
    /// What the rules or the card's text say should happen.
    pub expected: Option<String>,
    /// The Oracle text, rule number or lookup result it relied on.
    pub evidence: Option<String>,
    /// Scryfall addresses, rule numbers, card names in brackets.
    pub references: Vec<String>,
}

impl Report {
    /// The `report` of an answer: an object, or a plain string as its
    /// `what`. `None` when it says nothing (no `what`, a blank one, a
    /// strict schema's `null`).
    #[must_use]
    pub fn read(value: &Value) -> Option<Self> {
        let text = |value: Option<&Value>| {
            value
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| held(text, FIELD_CHARS))
        };
        let report = match value {
            Value::String(_) => Self {
                what: text(Some(value))?,
                ..Self::default()
            },
            Value::Object(fields) => Self {
                what: text(fields.get("what"))?,
                expected: text(fields.get("expected")),
                evidence: text(fields.get("evidence")),
                references: match fields.get("references") {
                    Some(Value::Array(items)) => items
                        .iter()
                        .filter_map(|item| text(Some(item)))
                        .take(MOST_REFERENCES)
                        .map(|reference| held(&reference, REFERENCE_CHARS))
                        .collect(),
                    other => text(other)
                        .map(|reference| held(&reference, REFERENCE_CHARS))
                        .into_iter()
                        .collect(),
                },
            },
            _ => return None,
        };
        Some(report)
    }

    /// Everything it says, for finding the cards it names.
    pub(super) fn words(&self) -> String {
        let mut words = self.what.clone();
        for part in [&self.expected, &self.evidence].into_iter().flatten() {
            words.push('\n');
            words.push_str(part);
        }
        for reference in &self.references {
            words.push('\n');
            words.push_str(reference);
        }
        words
    }
}

/// `text`, at most `most` characters of it.
fn held(text: &str, most: usize) -> String {
    match text.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}
