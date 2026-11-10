//! Unclaimed Territory — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type.
//! Set: MSC #275 — Marvel Super Heroes Commander | Scryfall ID: d3782952-3839-4a94-95bc-716611d3ece6 | Oracle ID: 584b15f2-6ae9-413a-8b8d-9244dbea4878
// IMPLEMENTED — choose-a-type, {C}, and any-color mana restricted to creature spells of the chosen type.

use baylee_cards_dsl::prelude::*;

static CHOSEN_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

card!(
    index = index::UNCLAIMED_TERRITORY,
    oracle_id = "584b15f2-6ae9-413a-8b8d-9244dbea4878",
    scryfall_id = "d3782952-3839-4a94-95bc-716611d3ece6",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unclaimed Territory",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_choice(ALL_MANA_COLORS)
            .restricted(&CHOSEN_TYPE_CREATURE_SPELL, SpendRider::None)]),
    ],
);
