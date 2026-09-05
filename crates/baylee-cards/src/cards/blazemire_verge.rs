//! Blazemire Verge — (no cost) — Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {R}. Activate only if you control a Swamp or a Mountain.
//! Set: DSK #256 — Duskmourn: House of Horror | Scryfall ID: d151c8e2-d715-470d-868a-f45191db9fa0 | Oracle ID: 977c2f33-b622-4172-9efb-7f523becd32b
// IMPLEMENTED — {B} always; {R} only with a Swamp or Mountain under your control (ActivationCondition::ControlCount).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static SWAMP_OR_MOUNTAIN: Filter = Filter::Or(&[
    Filter::HasSubtype(land::SWAMP),
    Filter::HasSubtype(land::MOUNTAIN),
]);

card! {
    index: 278,
    oracle_id: "977c2f33-b622-4172-9efb-7f523becd32b",
    scryfall_id: "d151c8e2-d715-470d-868a-f45191db9fa0",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
    face! {
        name: "Blazemire Verge",
        types: TypeSet::LAND,
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        AbilityDef::ActivatedConditional {
            cost: Cost::TAP,
            effects: &[Effect::mana(ManaColor::Red, 1)],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
            condition: ActivationCondition::ControlCount(&SWAMP_OR_MOUNTAIN, 1),
        },
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 278);
        assert_eq!(CARD.oracle_id, "977c2f33-b622-4172-9efb-7f523becd32b");
        assert_eq!(CARD.scryfall_id, "d151c8e2-d715-470d-868a-f45191db9fa0");
        assert_eq!(CARD.faces[0].name, "Blazemire Verge");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Red])
        );
        assert_eq!(CARD.abilities.len(), 2);
    }
}
