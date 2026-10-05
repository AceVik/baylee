//! Urza's Avenger — {6} — Artifact Creature — Shapeshifter
//! Oracle: {0}: This creature gets -1/-1 and gains your choice of banding, flying, first strike, or trample until end of turn. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: 5ED #405 — Fifth Edition | Scryfall ID: 60bd9559-1a8f-47d0-af6b-d0681cae4060 | Oracle ID: 1cfeb5c4-a2d0-4872-a907-a6d484be85a5
// PARTIAL — the ability is off the card; nothing offers a choice among keywords.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_AVENGER,
    oracle_id = "1cfeb5c4-a2d0-4872-a907-a6d484be85a5",
    scryfall_id = "60bd9559-1a8f-47d0-af6b-d0681cae4060",
    coverage = Coverage::Partial(
        "no effect offers a choice among keywords, so the -1/-1 plus one of \
         banding, flying, first strike or trample cannot be written"
    ),
    faces = &[face!(
        name = "Urza's Avenger",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(4),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "{0}: This creature gets -1/-1 and gains your choice of
    // banding, flying, first strike, or trample until end of turn." —
    // `Effect::PumpTarget` takes one fixed `KeywordSet`, and nothing in the
    // DSL asks the player to pick a single keyword out of a menu (the
    // corpus's `KWChoice` has no reader). Granting all four would be a
    // different card.
    abilities = &[],
);
