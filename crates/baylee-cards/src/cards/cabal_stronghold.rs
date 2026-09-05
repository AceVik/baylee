//! Cabal Stronghold — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Add {B} for each basic Swamp you control.
//! Set: DOM #238 — Dominaria | Scryfall ID: 0bda51ef-ee3e-48d4-92e2-c9083bbe0f80 | Oracle ID: 066cd584-773c-4623-be53-8f6feda5a26a
// IMPLEMENTED — tap for {C}; {3}, {T} to add {B} for each basic Swamp you control.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static BASIC_SWAMPS_YOU_CONTROL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(land::SWAMP),
]);

card! {
    index: 321,
    oracle_id: "066cd584-773c-4623-be53-8f6feda5a26a",
    scryfall_id: "0bda51ef-ee3e-48d4-92e2-c9083bbe0f80",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Cabal Stronghold",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{3}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::CountOf {
                    filter: &BASIC_SWAMPS_YOU_CONTROL,
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
        assert_eq!(CARD.index.get(), 321);
        assert_eq!(CARD.oracle_id, "066cd584-773c-4623-be53-8f6feda5a26a");
        assert_eq!(CARD.scryfall_id, "0bda51ef-ee3e-48d4-92e2-c9083bbe0f80");
        assert_eq!(CARD.faces[0].name, "Cabal Stronghold");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[]);
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
