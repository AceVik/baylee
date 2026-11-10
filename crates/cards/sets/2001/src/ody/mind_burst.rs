//! Mind Burst — {1}{B} — Sorcery
//! Oracle: Target player discards X cards, where X is one plus the number of cards named Mind Burst in all graveyards.
//! Set: ODY #148 — Odyssey | Scryfall ID: c07889a1-1582-4f14-8925-fa21a1a5fd65 | Oracle ID: efc32c4a-ad84-400d-b79f-5861a3e4fa2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_BURST,
    oracle_id = "efc32c4a-ad84-400d-b79f-5861a3e4fa2f",
    scryfall_id = "c07889a1-1582-4f14-8925-fa21a1a5fd65",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mind Burst",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
