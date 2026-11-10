//! Wall of Nets — {1}{W}{W} — Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: At end of combat, exile all creatures blocked by this creature.
//! Oracle: When this creature leaves the battlefield, return all cards exiled with it to the battlefield under their owners' control.
//! Set: EXO #24 — Exodus | Scryfall ID: c1da8e79-365d-4a36-87c5-648085828f9f | Oracle ID: cd112962-b297-403c-bede-c8b587f379b3
// PARTIAL — Exiling blocked creatures with a link for their return when this creature leaves is unsupported; both exile and return triggers are omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Exiling blocked creatures with a link for their return when this creature leaves is unsupported; both exile and return triggers are omitted.
card!(
    index = index::WALL_OF_NETS,
    oracle_id = "cd112962-b297-403c-bede-c8b587f379b3",
    scryfall_id = "c1da8e79-365d-4a36-87c5-648085828f9f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Exiling blocked creatures with a link for their return when this creature leaves is unsupported; both exile and return triggers are omitted."
    ),
    faces = &[face!(
        name = "Wall of Nets",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(7),
        keywords = KeywordSet::DEFENDER,
    ),],
);
