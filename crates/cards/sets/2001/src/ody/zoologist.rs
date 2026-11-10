//! Zoologist — {3}{G} — Creature — Human Druid
//! Oracle: {3}{G}, {T}: Reveal the top card of your library. If it's a creature card, put it onto the battlefield. Otherwise, put it into your graveyard.
//! Set: ODY #285 — Odyssey | Scryfall ID: be645675-d263-42c4-9e71-f98cf4c2aab5 | Oracle ID: 878e4e3b-c546-4174-906e-6fe8e64d9d41
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOOLOGIST,
    oracle_id = "878e4e3b-c546-4174-906e-6fe8e64d9d41",
    scryfall_id = "be645675-d263-42c4-9e71-f98cf4c2aab5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Zoologist",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
