//! Boseiju, Who Shelters All — (no cost) — Legendary Land
//! Oracle: Boseiju enters tapped.
//! Oracle: {T}, Pay 2 life: Add {C}. If that mana is spent on an instant or sorcery spell, that spell can't be countered.
//! Set: CHK #273 — Champions of Kamigawa | Scryfall ID: 0180d9a8-992c-4d55-8ac4-33a587786993 | Oracle ID: 36937483-30cb-449a-8028-75017a124922
// IMPLEMENTED — enters tapped; {T} and 2 life for {C}, with the uncounterable
// rider carried by the mana itself (SpendRider::Uncounterable).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOSEIJU_WHO_SHELTERS_ALL,
    oracle_id = "36937483-30cb-449a-8028-75017a124922",
    scryfall_id = "0180d9a8-992c-4d55-8ac4-33a587786993",
    faces = &[face!(
        name = "Boseiju, Who Shelters All",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(TapSelf, PayLife(2)),
        &[Effect::mana(ManaColor::Colorless, 1)
            .restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::Uncounterable)],
    )],
);
