//! Vodalian Serpent — {3}{U} — Creature — Serpent
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: If this creature was kicked, it enters with four +1/+1 counters on it.
//! Set: INV #86 — Invasion | Scryfall ID: 92adcf6c-ab14-414c-a5cb-56feae048c84 | Oracle ID: c39c1604-3bae-454d-9985-85101e51ec6e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VODALIAN_SERPENT,
    oracle_id = "c39c1604-3bae-454d-9985-85101e51ec6e",
    scryfall_id = "92adcf6c-ab14-414c-a5cb-56feae048c84",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Vodalian Serpent",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SERPENT],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
