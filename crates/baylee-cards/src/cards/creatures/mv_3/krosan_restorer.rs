//! Krosan Restorer — {2}{G} — Creature — Human Druid
//! Oracle: {T}: Untap target land.
//! Oracle: Threshold — {T}: Untap up to three target lands. Activate only if there are seven or more cards in your graveyard.
//! Set: DMR #168 — Dominaria Remastered | Scryfall ID: 12508498-ecb7-4df9-b83f-87ef3bc2f4e2 | Oracle ID: ea845951-0158-4646-acd9-3b0b28da40b1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_RESTORER,
    oracle_id = "ea845951-0158-4646-acd9-3b0b28da40b1",
    scryfall_id = "12508498-ecb7-4df9-b83f-87ef3bc2f4e2",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Restorer",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
