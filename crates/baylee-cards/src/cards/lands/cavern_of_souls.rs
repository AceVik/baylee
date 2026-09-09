//! Cavern of Souls — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.
//! Set: LCI #269 — The Lost Caverns of Ixalan | Scryfall ID: 3aad15a2-8a1b-4460-9b06-e85863081878 | Oracle ID: 89ca686a-7c72-4d8f-9290-e89635624a83
// IMPLEMENTED — choose-a-type, {C}, and the restricted any-color mana:
// it pays only for creature spells of the chosen type and makes them
// uncounterable (mana provenance + Uncounterable rider).

static CHOSEN_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 17,
    oracle_id: "89ca686a-7c72-4d8f-9290-e89635624a83",
    scryfall_id: "3aad15a2-8a1b-4460-9b06-e85863081878",
    faces: &[face! {
        name: "Cavern of Souls",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::ChooseSubtype],
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_choice(ALL_MANA_COLORS).restricted(
                &CHOSEN_TYPE_CREATURE_SPELL,
                SpendRider::Uncounterable,
            )]),
    ],
}
