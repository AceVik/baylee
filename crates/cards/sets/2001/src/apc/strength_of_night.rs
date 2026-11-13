//! Strength of Night — {2}{G} — Instant
//! Oracle: Kicker {B} (You may pay an additional {B} as you cast this spell.)
//! Oracle: Creatures you control get +1/+1 until end of turn. If this spell was kicked, Zombie creatures you control get an additional +2/+2 until end of turn.
//! Set: APC #86 — Apocalypse | Scryfall ID: 87aab031-4e44-44cd-89a7-6cffc7288cd1 | Oracle ID: 0045cf16-86dd-4417-ae36-88ca63b30c26
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STRENGTH_OF_NIGHT,
    oracle_id = "0045cf16-86dd-4417-ae36-88ca63b30c26",
    scryfall_id = "87aab031-4e44-44cd-89a7-6cffc7288cd1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Strength of Night",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
