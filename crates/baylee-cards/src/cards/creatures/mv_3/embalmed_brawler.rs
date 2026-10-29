//! Embalmed Brawler — {2}{B} — Creature — Zombie
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Zombie card you reveal in your hand.)
//! Oracle: Whenever this creature attacks or blocks, you lose 1 life for each +1/+1 counter on it.
//! Set: LGN #69 — Legions | Scryfall ID: 2e214da0-68c0-4cf6-ba12-e2b2394909c1 | Oracle ID: 42e95ad5-c10a-46fe-ab97-c7a734138586
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBALMED_BRAWLER,
    oracle_id = "42e95ad5-c10a-46fe-ab97-c7a734138586",
    scryfall_id = "2e214da0-68c0-4cf6-ba12-e2b2394909c1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Embalmed Brawler",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
