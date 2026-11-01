//! Captain's Maneuver — {X}{R}{W} — Instant
//! Oracle: The next X damage that would be dealt to target creature, planeswalker, or player this turn is dealt to another target creature, planeswalker, or player instead.
//! Set: HOP #85 — Planechase | Scryfall ID: 1ad535b8-7846-41a6-a981-fe97eaf588a6 | Oracle ID: a8b93d4d-bb67-4063-ac6d-7775be1b1f10
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAPTAIN_S_MANEUVER,
    oracle_id = "a8b93d4d-bb67-4063-ac6d-7775be1b1f10",
    scryfall_id = "1ad535b8-7846-41a6-a981-fe97eaf588a6",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Captain's Maneuver",
        mana_cost = mana!("{X}{R}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
