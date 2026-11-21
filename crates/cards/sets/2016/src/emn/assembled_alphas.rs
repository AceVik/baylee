//! Assembled Alphas — {5}{R} — Creature — Wolf
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, this creature deals 3 damage to that creature and 3 damage to that creature's controller.
//! Set: EMN #117 — Eldritch Moon | Scryfall ID: 8603c493-afb0-47a1-b4e3-9848b3824840 | Oracle ID: 151a681d-a350-4d1d-86cd-b80d58602810
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASSEMBLED_ALPHAS,
    oracle_id = "151a681d-a350-4d1d-86cd-b80d58602810",
    scryfall_id = "8603c493-afb0-47a1-b4e3-9848b3824840",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Assembled Alphas",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLF],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
