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

    /// A translation keeps every placeholder: a dropped `{0}` is a turn or
    /// a build the player never sees; and no phrase is empty.
    #[test]
    fn a_translation_keeps_every_placeholder_it_was_given() {
        for phrase in ReplayPhrase::ALL {
            let english = phrase.slots(Lang::En);
            for lang in Lang::ALL {
                assert_eq!(phrase.slots(lang), english, "{phrase:?} in {lang:?}");
                assert!(!phrase.text(lang).is_empty(), "{phrase:?} in {lang:?}");
            }
        }
    }
}
