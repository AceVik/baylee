//! Guiding Spirit — {1}{W}{U} — Creature — Angel Spirit
//! Oracle: Flying
//! Oracle: {T}: If the top card of target player's graveyard is a creature card, put that card on top of that player's library.
//! Set: VIS #131 — Visions | Scryfall ID: 5f96d184-0ef8-40f7-98bc-bd4c53c57072 | Oracle ID: c474b52f-2f50-416e-b6c2-07043c7b339d
// PARTIAL — Inspecting the top card of a graveyard by graveyard order ('If the top card of target player's graveyard is a creature card, put that card on top of that player's library') is not yet supported in Effect; in play, Guiding Spirit has flying.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GUIDING_SPIRIT,
    oracle_id = "c474b52f-2f50-416e-b6c2-07043c7b339d",
    scryfall_id = "5f96d184-0ef8-40f7-98bc-bd4c53c57072",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Inspecting the top card of a graveyard by graveyard order ('If the top card of target player's graveyard is a creature card, put that card on top of that player's library') is not yet supported in Effect; in play, Guiding Spirit has flying.",
    ),
    faces = &[face!(
        name = "Guiding Spirit",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL, subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(2),
    ),],
);
