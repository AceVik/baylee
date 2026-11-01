//! Peer Pressure — {3}{U} — Sorcery
//! Oracle: Choose a creature type. If you control more creatures of that type than each other player, you gain control of all creatures of that type. (This effect lasts indefinitely.)
//! Set: ONS #101 — Onslaught | Scryfall ID: be0110ba-49e4-4729-8a84-4d408b20df53 | Oracle ID: bd6b02f9-277e-4f6a-b0a4-d893a939cf73
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PEER_PRESSURE,
    oracle_id = "bd6b02f9-277e-4f6a-b0a4-d893a939cf73",
    scryfall_id = "be0110ba-49e4-4729-8a84-4d408b20df53",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Peer Pressure",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
