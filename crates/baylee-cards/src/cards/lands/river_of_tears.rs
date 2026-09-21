//! River of Tears — (no cost) — Land
//! Oracle: {T}: Add {U}. If you played a land this turn, add {B} instead.
//! Set: MKC #283 — Murders at Karlov Manor Commander | Scryfall ID: 67b626cc-1c12-4059-afa5-e5a1221ea1ba | Oracle ID: 8a83d284-75a0-4901-b7d9-c4b7586ee327
// PARTIAL — the {T} ability adds {U}; the "{B} instead" half is not
// expressible, so the ability is written unconditionally and labelled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVER_OF_TEARS,
    oracle_id = "8a83d284-75a0-4901-b7d9-c4b7586ee327",
    scryfall_id = "67b626cc-1c12-4059-afa5-e5a1221ea1ba",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "\"If you played a land this turn, add {B} instead\" is not expressible: no Condition \
         names a land played this turn and ManaSource has no branch that reads one, so the \
         {T} ability always adds {U}"
    ),
    faces = &[face!(name = "River of Tears", types = TypeSet::LAND,)],
    // NOT SUPPORTED: "If you played a land this turn, add {B} instead." —
    // Condition's five sentences count permanents, counters or the source, and
    // none of them asks what was played this turn; ManaSource likewise has no
    // branch that depends on it, so the printed replacement cannot be said.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
