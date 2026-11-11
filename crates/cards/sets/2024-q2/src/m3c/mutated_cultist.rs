//! Mutated Cultist — {2}{B} — Creature — Eldrazi Horror
//! Oracle: Devoid (This card has no color.)
//! Oracle: When you cast this spell, remove all counters from up to one target permanent or opponent. The next spell you cast this turn costs {1} less to cast for each counter removed this way.
//! Oracle: Deathtouch
//! Set: M3C #53 — Modern Horizons 3 Commander | Scryfall ID: cc4bb871-cde9-43a9-8089-5987fa0dd4ef | Oracle ID: 5168be9b-ffca-41c5-a944-92a1a2e30779
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MUTATED_CULTIST,
    oracle_id = "5168be9b-ffca-41c5-a944-92a1a2e30779",
    scryfall_id = "cc4bb871-cde9-43a9-8089-5987fa0dd4ef",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mutated Cultist",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELDRAZI, subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
