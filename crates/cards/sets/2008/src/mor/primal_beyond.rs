//! Primal Beyond — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Elemental spell or activate an ability of an Elemental.
//! Set: ECC #159 — Lorwyn Eclipsed Commander | Scryfall ID: cf01949c-1aa3-4ce6-aabf-00c6d0498f5c | Oracle ID: 541744d9-449d-420a-a5a1-2fffba18450f

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ELEMENTAL: Filter = Filter::HasSubtype(creature::ELEMENTAL);

card!(
    index = index::PRIMAL_BEYOND,
    oracle_id = "541744d9-449d-420a-a5a1-2fffba18450f",
    scryfall_id = "cf01949c-1aa3-4ce6-aabf-00c6d0498f5c",
    faces = &[face!(
        name = "Primal Beyond",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnlessReveal(&ELEMENTAL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &ELEMENTAL,
            SpendRider::None,
            ManaUse::SpellsOrAbilities
        )]),
    ],
);
