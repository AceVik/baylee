//! A two-word draft; the engine's encoded answer never appears as a number UI.
use crate::i18n::Lang;
use baylee_cards_dsl::TextWordKind;
use baylee_engine::choice::PlayerAction;

/// Independent old/new selections, confirmed together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    /// The offered vocabulary.
    pub kind: TextWordKind,
    /// Word to replace, in the engine's vocabulary order.
    pub from: Option<u8>,
    /// Replacement word, in the same order.
    pub to: Option<u8>,
}
impl Draft {
    /// Starts without making either decision for the player.
    #[must_use]
    pub const fn new(kind: TextWordKind) -> Self {
        Self {
            kind,
            from: None,
            to: None,
        }
    }
    /// Rows 0–4 select the old word; rows 5–9 select the new word.
    pub fn select(&mut self, index: usize) -> bool {
        let Ok(word) = u8::try_from(index % 5) else {
            return false;
        };
        match index {
            0..=4 => self.from = Some(word),
            5..=9 => self.to = Some(word),
            _ => return false,
        }
        true
    }
    /// Encode exactly one distinct ordered pair, only after both choices.
    #[must_use]
    pub fn answer(&self) -> Option<PlayerAction> {
        let (from, to) = (self.from?, self.to?);
        (from != to).then(|| {
            PlayerAction::ChooseNumber(u32::from(from) * 4 + u32::from(to - u8::from(to > from)))
        })
    }
}

/// Localized words, in the contract's WUBRG/basic-land order.
#[must_use]
pub fn words(kind: TextWordKind, lang: Lang) -> [&'static str; 5] {
    match (kind, lang) {
        (TextWordKind::Color, Lang::De) => ["Weiß", "Blau", "Schwarz", "Rot", "Grün"],
        (TextWordKind::Color, _) => ["White", "Blue", "Black", "Red", "Green"],
        (TextWordKind::BasicLandType, Lang::De) => ["Ebene", "Insel", "Sumpf", "Gebirge", "Wald"],
        (TextWordKind::BasicLandType, _) => ["Plains", "Island", "Swamp", "Mountain", "Forest"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_distinct_pairs_match_the_engine_encoding() {
        for kind in [TextWordKind::Color, TextWordKind::BasicLandType] {
            for from in 0..5 {
                for to in 0..5 {
                    let mut draft = Draft::new(kind);
                    assert!(draft.answer().is_none());
                    draft.select(from);
                    assert!(draft.answer().is_none());
                    draft.select(5 + to);
                    let Some(PlayerAction::ChooseNumber(encoded)) = draft.answer() else {
                        assert_eq!(from, to);
                        continue;
                    };
                    let pair =
                        baylee_engine::text_changes::TextReplacement::from_choice(kind, encoded)
                            .unwrap();
                    assert_eq!((usize::from(pair.from), usize::from(pair.to)), (from, to));
                }
            }
        }
    }
}
