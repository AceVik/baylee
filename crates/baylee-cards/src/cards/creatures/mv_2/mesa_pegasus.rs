//! Mesa Pegasus — {1}{W} — Creature — Pegasus
//! Oracle: Flying; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: ME1 #20 — Masters Edition | Scryfall ID: 8075e43a-7d36-4e07-a2ff-41e98e6c7778 | Oracle ID: 8161f5b8-6aab-4133-ba2c-2e7b5774153e
// PARTIAL — banding (CR 702.22) is not in the engine; a 1/1 flier.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MESA_PEGASUS,
    oracle_id = "8161f5b8-6aab-4133-ba2c-2e7b5774153e",
    scryfall_id = "8075e43a-7d36-4e07-a2ff-41e98e6c7778",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial("banding (CR 702.22) is not in the engine; a 1/1 flier"),
    faces = &[face!(
        name = "Mesa Pegasus",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PEGASUS],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: Banding
);
