//! Teroh's Vanguard — {3}{W} — Creature — Human Nomad
//! Oracle: Flash
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature enters, creatures you control gain protection from black until end of turn."
//! Set: TOR #19 — Torment | Scryfall ID: 1a39f909-7114-4482-8302-b2a56ca9f556 | Oracle ID: 32b0fc43-b5ce-4869-a780-946289305e5a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TEROH_S_VANGUARD,
    oracle_id = "32b0fc43-b5ce-4869-a780-946289305e5a",
    scryfall_id = "1a39f909-7114-4482-8302-b2a56ca9f556",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Teroh's Vanguard",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
