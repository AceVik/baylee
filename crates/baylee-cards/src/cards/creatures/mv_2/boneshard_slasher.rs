//! Boneshard Slasher — {1}{B} — Creature — Horror
//! Oracle: Flying
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature gets +2/+2 and has "When this creature becomes the target of a spell or ability, sacrifice it."
//! Set: TOR #50 — Torment | Scryfall ID: 27201370-32cc-4b90-890d-8c3f5362ad70 | Oracle ID: 1098ff82-8cad-4283-8493-5492f12ea200
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BONESHARD_SLASHER,
    oracle_id = "1098ff82-8cad-4283-8493-5492f12ea200",
    scryfall_id = "27201370-32cc-4b90-890d-8c3f5362ad70",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Boneshard Slasher",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
