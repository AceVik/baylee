//! Dining Room — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R} or {G}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #15 — Ravnica: Clue Edition | Scryfall ID: 1bf8dcb2-6fe7-4ab3-b290-fb427d116c74 | Oracle ID: 0880461e-8943-443b-90e7-ff84eef46550
// IMPLEMENTED — enters tapped, taps for {R} or {G}, {4}, {T}: investigate.

use baylee_cards_dsl::prelude::*;

use crate::tokens::CLUE;

card! {
    index: 429,
    oracle_id: "0880461e-8943-443b-90e7-ff84eef46550",
    scryfall_id: "1bf8dcb2-6fe7-4ab3-b290-fb427d116c74",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Dining Room",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::Green])]),
        activated!(
            Cost {
                mana: baylee_core::mana!("{4}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::CreateToken {
                token: &CLUE,
            }],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 429);
        assert_eq!(CARD.oracle_id, "0880461e-8943-443b-90e7-ff84eef46550");
        assert_eq!(CARD.scryfall_id, "1bf8dcb2-6fe7-4ab3-b290-fb427d116c74");
        assert_eq!(CARD.faces[0].name, "Dining Room");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[]);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green, Color::Red])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
