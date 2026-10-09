//! Reuse the catalog's checked-in vocabulary when a printing lacks translated types.
use crate::i18n::Lang;

/// `line` (an English type line) in `lang`, through the catalog's
/// checked-in vocabulary; a line it cannot read stays as it is.
#[must_use]
pub fn translated(line: &str, lang: Lang) -> String {
    if lang == Lang::En {
        return line.into();
    }
    let names = crate::type_names::dictionary();
    let (head, subtypes) = line
        .split_once('—')
        .map_or((line.trim(), None), |(a, b)| (a.trim(), Some(b.trim())));
    // A head already translated by the catalog remains authoritative.
    let Some(head) = names.get(head) else {
        return line.into();
    };
    let Some(subtypes) = subtypes else {
        return (*head).into();
    };
    let translated = if let Some(whole) = names.get(subtypes) {
        (*whole).into()
    } else {
        subtypes
            .split_whitespace()
            .map(|word| {
                let word = word.trim_end_matches(',');
                names.get(word).copied().unwrap_or(word)
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!("{head} — {translated}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_type_and_subtype_lines_use_the_shared_dictionary() {
        assert_eq!(
            translated("Creature — Human Soldier Ally", Lang::De),
            "Kreatur — Mensch, Soldat, Verbündeter"
        );
        assert_eq!(
            translated("Land — Forest Island", Lang::De),
            "Land — Wald, Insel"
        );
        assert_eq!(
            translated("Legendary Creature — Human Wizard", Lang::De),
            "Legendäre Kreatur — Mensch, Zauberer"
        );
        assert_eq!(
            translated("Kreatur — Mensch, Zauberer", Lang::De),
            "Kreatur — Mensch, Zauberer"
        );
        assert_eq!(translated("Creature — Human", Lang::En), "Creature — Human");
    }
}
