//! Names of card types in the client's language, from the shared catalog vocabulary.
use crate::i18n::Lang;
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn german() -> &'static BTreeMap<&'static str, &'static str> {
    static GERMAN: OnceLock<BTreeMap<&str, &str>> = OnceLock::new();
    GERMAN.get_or_init(|| {
        include_str!("../../../data/type-names.tsv")
            .lines()
            .filter_map(|row| {
                let mut fields = row.split('\t');
                let key = fields.next()?;
                let lang = fields.next()?;
                let value = fields.next()?;
                (lang == "de").then_some((key, value))
            })
            .collect()
    })
}

/// Translate one type or subtype, falling back to its canonical name.
#[must_use]
pub fn name(english: &str, lang: Lang) -> &str {
    if lang == Lang::En {
        english
    } else {
        german().get(english).copied().unwrap_or(english)
    }
}

/// The shared dictionary for assembling full type lines.
pub(crate) fn dictionary() -> &'static BTreeMap<&'static str, &'static str> {
    german()
}
