//! Pincer Spider — {2}{G} — Creature — Spider
//! Oracle: Kicker {3} (You may pay an additional {3} as you cast this spell.)
//! Oracle: Reach (This creature can block creatures with flying.)
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it.
//! Set: INV #201 — Invasion | Scryfall ID: 23271658-19ae-420d-beeb-4bed4fdbb891 | Oracle ID: fbe25c32-db20-464c-a0ed-d90e75b111a9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PINCER_SPIDER,
    oracle_id = "fbe25c32-db20-464c-a0ed-d90e75b111a9",
    scryfall_id = "23271658-19ae-420d-beeb-4bed4fdbb891",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Pincer Spider",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIDER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
