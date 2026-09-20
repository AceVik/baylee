//! Thespian's Stage — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: This land becomes a copy of target land, except it has this ability.
//! Set: 2XM #327 — Double Masters | Scryfall ID: 269a926d-7788-4668-8bd8-7572dbf5f5eb | Oracle ID: b01e698b-608a-4fc7-8073-b01d044743ec
// PARTIAL — {T}: Add {C} is built; the {2}, {T} copy ability has no DSL shape.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THESPIAN_S_STAGE,
    oracle_id = "b01e698b-608a-4fc7-8073-b01d044743ec",
    scryfall_id = "269a926d-7788-4668-8bd8-7572dbf5f5eb",
    faces = &[face!(name = "Thespian's Stage", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{2}, {T}: This land becomes a copy of target land — no DSL variant copies a target permanent"
    ),
    abilities = &[
        // NOT SUPPORTED: {2}, {T}: This land becomes a copy of target land, except it has this ability —
        // the only copy machinery in the vocabulary is `AbilityDef::CopyOnEnter` /
        // `AbilityDef::CopyOnEnterUntilEot` (a choice made *as the permanent enters*, not an
        // activation), `Effect::CreateTokenCopyOf` (a token, not the source becoming a copy) and
        // `Modifier::BecomeCopyOf(ObjectId)`, which takes a literal object id that no card file can
        // name for a target chosen when the ability resolves.
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    ],
);
