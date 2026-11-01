//! Sworn Defender — {2}{W}{W} — Creature — Human Knight
//! Oracle: {1}: This creature's power becomes the toughness of target creature blocking or being blocked by this creature minus 1 until end of turn, and its toughness becomes 1 plus the power of that creature until end of turn.
//! Set: ALL #19 — Alliances | Scryfall ID: 328e6ceb-30f7-415e-93b4-7075af0fed89 | Oracle ID: 02197d14-8d9f-4b1f-9ee0-ce8ef690843c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWORN_DEFENDER,
    oracle_id = "02197d14-8d9f-4b1f-9ee0-ce8ef690843c",
    scryfall_id = "328e6ceb-30f7-415e-93b4-7075af0fed89",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sworn Defender",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
