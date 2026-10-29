//! Shaman's Trance — {2}{R} — Instant
//! Oracle: Other players can't play lands or cast spells from their graveyards this turn. You may play lands and cast spells from other players' graveyards this turn as though those cards were in your graveyard.
//! Set: JUD #98 — Judgment | Scryfall ID: dfc33a4f-9ec2-4324-82a1-a4b9700572f2 | Oracle ID: 619d8d81-9e10-4353-b30d-a0f9c3451cd2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHAMAN_S_TRANCE,
    oracle_id = "619d8d81-9e10-4353-b30d-a0f9c3451cd2",
    scryfall_id = "dfc33a4f-9ec2-4324-82a1-a4b9700572f2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Shaman's Trance",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
