//! Cabal Coffers — (no cost) — Land
//! Oracle: {2}, {T}: Add {B} for each Swamp you control.
//! Set: MH2 #301 — Modern Horizons 2 | Scryfall ID: e1efb0d3-2c72-46ff-bdc1-1069967365a0 | Oracle ID: 7358e164-5704-4e78-9b21-6a9bf2a968ce
// IMPLEMENTED — {2}, {T} to add {B} for each Swamp you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMPS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(land::SWAMP),
]);

card! {
    index: 319,
    oracle_id: "7358e164-5704-4e78-9b21-6a9bf2a968ce",
    scryfall_id: "e1efb0d3-2c72-46ff-bdc1-1069967365a0",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Cabal Coffers",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::CountOf {
                    filter: &SWAMPS_YOU_CONTROL,
                    zone: ZoneSel::Battlefield,
                },
            )],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 319);
        assert_eq!(CARD.oracle_id, "7358e164-5704-4e78-9b21-6a9bf2a968ce");
        assert_eq!(CARD.scryfall_id, "e1efb0d3-2c72-46ff-bdc1-1069967365a0");
        assert_eq!(CARD.faces[0].name, "Cabal Coffers");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[]);
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 1);
    }
}
