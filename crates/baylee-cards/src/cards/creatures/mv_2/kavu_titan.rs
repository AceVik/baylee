//! Kavu Titan — {1}{G} — Creature — Kavu
//! Oracle: Kicker {2}{G} (You may pay an additional {2}{G} as you cast this spell.)
//! Oracle: If this creature was kicked, it enters with three +1/+1 counters on it and with trample.
//! Set: INV #194 — Invasion | Scryfall ID: 2c5fb86d-1d9a-4da2-bb5b-4266faa20197 | Oracle ID: df166d0e-f944-404d-83b9-8cf53e220774
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KAVU_TITAN,
    oracle_id = "df166d0e-f944-404d-83b9-8cf53e220774",
    scryfall_id = "2c5fb86d-1d9a-4da2-bb5b-4266faa20197",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Kavu Titan",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
