//! Royal Herbalist — {W} — Creature — Human Cleric
//! Oracle: {2}, Exile the top card of your library: You gain 1 life.
//! Set: ALL #15a — Alliances | Scryfall ID: 027e03e1-1a39-47ba-b206-44d022b4c346 | Oracle ID: b3baf498-3be7-4a22-8fed-806d8d8ac748
// NOT SUPPORTED: exiling the top card of your library as part of an activation cost has no CostPart representation.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROYAL_HERBALIST,
    oracle_id = "b3baf498-3be7-4a22-8fed-806d8d8ac748",
    scryfall_id = "027e03e1-1a39-47ba-b206-44d022b4c346",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Exiling the top card of your library as part of an activation cost has no CostPart representation; in play, Royal Herbalist's '{2}: You gain 1 life' ability activates without exiling the top card of your library.",
    ),
    faces = &[face!(
        name = "Royal Herbalist",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(cost!("{2}"), &[Effect::gain_life(1)])],
);
