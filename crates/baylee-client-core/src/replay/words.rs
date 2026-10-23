//! What a replay says, in every language (§7.6): its own list, so the
//! replay's words don't grow [`crate::i18n::Phrase`]'s, which every lane
//! edits. The keymap's rows stay `Phrase`s (`prefs::Action::label` is one).

use crate::i18n::Lang;
pub use crate::i18n::ReplayPhrase;

impl ReplayPhrase {
    /// The placeholders this phrase carries, as their indices.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect()
    }

    /// A result's word.
    #[must_use]
    pub const fn outcome(outcome: super::library::Outcome) -> Self {
        use super::library::Outcome;
        match outcome {
            Outcome::Won => Self::Won,
            Outcome::Lost => Self::Lost,
            Outcome::Draw => Self::Draw,
            Outcome::Unfinished => Self::Unfinished,
            Outcome::Unknown => Self::Ended,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayPhrase;
    use crate::i18n::Lang;

    /// No phrase is empty in any language (that every language keeps
    /// every argument is `i18n::tests`' parity test).
    #[test]
    fn no_phrase_is_empty() {
        for phrase in ReplayPhrase::ALL {
            for lang in Lang::ALL {
                assert!(!phrase.text(lang).is_empty(), "{phrase:?} in {lang:?}");
            }
        }
    }
}
