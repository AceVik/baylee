//! Crypt of Agadeem — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {2}, {T}: Add {B} for each black creature card in your graveyard.
//! Set: TDC #354 — Tarkir: Dragonstorm Commander | Scryfall ID: 18adafe6-de67-4101-8b39-b53ab695ec4a | Oracle ID: 4fe8af73-c84a-44bd-9739-ee5c8b027874
// IMPLEMENTED — enters tapped; {T}: add {B}; {2}, {T}: add {B} for each black creature card in your graveyard.

use baylee_cards_dsl::prelude::*;

static BLACK_CREATURE_CARD: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card! {
    index: 390,
    oracle_id: "4fe8af73-c84a-44bd-9739-ee5c8b027874",
    scryfall_id: "18adafe6-de67-4101-8b39-b53ab695ec4a",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Crypt of Agadeem",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        mana_ability!(
            Cost {
                mana: baylee_core::mana!("{2}"),
                parts: &[CostPart::TapSelf],
            },
            &[Effect::mana_dynamic(
                ManaColor::Black,
                Amount::CountOf {
                    filter: &BLACK_CREATURE_CARD,
                    zone: ZoneSel::GraveyardYou,
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
        assert_eq!(CARD.index.get(), 390);
        assert_eq!(CARD.oracle_id, "4fe8af73-c84a-44bd-9739-ee5c8b027874");
        assert_eq!(CARD.scryfall_id, "18adafe6-de67-4101-8b39-b53ab695ec4a");
        assert_eq!(CARD.faces[0].name, "Crypt of Agadeem");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[]);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
