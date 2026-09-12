//! How a printed oracle text is cut into sentences.
//!
//! Three lines of code that have to be the same three lines in two crates
//! that cannot see each other. Codegen writes a per-ability *sentence
//! index* from the English oracle text; a client resolves that index
//! against the printed text of whatever printing and language the player
//! chose, and `baylee-client-core` reaches neither `baylee-cards-codegen`
//! nor `baylee-cards`. `baylee-core` is the one crate both ends depend on,
//! so the split lives here and neither end owns it.
//!
//! A split that drifted would not fail loudly: it would point at the
//! sentence beside the right one, and a stack entry would confidently say
//! the wrong thing.

/// The sentences a printed oracle text is made of.
///
/// Scryfall prints one rules sentence per line and separates paragraphs
/// with a blank one. Blank lines are dropped and every line is trimmed, so
/// the index this yields is an index into **this** iterator and not into
/// [`str::lines`].
///
/// A localized text is split the same way and is expected to have the same
/// number of sentences; where it does not — an old printing with
/// pre-errata wording, a translation that joins two lines — the count is
/// what says so, which is why the count travels beside the index.
pub fn sentences(oracle: &str) -> impl Iterator<Item = &str> {
    oracle.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// How many sentences a printed oracle text is made of.
#[must_use]
pub fn sentence_count(oracle: &str) -> usize {
    sentences(oracle).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The blank line between two paragraphs is a separator, not a
    /// sentence — an index that counted it would be off by one for every
    /// ability below the first paragraph.
    #[test]
    fn a_blank_line_between_paragraphs_is_not_a_sentence() {
        let oracle = "Flying\n\nWhen this creature enters, draw a card.\n";
        let got: Vec<&str> = sentences(oracle).collect();
        assert_eq!(got, ["Flying", "When this creature enters, draw a card."]);
        assert_eq!(sentence_count(oracle), 2);
    }

    /// Trailing whitespace is Scryfall's, not the card's.
    #[test]
    fn a_sentence_carries_no_surrounding_whitespace() {
        assert_eq!(sentences("  {T}: Add {G}.  ").next(), Some("{T}: Add {G}."));
    }
}
