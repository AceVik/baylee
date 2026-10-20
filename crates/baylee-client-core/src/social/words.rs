//! What the chat says, in every language (§10.1): its own list, so the
//! chat's words don't grow [`crate::i18n::Phrase`]'s.

pub use crate::i18n::ChatPhrase;
use crate::i18n::Lang;

impl ChatPhrase {
    /// The placeholders this phrase carries, as their indices.
    #[must_use]
    pub fn slots(self, lang: Lang) -> Vec<usize> {
        let text = self.text(lang);
        (0..10)
            .filter(|i| text.contains(&format!("{{{i}}}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::ChatPhrase;
    use crate::i18n::Lang;

    /// A translation keeps every placeholder: a dropped `{0}` is a name or
    /// a count the player never sees.
    #[test]
    fn a_translation_keeps_every_placeholder_it_was_given() {
        for phrase in ChatPhrase::ALL {
            let english = phrase.slots(Lang::En);
            for lang in Lang::ALL {
                assert_eq!(phrase.slots(lang), english, "{phrase:?} in {lang:?}");
                assert!(!phrase.text(lang).is_empty(), "{phrase:?} in {lang:?}");
            }
        }
    }

    /// No straight apostrophes in German: the shell's typography uses the
    /// typographic ones, and a chat phrase reads like the rest.
    #[test]
    fn german_quotes_are_typographic() {
        for phrase in ChatPhrase::ALL {
            assert!(!phrase.text(Lang::De).contains('"'), "{phrase:?}");
        }
    }
}
