//! Wormfang Crab — {3}{U} — Creature — Nightmare Crab
//! Oracle: This creature can't be blocked.
//! Oracle: When this creature enters, an opponent chooses a permanent you control other than this creature and exiles it.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: JUD #56 — Judgment | Scryfall ID: dcf56dcf-ec1a-4298-8644-1fe248443b7e | Oracle ID: 4216f9ce-bdd9-4213-8c3f-e2ef4cd848f9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORMFANG_CRAB,
    oracle_id = "4216f9ce-bdd9-4213-8c3f-e2ef4cd848f9",
    scryfall_id = "dcf56dcf-ec1a-4298-8644-1fe248443b7e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wormfang Crab",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::CRAB],
        power = Some(3),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
