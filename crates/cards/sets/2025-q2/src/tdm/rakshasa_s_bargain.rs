//! Rakshasa's Bargain — {2/B}{2/G}{2/U} — Instant
//! Oracle: Look at the top four cards of your library. Put two of them into your hand and the rest into your graveyard.
//! Set: TDM #214 — Tarkir: Dragonstorm | Scryfall ID: 5c409f4f-3b2c-4c33-b850-55b2a46f51ca | Oracle ID: d87be8c6-7a1f-4db8-ada1-b1a030f03877
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAKSHASA_S_BARGAIN,
    oracle_id = "d87be8c6-7a1f-4db8-ada1-b1a030f03877",
    scryfall_id = "5c409f4f-3b2c-4c33-b850-55b2a46f51ca",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Rakshasa's Bargain",
        mana_cost = mana!("{2/B}{2/G}{2/U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
