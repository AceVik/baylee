//! Avatar of Will — {6}{U}{U} — Creature — Avatar
//! Oracle: If an opponent has no cards in hand, this spell costs {6} less to cast.
//! Oracle: Flying
//! Set: PCY #30 — Prophecy | Scryfall ID: bf65efc7-6ab5-4116-b003-1f028af80939 | Oracle ID: 13aa88a0-4b47-4012-ba50-002e51e637cf
// PARTIAL — 5/6 flying.
// NOT SUPPORTED: conditional {6} self-cost reduction when an opponent has no cards in hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVATAR_OF_WILL,
    oracle_id = "13aa88a0-4b47-4012-ba50-002e51e637cf",
    scryfall_id = "bf65efc7-6ab5-4116-b003-1f028af80939",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "self-cost reduction when an opponent has no cards in hand is not supported",
    ),
    faces = &[face!(
        name = "Avatar of Will",
        mana_cost = mana!("{6}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(5),
        toughness = Some(6),
    ),],
);
