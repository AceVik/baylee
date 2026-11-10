//! Gempalm Polluter — {5}{B} — Creature — Zombie
//! Oracle: Cycling {B}{B} ({B}{B}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have target player lose life equal to the number of Zombies on the battlefield.
//! Set: LGN #70 — Legions | Scryfall ID: 8e9943ac-9e3f-4ee0-b5fd-3b0fb17097d8 | Oracle ID: bd532f8d-789f-4ee1-a2f3-ae30e8282c76
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMPALM_POLLUTER,
    oracle_id = "bd532f8d-789f-4ee1-a2f3-ae30e8282c76",
    scryfall_id = "8e9943ac-9e3f-4ee0-b5fd-3b0fb17097d8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gempalm Polluter",
        mana_cost = mana!("{5}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
