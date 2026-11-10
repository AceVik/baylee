//! Jilt — {1}{U} — Instant
//! Oracle: Kicker {1}{R} (You may pay an additional {1}{R} as you cast this spell.)
//! Oracle: Return target creature to its owner's hand. If this spell was kicked, it deals 2 damage to another target creature.
//! Set: APC #27 — Apocalypse | Scryfall ID: a010d2b1-960d-4032-a47a-61fe0998bee3 | Oracle ID: 382bc554-d170-4c19-82dd-829f254e855e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JILT,
    oracle_id = "382bc554-d170-4c19-82dd-829f254e855e",
    scryfall_id = "a010d2b1-960d-4032-a47a-61fe0998bee3",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Jilt",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
