//! Conservatory — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {W}.
//! Oracle: {4}, {T}: Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
//! Set: CLU #14 — Ravnica: Clue Edition | Scryfall ID: 8bab22c5-4742-4f2e-bda1-26c72b09cd9c | Oracle ID: 8f88b0bf-81bd-4223-9dad-d8c49ff4b87a
// IMPLEMENTED — enters tapped, taps for {G} or {W}, {4}, {T}: investigate.

use baylee_cards_dsl::prelude::*;

use crate::tokens::CLUE;

card! {
    index: 370,
    oracle_id: "8f88b0bf-81bd-4223-9dad-d8c49ff4b87a",
    scryfall_id: "8bab22c5-4742-4f2e-bda1-26c72b09cd9c",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Conservatory",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    abilities: &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::White])]),
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
        assert_eq!(CARD.index.get(), 370);
        assert_eq!(CARD.oracle_id, "8f88b0bf-81bd-4223-9dad-d8c49ff4b87a");
        assert_eq!(CARD.scryfall_id, "8bab22c5-4742-4f2e-bda1-26c72b09cd9c");
        assert_eq!(CARD.faces[0].name, "Conservatory");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[]);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green, Color::White])
        );
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
