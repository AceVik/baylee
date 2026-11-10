//! Eclipsed Realms — (no cost) — Land
//! Oracle: As this land enters, choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a spell of the chosen type or activate an ability of a source of the chosen type.
//! Set: ECL #263 — Lorwyn Eclipsed | Scryfall ID: a174f0db-8b4f-4c37-9583-44c92d37b9c0 | Oracle ID: 5715ed43-395c-4877-99a7-8e28e7bf9dce

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::ECLIPSED_REALMS,
    oracle_id = "5715ed43-395c-4877-99a7-8e28e7bf9dce",
    scryfall_id = "a174f0db-8b4f-4c37-9583-44c92d37b9c0",
    faces = &[face!(
        name = "Eclipsed Realms",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::ChooseSubtypeFrom(&[
            creature::ELEMENTAL,
            creature::ELF,
            creature::FAERIE,
            creature::GIANT,
            creature::GOBLIN,
            creature::KITHKIN,
            creature::MERFOLK,
            creature::TREEFOLK
        ])],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &Filter::MatchesChosenTypeOfSource,
            SpendRider::None,
            ManaUse::SpellsOrAbilities
        )]),
    ],
);
