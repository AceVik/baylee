//! Primal Beyond — (no cost) — Land
//! Oracle: As this land enters, you may reveal an Elemental card from your hand. If you don't, this land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Elemental spell or activate an ability of an Elemental.
//! Set: ECC #159 — Lorwyn Eclipsed Commander | Scryfall ID: cf01949c-1aa3-4ce6-aabf-00c6d0498f5c | Oracle ID: 541744d9-449d-420a-a5a1-2fffba18450f
// PARTIAL — the entry clause and {C} are whole; the second ability makes the
// mana but understates what it may be spent on.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

// "An Elemental" — read twice, and the same word both times: which card in
// hand answers the entry clause, and what the second ability's mana may be
// spent on. One `static` rather than two, because a card that says the
// creature type once means it once.
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
    coverage = Coverage::Partial("the mana restriction names spells only, not activations"),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or activate an ability of an Elemental" — a
        // `ManaRestriction` names the spells its mana may be spent on, and an
        // activation is not a spell.
        mana_ability!(&[Effect::mana_of_any_color().restricted(&ELEMENTAL, SpendRider::None)]),
    ],
);
