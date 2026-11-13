//! Avatar of Might — {6}{G}{G} — Creature — Avatar
//! Oracle: If an opponent controls at least four more creatures than you, this spell costs {6} less to cast.
//! Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.)
//! Set: 10E #251 — Tenth Edition | Scryfall ID: ca570876-23b2-4915-a5b9-20f1fa976fab | Oracle ID: 68d84ffa-5bd0-4ed9-a987-6cef13095438
// PARTIAL — 8/8 trample.
// NOT SUPPORTED: conditional {6} self-cost reduction when an opponent controls at least four more creatures than you.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVATAR_OF_MIGHT,
    oracle_id = "68d84ffa-5bd0-4ed9-a987-6cef13095438",
    scryfall_id = "ca570876-23b2-4915-a5b9-20f1fa976fab",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "self-cost reduction when an opponent controls at least four more creatures is not supported",
    ),
    faces = &[face!(
        name = "Avatar of Might",
        mana_cost = mana!("{6}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(8),
        toughness = Some(8),
    ),],
);
