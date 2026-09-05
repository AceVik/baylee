//! Cryptic Caves — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}, Sacrifice this land: Draw a card. Activate only if you control five or more lands.
//! Set: FDN #771 — Foundations | Scryfall ID: 9d126c8f-a3df-497c-aad5-a448c55f51cc | Oracle ID: b2eb7a64-a307-4a78-a25d-63fb3ae1e237
// IMPLEMENTED — {T} → {C}; {1}, {T}, sacrifice self → draw a card (only if you control five or more lands).

use baylee_cards_dsl::prelude::*;

card! {
    index: 392,
    oracle_id: "b2eb7a64-a307-4a78-a25d-63fb3ae1e237",
    scryfall_id: "9d126c8f-a3df-497c-aad5-a448c55f51cc",
    faces: &[
    face! {
        name: "Cryptic Caves",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        // {T}: Add {C}.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // {1}, {T}, Sacrifice this land: Draw a card. Activate only if you control five or more lands.
        AbilityDef::ActivatedConditional {
            cost: Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf, CostPart::SacrificeSelf],
            },
            effects: &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::ControlCount(&Filter::LAND, 5),
        },
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 392);
        assert_eq!(CARD.oracle_id, "b2eb7a64-a307-4a78-a25d-63fb3ae1e237");
        assert_eq!(CARD.scryfall_id, "9d126c8f-a3df-497c-aad5-a448c55f51cc");
        assert_eq!(CARD.faces[0].name, "Cryptic Caves");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.color_identity, ColorSet::EMPTY);
        assert_eq!(CARD.abilities.len(), 2);
    }
}
