//! Mind Twist — {X}{B} — Sorcery
//! Oracle: Target player discards X cards at random.
//! Set: ME3 #72 — Masters Edition III | Scryfall ID: 9763ea41-55c4-4b0a-9dc2-91ad4938b343 | Oracle ID: 78f9c223-9982-4282-a496-a6f892f0a5bf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_TWIST,
    oracle_id = "78f9c223-9982-4282-a496-a6f892f0a5bf",
    scryfall_id = "9763ea41-55c4-4b0a-9dc2-91ad4938b343",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mind Twist",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
