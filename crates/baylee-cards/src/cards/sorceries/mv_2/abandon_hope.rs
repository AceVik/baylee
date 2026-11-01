//! Abandon Hope — {X}{1}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Look at target opponent's hand and choose X cards from it. That player discards those cards.
//! Set: TMP #107 — Tempest | Scryfall ID: 942cf220-472c-48f6-8f60-993939ea5ab8 | Oracle ID: 8adbba6e-03ef-4278-aec5-8a4496b377a8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABANDON_HOPE,
    oracle_id = "8adbba6e-03ef-4278-aec5-8a4496b377a8",
    scryfall_id = "942cf220-472c-48f6-8f60-993939ea5ab8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Abandon Hope",
        mana_cost = mana!("{X}{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
