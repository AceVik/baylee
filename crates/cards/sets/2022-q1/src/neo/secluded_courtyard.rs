//! Secluded Courtyard — (no cost) — Land
//! Oracle: As this land enters, choose a creature type.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the chosen type.
//! Set: MSC #265 — Marvel Super Heroes Commander | Scryfall ID: c0d17d04-cf0b-4918-bfec-b34b0d98a602 | Oracle ID: 79ba18fd-f184-43c1-86df-56ee18ce806c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

static CHOSEN_TYPE_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);

card!(
    index = index::SECLUDED_COURTYARD,
    oracle_id = "79ba18fd-f184-43c1-86df-56ee18ce806c",
    scryfall_id = "c0d17d04-cf0b-4918-bfec-b34b0d98a602",
    faces = &[face!(
        name = "Secluded Courtyard",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::ChooseSubtype],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &CHOSEN_TYPE_CREATURE_SPELL,
            SpendRider::None,
            ManaUse::SpellsOrAbilities
        )]),
    ],
);
