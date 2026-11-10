//! Krosan Vorine — {3}{G} — Creature — Cat Beast
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Oracle: This creature can't be blocked by more than one creature.
//! Set: VMA #219 — Vintage Masters | Scryfall ID: 31bd1b2f-6bea-4bd1-8a6e-50ab2d34bf82 | Oracle ID: ddb95c3a-b383-4987-ba24-72e95f8e33bf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_VORINE,
    oracle_id = "ddb95c3a-b383-4987-ba24-72e95f8e33bf",
    scryfall_id = "31bd1b2f-6bea-4bd1-8a6e-50ab2d34bf82",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Vorine",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
