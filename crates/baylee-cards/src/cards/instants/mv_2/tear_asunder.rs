//! Tear Asunder — {1}{G} — Instant
//! Oracle: Kicker {1}{B} (You may pay an additional {1}{B} as you cast this spell.)
//! Oracle: Exile target artifact or enchantment. If this spell was kicked, exile target nonland permanent instead.
//! Set: EOC #109 — Edge of Eternities Commander | Scryfall ID: e408c673-4a1f-45db-827a-75c501e1b3d6 | Oracle ID: 610af0f7-b5e3-43fb-9d02-7c59bd99034c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TEAR_ASUNDER,
    oracle_id = "610af0f7-b5e3-43fb-9d02-7c59bd99034c",
    scryfall_id = "e408c673-4a1f-45db-827a-75c501e1b3d6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Tear Asunder",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
