use super::lang::Lang;
use super::{Phrase, server_message};

/// A refusal the prompt bar has to draw, in whichever form it arrived.
///
/// Client-owned phrases and known engine/server messages follow the current
/// language. Unknown diagnostics keep the original message so newer servers
/// remain readable without a lockstep client update.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// A sentence this client owns.
    Said(Phrase),
    /// A sentence another process sent, in its own words.
    Verbatim(String),
}

impl Refusal {
    /// The sentence, in `lang` where this client has a say in it.
    #[must_use]
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Self::Said(phrase) => phrase.text(lang).to_string(),
            Self::Verbatim(prose) => server_message(lang, prose),
        }
    }
}
