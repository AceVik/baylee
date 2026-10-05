//! Glyph of Delusion — {U} — Instant
//! Oracle: Put X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains "This creature doesn't untap during your untap step if it has a glyph counter on it" and "At the beginning of your upkeep, remove a glyph counter from this creature."
//! Set: LEG #60 — Legends | Scryfall ID: ee39da13-4b8a-4796-a7c2-aaa11992d573 | Oracle ID: e23ba169-4eb5-4083-a199-ac7e3026e67f
// PARTIAL — nothing is built: the target pair, the counter and the grants
// cannot be stated.
// NOT SUPPORTED: "Put X glyph counters on target creature that target Wall
// blocked this turn, where X is the power of that blocked creature." — no
// filter selects a creature a chosen Wall blocked this turn (there is no
// blocked-this-turn history and no target spec for "blocked by the first
// target"), and a glyph counter is not an assigned `counters::` id.
// NOT SUPPORTED: "The creature gains \"This creature doesn't untap during your
// untap step if it has a glyph counter on it\" and \"At the beginning of your
// upkeep, remove a glyph counter from this creature.\"" — the untap
// restriction has no keyword bit and `Modifier::GrantStatic` carries no
// counter condition; the upkeep removal would be `Modifier::GrantTriggered`
// with `Effect::RemoveCounterSelf`, but it is moot without a glyph counter id.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GLYPH_OF_DELUSION,
    oracle_id = "e23ba169-4eb5-4083-a199-ac7e3026e67f",
    scryfall_id = "ee39da13-4b8a-4796-a7c2-aaa11992d573",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Glyph of Delusion",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "the target pair (a Wall that blocked this turn and the creature it \
         blocked) is not selectable, glyph counters have no assigned counter \
         id, and the two granted abilities cannot be granted"
    ),
);
