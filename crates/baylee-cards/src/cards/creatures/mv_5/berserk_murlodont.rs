//! Berserk Murlodont — {4}{G} — Creature — Beast
//! Oracle: Whenever a Beast becomes blocked, it gets +1/+1 until end of turn for each creature blocking it.
//! Set: LGN #117 — Legions | Scryfall ID: 499c4674-dd9f-4848-8447-721f842a0213 | Oracle ID: a762e8cb-9d33-41fc-b2e8-0c5ad4d7268e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BERSERK_MURLODONT,
    oracle_id = "a762e8cb-9d33-41fc-b2e8-0c5ad4d7268e",
    scryfall_id = "499c4674-dd9f-4848-8447-721f842a0213",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Berserk Murlodont",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
