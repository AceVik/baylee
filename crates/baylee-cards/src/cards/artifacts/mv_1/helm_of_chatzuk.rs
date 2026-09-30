//! Helm of Chatzuk — {1} — Artifact
//! Oracle: {1}, {T}: Target creature gains banding until end of turn. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding a player controls are blocking or being blocked by a creature, that player divides that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
//! Set: 5ED #376 — Fifth Edition | Scryfall ID: 9664a59d-182a-429b-a6f7-047b8b8e5ffe | Oracle ID: 948e3bb7-8265-4e95-acd1-a0c4f22441df
// PARTIAL — banding (CR 702.22) is not in the engine.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELM_OF_CHATZUK,
    oracle_id = "948e3bb7-8265-4e95-acd1-a0c4f22441df",
    scryfall_id = "9664a59d-182a-429b-a6f7-047b8b8e5ffe",
    coverage = Coverage::Partial("banding (CR 702.22) is not in the engine"),
    faces = &[face!(
        name = "Helm of Chatzuk",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    // NOT SUPPORTED: {1}, {T}: Target creature gains banding until end of turn.
);
