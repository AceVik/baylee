//! Cinder Seer — {3}{R} — Creature — Human Wizard
//! Oracle: {2}{R}, {T}: Reveal any number of red cards in your hand. This creature deals X damage to any target, where X is the number of cards revealed this way.
//! Set: UDS #78 — Urza's Destiny | Scryfall ID: d96e7522-e0bc-4e23-8e4b-40a0c28ea986 | Oracle ID: 64b63797-9db0-41fa-b2eb-fbf51de0e337
// PARTIAL — revealing any number of red hand cards and counting only the revealed cards is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CINDER_SEER,
    oracle_id = "64b63797-9db0-41fa-b2eb-fbf51de0e337",
    scryfall_id = "d96e7522-e0bc-4e23-8e4b-40a0c28ea986",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "revealing any number of red hand cards and counting only the revealed cards is unsupported"
    ),
    faces = &[face!(
        name = "Cinder Seer",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: revealing any number of red hand cards and counting only the revealed cards is unsupported.
);
