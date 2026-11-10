//! Reborn Hero — {2}{W} — Creature — Human Soldier
//! Oracle: Vigilance
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature dies, you may pay {W}{W}. If you do, return this card to the battlefield under your control."
//! Set: TOR #14 — Torment | Scryfall ID: b4acbc7f-c86e-4285-a66d-d7b03fe44377 | Oracle ID: 5562b4bd-1485-47d4-a087-d93dafc37272
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REBORN_HERO,
    oracle_id = "5562b4bd-1485-47d4-a087-d93dafc37272",
    scryfall_id = "b4acbc7f-c86e-4285-a66d-d7b03fe44377",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Reborn Hero",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
