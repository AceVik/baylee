//! Ghastly Remains — {B}{B}{B} — Creature — Zombie
//! Oracle: Amplify 1 (As this creature enters, put a +1/+1 counter on it for each Zombie card you reveal in your hand.)
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard, you may pay {B}{B}{B}. If you do, return it to your hand.
//! Set: LGN #71 — Legions | Scryfall ID: 63e67323-df54-4043-a6b6-18bb89ef1f62 | Oracle ID: 0eafe734-19f2-492d-bfc2-d0c75ece45b2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GHASTLY_REMAINS,
    oracle_id = "0eafe734-19f2-492d-bfc2-d0c75ece45b2",
    scryfall_id = "63e67323-df54-4043-a6b6-18bb89ef1f62",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ghastly Remains",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
