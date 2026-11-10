//! The English Oracle text of every card in the pool, per face.
//!
//! [`crate::lines`] says which sentence an ability was printed as, as an
//! index into this text; this is the text. A client draws the sentence in
//! the player's language when it has one that pairs with it
//! (`baylee_cardtext`), and this one otherwise — with no gateway, no network,
//! for a card nobody translated, and for a translation whose lines do not
//! line up. So a row that has a line in the table always has words, and they
//! are Wizards' words rather than a client's.
//!
//! The table is [`crate::generated_oracle::ORACLE`], written by
//! `cargo xtask codegen --tables` from the payload cache the line table is
//! counted against; `oracle_tests` holds the two to the same sentence counts.

use baylee_core::ids::CardIndex;

/// One face's English Oracle text, if the pool has the card and the face.
#[must_use]
pub fn face(card: CardIndex, face: usize) -> Option<&'static str> {
    crate::generated_oracle::ORACLE
        .get(card.get() as usize)?
        .get(face)
        .copied()
}

/// The `line`th sentence of one face's English Oracle text, split the way
/// the line table counted it (`baylee_cardtext::sentences`).
#[must_use]
pub fn sentence(card: CardIndex, face: usize, line: u8) -> Option<&'static str> {
    baylee_core::oracle::sentences(self::face(card, face)?).nth(usize::from(line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_lines::ABILITY_LINES;

    /// Faces the line table has a row for, and every one of them whose
    /// sentence count is not the count of the Oracle text `oracle` answers.
    fn disagreements(
        oracle: impl Fn(CardIndex, usize) -> Option<&'static str>,
    ) -> (usize, Vec<String>) {
        let mut faces = 0;
        let mut wrong = Vec::new();
        for (index, row) in ABILITY_LINES.iter().enumerate() {
            let card = CardIndex::new(index as u32);
            for (at, lines) in row.iter().enumerate() {
                faces += 1;
                let counted = oracle(card, at).map(baylee_core::oracle::sentence_count);
                if counted != Some(usize::from(lines.sentences)) {
                    wrong.push(format!(
                        "card {index} face {at}: the line table counted {}, the Oracle has {counted:?}",
                        lines.sentences
                    ));
                }
            }
        }
        (faces, wrong)
    }

    /// The two generated tables describe the same text. A sentence index
    /// into one and a sentence taken from the other are only the same
    /// sentence while this holds, and a row drawn from the Oracle is the
    /// fallback for every row the player's language cannot supply.
    ///
    /// Measured 2026-09-24: 2561 faces. The floor is the population and not
    /// a ratio, because a stale table is exactly one that has fewer rows.
    #[test]
    fn the_line_table_counts_the_oracle_it_indexes() {
        let (faces, wrong) = disagreements(face);
        assert!(faces >= 2500, "only {faces} faces have a line-table row");
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// The injection: the same check over an Oracle one sentence short on
    /// one face has to name that face and nothing else.
    #[test]
    fn a_face_one_sentence_short_is_caught() {
        let (target, _) = ABILITY_LINES
            .iter()
            .enumerate()
            .find(|(_, row)| row.first().is_some_and(|f| f.sentences >= 2))
            .expect("a card with two sentences");
        let target = CardIndex::new(target as u32);
        let (_, wrong) = disagreements(|card, at| {
            if card == target && at == 0 {
                Some("One sentence.")
            } else {
                face(card, at)
            }
        });
        assert_eq!(wrong.len(), 1, "{wrong:?}");
    }

    /// Every card the pool compiles has its text, face for face: a row the
    /// table leaves empty is a card whose every ability would fall to no
    /// words at all, which is the one outcome the table exists to rule out.
    #[test]
    fn every_card_in_the_pool_has_its_oracle_on_every_face() {
        let mut missing = Vec::new();
        let mut cards = 0;
        for def in crate::all() {
            cards += 1;
            let printed = crate::generated_oracle::ORACLE
                .get(def.index.get() as usize)
                .map_or(0, |faces| faces.len());
            if printed < def.faces.len() {
                missing.push(format!(
                    "{} ({} of {} faces)",
                    def.name(),
                    printed,
                    def.faces.len()
                ));
            }
        }
        assert!(cards >= 2700, "only {cards} cards in the pool");
        assert!(
            missing.is_empty(),
            "{} card(s) have no Oracle text:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    #[test]
    fn a_sentence_is_one_printed_line() {
        let card = crate::decks::by_name("Mind Stone").expect("in the pool");
        assert_eq!(
            sentence(card, 0, 1),
            Some("{1}, {T}, Sacrifice this artifact: Draw a card.")
        );
        assert_eq!(sentence(card, 0, 2), None);
        assert_eq!(sentence(card, 1, 0), None);
    }
}
